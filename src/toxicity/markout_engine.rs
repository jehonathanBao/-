use std::collections::BTreeMap;

use crate::types::{
    market::{AggressorSide, NormalizedTrade, Venue},
    markout::{
        empty_venue_markout_breakdown, DirectionalMarkoutStats, MarkoutHorizonMs, MarkoutQuality,
        MarkoutSample, MarkoutSampleStatus, MarkoutState, MarkoutWindowSummary,
        VenueMarkoutBreakdown,
    },
};

pub const DEFAULT_MARKOUT_HORIZONS_MS: [u64; 3] = [1000, 5000, 15000];
pub const DEFAULT_MARKOUT_MAX_AGE_MS: i64 = 120_000;
pub const DEFAULT_MARKOUT_EXPIRE_GRACE_MS: i64 = 5_000;

#[derive(Debug)]
pub struct MarkoutEngine {
    horizons_ms: Vec<MarkoutHorizonMs>,
    max_age_ms: i64,
    expire_grace_ms: i64,
    samples: BTreeMap<String, MarkoutSample>,
}

impl MarkoutEngine {
    pub fn new(horizons_ms: Vec<MarkoutHorizonMs>, max_age_ms: i64, expire_grace_ms: i64) -> Self {
        let horizons_ms = if horizons_ms.is_empty() {
            DEFAULT_MARKOUT_HORIZONS_MS.to_vec()
        } else {
            horizons_ms
        };
        Self {
            horizons_ms,
            max_age_ms: max_age_ms.max(1),
            expire_grace_ms: expire_grace_ms.max(0),
            samples: BTreeMap::new(),
        }
    }

    pub fn on_trade(&mut self, trade: &NormalizedTrade) {
        if !trade.price.is_finite()
            || !trade.size_btc.is_finite()
            || trade.price <= 0.0
            || trade.size_btc <= 0.0
        {
            return;
        }

        for horizon_ms in &self.horizons_ms {
            let id = sample_id(trade, *horizon_ms);
            self.samples.entry(id.clone()).or_insert(MarkoutSample {
                id,
                venue: trade.venue,
                symbol: trade.symbol.clone(),
                trade_ts: trade.ts,
                horizon_ms: *horizon_ms,
                direction: trade.aggressor_side,
                trade_price: trade.price,
                size_btc: trade.size_btc,
                size_usd: trade.size_usd,
                future_ts: None,
                future_mid: None,
                markout_bps: None,
                status: MarkoutSampleStatus::Pending,
            });
        }
    }

    pub fn resolve_due_samples<F>(&mut self, now_ts: i64, get_mid_at_or_before: F)
    where
        F: Fn(i64) -> Option<f64>,
    {
        for sample in self.samples.values_mut() {
            if sample.status != MarkoutSampleStatus::Pending {
                continue;
            }
            let due_ts = sample.trade_ts + sample.horizon_ms as i64;
            if now_ts < due_ts {
                continue;
            }
            if let Some(future_mid) = get_mid_at_or_before(due_ts) {
                if future_mid.is_finite() && future_mid > 0.0 {
                    sample.future_ts = Some(due_ts);
                    sample.future_mid = Some(future_mid);
                    sample.markout_bps = Some(calculate_markout_bps(
                        sample.direction,
                        sample.trade_price,
                        future_mid,
                    ));
                    sample.status = MarkoutSampleStatus::Resolved;
                }
            } else if now_ts > due_ts + self.expire_grace_ms {
                sample.status = MarkoutSampleStatus::Expired;
            }
        }
        self.prune(now_ts);
    }

