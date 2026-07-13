use serde::Deserialize;

use crate::{
    normalizers::symbol::require_symbol,
    types::market::{AggressorSide, NormalizedTrade, Venue},
};

#[derive(Debug, Deserialize)]
pub struct BinanceAggTrade {
    pub s: String,
    #[serde(default)]
    pub a: Option<serde_json::Value>,
    #[serde(default)]
    pub t: Option<serde_json::Value>,
    pub p: String,
    pub q: String,
    #[serde(rename = "T")]
    pub trade_time: Option<i64>,
    #[serde(rename = "E")]
    pub event_time: Option<i64>,
    pub m: bool,
}

#[derive(Debug, Deserialize)]
pub struct BybitTrade {
    pub s: String,
    #[serde(rename = "T")]
    pub trade_time: Option<i64>,
    pub p: String,
    pub v: String,
    #[serde(rename = "S")]
    pub side: String,
    pub i: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OkxTrade {
    #[serde(rename = "instId")]
    pub inst_id: Option<String>,
    #[serde(rename = "tradeId")]
    pub trade_id: Option<String>,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub ts: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BitfinexTrade {
    pub symbol: String,
    pub trade_id: serde_json::Value,
    pub ts: i64,
    pub amount: f64,
    pub price: f64,
}

pub fn normalize_binance_agg_trade(raw: BinanceAggTrade) -> Option<NormalizedTrade> {
    build_trade(
        Venue::Binance,
        &raw.s,
        raw.trade_time.or(raw.event_time).unwrap_or_else(now_ms),
        raw.p.parse().ok()?,
        raw.q.parse().ok()?,
        if raw.m {
            AggressorSide::Sell
        } else {
            AggressorSide::Buy
        },
        raw.a
            .or(raw.t)
            .map(|v| v.to_string().trim_matches('"').to_string()),
    )
}

pub fn normalize_bybit_trade(raw: BybitTrade) -> Option<NormalizedTrade> {
    let side = match raw.side.as_str() {
        "Buy" => AggressorSide::Buy,
        "Sell" => AggressorSide::Sell,
        _ => return None,
    };
    build_trade(
        Venue::Bybit,
        &raw.s,
        raw.trade_time.unwrap_or_else(now_ms),
        raw.p.parse().ok()?,
        raw.v.parse().ok()?,
        side,
        raw.i,
    )
}

pub fn normalize_okx_trade(raw: OkxTrade) -> Option<NormalizedTrade> {
    let _ = raw;
    None
}

pub fn normalize_okx_trade_with_contract_value(
    raw: OkxTrade,
    ct_val_base: f64,
) -> Option<NormalizedTrade> {
    if !ct_val_base.is_finite() || ct_val_base <= 0.0 {
        return None;
    }
    let side = match raw.side.as_str() {
        "buy" => AggressorSide::Buy,
        "sell" => AggressorSide::Sell,
        _ => return None,
    };
    build_trade(
        Venue::Okx,
        raw.inst_id.as_deref().unwrap_or("BTC-USDT-SWAP"),
        raw.ts.and_then(|ts| ts.parse().ok()).unwrap_or_else(now_ms),
        raw.px.parse().ok()?,
        raw.sz.parse::<f64>().ok()? * ct_val_base,
        side,
        raw.trade_id,
    )
}

pub fn normalize_bitfinex_trade(raw: BitfinexTrade) -> Option<NormalizedTrade> {
    let side = if raw.amount >= 0.0 {
        AggressorSide::Buy
    } else {
        AggressorSide::Sell
    };
    build_trade(
        Venue::Bitfinex,
        &raw.symbol,
        raw.ts,
        raw.price,
        raw.amount.abs(),
        side,
        Some(raw.trade_id.to_string().trim_matches('"').to_string()),
    )
}

fn build_trade(
    venue: Venue,
    symbol: &str,
    ts: i64,
    price: f64,
    size_btc: f64,
    aggressor_side: AggressorSide,
    trade_id: Option<String>,
) -> Option<NormalizedTrade> {
    if !price.is_finite() || !size_btc.is_finite() || price <= 0.0 || size_btc <= 0.0 {
        return None;
    }
    Some(NormalizedTrade {
        venue,
        symbol: require_symbol(venue, symbol).ok()?,
        ts,
        price,
        size_btc,
        size_usd: price * size_btc,
        aggressor_side,
        trade_id,
    })
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
