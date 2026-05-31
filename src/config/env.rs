use std::{env, net::IpAddr};

use anyhow::{anyhow, Context};

use crate::{
    config::{
        thresholds,
        venues::{VenueConfig, VenueConfigs},
    },
    types::market::Venue,
    types::toxic::ToxicSeverity,
};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub read_only: bool,
    pub api_host: IpAddr,
    pub api_port: u16,
    pub symbol: String,
    pub toxic_volume_alert_btc: f64,
    pub windows_ms: Vec<u64>,
    pub markout_horizons_ms: Vec<u64>,
    pub sweep_windows_ms: Vec<u64>,
    pub venues: VenueConfigs,
    pub flow_compute_interval_ms: u64,
    pub markout_resolve_interval_ms: u64,
    pub sweep_compute_interval_ms: u64,
    pub toxic_compute_interval_ms: u64,
    pub telegram_enabled: bool,
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
    pub alert_dedup_window_ms: i64,
    pub alert_min_severity: ToxicSeverity,
    pub alert_require_cross_venue: bool,
    pub alert_require_markout: bool,
    pub alert_require_liquidity_drain: bool,
    pub sqlite_enabled: bool,
    pub sqlite_path: String,
    pub snapshot_persist_interval_ms: u64,
    pub raw_snapshot_enabled: bool,
    pub raw_snapshot_sample_rate_ms: u64,
    pub replay_enabled: bool,
    pub replay_report_dir: String,
    pub vpin_enabled: bool,
    pub vpin_bucket_size_btc: f64,
    pub vpin_lookback_buckets: usize,
    pub vpin_min_buckets: usize,
    pub vpin_spike_zscore: f64,
    pub vpin_high_threshold: f64,
    pub vpin_extreme_threshold: f64,
    pub vpin_persist_buckets: bool,
    pub liquidation_enabled: bool,
    pub liquidation_lookback_ms: i64,
    pub liquidation_cluster_band_bps: f64,
    pub liquidation_min_cluster_distance_bps: f64,
    pub liquidation_max_cluster_distance_bps: f64,
    pub liquidation_proximity_threshold_bps: f64,
    pub liquidation_min_cluster_touches: usize,
    pub liquidation_pressure_threshold: f64,
    pub liq_hunt_cluster_large_notional_usd: f64,
    pub liq_hunt_near_distance_bps: f64,
    pub liq_hunt_active_score: f64,
    pub liq_hunt_likely_score: f64,
    pub liq_hunt_watch_score: f64,
    pub book_stale_ms: i64,
    pub max_buffer_age_ms: i64,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_env_with_config_file("config/default")
    }

    pub fn from_env_with_config_file(config_file: &str) -> anyhow::Result<Self> {
        let settings = ::config::Config::builder()
            .add_source(::config::File::with_name("config/default").required(false))
            .add_source(::config::File::with_name(config_file).required(false))
            .build()
            .context("failed to load config/default")?;

        let read_only = bool_setting(&settings, "READ_ONLY", "read_only", true);
        if !read_only {
            return Err(anyhow!("READ_ONLY must be true"));
        }

        Ok(Self {
            app_env: string_setting(&settings, "APP_ENV", "app_env", "development"),
            read_only,
            api_host: string_setting(&settings, "API_HOST", "api_host", "127.0.0.1")
                .parse()
                .context("API_HOST must be an IP address")?,
            api_port: u16_setting(&settings, "API_PORT", "api_port", 3000)?,
            symbol: string_setting(&settings, "SYMBOL", "symbol", "BTC-PERP"),
            toxic_volume_alert_btc: f64_setting(
                &settings,
                "TOXIC_VOLUME_ALERT_BTC",
                "toxic_volume_alert_btc",
                thresholds::DEFAULT_TOXIC_VOLUME_ALERT_BTC,
            )?,
            windows_ms: parse_u64_list("WINDOWS_MS", &thresholds::DEFAULT_WINDOWS_MS),
            markout_horizons_ms: parse_u64_list("MARKOUT_HORIZONS_MS", &[1000, 5000, 15000]),
            sweep_windows_ms: parse_u64_list("SWEEP_WINDOWS_MS", &[1000, 5000, 15000]),
            venues: VenueConfigs {
                binance: VenueConfig {
                    venue: Venue::Binance,
                    enabled: bool_setting(&settings, "ENABLE_BINANCE", "enable_binance", false),
                },
                bybit: VenueConfig {
                    venue: Venue::Bybit,
                    enabled: bool_setting(&settings, "ENABLE_BYBIT", "enable_bybit", false),
                },
                okx: VenueConfig {
                    venue: Venue::Okx,
                    enabled: bool_setting(&settings, "ENABLE_OKX", "enable_okx", false),
                },
            },
            flow_compute_interval_ms: parse_u64("FLOW_COMPUTE_INTERVAL_MS", 250)?,
            markout_resolve_interval_ms: parse_u64("MARKOUT_RESOLVE_INTERVAL_MS", 250)?,
            sweep_compute_interval_ms: parse_u64("SWEEP_COMPUTE_INTERVAL_MS", 250)?,
            toxic_compute_interval_ms: parse_u64("TOXIC_COMPUTE_INTERVAL_MS", 250)?,
            telegram_enabled: parse_bool("TELEGRAM_ENABLED", false),
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default(),
            telegram_chat_id: env::var("TELEGRAM_CHAT_ID").unwrap_or_default(),
            alert_dedup_window_ms: parse_i64("ALERT_DEDUP_WINDOW_MS", 30_000)?,
            alert_min_severity: parse_toxic_severity("ALERT_MIN_SEVERITY", ToxicSeverity::Alert),
            alert_require_cross_venue: parse_bool("ALERT_REQUIRE_CROSS_VENUE", true),
            alert_require_markout: parse_bool("ALERT_REQUIRE_MARKOUT", true),
            alert_require_liquidity_drain: parse_bool("ALERT_REQUIRE_LIQUIDITY_DRAIN", false),
            sqlite_enabled: parse_bool("SQLITE_ENABLED", true),
            sqlite_path: env::var("SQLITE_PATH")
                .unwrap_or_else(|_| ".runtime/btc-toxic-flow.sqlite".to_string()),
            snapshot_persist_interval_ms: parse_u64("SNAPSHOT_PERSIST_INTERVAL_MS", 1000)?,
            raw_snapshot_enabled: parse_bool("RAW_SNAPSHOT_ENABLED", false),
            raw_snapshot_sample_rate_ms: parse_u64("RAW_SNAPSHOT_SAMPLE_RATE_MS", 1000)?,
            replay_enabled: parse_bool("REPLAY_ENABLED", true),
            replay_report_dir: env::var("REPLAY_REPORT_DIR")
                .unwrap_or_else(|_| ".runtime/reports".to_string()),
            vpin_enabled: parse_bool("VPIN_ENABLED", true),
            vpin_bucket_size_btc: parse_f64("VPIN_BUCKET_SIZE_BTC", 100.0)?,
            vpin_lookback_buckets: parse_usize("VPIN_LOOKBACK_BUCKETS", 50)?,
            vpin_min_buckets: parse_usize("VPIN_MIN_BUCKETS", 10)?,
            vpin_spike_zscore: parse_f64("VPIN_SPIKE_ZSCORE", 2.5)?,
            vpin_high_threshold: parse_f64("VPIN_HIGH_THRESHOLD", 0.70)?,
            vpin_extreme_threshold: parse_f64("VPIN_EXTREME_THRESHOLD", 0.85)?,
            vpin_persist_buckets: parse_bool("VPIN_PERSIST_BUCKETS", true),
            liquidation_enabled: parse_bool("LIQUIDATION_ENABLED", true),
            liquidation_lookback_ms: parse_i64("LIQUIDATION_LOOKBACK_MS", 120_000)?,
            liquidation_cluster_band_bps: parse_f64("LIQUIDATION_CLUSTER_BAND_BPS", 6.0)?,
            liquidation_min_cluster_distance_bps: parse_f64(
                "LIQUIDATION_MIN_CLUSTER_DISTANCE_BPS",
                5.0,
            )?,
            liquidation_max_cluster_distance_bps: parse_f64(
                "LIQUIDATION_MAX_CLUSTER_DISTANCE_BPS",
                150.0,
            )?,
            liquidation_proximity_threshold_bps: parse_f64(
                "LIQUIDATION_PROXIMITY_THRESHOLD_BPS",
                25.0,
            )?,
            liquidation_min_cluster_touches: parse_usize("LIQUIDATION_MIN_CLUSTER_TOUCHES", 3)?,
            liquidation_pressure_threshold: parse_f64("LIQUIDATION_PRESSURE_THRESHOLD", 0.65)?,
            liq_hunt_cluster_large_notional_usd: parse_f64(
                "LIQ_HUNT_CLUSTER_LARGE_NOTIONAL_USD",
                50_000_000.0,
            )?,
            liq_hunt_near_distance_bps: parse_f64("LIQ_HUNT_NEAR_DISTANCE_BPS", 25.0)?,
            liq_hunt_active_score: parse_f64("LIQ_HUNT_ACTIVE_SCORE", 75.0)?,
            liq_hunt_likely_score: parse_f64("LIQ_HUNT_LIKELY_SCORE", 50.0)?,
            liq_hunt_watch_score: parse_f64("LIQ_HUNT_WATCH_SCORE", 30.0)?,
            book_stale_ms: parse_i64("BOOK_STALE_MS", 5000)?,
            max_buffer_age_ms: parse_i64("MAX_BUFFER_AGE_MS", 120000)?,
        })
    }
}