    pub fn get_state(&self, now_ts: i64, has_price_index: bool) -> MarkoutState {
        let mut summaries = BTreeMap::new();
        for horizon_ms in &self.horizons_ms {
            summaries.insert(
                horizon_ms.to_string(),
                self.summary_for_horizon(*horizon_ms),
            );
        }

        MarkoutState {
            symbol: "BTC-PERP".to_string(),
            updated_at: now_ts,
            horizons_ms: self.horizons_ms.clone(),
            summaries,
            quality: MarkoutQuality {
                pending_samples: self
                    .samples
                    .values()
                    .filter(|sample| sample.status == MarkoutSampleStatus::Pending)
                    .count(),
                resolved_samples: self
                    .samples
                    .values()
                    .filter(|sample| sample.status == MarkoutSampleStatus::Resolved)
                    .count(),
                expired_samples: self
                    .samples
                    .values()
                    .filter(|sample| sample.status == MarkoutSampleStatus::Expired)
                    .count(),
                has_price_index,
            },
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    fn summary_for_horizon(&self, horizon_ms: MarkoutHorizonMs) -> MarkoutWindowSummary {
        let mut buy_acc = StatsAccumulator::default();
        let mut sell_acc = StatsAccumulator::default();
        let mut venue_acc: BTreeMap<String, VenueMarkoutAccumulator> = Venue::ALL
            .into_iter()
            .map(|venue| {
                (
                    venue.as_key().to_string(),
                    VenueMarkoutAccumulator::default(),
                )
            })
            .collect();

        for sample in self.samples.values() {
            if sample.horizon_ms != horizon_ms || sample.status != MarkoutSampleStatus::Resolved {
                continue;
            }
            let Some(markout_bps) = sample.markout_bps else {
                continue;
            };
            match sample.direction {
                AggressorSide::Buy => buy_acc.add(sample, markout_bps),
                AggressorSide::Sell => sell_acc.add(sample, markout_bps),
            }
            let venue = venue_acc
                .entry(sample.venue.as_key().to_string())
                .or_default();
            match sample.direction {
                AggressorSide::Buy => venue.buy.add(sample, markout_bps),
                AggressorSide::Sell => venue.sell.add(sample, markout_bps),
            }
        }

        MarkoutWindowSummary {
            horizon_ms,
            buy: buy_acc.finish(),
            sell: sell_acc.finish(),
            venue_breakdown: finish_venue_breakdown(venue_acc),
        }
    }

    fn prune(&mut self, now_ts: i64) {
        let cutoff = now_ts - self.max_age_ms;
        self.samples.retain(|_, sample| sample.trade_ts >= cutoff);
    }
}

pub fn calculate_markout_bps(direction: AggressorSide, trade_price: f64, future_mid: f64) -> f64 {
    match direction {
        AggressorSide::Buy => ((future_mid - trade_price) / trade_price) * 10_000.0,
        AggressorSide::Sell => ((trade_price - future_mid) / trade_price) * 10_000.0,
    }
}

fn sample_id(trade: &NormalizedTrade, horizon_ms: MarkoutHorizonMs) -> String {
    if let Some(trade_id) = &trade.trade_id {
        format!("{}:{trade_id}:{horizon_ms}", trade.venue)
    } else {
        format!(
            "{}:{}:{}:{}:{:?}:{}",
            trade.venue, trade.ts, trade.price, trade.size_btc, trade.aggressor_side, horizon_ms
        )
    }
}

#[derive(Debug, Default)]
struct StatsAccumulator {
    count: u64,
    volume_btc: f64,
    volume_usd: f64,
    markout_sum: f64,
    weighted_markout_sum: f64,
    positive_count: u64,
    negative_count: u64,
    positive_volume_btc: f64,
    negative_volume_btc: f64,
}

impl StatsAccumulator {
    fn add(&mut self, sample: &MarkoutSample, markout_bps: f64) {
        self.count += 1;
        self.volume_btc += sample.size_btc;
        self.volume_usd += sample.size_usd;
        self.markout_sum += markout_bps;
        self.weighted_markout_sum += markout_bps * sample.size_btc;
        if markout_bps > 0.0 {
            self.positive_count += 1;
            self.positive_volume_btc += sample.size_btc;
        } else if markout_bps < 0.0 {
            self.negative_count += 1;
            self.negative_volume_btc += sample.size_btc;
        }
    }

    fn finish(self) -> DirectionalMarkoutStats {
        DirectionalMarkoutStats {
            count: self.count,
            volume_btc: self.volume_btc,
            volume_usd: self.volume_usd,
            avg_markout_bps: (self.count > 0).then_some(self.markout_sum / self.count as f64),
            volume_weighted_markout_bps: (self.volume_btc > 0.0)
                .then_some(self.weighted_markout_sum / self.volume_btc),
            positive_count: self.positive_count,
            negative_count: self.negative_count,
            positive_volume_btc: self.positive_volume_btc,
            negative_volume_btc: self.negative_volume_btc,
        }
    }
}

#[derive(Debug, Default)]
struct VenueMarkoutAccumulator {
    buy: StatsAccumulator,
    sell: StatsAccumulator,
}

fn finish_venue_breakdown(
    accumulators: BTreeMap<String, VenueMarkoutAccumulator>,
) -> BTreeMap<String, VenueMarkoutBreakdown> {
    let mut output = empty_venue_markout_breakdown();
    for (venue, accumulator) in accumulators {
        output.insert(
            venue,
            VenueMarkoutBreakdown {
                buy: accumulator.buy.finish(),
                sell: accumulator.sell.finish(),
            },
        );
    }
    output
}
