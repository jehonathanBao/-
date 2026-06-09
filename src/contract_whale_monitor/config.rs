use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{OnceLock, RwLock},
};

use super::{types::ContractWhaleThresholds, LOG_PREFIX, LOG_TARGET};

const DEFAULT_VOLUME_STRENGTH_WEIGHT: f64 = 35.0;
const DEFAULT_DYNAMIC_MULTIPLE_WEIGHT: f64 = 20.0;
const DEFAULT_DOMINANCE_WEIGHT: f64 = 15.0;
const DEFAULT_PRICE_IMPACT_WEIGHT: f64 = 15.0;
const DEFAULT_MULTI_EXCHANGE_WEIGHT: f64 = 10.0;
const DEFAULT_DATA_QUALITY_WEIGHT: f64 = 5.0;
const DEFAULT_SINGLE_EXCHANGE_PENALTY: f64 = 10.0;
const DEFAULT_LIQUIDATION_PENALTY: f64 = 10.0;
const DEFAULT_WS_LATENCY_PENALTY: f64 = 15.0;
const DEFAULT_WARMUP_PENALTY: f64 = 20.0;
const DEFAULT_PRICE_ANOMALY_PENALTY: f64 = 20.0;
const DEFAULT_WS_LATENCY_HIGH_MS: i64 = 1_000;
const DEFAULT_WARMUP_MS: i64 = 60_000;
const DEFAULT_MIN_DYNAMIC_SAMPLES: usize = 20;
const DEFAULT_SINGLE_EXCHANGE_DQ_PENALTY: u8 = 15;
const DEFAULT_CT_VAL_MISSING_DQ_PENALTY: u8 = 20;
const DEFAULT_FLOW_1S_RETENTION_DAYS: i64 = 14;
const DEFAULT_SIGNAL_RETENTION_DAYS: i64 = 365;

static GLOBAL_CONFIG: OnceLock<RwLock<ContractWhaleRuntimeConfig>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ContractWhaleRuntimeConfig {
    pub exchanges: ContractWhaleExchangeConfig,
    pub scoring: ContractWhaleScoringConfig,
    pub symbols: BTreeMap<String, ContractWhaleSymbolConfig>,
    pub data_quality: ContractWhaleDataQualityConfig,
    pub retention: ContractWhaleRetentionConfig,
}

impl ContractWhaleRuntimeConfig {
    pub fn thresholds_for_symbol_window(
        &self,
        symbol: &str,
        window_sec: u64,
    ) -> ContractWhaleThresholds {
        if normalize_symbol(symbol) == "BTC"
            && self.threshold_profile() == ContractWhaleThresholdProfile::BinanceBitfinex
        {
            return ContractWhaleThresholds::binance_bitfinex_for_window(window_sec);
        }
        self.symbols
            .get(&normalize_symbol(symbol))
            .and_then(|symbol_config| symbol_config.thresholds_btc.get(&window_sec).copied())
            .unwrap_or_else(|| ContractWhaleThresholds::for_window(window_sec))
    }

    pub fn symbol_enabled(&self, symbol: &str) -> bool {
        self.symbols
            .get(&normalize_symbol(symbol))
            .map(|config| config.enabled)
            .unwrap_or(false)
    }

    pub fn exchange_enabled(&self, exchange: &str) -> bool {
        match exchange.to_ascii_lowercase().as_str() {
            "binance" => self.exchanges.binance_enabled,
            "okx" => self.exchanges.okx_enabled,
            "bitfinex" => self.exchanges.bitfinex_enabled,
            _ => false,
        }
    }

    pub fn enabled_exchanges(&self) -> Vec<String> {
        ["binance", "okx", "bitfinex"]
            .into_iter()
            .filter(|exchange| self.exchange_enabled(exchange))
            .map(ToString::to_string)
            .collect()
    }

    pub fn active_exchange_count(&self) -> usize {
        self.enabled_exchanges().len()
    }

    pub fn disabled_exchanges(&self) -> Vec<String> {
        ["binance", "okx", "bitfinex"]
            .into_iter()
            .filter(|exchange| !self.exchange_enabled(exchange))
            .map(ToString::to_string)
            .collect()
    }

    pub fn enabled_exchange_set(&self) -> BTreeSet<String> {
        self.enabled_exchanges().into_iter().collect()
    }

