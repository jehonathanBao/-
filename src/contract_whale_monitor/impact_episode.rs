//! Deterministic aggregation of adjacent contract shock fragments.

use std::collections::HashSet;

use sha2::{Digest, Sha256};

const MAX_EPISODE_SPAN_MS: i64 = 30 * 60 * 1_000;

pub use super::impact_grade::ContractImpactEpisode;

#[derive(Debug, Clone)]
pub struct ImpactBucketContribution {
    pub identity: String,
    pub source: String,
    pub volume_btc: f64,
    pub notional_usd: f64,
}

#[derive(Debug, Clone)]
pub struct ImpactEventFragment {
    pub event_id: String,
    pub symbol: String,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub total_volume_btc: f64,
    pub total_notional_usd: f64,
    pub net_volume_btc: f64,
    pub unique_turnover_btc: Option<f64>,
    pub unique_turnover_notional_usd: Option<f64>,
    pub live_liquidation_btc: Option<f64>,
    pub live_liquidation_notional_usd: Option<f64>,
    pub peak_abs_price_move_pct: Option<f64>,
    pub peak_abs_oi_change_pct: Option<f64>,
    pub confirmed_sources: Vec<String>,
    pub data_quality: u8,
    pub robust_percentile: Option<f64>,
    pub robust_z: Option<f64>,
    pub baseline_sample_count: usize,
    /// Representative price used to prevent unrelated shocks being merged.
    pub anchor_price: Option<f64>,
    pub flow_buckets: Vec<ImpactBucketContribution>,
    pub liquidation_buckets: Vec<ImpactBucketContribution>,
}

pub fn aggregate_shock_episodes(
    mut fragments: Vec<ImpactEventFragment>,
    gap_seconds: i64,
) -> Vec<ContractImpactEpisode> {
    fragments.sort_by_key(|fragment| {
        (
            fragment.symbol.clone(),
            fragment.start_time_ms,
            fragment.event_id.clone(),
        )
    });
    let mut result = Vec::new();
    let mut current: Option<EpisodeAccumulator> = None;
    for fragment in fragments {
        let should_merge = current.as_ref().is_some_and(|episode| {
            episode.symbol == fragment.symbol
                && compatible_flow_direction(episode.net_volume_btc, fragment.net_volume_btc)
                && price_is_continuous(episode.last_anchor_price, fragment.anchor_price)
                && flow_is_persistent(episode.net_volume_btc, fragment.net_volume_btc)
                && fragment.start_time_ms
                    <= episode
                        .end_time_ms
                        .saturating_add(gap_seconds.saturating_mul(1_000))
                && fragment
                    .end_time_ms
                    .saturating_sub(episode.start_time_ms)
                    <= MAX_EPISODE_SPAN_MS
        });
        if !should_merge {
            if let Some(accumulator) = current.take() {
                result.push(accumulator.finish());
            }
            current = Some(EpisodeAccumulator::new(fragment));
        } else if let Some(accumulator) = current.as_mut() {
            accumulator.merge(fragment);
        }
    }
    if let Some(accumulator) = current {
        result.push(accumulator.finish());
    }
    result
}

fn price_is_continuous(previous: Option<f64>, next: Option<f64>) -> bool {
    match (previous, next) {
        (Some(previous), Some(next)) if previous > 0.0 && next > 0.0 =>
            ((next - previous).abs() / previous) <= 0.02,
        _ => true,
    }
}

fn flow_is_persistent(previous_net: f64, next_net: f64) -> bool {
    previous_net.abs() <= f64::EPSILON
        || next_net.abs() <= f64::EPSILON
        || (previous_net.signum() == next_net.signum()
            && next_net.abs() >= previous_net.abs() * 0.02)
}

fn compatible_flow_direction(current_net: f64, next_net: f64) -> bool {
    let current_sign = current_net.total_cmp(&0.0);
    let next_sign = next_net.total_cmp(&0.0);
    current_sign == std::cmp::Ordering::Equal
        || next_sign == std::cmp::Ordering::Equal
        || current_sign == next_sign
}

