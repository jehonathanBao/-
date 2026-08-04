//! Deterministic aggregation of adjacent contract shock fragments.

use std::collections::HashSet;

use sha2::{Digest, Sha256};

pub use super::impact_grade::ContractImpactEpisode;

#[derive(Debug, Clone)]
pub struct ImpactBucketContribution {
    pub identity: String,
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
                && fragment.start_time_ms
                    <= episode
                        .end_time_ms
                        .saturating_add(gap_seconds.saturating_mul(1_000))
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

struct EpisodeAccumulator {
    episode_id: String,
    symbol: String,
    start_time_ms: i64,
    end_time_ms: i64,
    source_event_ids: Vec<String>,
    total_volume_btc: f64,
    total_notional_usd: f64,
    net_volume_btc: f64,
    unique_turnover_btc: Option<f64>,
    unique_turnover_notional_usd: Option<f64>,
    live_liquidation_btc: Option<f64>,
    live_liquidation_notional_usd: Option<f64>,
    peak_abs_price_move_pct: Option<f64>,
    peak_abs_oi_change_pct: Option<f64>,
    confirmed_sources: HashSet<String>,
    data_quality: u8,
    robust_percentile: Option<f64>,
    robust_z: Option<f64>,
    baseline_sample_count: usize,
    flow_bucket_ids: HashSet<String>,
    liquidation_bucket_ids: HashSet<String>,
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
            total_volume_btc: 0.0,
            total_notional_usd: 0.0,
            net_volume_btc: 0.0,
            unique_turnover_btc: None,
            unique_turnover_notional_usd: None,
            live_liquidation_btc: None,
            live_liquidation_notional_usd: None,
            peak_abs_price_move_pct: None,
            peak_abs_oi_change_pct: None,
            confirmed_sources: HashSet::new(),
            data_quality: fragment.data_quality,
            robust_percentile: fragment.robust_percentile,
            robust_z: fragment.robust_z,
            baseline_sample_count: fragment.baseline_sample_count,
            flow_bucket_ids: HashSet::new(),
            liquidation_bucket_ids: HashSet::new(),
        };
        accumulator.merge(fragment);
        accumulator
    }

    fn merge(&mut self, fragment: ImpactEventFragment) {
        self.end_time_ms = self.end_time_ms.max(fragment.end_time_ms);
        self.source_event_ids.push(fragment.event_id);
        if fragment.flow_buckets.is_empty() {
            self.total_volume_btc += fragment.total_volume_btc;
            self.total_notional_usd += fragment.total_notional_usd;
        } else {
            for bucket in fragment.flow_buckets {
                if self.flow_bucket_ids.insert(bucket.identity) {
                    self.total_volume_btc += bucket.volume_btc;
                    self.total_notional_usd += bucket.notional_usd;
                }
            }
        }
        self.net_volume_btc += fragment.net_volume_btc;
        self.unique_turnover_btc = max_opt(self.unique_turnover_btc, fragment.unique_turnover_btc);
        self.unique_turnover_notional_usd = max_opt(
            self.unique_turnover_notional_usd,
            fragment.unique_turnover_notional_usd,
        );
        let mut liquidation_btc = 0.0;
        let mut liquidation_usd = 0.0;
        if fragment.liquidation_buckets.is_empty() {
            liquidation_btc = fragment.live_liquidation_btc.unwrap_or_default();
            liquidation_usd = fragment.live_liquidation_notional_usd.unwrap_or_default();
        } else {
            for bucket in fragment.liquidation_buckets {
                if self.liquidation_bucket_ids.insert(bucket.identity) {
                    liquidation_btc += bucket.volume_btc;
                    liquidation_usd += bucket.notional_usd;
                }
            }
        }
        self.live_liquidation_btc =
            Some(self.live_liquidation_btc.unwrap_or_default() + liquidation_btc);
        self.live_liquidation_notional_usd =
            Some(self.live_liquidation_notional_usd.unwrap_or_default() + liquidation_usd);
        self.peak_abs_price_move_pct = max_opt(
            self.peak_abs_price_move_pct,
            fragment.peak_abs_price_move_pct,
        );
        self.peak_abs_oi_change_pct =
            max_opt(self.peak_abs_oi_change_pct, fragment.peak_abs_oi_change_pct);
        self.confirmed_sources.extend(fragment.confirmed_sources);
        self.data_quality = self.data_quality.max(fragment.data_quality);
        self.robust_percentile = max_opt(self.robust_percentile, fragment.robust_percentile);
        self.robust_z = max_opt(self.robust_z, fragment.robust_z);
        self.baseline_sample_count = self
            .baseline_sample_count
            .max(fragment.baseline_sample_count);
    }

    fn finish(self) -> ContractImpactEpisode {
        let mut source_event_ids = self.source_event_ids;
        source_event_ids.sort();
        source_event_ids.dedup();
        let mut confirmed_sources: Vec<String> = self.confirmed_sources.into_iter().collect();
        confirmed_sources.sort();
        ContractImpactEpisode {
            episode_id: self.episode_id,
            symbol: self.symbol,
            start_time_ms: self.start_time_ms,
            end_time_ms: self.end_time_ms,
            source_event_ids,
            total_volume_btc: self.total_volume_btc,
            total_notional_usd: self.total_notional_usd,
            net_volume_btc: self.net_volume_btc,
            unique_turnover_btc: self.unique_turnover_btc,
            unique_turnover_notional_usd: self.unique_turnover_notional_usd,
            live_liquidation_btc: self.live_liquidation_btc.filter(|value| *value > 0.0),
            live_liquidation_notional_usd: self
                .live_liquidation_notional_usd
                .filter(|value| *value > 0.0),
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
