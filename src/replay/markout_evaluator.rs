use serde::{Deserialize, Serialize};

use crate::types::orderbook_delta::ManipulationSignalV2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayPricePoint {
    pub ts_ms: i64,
    pub mid: f64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalMarkout {
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub markout_30s_bps: Option<f64>,
}

pub fn evaluate_candidate_markout(
    signal: &ManipulationSignalV2,
    prices: &[ReplayPricePoint],
) -> SignalMarkout {
    SignalMarkout {
        markout_1s_bps: markout_at(signal, prices, 1_000),
        markout_5s_bps: markout_at(signal, prices, 5_000),
        markout_30s_bps: markout_at(signal, prices, 30_000),
    }
}

fn markout_at(
    signal: &ManipulationSignalV2,
    prices: &[ReplayPricePoint],
    horizon_ms: i64,
) -> Option<f64> {
    let start = price_at_or_before(prices, signal.observed_end_ms)?;
    let future = price_at_or_after(prices, signal.observed_end_ms + horizon_ms)?;
    let raw_bps = ((future - start) / start.max(1.0)) * 10_000.0;
    match signal.side {
        crate::types::orderbook_wall::OrderbookWallSide::Ask => Some(raw_bps),
        crate::types::orderbook_wall::OrderbookWallSide::Bid => Some(-raw_bps),
    }
}

fn price_at_or_before(prices: &[ReplayPricePoint], ts_ms: i64) -> Option<f64> {
    prices
        .iter()
        .rev()
        .find(|point| point.ts_ms <= ts_ms)
        .map(|point| point.mid)
}

fn price_at_or_after(prices: &[ReplayPricePoint], ts_ms: i64) -> Option<f64> {
    prices
        .iter()
        .find(|point| point.ts_ms >= ts_ms)
        .map(|point| point.mid)
}
