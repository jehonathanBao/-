use super::super::{LOG_PREFIX, LOG_TARGET};

#[derive(Debug, Clone, PartialEq)]
pub struct HourlyDeltaAlertConfig {
    pub enabled: bool,
    pub exchange: String,
    pub symbol: String,
    pub interval: String,
    pub threshold_btc: f64,
    pub close_grace_seconds: u64,
    pub startup_backfill_hours: u32,
    pub discord_enabled: bool,
    pub dry_run: bool,
    pub outbox_poll_interval_ms: u64,
    pub outbox_batch_size: usize,
    pub outbox_max_attempts: usize,
    pub outbox_base_retry_seconds: i64,
    pub outbox_max_retry_seconds: i64,
    pub rest_retry_max: u32,
    pub rest_retry_base_ms: u64,
}

impl Default for HourlyDeltaAlertConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            exchange: "binance".to_string(),
            symbol: "BTCUSDT".to_string(),
            interval: "1h".to_string(),
            threshold_btc: 1000.0,
            close_grace_seconds: 5,
            startup_backfill_hours: 2,
            discord_enabled: true,
            dry_run: true,
            outbox_poll_interval_ms: 1_000,
            outbox_batch_size: 10,
            outbox_max_attempts: 6,
            outbox_base_retry_seconds: 2,
            outbox_max_retry_seconds: 300,
            rest_retry_max: 5,
            rest_retry_base_ms: 500,
        }
    }
}

impl HourlyDeltaAlertConfig {
    pub fn matches_stream(&self, exchange: &str, symbol: &str, interval: &str) -> bool {
        exchange.eq_ignore_ascii_case(&self.exchange)
            && symbol.eq_ignore_ascii_case(&self.symbol)
            && interval.eq_ignore_ascii_case(&self.interval)
    }

    pub fn effective_dry_run(&self, parent_dry_run: bool) -> bool {
        self.dry_run || parent_dry_run
    }
}

pub fn load_hourly_delta_alert_config_from_settings(
    settings: &::config::Config,
) -> HourlyDeltaAlertConfig {
    let defaults = HourlyDeltaAlertConfig::default();
    HourlyDeltaAlertConfig {
        enabled: bool_setting(
            settings,
            "HOURLY_DELTA_ALERT_ENABLED",
            "contract_whale_monitor.hourly_delta_alert.enabled",
            defaults.enabled,
        ),
        exchange: string_setting(
            settings,
            "contract_whale_monitor.hourly_delta_alert.exchange",
            &defaults.exchange,
        ),
        symbol: string_setting(
            settings,
            "contract_whale_monitor.hourly_delta_alert.symbol",
            &defaults.symbol,
        )
        .to_ascii_uppercase(),
        interval: string_setting(
            settings,
            "contract_whale_monitor.hourly_delta_alert.interval",
            &defaults.interval,
        )
        .to_ascii_lowercase(),
        threshold_btc: f64_setting(
            settings,
            "HOURLY_DELTA_ALERT_THRESHOLD_BTC",
            "contract_whale_monitor.hourly_delta_alert.threshold_btc",
            defaults.threshold_btc,
        )
        .max(0.0),
        close_grace_seconds: u64_setting(
            settings,
            "HOURLY_DELTA_ALERT_CLOSE_GRACE_SECONDS",
            "contract_whale_monitor.hourly_delta_alert.close_grace_seconds",
            defaults.close_grace_seconds,
        )
        .clamp(0, 60),
        startup_backfill_hours: u32_setting(
            settings,
            "HOURLY_DELTA_ALERT_STARTUP_BACKFILL_HOURS",
            "contract_whale_monitor.hourly_delta_alert.startup_backfill_hours",
            defaults.startup_backfill_hours,
        )
        .clamp(1, 24),
        discord_enabled: bool_setting(
            settings,
            "HOURLY_DELTA_ALERT_DISCORD_ENABLED",
            "contract_whale_monitor.hourly_delta_alert.discord_enabled",
            defaults.discord_enabled,
        ),
        dry_run: bool_setting(
            settings,
            "HOURLY_DELTA_ALERT_DRY_RUN",
            "contract_whale_monitor.hourly_delta_alert.dry_run",
            defaults.dry_run,
        ),
        outbox_poll_interval_ms: u64_setting(
            settings,
            "HOURLY_DELTA_ALERT_OUTBOX_POLL_INTERVAL_MS",
            "contract_whale_monitor.hourly_delta_alert.outbox_poll_interval_ms",
            defaults.outbox_poll_interval_ms,
        )
        .clamp(100, 60_000),
        outbox_batch_size: usize_setting(
            settings,
            "contract_whale_monitor.hourly_delta_alert.outbox_batch_size",
            defaults.outbox_batch_size,
        )
        .clamp(1, 100),
        outbox_max_attempts: usize_setting(
            settings,
            "contract_whale_monitor.hourly_delta_alert.outbox_max_attempts",
            defaults.outbox_max_attempts,
        )
        .clamp(1, 10),
        outbox_base_retry_seconds: i64_setting(
            settings,
            "contract_whale_monitor.hourly_delta_alert.outbox_base_retry_seconds",
            defaults.outbox_base_retry_seconds,
        )
        .clamp(1, 60),
        outbox_max_retry_seconds: i64_setting(
            settings,
            "contract_whale_monitor.hourly_delta_alert.outbox_max_retry_seconds",
            defaults.outbox_max_retry_seconds,
        )
        .clamp(1, 3_600),
        rest_retry_max: u32_setting(
            settings,
            "HOURLY_DELTA_ALERT_REST_RETRY_MAX",
            "contract_whale_monitor.hourly_delta_alert.rest_retry_max",
            defaults.rest_retry_max,
        )
        .clamp(1, 20),
        rest_retry_base_ms: u64_setting(
            settings,
            "HOURLY_DELTA_ALERT_REST_RETRY_BASE_MS",
            "contract_whale_monitor.hourly_delta_alert.rest_retry_base_ms",
            defaults.rest_retry_base_ms,
        )
        .clamp(100, 10_000),
    }
}

