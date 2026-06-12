use serde::Deserialize;
use serde_json::Value;

use crate::normalizers::trade::now_ms;

use super::types::{SpotExchange, SpotTrade, SpotTradeSide};

#[derive(Debug, Deserialize)]
pub struct BinanceSpotAggTrade {
    pub s: String,
    pub p: String,
    pub q: String,
    #[serde(rename = "T")]
    pub trade_time: Option<i64>,
    #[serde(rename = "E")]
    pub event_time: Option<i64>,
    pub m: bool,
    #[serde(default)]
    pub a: Option<serde_json::Value>,
    #[serde(default)]
    pub t: Option<serde_json::Value>,
}

pub fn normalize_binance_spot_trade(raw: BinanceSpotAggTrade) -> Option<SpotTrade> {
    let symbol = normalize_binance_symbol(&raw.s)?;
    let price = parse_positive(&raw.p)?;
    let qty_base = parse_positive(&raw.q)?;
    let side = if raw.m {
        SpotTradeSide::Sell
    } else {
        SpotTradeSide::Buy
    };
    let trade_id = raw.a.or(raw.t).map(|value| match value {
        Value::String(text) => text,
        other => other.to_string(),
    });
    Some(SpotTrade {
        ts: raw.trade_time.or(raw.event_time).unwrap_or_else(now_ms),
        exchange: SpotExchange::Binance,
        symbol,
        market: "spot".to_string(),
        price,
        qty_base,
        notional_usd: price * qty_base,
        side,
        trade_id,
    })
}

pub fn normalize_coinbase_market_trades_json(payload: &Value) -> Vec<SpotTrade> {
    payload
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|event| {
            event
                .get("trades")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(normalize_coinbase_trade_value)
        })
        .collect()
}

pub fn normalize_bitfinex_trade_value(symbol: &str, trade: &Value) -> Option<SpotTrade> {
    let values = trade.as_array()?;
    let trade_id = values.first()?.clone();
    let ts = values.get(1)?.as_i64()?;
    let amount = values.get(2)?.as_f64()?;
    let price = values.get(3)?.as_f64()?;
    let symbol = normalize_bitfinex_symbol(symbol)?;
    if !price.is_finite() || !amount.is_finite() || price <= 0.0 || amount == 0.0 {
        return None;
    }
    let qty_base = amount.abs();
    let side = if amount >= 0.0 {
        SpotTradeSide::Buy
    } else {
        SpotTradeSide::Sell
    };
    Some(SpotTrade {
        ts,
        exchange: SpotExchange::Bitfinex,
        symbol,
        market: "spot".to_string(),
        price,
        qty_base,
        notional_usd: price * qty_base,
        side,
        trade_id: Some(trade_id.to_string().trim_matches('"').to_string()),
    })
}

pub fn normalize_coinbase_trade_value(trade: &Value) -> Option<SpotTrade> {
    let product_id = trade.get("product_id")?.as_str()?;
    let symbol = normalize_coinbase_symbol(product_id)?;
    let price = parse_positive(trade.get("price")?.as_str()?)?;
    let qty_base = parse_positive(trade.get("size")?.as_str()?)?;
    let maker_side = trade.get("side")?.as_str()?.to_ascii_uppercase();
    let side = match maker_side.as_str() {
        "BUY" => SpotTradeSide::Sell,
        "SELL" => SpotTradeSide::Buy,
        _ => return None,
    };
    Some(SpotTrade {
        ts: trade
            .get("time")
            .and_then(Value::as_str)
            .and_then(parse_rfc3339_ms)
            .unwrap_or_else(now_ms),
        exchange: SpotExchange::Coinbase,
        symbol,
        market: "spot".to_string(),
        price,
        qty_base,
        notional_usd: price * qty_base,
        side,
        trade_id: trade
            .get("trade_id")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn normalize_binance_symbol(symbol: &str) -> Option<String> {
    match symbol.to_ascii_uppercase().as_str() {
        "BTCUSDT" => Some("BTC".to_string()),
        "ETHUSDT" => Some("ETH".to_string()),
        _ => None,
    }
}

fn normalize_coinbase_symbol(product_id: &str) -> Option<String> {
    match product_id.to_ascii_uppercase().as_str() {
        "BTC-USD" => Some("BTC".to_string()),
        "ETH-USD" => Some("ETH".to_string()),
        _ => None,
    }
}

fn normalize_bitfinex_symbol(symbol: &str) -> Option<String> {
    match symbol.to_ascii_uppercase().as_str() {
        "TBTCUSD" | "BTCUSD" => Some("BTC".to_string()),
        "TETHUSD" | "ETHUSD" => Some("ETH".to_string()),
        _ => None,
    }
}

fn parse_positive(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite() && *number > 0.0)
}

fn parse_rfc3339_ms(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|ts| ts.timestamp_millis())
}