struct EpisodeAccumulator {
    episode_id: String,
    symbol: String,
    start_time_ms: i64,
    end_time_ms: i64,
    source_event_ids: Vec<String>,
    peak_window_volume_btc: f64,
    fallback_peak_notional_usd: f64,
    total_volume_btc: f64,
    total_notional_usd: f64,
    net_volume_btc: f64,
    fallback_unique_turnover_btc: Option<f64>,
    fallback_unique_turnover_notional_usd: Option<f64>,
    live_liquidation_btc: Option<f64>,
    live_liquidation_notional_usd: Option<f64>,
    fallback_live_liquidation_btc: Option<f64>,
    fallback_live_liquidation_notional_usd: Option<f64>,
    peak_abs_price_move_pct: Option<f64>,
    peak_abs_oi_change_pct: Option<f64>,
    confirmed_sources: HashSet<String>,
    data_quality: u8,
    robust_percentile: Option<f64>,
    robust_z: Option<f64>,
    baseline_sample_count: usize,
    last_anchor_price: Option<f64>,
    flow_bucket_ids: HashSet<String>,
    liquidation_bucket_ids: HashSet<String>,
    raw_flow_observed: bool,
    raw_liquidation_observed: bool,
}

impl EpisodeAccumulator {
    fn new(fragment: ImpactEventFragment) -> Self {
        let episode_id = deterministic_episode_id(&fragment.symbol, &fragment.event_id);
        let mut accumulator = Self {
            episode_id,
            symbol: fragment.symbol.clone(),
            start_time_ms: fragment.start_time_ms,
            end_time_ms: fragment.end_time_ms,
            source_event_ids: Vec::new(),
            peak_window_volume_btc: 0.0,
            fallback_peak_notional_usd: 0.0,
            total_volume_btc: 0.0,
            total_notional_usd: 0.0,
            net_volume_btc: 0.0,
            fallback_unique_turnover_btc: None,
            fallback_unique_turnover_notional_usd: None,
            live_liquidation_btc: None,
            live_liquidation_notional_usd: None,
            fallback_live_liquidation_btc: None,
            fallback_live_liquidation_notional_usd: None,
            peak_abs_price_move_pct: None,
            peak_abs_oi_change_pct: None,
            confirmed_sources: HashSet::new(),
            data_quality: fragment.data_quality,
            robust_percentile: fragment.robust_percentile,
            robust_z: fragment.robust_z,
            baseline_sample_count: fragment.baseline_sample_count,
            last_anchor_price: fragment.anchor_price,
            flow_bucket_ids: HashSet::new(),
            liquidation_bucket_ids: HashSet::new(),
            raw_flow_observed: false,
            raw_liquidation_observed: false,
        };
        accumulator.merge(fragment);
        accumulator
    }

    fn merge(&mut self, fragment: ImpactEventFragment) {
        let had_fragment = !self.source_event_ids.is_empty();
        self.end_time_ms = self.end_time_ms.max(fragment.end_time_ms);
        self.last_anchor_price = fragment.anchor_price.or(self.last_anchor_price);
        self.source_event_ids.push(fragment.event_id);
        // Preserve detector-provided event turnover separately from the raw
        // market-window aggregate below. The latter is deduplicated bucket
        // volume, not unique trader turnover.
        self.fallback_unique_turnover_btc = max_opt(
            self.fallback_unique_turnover_btc,
            fragment.unique_turnover_btc,
        );
        self.fallback_unique_turnover_notional_usd = max_opt(
            self.fallback_unique_turnover_notional_usd,
            fragment.unique_turnover_notional_usd,
        );
        self.peak_window_volume_btc = self
            .peak_window_volume_btc
            .max(fragment.total_volume_btc.max(0.0));
        self.fallback_peak_notional_usd = self
            .fallback_peak_notional_usd
            .max(fragment.total_notional_usd.max(0.0));
        if fragment.flow_buckets.is_empty() {
            // No raw market buckets: retain the detector-provided turnover
            // captured above as the only available event-level evidence.
        } else {
            self.raw_flow_observed = true;
            for bucket in fragment.flow_buckets {
                if self.flow_bucket_ids.insert(bucket.identity) {
                    self.total_volume_btc += bucket.volume_btc;
                    self.total_notional_usd += bucket.notional_usd;
                }
            }
        }
        self.net_volume_btc += fragment.net_volume_btc;
        let mut liquidation_btc = 0.0;
        let mut liquidation_usd = 0.0;
        if fragment.liquidation_buckets.is_empty() {
            self.fallback_live_liquidation_btc = max_opt(
                self.fallback_live_liquidation_btc,
                fragment.live_liquidation_btc,
            );
            self.fallback_live_liquidation_notional_usd = max_opt(
                self.fallback_live_liquidation_notional_usd,
                fragment.live_liquidation_notional_usd,
            );
        } else {
            self.raw_liquidation_observed = true;
            for bucket in fragment.liquidation_buckets {
                if self.liquidation_bucket_ids.insert(bucket.identity) {
                    liquidation_btc += bucket.volume_btc;
                    liquidation_usd += bucket.notional_usd;
                }
            }
        }
        if self.raw_liquidation_observed {
            self.live_liquidation_btc =
                Some(self.live_liquidation_btc.unwrap_or_default() + liquidation_btc);
            self.live_liquidation_notional_usd =
                Some(self.live_liquidation_notional_usd.unwrap_or_default() + liquidation_usd);
        }
        self.peak_abs_price_move_pct = max_opt(
            self.peak_abs_price_move_pct,
            fragment.peak_abs_price_move_pct,
        );
        self.peak_abs_oi_change_pct =
            max_opt(self.peak_abs_oi_change_pct, fragment.peak_abs_oi_change_pct);
        self.confirmed_sources.extend(fragment.confirmed_sources);
        // Evidence quality is conjunctive across an episode: one degraded
        // fragment must not be hidden by a stronger neighboring fragment.
        self.data_quality = self.data_quality.min(fragment.data_quality);
        // Relative scores belong to the complete episode volume and must be
        // recomputed against its baseline after aggregation. Never promote by
        // taking the maximum fragment percentile/z-score.
        if had_fragment {
            self.robust_percentile = None;
            self.robust_z = None;
            self.baseline_sample_count = 0;
        }
    }