impl AppConfig {
    pub fn config_source_label(&self) -> &'static str {
        "env_overrides_toml_overrides_defaults"
    }
}

fn string_setting(
    settings: &::config::Config,
    env_key: &str,
    toml_key: &str,
    default: &str,
) -> String {
    env::var(env_key)
        .ok()
        .or_else(|| settings.get_string(toml_key).ok())
        .unwrap_or_else(|| default.to_string())
}

fn bool_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: bool) -> bool {
    env::var(env_key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .or_else(|| settings.get_bool(toml_key).ok())
        .unwrap_or(default)
}

fn u16_setting(
    settings: &::config::Config,
    env_key: &str,
    toml_key: &str,
    default: u16,
) -> anyhow::Result<u16> {
    if let Ok(value) = env::var(env_key) {
        return Ok(value.parse::<u16>()?);
    }
    Ok(settings
        .get_int(toml_key)
        .ok()
        .and_then(|value| u16::try_from(value).ok())
        .unwrap_or(default))
}

fn f64_setting(
    settings: &::config::Config,
    env_key: &str,
    toml_key: &str,
    default: f64,
) -> anyhow::Result<f64> {
    if let Ok(value) = env::var(env_key) {
        return Ok(value.parse::<f64>()?);
    }
    Ok(settings.get_float(toml_key).unwrap_or(default))
}

fn parse_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}

