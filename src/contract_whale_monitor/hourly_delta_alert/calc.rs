use serde::Deserialize;
use serde_json::Value;

use super::types::{
    ClosedHourlyKline, HourlyDeltaDataStatus, HourlyDeltaDirection, HourlyDeltaResult,
};

#[derive(Debug, Deserialize)]
struct BinanceWsEnvelope {
    #[serde(default)]
    stream: Option<String>,
    #[serde(default)]
    data: Option<Value>,
    #[serde(rename = "e")]
    #[serde(default)]
    event_type: Option<String>,
    #[serde(rename = "k")]
    #[serde(default)]
    kline: Option<BinanceKlinePayload>,
    #[serde(rename = "s")]
    #[serde(default)]
    symbol: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BinanceKlinePayload {
    #[serde(rename = "t")]
    open_time_ms: i64,
    #[serde(rename = "T")]
    close_time_ms: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "i")]
    interval: String,
    #[serde(rename = "v")]
    volume: String,
    #[serde(rename = "V")]
    taker_buy_volume: String,
    #[serde(rename = "x")]
    is_closed: bool,
}

pub fn compute_hourly_delta(
    kline: &ClosedHourlyKline,
    threshold_btc: f64,
) -> Option<HourlyDeltaResult> {
    if !kline.is_closed {
        return None;
    }
    if !kline.volume_btc.is_finite()
        || !kline.taker_buy_btc.is_finite()
        || kline.volume_btc < 0.0
        || kline.taker_buy_btc < 0.0
        || kline.taker_buy_btc > kline.volume_btc + 1e-9
    {
        return None;
    }

    let taker_sell_btc = kline.taker_sell_btc();
    let delta_btc = (2.0 * kline.taker_buy_btc) - kline.volume_btc;
    let direction = if delta_btc > 0.0 {
        HourlyDeltaDirection::NetBuy
    } else if delta_btc < 0.0 {
        HourlyDeltaDirection::NetSell
    } else {
        HourlyDeltaDirection::Flat
    };
    let threshold = threshold_btc.max(0.0);
    let above_threshold = delta_btc.abs() > threshold;

    Some(HourlyDeltaResult {
        record_key: kline.record_key(),
        exchange: kline.exchange.clone(),
        symbol: kline.symbol.clone(),
        interval: kline.interval.clone(),
        kline_open_time_ms: kline.open_time_ms,
        kline_close_time_ms: kline.close_time_ms,
        taker_buy_btc: kline.taker_buy_btc,
        taker_sell_btc,
        delta_btc,
        volume_btc: kline.volume_btc,
        direction,
        above_threshold,
        threshold_btc: threshold,
        data_status: HourlyDeltaDataStatus::Closed,
    })
}

pub fn should_alert(delta_btc: f64, threshold_btc: f64) -> bool {
    delta_btc.is_finite() && threshold_btc.is_finite() && delta_btc.abs() > threshold_btc.max(0.0)
}

pub fn parse_binance_kline_ws_message(
    text: &str,
    exchange: &str,
) -> anyhow::Result<Option<ClosedHourlyKline>> {
    let envelope: BinanceWsEnvelope = serde_json::from_str(text)?;
    let payload = if let Some(kline) = envelope.kline {
        kline
    } else if let Some(data) = envelope.data {
        let nested: BinanceWsEnvelope = serde_json::from_value(data)?;
        match nested.kline {
            Some(kline) => kline,
            None => return Ok(None),
        }
    } else {
        return Ok(None);
    };

    if let Some(event_type) = envelope.event_type.as_deref() {
        if !event_type.eq_ignore_ascii_case("kline") {
            return Ok(None);
        }
    }
    if let Some(stream) = envelope.stream.as_deref() {
        if !stream.to_ascii_lowercase().contains("kline") {
            return Ok(None);
        }
    }

    let volume_btc = parse_f64(&payload.volume)?;
    let taker_buy_btc = parse_f64(&payload.taker_buy_volume)?;
    let symbol = envelope
        .symbol
        .as_deref()
        .unwrap_or(payload.symbol.as_str())
        .to_ascii_uppercase();

    Ok(Some(ClosedHourlyKline {
        exchange: exchange.to_ascii_lowercase(),
        symbol,
        interval: payload.interval.to_ascii_lowercase(),
        open_time_ms: payload.open_time_ms,
        close_time_ms: payload.close_time_ms,
        volume_btc,
        taker_buy_btc,
        is_closed: payload.is_closed,
    }))
}

