use std::time::Duration;

use serde::Deserialize;

use super::{
    manager::normalize_symbol,
    types::{MarketPriceSnapshot, PriceSource},
};
use crate::normalizers::trade::now_ms;

const BINANCE_FAPI_TICKER_24H: &str = "https://fapi.binance.com/fapi/v1/ticker/24hr";
const BINANCE_FAPI_PREMIUM_INDEX: &str = "https://fapi.binance.com/fapi/v1/premiumIndex";
const BINANCE_SPOT_TICKER_24H: &str = "https://api.binance.com/api/v3/ticker/24hr";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceTicker24h {
    last_price: String,
    price_change_percent: Option<String>,
    quote_volume: Option<String>,
    high_price: Option<String>,
    low_price: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinancePremiumIndex {
    mark_price: String,
}

pub async fn fetch_market_price_snapshot(raw_symbol: &str) -> Option<MarketPriceSnapshot> {
    let symbol = normalize_symbol(raw_symbol).ok()?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1_800))
        .build()
        .ok()?;

    if let Some(snapshot) = fetch_ticker_24h(
        &client,
        BINANCE_FAPI_TICKER_24H,
        &symbol,
        PriceSource::MarketPerp,
    )
    .await
    {
        return Some(snapshot);
    }
    if let Some(snapshot) = fetch_mark_price(&client, &symbol).await {
        return Some(snapshot);
    }
    fetch_ticker_24h(
        &client,
        BINANCE_SPOT_TICKER_24H,
        &symbol,
        PriceSource::MarketSpot,
    )
    .await
}

async fn fetch_ticker_24h(
    client: &reqwest::Client,
    endpoint: &str,
    symbol: &str,
    source: PriceSource,
) -> Option<MarketPriceSnapshot> {
    let ticker = client
        .get(endpoint)
        .query(&[("symbol", symbol)])
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<BinanceTicker24h>()
        .await
        .ok()?;
    let price = parse_positive(&ticker.last_price)?;
    Some(MarketPriceSnapshot {
        price,
        source,
        updated_at_ms: now_ms(),
        change_24h_pct: ticker
            .price_change_percent
            .as_deref()
            .and_then(parse_finite),
        volume_24h_usd: ticker.quote_volume.as_deref().and_then(parse_finite),
        high_24h: ticker.high_price.as_deref().and_then(parse_positive),
        low_24h: ticker.low_price.as_deref().and_then(parse_positive),
        stale: false,
        fallback_reason: None,
    })
}

async fn fetch_mark_price(client: &reqwest::Client, symbol: &str) -> Option<MarketPriceSnapshot> {
    let mark = client
        .get(BINANCE_FAPI_PREMIUM_INDEX)
        .query(&[("symbol", symbol)])
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<BinancePremiumIndex>()
        .await
        .ok()?;
    Some(MarketPriceSnapshot {
        price: parse_positive(&mark.mark_price)?,
        source: PriceSource::MarkPrice,
        updated_at_ms: now_ms(),
        change_24h_pct: None,
        volume_24h_usd: None,
        high_24h: None,
        low_24h: None,
        stale: false,
        fallback_reason: Some("perp_last_price_unavailable_using_mark_price".to_string()),
    })
}

fn parse_positive(value: &str) -> Option<f64> {
    parse_finite(value).filter(|value| *value > 0.0)
}

fn parse_finite(value: &str) -> Option<f64> {
    let parsed = value.parse::<f64>().ok()?;
    parsed.is_finite().then_some(parsed)
}