fn parse_u64(key: &str, default: u64) -> anyhow::Result<u64> {
    Ok(env::var(key)
        .ok()
        .map(|v| v.parse::<u64>())
        .transpose()?
        .unwrap_or(default))
}

fn parse_i64(key: &str, default: i64) -> anyhow::Result<i64> {
    Ok(env::var(key)
        .ok()
        .map(|v| v.parse::<i64>())
        .transpose()?
        .unwrap_or(default))
}

fn parse_f64(key: &str, default: f64) -> anyhow::Result<f64> {
    Ok(env::var(key)
        .ok()
        .map(|v| v.parse::<f64>())
        .transpose()?
        .unwrap_or(default))
}

fn parse_usize(key: &str, default: usize) -> anyhow::Result<usize> {
    Ok(env::var(key)
        .ok()
        .map(|v| v.parse::<usize>())
        .transpose()?
        .unwrap_or(default))
}

fn parse_u64_list(key: &str, default: &[u64]) -> Vec<u64> {
    env::var(key)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|part| part.trim().parse::<u64>().ok())
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| default.to_vec())
}

fn parse_toxic_severity(key: &str, default: ToxicSeverity) -> ToxicSeverity {
    env::var(key)
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(ToxicSeverity::Normal),
            "watch" => Some(ToxicSeverity::Watch),
            "warning" => Some(ToxicSeverity::Warning),
            "alert" => Some(ToxicSeverity::Alert),
            "extreme" => Some(ToxicSeverity::Extreme),
            _ => None,
        })
        .unwrap_or(default)
}
