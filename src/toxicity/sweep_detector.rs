use crate::types::{
    flow::FlowWindow,
    market::{AggressorSide, NormalizedTrade, Venue},
    sweep::{
        empty_venue_sweep_breakdown, LiquidityThinnessResult, SweepDirection, SweepResult,
        SweepWindowMs, VenueSweepBreakdown,
    },
};

#[derive(Debug, Clone)]
pub struct SweepParams {
    pub min_swept_volume_btc: f64,
    pub min_same_direction_trades: u64,
    pub min_net_dominance_ratio: f64,
    pub min_price_impact_bps: f64,
    pub min_depth_drop_ratio: f64,
    pub min_spread_widen_ratio: f64,
}

impl Default for SweepParams {
    fn default() -> Self {
        Self {
            min_swept_volume_btc: 100.0,
            min_same_direction_trades: 3,
            min_net_dominance_ratio: 0.6,
            min_price_impact_bps: 1.0,
            min_depth_drop_ratio: 0.2,
            min_spread_widen_ratio: 0.2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SweepInput {
    pub symbol: String,
    pub window_ms: SweepWindowMs,
    pub trades: Vec<NormalizedTrade>,
    pub flow_window: FlowWindow,
    pub liquidity: Option<LiquidityThinnessResult>,
}

#[derive(Debug, Clone)]
pub struct SweepDetector {
    params: SweepParams,
}

impl SweepDetector {
    pub fn new(params: SweepParams) -> Self {
        Self { params }
    }

    pub fn with_params(&self, params: SweepParams) -> Self {
        Self { params }
    }

    pub fn detect(&self, input: SweepInput) -> SweepResult {
        let venue_breakdown = build_venue_breakdown(&input.trades);
        let buy_trade_count = input
            .trades
            .iter()
            .filter(|trade| trade.aggressor_side == AggressorSide::Buy)
            .count() as u64;
        let sell_trade_count = input
            .trades
            .iter()
            .filter(|trade| trade.aggressor_side == AggressorSide::Sell)
            .count() as u64;
        let dominance_ratio = if input.flow_window.abs_aggressive_btc > 0.0 {
            input.flow_window.net_aggressive_btc.abs() / input.flow_window.abs_aggressive_btc
        } else {
            0.0
        };

        let price_impact_bps = input.flow_window.price_move_bps;
        let liquidity_supports_buy = input
            .liquidity
            .as_ref()
            .is_some_and(|liquidity| liquidity.ask_thin || liquidity.spread_widened);
        let liquidity_supports_sell = input
            .liquidity
            .as_ref()
            .is_some_and(|liquidity| liquidity.bid_thin || liquidity.spread_widened);

        let buy_sweep = input.flow_window.aggressive_buy_btc >= self.params.min_swept_volume_btc
            && input.flow_window.aggressive_buy_btc > input.flow_window.aggressive_sell_btc
            && dominance_ratio >= self.params.min_net_dominance_ratio
            && buy_trade_count >= self.params.min_same_direction_trades
            && price_impact_bps.is_some_and(|bps| bps >= self.params.min_price_impact_bps)
            && liquidity_supports_buy;

        let sell_sweep = input.flow_window.aggressive_sell_btc >= self.params.min_swept_volume_btc
            && input.flow_window.aggressive_sell_btc > input.flow_window.aggressive_buy_btc
            && dominance_ratio >= self.params.min_net_dominance_ratio
            && sell_trade_count >= self.params.min_same_direction_trades
            && price_impact_bps.is_some_and(|bps| bps <= -self.params.min_price_impact_bps)
            && liquidity_supports_sell;

        let direction = if buy_sweep {
            SweepDirection::Buy
        } else if sell_sweep {
            SweepDirection::Sell
        } else {
            SweepDirection::None
        };

        let (swept_volume_btc, swept_volume_usd, same_direction_trade_count, leader_venue) =
            match direction {
                SweepDirection::Buy => (
                    input.flow_window.aggressive_buy_btc,
                    input.flow_window.aggressive_buy_usd,
                    buy_trade_count,
                    leader_venue(&venue_breakdown, SweepDirection::Buy),
                ),
                SweepDirection::Sell => (
                    input.flow_window.aggressive_sell_btc,
                    input.flow_window.aggressive_sell_usd,
                    sell_trade_count,
                    leader_venue(&venue_breakdown, SweepDirection::Sell),
                ),
                SweepDirection::None => (0.0, 0.0, buy_trade_count.max(sell_trade_count), None),
            };

        SweepResult {
            symbol: input.symbol,
            window_ms: input.window_ms,
            direction,
            sweep_detected: direction != SweepDirection::None,
            swept_volume_btc,
            swept_volume_usd,
            aggressive_buy_btc: input.flow_window.aggressive_buy_btc,
            aggressive_sell_btc: input.flow_window.aggressive_sell_btc,
            net_aggressive_btc: input.flow_window.net_aggressive_btc,
            trade_count: input.flow_window.trade_count,
            same_direction_trade_count,
            price_start: input.flow_window.mid_start,
            price_end: input.flow_window.mid_end,
            price_impact_bps,
            leader_venue,
            venue_breakdown,
            liquidity: input.liquidity,
            reason_codes: self.reason_codes(
                direction,
                dominance_ratio,
                buy_trade_count,
                sell_trade_count,
                price_impact_bps,
            ),
        }
    }

    fn reason_codes(
        &self,
        direction: SweepDirection,
        dominance_ratio: f64,
        buy_trade_count: u64,
        sell_trade_count: u64,
        price_impact_bps: Option<f64>,
    ) -> Vec<String> {
        if direction == SweepDirection::None {
            return Vec::new();
        }

        let mut codes = Vec::new();
        match direction {
            SweepDirection::Buy => {
                codes.push("buy_dominant_flow".to_string());
                if buy_trade_count >= self.params.min_same_direction_trades {
                    codes.push("same_direction_trade_count".to_string());
                }
                if price_impact_bps.is_some_and(|bps| bps >= self.params.min_price_impact_bps) {
                    codes.push("positive_price_impact".to_string());
                }
            }
            SweepDirection::Sell => {
                codes.push("sell_dominant_flow".to_string());
                if sell_trade_count >= self.params.min_same_direction_trades {
                    codes.push("same_direction_trade_count".to_string());
                }
                if price_impact_bps.is_some_and(|bps| bps <= -self.params.min_price_impact_bps) {
                    codes.push("negative_price_impact".to_string());
                }
            }
            SweepDirection::None => {}
        }
        if dominance_ratio >= self.params.min_net_dominance_ratio {
            codes.push("net_dominance".to_string());
        }
        codes
    }
}

impl Default for SweepDetector {
    fn default() -> Self {
        Self::new(SweepParams::default())
    }
}

fn build_venue_breakdown(
    trades: &[NormalizedTrade],
) -> std::collections::BTreeMap<String, VenueSweepBreakdown> {
    let mut breakdown = empty_venue_sweep_breakdown();
    for trade in trades {
        let entry = breakdown
            .entry(trade.venue.as_key().to_string())
            .or_default();
        match trade.aggressor_side {
            AggressorSide::Buy => entry.swept_buy_btc += trade.size_btc,
            AggressorSide::Sell => entry.swept_sell_btc += trade.size_btc,
        }
        entry.net_swept_btc = entry.swept_buy_btc - entry.swept_sell_btc;
        entry.trade_count += 1;
    }
    breakdown
}

fn leader_venue(
    breakdown: &std::collections::BTreeMap<String, VenueSweepBreakdown>,
    direction: SweepDirection,
) -> Option<Venue> {
    Venue::ALL.into_iter().max_by(|left, right| {
        let left_volume = venue_direction_volume(breakdown, *left, direction);
        let right_volume = venue_direction_volume(breakdown, *right, direction);
        left_volume.total_cmp(&right_volume)
    })
}

fn venue_direction_volume(
    breakdown: &std::collections::BTreeMap<String, VenueSweepBreakdown>,
    venue: Venue,
    direction: SweepDirection,
) -> f64 {
    let Some(entry) = breakdown.get(venue.as_key()) else {
        return 0.0;
    };
    match direction {
        SweepDirection::Buy => entry.swept_buy_btc,
        SweepDirection::Sell => entry.swept_sell_btc,
        SweepDirection::None => 0.0,
    }
}