    fn finish(self) -> ContractImpactEpisode {
        let mut source_event_ids = self.source_event_ids;
        source_event_ids.sort();
        source_event_ids.dedup();
        let mut confirmed_sources: Vec<String> = self.confirmed_sources.into_iter().collect();
        confirmed_sources.sort();
        let (
            total_volume_btc,
            total_notional_usd,
            unique_turnover_btc,
            unique_turnover_notional_usd,
        ) = if self.raw_flow_observed {
            (
                self.total_volume_btc,
                self.total_notional_usd,
                self.fallback_unique_turnover_btc.filter(|value| *value > 0.0),
                self.fallback_unique_turnover_notional_usd
                    .filter(|value| *value > 0.0),
            )
        } else {
            (
                self.peak_window_volume_btc,
                self.fallback_peak_notional_usd,
                self.fallback_unique_turnover_btc,
                self.fallback_unique_turnover_notional_usd,
            )
        };
        let (live_liquidation_btc, live_liquidation_notional_usd) = if self.raw_liquidation_observed
        {
            (
                self.live_liquidation_btc.filter(|value| *value > 0.0),
                self.live_liquidation_notional_usd
                    .filter(|value| *value > 0.0),
            )
        } else {
            (
                self.fallback_live_liquidation_btc,
                self.fallback_live_liquidation_notional_usd,
            )
        };
        ContractImpactEpisode {
            episode_id: self.episode_id,
            symbol: self.symbol,
            start_time_ms: self.start_time_ms,
            end_time_ms: self.end_time_ms,
            source_event_ids,
            peak_window_volume_btc: self.peak_window_volume_btc,
            total_volume_btc,
            total_notional_usd,
            net_volume_btc: self.net_volume_btc,
            unique_turnover_btc,
            unique_turnover_notional_usd,
            live_liquidation_btc,
            live_liquidation_notional_usd,
            peak_abs_price_move_pct: self.peak_abs_price_move_pct,
            peak_abs_oi_change_pct: self.peak_abs_oi_change_pct,
            confirmed_sources,
            data_quality: self.data_quality,
            robust_percentile: self.robust_percentile,
            robust_z: self.robust_z,
            baseline_sample_count: self.baseline_sample_count,
        }
    }
}

fn max_opt(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn deterministic_episode_id(symbol: &str, first_event_id: &str) -> String {
    let payload = format!("cwm-impact-episode:v3:{symbol}:{first_event_id}");
    let digest = Sha256::digest(payload.as_bytes());
    let digest_hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("episode-{digest_hex}")
}
