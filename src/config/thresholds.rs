use crate::types::toxic::ToxicSeverity;

pub const DEFAULT_TOXIC_VOLUME_ALERT_BTC: f64 = 1000.0;
pub const DEFAULT_WINDOWS_MS: [u64; 4] = [1000, 5000, 15000, 60000];

#[derive(Debug, Clone)]
pub struct ToxicVolumeParams {
    pub threshold_btc: f64,
    pub min_large_flow_btc: f64,
    pub markout_1s_bps: f64,
    pub markout_5s_bps: f64,
    pub min_depth_drop_ratio: f64,
    pub min_cross_venue_count: usize,
    pub recent_event_limit: usize,
}

impl Default for ToxicVolumeParams {
    fn default() -> Self {
        Self {
            threshold_btc: DEFAULT_TOXIC_VOLUME_ALERT_BTC,
            min_large_flow_btc: 100.0,
            markout_1s_bps: 1.0,
            markout_5s_bps: 3.0,
            min_depth_drop_ratio: 0.30,
            min_cross_venue_count: 2,
            recent_event_limit: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlertGateConfig {
    pub dedup_window_ms: i64,
    pub min_severity: ToxicSeverity,
    pub require_cross_venue: bool,
    pub require_markout: bool,
    pub require_liquidity_drain: bool,
}

impl Default for AlertGateConfig {
    fn default() -> Self {
        Self {
            dedup_window_ms: 30_000,
            min_severity: ToxicSeverity::Alert,
            require_cross_venue: true,
            require_markout: true,
            require_liquidity_drain: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VpinParams {
    pub enabled: bool,
    pub bucket_size_btc: f64,
    pub lookback_buckets: usize,
    pub min_buckets: usize,
    pub spike_zscore: f64,
    pub high_threshold: f64,
    pub extreme_threshold: f64,
    pub max_recent_buckets: usize,
    pub persist_buckets: bool,
}

impl Default for VpinParams {
    fn default() -> Self {
        Self {
            enabled: true,
            bucket_size_btc: 100.0,
            lookback_buckets: 50,
            min_buckets: 10,
            spike_zscore: 2.5,
            high_threshold: 0.70,
            extreme_threshold: 0.85,
            max_recent_buckets: 500,
            persist_buckets: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiquidationClusterParams {
    pub enabled: bool,
    pub lookback_ms: i64,
    pub cluster_band_bps: f64,
    pub min_cluster_distance_bps: f64,
    pub max_cluster_distance_bps: f64,
    pub proximity_threshold_bps: f64,
    pub min_touches: usize,
    pub pressure_threshold: f64,
}

impl Default for LiquidationClusterParams {
    fn default() -> Self {
        Self {
            enabled: true,
            lookback_ms: 120_000,
            cluster_band_bps: 6.0,
            min_cluster_distance_bps: 5.0,
            max_cluster_distance_bps: 150.0,
            proximity_threshold_bps: 25.0,
            min_touches: 3,
            pressure_threshold: 0.65,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiqHuntParams {
    pub cluster_large_notional_usd: f64,
    pub near_distance_bps: f64,
    pub active_score: f64,
    pub likely_score: f64,
    pub watch_score: f64,
    pub recent_result_limit: usize,
}

impl Default for LiqHuntParams {
    fn default() -> Self {
        Self {
            cluster_large_notional_usd: 50_000_000.0,
            near_distance_bps: 25.0,
            active_score: 75.0,
            likely_score: 50.0,
            watch_score: 30.0,
            recent_result_limit: 100,
        }
    }
}
