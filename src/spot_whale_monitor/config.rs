use std::{
    collections::BTreeMap,
    sync::{OnceLock, RwLock},
};

use super::types::SpotWhaleThresholds;

static GLOBAL_CONFIG: OnceLock<RwLock<SpotWhaleRuntimeConfig>> = OnceLock::new();
type ThresholdDefaults = (f64, f64, f64, f64, f64, f64);

#[derive(Debug, Clone)]
pub struct SpotWhaleRuntimeConfig {
    pub exchanges: SpotWhaleExchangeConfig,
    pub symbols: BTreeMap<String, SpotWhaleSymbolConfig>,
    pub data_quality: SpotWhaleDataQualityConfig,
}

impl SpotWhaleRuntimeConfig {
    pub fn enabled_symbols(&self) -> Vec<String> {
        self.symbols
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(symbol, _)| symbol.clone())
            .collect()
    }

    pub fn symbol_enabled(&self, symbol: &str) -> bool {
        self.symbols
            .get(&normalize_symbol(symbol))
            .map(|config| config.enabled)
            .unwrap_or(false)
    }

    pub fn thresholds_for_symbol_window(
        &self,
        symbol: &str,
        window_sec: u64,
    ) -> SpotWhaleThresholds {
        self.symbols
            .get(&normalize_symbol(symbol))
            .and_then(|config| config.thresholds.get(&window_sec).copied())
            .unwrap_or_else(|| SpotWhaleSymbolConfig::btc_default(false).thresholds[&window_sec])
    }
}

