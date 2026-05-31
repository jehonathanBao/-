use crate::{
    market_data::price_index::PriceSnapshot,
    types::sweep::{LiquidityThinnessResult, SweepWindowMs},
};

#[derive(Debug, Clone)]
pub struct LiquidityThinness {
    min_depth_drop_ratio: f64,
    min_spread_widen_ratio: f64,
}

impl LiquidityThinness {
    pub fn new(min_depth_drop_ratio: f64, min_spread_widen_ratio: f64) -> Self {
        Self {
            min_depth_drop_ratio,
            min_spread_widen_ratio,
        }
    }

    pub fn detect(
        &self,
        symbol: impl Into<String>,
        window_ms: SweepWindowMs,
        start: Option<PriceSnapshot>,
        end: Option<PriceSnapshot>,
    ) -> LiquidityThinnessResult {
        let mut result = LiquidityThinnessResult {
            symbol: symbol.into(),
            window_ms,
            ..LiquidityThinnessResult::default()
        };

        let (Some(start), Some(end)) = (start, end) else {
            result
                .reason_codes
                .push("missing_price_snapshot".to_string());
            return result;
        };

        result.bid_depth_start_btc = start.bid_depth_btc_10bps_median;
        result.bid_depth_end_btc = end.bid_depth_btc_10bps_median;
        result.ask_depth_start_btc = start.ask_depth_btc_10bps_median;
        result.ask_depth_end_btc = end.ask_depth_btc_10bps_median;
        result.spread_start_bps = start.spread_bps_median;
        result.spread_end_bps = end.spread_bps_median;

        result.bid_depth_drop_ratio =
            drop_ratio(result.bid_depth_start_btc, result.bid_depth_end_btc);
        result.ask_depth_drop_ratio =
            drop_ratio(result.ask_depth_start_btc, result.ask_depth_end_btc);
        result.spread_widen_ratio = widen_ratio(result.spread_start_bps, result.spread_end_bps);

        result.bid_thin = result
            .bid_depth_drop_ratio
            .is_some_and(|ratio| ratio >= self.min_depth_drop_ratio);
        result.ask_thin = result
            .ask_depth_drop_ratio
            .is_some_and(|ratio| ratio >= self.min_depth_drop_ratio);
        result.spread_widened = result
            .spread_widen_ratio
            .is_some_and(|ratio| ratio >= self.min_spread_widen_ratio);

        if result.bid_thin {
            result
                .reason_codes
                .push("bid_liquidity_thinned".to_string());
        }
        if result.ask_thin {
            result
                .reason_codes
                .push("ask_liquidity_thinned".to_string());
        }
        if result.spread_widened {
            result.reason_codes.push("spread_widened".to_string());
        }

        result
    }
}

impl Default for LiquidityThinness {
    fn default() -> Self {
        Self::new(0.2, 0.2)
    }
}

fn drop_ratio(start: Option<f64>, end: Option<f64>) -> Option<f64> {
    let (Some(start), Some(end)) = (start, end) else {
        return None;
    };
    (start > 0.0).then_some((start - end) / start)
}

fn widen_ratio(start: Option<f64>, end: Option<f64>) -> Option<f64> {
    let (Some(start), Some(end)) = (start, end) else {
        return None;
    };
    (start > 0.0).then_some((end - start) / start)
}