    pub fn threshold_profile(&self) -> ContractWhaleThresholdProfile {
        if self.exchanges.binance_enabled
            && self.exchanges.bitfinex_enabled
            && !self.exchanges.okx_enabled
        {
            ContractWhaleThresholdProfile::BinanceBitfinex
        } else {
            ContractWhaleThresholdProfile::ThreeExchange
        }
    }

    pub fn threshold_profile_key(&self) -> &'static str {
        self.threshold_profile().as_key()
    }

    pub fn notional_thresholds_usd(&self) -> ContractWhaleNotionalThresholds {
        self.threshold_profile().notional_thresholds_usd()
    }
}

impl Default for ContractWhaleRuntimeConfig {
    fn default() -> Self {
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "BTC".to_string(),
            ContractWhaleSymbolConfig::btc_default(true),
        );
        symbols.insert(
            "ETH".to_string(),
            ContractWhaleSymbolConfig::eth_default(true),
        );
        symbols.insert(
            "SOL".to_string(),
            ContractWhaleSymbolConfig::sol_default(false),
        );
        Self {
            exchanges: ContractWhaleExchangeConfig::default(),
            scoring: ContractWhaleScoringConfig::default(),
            symbols,
            data_quality: ContractWhaleDataQualityConfig::default(),
            retention: ContractWhaleRetentionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractWhaleThresholdProfile {
    ThreeExchange,
    BinanceBitfinex,
}

impl ContractWhaleThresholdProfile {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::ThreeExchange => "three_exchange",
            Self::BinanceBitfinex => "binance_bitfinex",
        }
    }