fn bool_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: bool) -> bool {
    std::env::var(env_key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .or_else(|| settings.get_bool(toml_key).ok())
        .unwrap_or(default)
}

fn string_setting(settings: &::config::Config, path: &str, default: &str) -> String {
    settings
        .get_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn f64_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: f64) -> f64 {
    if let Ok(value) = std::env::var(env_key) {
        if let Ok(parsed) = value.parse::<f64>() {
            if parsed.is_finite() {
                return parsed;
            }
        }
        warn_invalid(toml_key, value, default);
        return default;
    }
    match settings.get_float(toml_key) {
        Ok(value) if value.is_finite() => value,
        Ok(value) => {
            warn_invalid(toml_key, value, default);
            default
        }
        Err(_) => default,
    }
}

fn u64_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: u64) -> u64 {
    if let Ok(value) = std::env::var(env_key) {
        if let Ok(parsed) = value.parse::<u64>() {
            return parsed;
        }
        warn_invalid(toml_key, value, default);
        return default;
    }
    match settings.get_int(toml_key) {
        Ok(value) if value >= 0 => value as u64,
        Ok(value) => {
            warn_invalid(toml_key, value, default);
            default
        }
        Err(_) => default,
    }
}

fn u32_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: u32) -> u32 {
    u64_setting(settings, env_key, toml_key, default as u64).min(u32::MAX as u64) as u32
}

fn i64_setting(settings: &::config::Config, path: &str, default: i64) -> i64 {
    match settings.get_int(path) {
        Ok(value) if value >= 0 => value,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn usize_setting(settings: &::config::Config, path: &str, default: usize) -> usize {
    match settings.get_int(path) {
        Ok(value) if value > 0 => value as usize,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn warn_invalid<T: std::fmt::Display, D: std::fmt::Display>(path: &str, value: T, default: D) {
    tracing::warn!(
        target: LOG_TARGET,
        path,
        value = %value,
        default = %default,
        "{} invalid hourly_delta_alert config value, using default",
        LOG_PREFIX
    );
}