/// Binance REST kline array:
/// [
///   open_time, open, high, low, close, volume, close_time, ...,
///   taker_buy_base_asset_volume, ...
/// ]
pub fn parse_binance_rest_kline_row(
    row: &Value,
    exchange: &str,
    symbol: &str,
    interval: &str,
) -> anyhow::Result<ClosedHourlyKline> {
    let arr = row
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("kline row is not an array"))?;
    if arr.len() < 11 {
        anyhow::bail!("kline row too short: {}", arr.len());
    }
    let open_time_ms = json_i64(&arr[0])?;
    let close_time_ms = json_i64(&arr[6])?;
    let volume_btc = json_f64(&arr[5])?;
    let taker_buy_btc = json_f64(&arr[9])?;
    Ok(ClosedHourlyKline {
        exchange: exchange.to_ascii_lowercase(),
        symbol: symbol.to_ascii_uppercase(),
        interval: interval.to_ascii_lowercase(),
        open_time_ms,
        close_time_ms,
        volume_btc,
        taker_buy_btc,
        is_closed: true,
    })
}

pub fn parse_binance_rest_klines(
    body: &Value,
    exchange: &str,
    symbol: &str,
    interval: &str,
) -> anyhow::Result<Vec<ClosedHourlyKline>> {
    let rows = body
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("klines response is not an array"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(parse_binance_rest_kline_row(
            row, exchange, symbol, interval,
        )?);
    }
    Ok(out)
}

fn parse_f64(raw: &str) -> anyhow::Result<f64> {
    let value = raw
        .trim()
        .parse::<f64>()
        .map_err(|error| anyhow::anyhow!("invalid f64 '{raw}': {error}"))?;
    if !value.is_finite() {
        anyhow::bail!("non-finite f64 '{raw}'");
    }
    Ok(value)
}

fn json_i64(value: &Value) -> anyhow::Result<i64> {
    if let Some(v) = value.as_i64() {
        return Ok(v);
    }
    if let Some(v) = value.as_u64() {
        return Ok(v as i64);
    }
    if let Some(s) = value.as_str() {
        return s
            .parse::<i64>()
            .map_err(|error| anyhow::anyhow!("invalid i64 '{s}': {error}"));
    }
    anyhow::bail!("expected i64, got {value}")
}

fn json_f64(value: &Value) -> anyhow::Result<f64> {
    if let Some(v) = value.as_f64() {
        if v.is_finite() {
            return Ok(v);
        }
    }
    if let Some(s) = value.as_str() {
        return parse_f64(s);
    }
    anyhow::bail!("expected f64, got {value}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_kline(volume: f64, buy: f64, closed: bool) -> ClosedHourlyKline {
        ClosedHourlyKline {
            exchange: "binance".into(),
            symbol: "BTCUSDT".into(),
            interval: "1h".into(),
            open_time_ms: 1_700_000_000_000,
            close_time_ms: 1_700_003_599_999,
            volume_btc: volume,
            taker_buy_btc: buy,
            is_closed: closed,
        }
    }

    #[test]
    fn delta_formula_and_strict_threshold() {
        let r = compute_hourly_delta(&sample_kline(7_700.0, 2_450.0, true), 1000.0).unwrap();
        assert!((r.delta_btc - (-2_800.0)).abs() < 1e-9);
        assert_eq!(r.direction, HourlyDeltaDirection::NetSell);
        assert!(r.above_threshold);

        let buy = compute_hourly_delta(&sample_kline(5_000.0, 3_100.0, true), 1000.0).unwrap();
        assert!((buy.delta_btc - 1_200.0).abs() < 1e-9);
        assert!(buy.above_threshold);

        assert!(
            !compute_hourly_delta(&sample_kline(2_000.0, 1_499.995, true), 1000.0)
                .unwrap()
                .above_threshold
        );
        assert!(
            !compute_hourly_delta(&sample_kline(2_000.0, 1_500.0, true), 1000.0)
                .unwrap()
                .above_threshold
        );
        assert!(!should_alert(1000.0, 1000.0));
        assert!(!should_alert(-1000.0, 1000.0));
        assert!(should_alert(1000.01, 1000.0));
    }

    #[test]
    fn unclosed_kline_rejected() {
        assert!(compute_hourly_delta(&sample_kline(5_000.0, 3_000.0, false), 1000.0).is_none());
    }

    #[test]
    fn parse_ws_closed_and_open() {
        let closed = r#"{"e":"kline","s":"BTCUSDT","k":{"t":1700000000000,"T":1700003599999,"s":"BTCUSDT","i":"1h","v":"100.5","V":"40.2","x":true}}"#;
        let k = parse_binance_kline_ws_message(closed, "binance")
            .unwrap()
            .unwrap();
        assert!(k.is_closed);
        assert!((k.volume_btc - 100.5).abs() < 1e-9);
        assert!((k.taker_buy_btc - 40.2).abs() < 1e-9);

        let open = r#"{"e":"kline","s":"BTCUSDT","k":{"t":1,"T":2,"s":"BTCUSDT","i":"1h","v":"1","V":"0.5","x":false}}"#;
        let o = parse_binance_kline_ws_message(open, "binance")
            .unwrap()
            .unwrap();
        assert!(!o.is_closed);
    }
}