    pub fn notional_thresholds_usd(self) -> ContractWhaleNotionalThresholds {
        match self {
            Self::ThreeExchange => ContractWhaleNotionalThresholds {
                high: 50_000_000.0,
                critical: 120_000_000.0,
                s: 250_000_000.0,
            },
            Self::BinanceBitfinex => ContractWhaleNotionalThresholds {
                high: 40_000_000.0,
                critical: 95_000_000.0,
                s: 200_000_000.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ContractWhaleNotionalThresholds {
    pub high: f64,
    pub critical: f64,
    pub s: f64,
}

#[derive(Debug, Clone)]
pub struct ContractWhaleExchangeConfig {
    pub binance_enabled: bool,
    pub okx_enabled: bool,
    pub bitfinex_enabled: bool,
}

impl Default for ContractWhaleExchangeConfig {
    fn default() -> Self {
        Self {
            binance_enabled: true,
            okx_enabled: false,
            bitfinex_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleScoringConfig {
    pub volume_strength_weight: f64,
    pub dynamic_multiple_weight: f64,
    pub dominance_weight: f64,
    pub price_impact_weight: f64,
    pub multi_exchange_weight: f64,
    pub data_quality_weight: f64,
    pub penalties: ContractWhaleScoringPenalties,
}

impl Default for ContractWhaleScoringConfig {
    fn default() -> Self {
        Self {
            volume_strength_weight: DEFAULT_VOLUME_STRENGTH_WEIGHT,
            dynamic_multiple_weight: DEFAULT_DYNAMIC_MULTIPLE_WEIGHT,
            dominance_weight: DEFAULT_DOMINANCE_WEIGHT,
            price_impact_weight: DEFAULT_PRICE_IMPACT_WEIGHT,
            multi_exchange_weight: DEFAULT_MULTI_EXCHANGE_WEIGHT,
            data_quality_weight: DEFAULT_DATA_QUALITY_WEIGHT,
            penalties: ContractWhaleScoringPenalties::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleScoringPenalties {
    pub single_exchange_only: f64,
    pub liquidation_suspected: f64,
    pub websocket_latency_high: f64,
    pub warmup_period: f64,
    pub price_jump_anomaly: f64,
}

impl Default for ContractWhaleScoringPenalties {
    fn default() -> Self {
        Self {
            single_exchange_only: DEFAULT_SINGLE_EXCHANGE_PENALTY,
            liquidation_suspected: DEFAULT_LIQUIDATION_PENALTY,
            websocket_latency_high: DEFAULT_WS_LATENCY_PENALTY,
            warmup_period: DEFAULT_WARMUP_PENALTY,
            price_jump_anomaly: DEFAULT_PRICE_ANOMALY_PENALTY,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleSymbolConfig {
    pub enabled: bool,
    pub thresholds_btc: BTreeMap<u64, ContractWhaleThresholds>,
}

impl ContractWhaleSymbolConfig {
    fn btc_default(enabled: bool) -> Self {
        let thresholds_btc = [5_u64, 15, 60]
            .into_iter()
            .map(|window_sec| (window_sec, ContractWhaleThresholds::for_window(window_sec)))
            .collect();
        Self {
            enabled,
            thresholds_btc,
        }
    }

    fn eth_default(enabled: bool) -> Self {
        Self::custom_default(
            enabled,
            [
                (5, (5_000.0, 10_000.0, 18_000.0)),
                (15, (9_000.0, 18_000.0, 32_000.0)),
                (60, (20_000.0, 40_000.0, 75_000.0)),
            ],
        )
    }

    fn sol_default(enabled: bool) -> Self {
        Self::custom_default(
            enabled,
            [
                (5, (100_000.0, 220_000.0, 400_000.0)),
                (15, (220_000.0, 450_000.0, 800_000.0)),
                (60, (650_000.0, 1_200_000.0, 2_200_000.0)),
            ],
        )
    }

    fn custom_default<const N: usize>(enabled: bool, values: [(u64, (f64, f64, f64)); N]) -> Self {
        let thresholds_btc = values
            .into_iter()
            .map(|(window_sec, (high_btc, critical_btc, s_btc))| {
                (
                    window_sec,
                    ContractWhaleThresholds {
                        high_btc,
                        critical_btc,
                        s_btc,
                    },
                )
            })
            .collect();
        Self {
            enabled,
            thresholds_btc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleRetentionConfig {
    pub flow_1s_days: i64,
    pub signals_days: i64,
}

impl Default for ContractWhaleRetentionConfig {
    fn default() -> Self {
        Self {
            flow_1s_days: DEFAULT_FLOW_1S_RETENTION_DAYS,
            signals_days: DEFAULT_SIGNAL_RETENTION_DAYS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleDataQualityConfig {
    pub min_discord_quality: u8,
    pub min_critical_quality: u8,
    pub high_latency_ms: i64,
    pub warmup_ms: i64,
    pub min_dynamic_samples: usize,
    pub single_exchange_penalty: u8,
    pub ct_val_missing_penalty: u8,
}

impl Default for ContractWhaleDataQualityConfig {
    fn default() -> Self {
        Self {
            min_discord_quality: 70,
            min_critical_quality: 70,
            high_latency_ms: DEFAULT_WS_LATENCY_HIGH_MS,
            warmup_ms: DEFAULT_WARMUP_MS,
            min_dynamic_samples: DEFAULT_MIN_DYNAMIC_SAMPLES,
            single_exchange_penalty: DEFAULT_SINGLE_EXCHANGE_DQ_PENALTY,
            ct_val_missing_penalty: DEFAULT_CT_VAL_MISSING_DQ_PENALTY,
        }
    }
}

pub fn contract_whale_runtime_config() -> ContractWhaleRuntimeConfig {
    global_config()
        .read()
        .expect("cwm config lock poisoned")
        .clone()
}

pub fn set_contract_whale_runtime_config(config: ContractWhaleRuntimeConfig) {
    *global_config().write().expect("cwm config lock poisoned") = config;
}

pub fn reset_contract_whale_runtime_config() {
    set_contract_whale_runtime_config(ContractWhaleRuntimeConfig::default());
}

pub fn load_contract_whale_runtime_config_from_settings(
    settings: &::config::Config,
) -> ContractWhaleRuntimeConfig {
    ContractWhaleRuntimeConfig {
        exchanges: load_exchange_config(settings),
        scoring: load_scoring_config(settings),
        data_quality: load_data_quality_config(settings),
        symbols: load_symbol_configs(settings),
        retention: load_retention_config(settings),
    }
}

fn load_exchange_config(settings: &::config::Config) -> ContractWhaleExchangeConfig {
    ContractWhaleExchangeConfig {
        binance_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_BINANCE_ENABLED",
            "contract_whale_monitor.exchanges.binance.enabled",
            bool_setting(settings, "ENABLE_BINANCE", "enable_binance", true),
        ),
        okx_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_OKX_ENABLED",
            "contract_whale_monitor.exchanges.okx.enabled",
            bool_setting(settings, "ENABLE_OKX", "enable_okx", false),
        ),
        bitfinex_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_BITFINEX_ENABLED",
            "contract_whale_monitor.exchanges.bitfinex.enabled",
            true,
        ),
    }
}

fn global_config() -> &'static RwLock<ContractWhaleRuntimeConfig> {
    GLOBAL_CONFIG.get_or_init(|| RwLock::new(ContractWhaleRuntimeConfig::default()))
}

fn load_scoring_config(settings: &::config::Config) -> ContractWhaleScoringConfig {
    let defaults = ContractWhaleScoringConfig::default();
    ContractWhaleScoringConfig {
        volume_strength_weight: positive_float_setting(
            settings,
            "contract_whale_monitor.scoring.volume_strength_weight",
            defaults.volume_strength_weight,
        ),
        dynamic_multiple_weight: positive_float_setting(
            settings,
            "contract_whale_monitor.scoring.dynamic_multiple_weight",
            defaults.dynamic_multiple_weight,
        ),
        dominance_weight: positive_float_setting(
            settings,
            "contract_whale_monitor.scoring.dominance_weight",
            defaults.dominance_weight,
        ),
        price_impact_weight: positive_float_setting(
            settings,
            "contract_whale_monitor.scoring.price_impact_weight",
            defaults.price_impact_weight,
        ),
        multi_exchange_weight: positive_float_setting(
            settings,
            "contract_whale_monitor.scoring.multi_exchange_weight",
            defaults.multi_exchange_weight,
        ),
        data_quality_weight: positive_float_setting(
            settings,
            "contract_whale_monitor.scoring.data_quality_weight",
            defaults.data_quality_weight,
        ),
        penalties: ContractWhaleScoringPenalties {
            single_exchange_only: non_negative_float_setting(
                settings,
                "contract_whale_monitor.scoring.penalties.single_exchange_only",
                defaults.penalties.single_exchange_only,
            ),
            liquidation_suspected: non_negative_float_setting(
                settings,
                "contract_whale_monitor.scoring.penalties.liquidation_suspected",
                defaults.penalties.liquidation_suspected,
            ),
            websocket_latency_high: non_negative_float_setting(
                settings,
                "contract_whale_monitor.scoring.penalties.websocket_latency_high",
                defaults.penalties.websocket_latency_high,
            ),
            warmup_period: non_negative_float_setting(
                settings,
                "contract_whale_monitor.scoring.penalties.warmup_period",
                defaults.penalties.warmup_period,
            ),
            price_jump_anomaly: non_negative_float_setting(
                settings,
                "contract_whale_monitor.scoring.penalties.price_jump_anomaly",
                defaults.penalties.price_jump_anomaly,
            ),
        },
    }
}

fn load_data_quality_config(settings: &::config::Config) -> ContractWhaleDataQualityConfig {
    let defaults = ContractWhaleDataQualityConfig::default();
    ContractWhaleDataQualityConfig {
        min_discord_quality: u8_setting(
            settings,
            "contract_whale_monitor.data_quality.min_discord_quality",
            defaults.min_discord_quality,
        ),
        min_critical_quality: u8_setting(
            settings,
            "contract_whale_monitor.data_quality.min_critical_quality",
            defaults.min_critical_quality,
        ),
        high_latency_ms: i64_setting(
            settings,
            "contract_whale_monitor.data_quality.high_latency_ms",
            defaults.high_latency_ms,
        ),
        warmup_ms: i64_setting(
            settings,
            "contract_whale_monitor.data_quality.warmup_ms",
            defaults.warmup_ms,
        ),
        min_dynamic_samples: usize_setting(
            settings,
            "contract_whale_monitor.data_quality.min_dynamic_samples",
            defaults.min_dynamic_samples,
        ),
        single_exchange_penalty: u8_setting(
            settings,
            "contract_whale_monitor.data_quality.single_exchange_penalty",
            defaults.single_exchange_penalty,
        ),
        ct_val_missing_penalty: u8_setting(
            settings,
            "contract_whale_monitor.data_quality.ct_val_missing_penalty",
            defaults.ct_val_missing_penalty,
        ),
    }
}

fn load_retention_config(settings: &::config::Config) -> ContractWhaleRetentionConfig {
    let defaults = ContractWhaleRetentionConfig::default();
    ContractWhaleRetentionConfig {
        flow_1s_days: i64_setting(
            settings,
            "contract_whale_monitor.retention.flow_1s_days",
            defaults.flow_1s_days,
        ),
        signals_days: i64_setting(
            settings,
            "contract_whale_monitor.retention.signals_days",
            defaults.signals_days,
        ),
    }
}

fn load_symbol_configs(settings: &::config::Config) -> BTreeMap<String, ContractWhaleSymbolConfig> {
    let mut symbols = ContractWhaleRuntimeConfig::default().symbols;
    if let Ok(table) = settings.get_table("contract_whale_monitor.symbols") {
        for symbol in table.keys() {
            let symbol_key = normalize_symbol(symbol);
            let path = format!("contract_whale_monitor.symbols.{symbol}");
            let default_enabled = matches!(symbol_key.as_str(), "BTC" | "ETH");
            let enabled = settings
                .get_bool(&format!("{path}.enabled"))
                .unwrap_or(default_enabled);
            let fallback_thresholds = symbols
                .get(&symbol_key)
                .map(|config| config.thresholds_btc.clone())
                .unwrap_or_else(|| ContractWhaleSymbolConfig::btc_default(false).thresholds_btc);
            symbols.insert(
                symbol_key,
                ContractWhaleSymbolConfig {
                    enabled,
                    thresholds_btc: load_symbol_thresholds(settings, &path, &fallback_thresholds),
                },
            );
        }
    }
    symbols
}

fn load_symbol_thresholds(
    settings: &::config::Config,
    symbol_path: &str,
    fallback: &BTreeMap<u64, ContractWhaleThresholds>,
) -> BTreeMap<u64, ContractWhaleThresholds> {
    [5_u64, 15, 60]
        .into_iter()
        .map(|window_sec| {
            let default = fallback
                .get(&window_sec)
                .copied()
                .unwrap_or_else(|| ContractWhaleThresholds::for_window(window_sec));
            (
                window_sec,
                ContractWhaleThresholds {
                    high_btc: positive_float_setting(
                        settings,
                        &format!("{symbol_path}.thresholds_btc.high.{window_sec}"),
                        default.high_btc,
                    ),
                    critical_btc: positive_float_setting(
                        settings,
                        &format!("{symbol_path}.thresholds_btc.critical.{window_sec}"),
                        default.critical_btc,
                    ),
                    s_btc: positive_float_setting(
                        settings,
                        &format!("{symbol_path}.thresholds_btc.s.{window_sec}"),
                        default.s_btc,
                    ),
                },
            )
        })
        .collect()
}

fn positive_float_setting(settings: &::config::Config, path: &str, default: f64) -> f64 {
    match settings.get_float(path) {
        Ok(value) if value.is_finite() && value > 0.0 => value,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn non_negative_float_setting(settings: &::config::Config, path: &str, default: f64) -> f64 {
    match settings.get_float(path) {
        Ok(value) if value.is_finite() && value >= 0.0 => value,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
}

fn u8_setting(settings: &::config::Config, path: &str, default: u8) -> u8 {
    match settings.get_int(path) {
        Ok(value) if (0..=100).contains(&value) => value as u8,
        Ok(value) => {
            warn_invalid(path, value, default);
            default
        }
        Err(_) => default,
    }
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

fn bool_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: bool) -> bool {
    std::env::var(env_key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .or_else(|| settings.get_bool(toml_key).ok())
        .unwrap_or(default)
}

fn warn_invalid<T: std::fmt::Display, D: std::fmt::Display>(path: &str, value: T, default: D) {
    tracing::warn!(
        target: LOG_TARGET,
        path,
        value = %value,
        default = %default,
        "{} invalid config value, using default",
        LOG_PREFIX
    );
}

fn normalize_symbol(symbol: &str) -> String {
    symbol
        .trim()
        .split(['-', '_', '/', ':'])
        .next()
        .unwrap_or(symbol)
        .to_ascii_uppercase()
}