impl Default for SpotWhaleRuntimeConfig {
    fn default() -> Self {
        let mut symbols = BTreeMap::new();
        symbols.insert("BTC".to_string(), SpotWhaleSymbolConfig::btc_default(true));
        symbols.insert("ETH".to_string(), SpotWhaleSymbolConfig::eth_default(true));
        Self {
            exchanges: SpotWhaleExchangeConfig::default(),
            symbols,
            data_quality: SpotWhaleDataQualityConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpotWhaleExchangeConfig {
    pub binance_enabled: bool,
    pub coinbase_enabled: bool,
    pub bitfinex_enabled: bool,
}

impl Default for SpotWhaleExchangeConfig {
    fn default() -> Self {
        Self {
            binance_enabled: true,
            coinbase_enabled: true,
            bitfinex_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpotWhaleSymbolConfig {
    pub enabled: bool,
    pub thresholds: BTreeMap<u64, SpotWhaleThresholds>,
}

impl SpotWhaleSymbolConfig {
    fn btc_default(enabled: bool) -> Self {
        Self::custom_default(
            enabled,
            [
                (
                    5,
                    (
                        180.0,
                        420.0,
                        800.0,
                        15_000_000.0,
                        35_000_000.0,
                        75_000_000.0,
                    ),
                ),
                (
                    15,
                    (
                        300.0,
                        700.0,
                        1_200.0,
                        25_000_000.0,
                        60_000_000.0,
                        120_000_000.0,
                    ),
                ),
                (
                    60,
                    (
                        800.0,
                        1_500.0,
                        3_000.0,
                        60_000_000.0,
                        120_000_000.0,
                        240_000_000.0,
                    ),
                ),
            ],
        )
    }

    fn eth_default(enabled: bool) -> Self {
        Self::custom_default(
            enabled,
            [
                (
                    5,
                    (
                        2_000.0,
                        4_500.0,
                        8_000.0,
                        8_000_000.0,
                        20_000_000.0,
                        40_000_000.0,
                    ),
                ),
                (
                    15,
                    (
                        4_000.0,
                        9_000.0,
                        16_000.0,
                        15_000_000.0,
                        40_000_000.0,
                        80_000_000.0,
                    ),
                ),
                (
                    60,
                    (
                        10_000.0,
                        22_000.0,
                        45_000.0,
                        40_000_000.0,
                        80_000_000.0,
                        160_000_000.0,
                    ),
                ),
            ],
        )
    }

    fn custom_default<const N: usize>(
        enabled: bool,
        values: [(u64, ThresholdDefaults); N],
    ) -> Self {
        let thresholds = values
            .into_iter()
            .map(
                |(
                    window_sec,
                    (
                        high_base,
                        critical_base,
                        s_base,
                        high_notional_usd,
                        critical_notional_usd,
                        s_notional_usd,
                    ),
                )| {
                    (
                        window_sec,
                        SpotWhaleThresholds {
                            high_base,
                            critical_base,
                            s_base,
                            high_notional_usd,
                            critical_notional_usd,
                            s_notional_usd,
                        },
                    )
                },
            )
            .collect();
        Self {
            enabled,
            thresholds,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpotWhaleDataQualityConfig {
    pub min_discord_quality: u8,
    pub warmup_ms: i64,
    pub single_exchange_penalty: u8,
    pub heartbeat_stale_ms: i64,
}

impl Default for SpotWhaleDataQualityConfig {
    fn default() -> Self {
        Self {
            min_discord_quality: 70,
            warmup_ms: 60_000,
            single_exchange_penalty: 20,
            heartbeat_stale_ms: 45_000,
        }
    }
}

pub fn spot_whale_runtime_config() -> SpotWhaleRuntimeConfig {
    global_config()
        .read()
        .expect("spot whale config lock poisoned")
        .clone()
}

pub fn set_spot_whale_runtime_config(config: SpotWhaleRuntimeConfig) {
    *global_config()
        .write()
        .expect("spot whale config lock poisoned") = config;
}

pub fn load_spot_whale_runtime_config_from_settings(
    settings: &::config::Config,
) -> SpotWhaleRuntimeConfig {
    SpotWhaleRuntimeConfig {
        exchanges: SpotWhaleExchangeConfig {
            binance_enabled: bool_setting(
                settings,
                "SPOT_WHALE_BINANCE_ENABLED",
                "spot_whale_monitor.exchanges.binance.enabled",
                true,
            ),
            coinbase_enabled: bool_setting(
                settings,
                "SPOT_WHALE_COINBASE_ENABLED",
                "spot_whale_monitor.exchanges.coinbase.enabled",
                true,
            ),
            bitfinex_enabled: bool_setting(
                settings,
                "SPOT_WHALE_BITFINEX_ENABLED",
                "spot_whale_monitor.exchanges.bitfinex.enabled",
                true,
            ),
        },
        symbols: load_symbol_configs(settings),
        data_quality: SpotWhaleDataQualityConfig {
            min_discord_quality: u8_setting(
                settings,
                "spot_whale_monitor.data_quality.min_discord_quality",
                70,
            ),
            warmup_ms: i64_setting(
                settings,
                "spot_whale_monitor.data_quality.warmup_ms",
                60_000,
            ),
            single_exchange_penalty: u8_setting(
                settings,
                "spot_whale_monitor.data_quality.single_exchange_penalty",
                20,
            ),
            heartbeat_stale_ms: i64_setting(
                settings,
                "spot_whale_monitor.data_quality.heartbeat_stale_ms",
                45_000,
            ),
        },
    }
}

fn global_config() -> &'static RwLock<SpotWhaleRuntimeConfig> {
    GLOBAL_CONFIG.get_or_init(|| RwLock::new(SpotWhaleRuntimeConfig::default()))
}

fn load_symbol_configs(settings: &::config::Config) -> BTreeMap<String, SpotWhaleSymbolConfig> {
    let mut symbols = SpotWhaleRuntimeConfig::default().symbols;
    if let Ok(table) = settings.get_table("spot_whale_monitor.symbols") {
        for symbol in table.keys() {
            let symbol_key = normalize_symbol(symbol);
            let path = format!("spot_whale_monitor.symbols.{symbol}");
            let enabled = settings
                .get_bool(&format!("{path}.enabled"))
                .unwrap_or(matches!(symbol_key.as_str(), "BTC" | "ETH"));
            let fallback = symbols
                .get(&symbol_key)
                .cloned()
                .unwrap_or_else(|| SpotWhaleSymbolConfig::btc_default(false));
            symbols.insert(
                symbol_key,
                SpotWhaleSymbolConfig {
                    enabled,
                    thresholds: load_symbol_thresholds(settings, &path, &fallback.thresholds),
                },
            );
        }
    }
    symbols
}

fn load_symbol_thresholds(
    settings: &::config::Config,
    path: &str,
    fallback: &BTreeMap<u64, SpotWhaleThresholds>,
) -> BTreeMap<u64, SpotWhaleThresholds> {
    [5_u64, 15, 60]
        .into_iter()
        .map(|window_sec| {
            let default = fallback.get(&window_sec).copied().unwrap_or_else(|| {
                SpotWhaleSymbolConfig::btc_default(false).thresholds[&window_sec]
            });
            (
                window_sec,
                SpotWhaleThresholds {
                    high_base: positive_float_setting(
                        settings,
                        &format!("{path}.thresholds_base.high.{window_sec}"),
                        default.high_base,
                    ),
                    critical_base: positive_float_setting(
                        settings,
                        &format!("{path}.thresholds_base.critical.{window_sec}"),
                        default.critical_base,
                    ),
                    s_base: positive_float_setting(
                        settings,
                        &format!("{path}.thresholds_base.s.{window_sec}"),
                        default.s_base,
                    ),
                    high_notional_usd: positive_float_setting(
                        settings,
                        &format!("{path}.thresholds_usd.high.{window_sec}"),
                        default.high_notional_usd,
                    ),
                    critical_notional_usd: positive_float_setting(
                        settings,
                        &format!("{path}.thresholds_usd.critical.{window_sec}"),
                        default.critical_notional_usd,
                    ),
                    s_notional_usd: positive_float_setting(
                        settings,
                        &format!("{path}.thresholds_usd.s.{window_sec}"),
                        default.s_notional_usd,
                    ),
                },
            )
        })
        .collect()
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().trim_end_matches("-SPOT").to_ascii_uppercase()
}

fn bool_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: bool) -> bool {
    std::env::var(env_key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .or_else(|| settings.get_bool(toml_key).ok())
        .unwrap_or(default)
}

fn positive_float_setting(settings: &::config::Config, path: &str, default: f64) -> f64 {
    match settings.get_float(path) {
        Ok(value) if value.is_finite() && value > 0.0 => value,
        _ => default,
    }
}

fn i64_setting(settings: &::config::Config, path: &str, default: i64) -> i64 {
    settings
        .get_int(path)
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn u8_setting(settings: &::config::Config, path: &str, default: u8) -> u8 {
    settings
        .get_int(path)
        .ok()
        .and_then(|value| u8::try_from(value).ok())
        .filter(|value| *value <= 100)
        .unwrap_or(default)
}
