use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{OnceLock, RwLock},
};

use super::{
    types::{ContractWhaleMarketType, ContractWhaleSourceRole, ContractWhaleThresholds},
    LOG_PREFIX, LOG_TARGET,
};

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
        self.exchange_platform(exchange)
            .is_some_and(ContractWhalePlatformConfig::contract_markets_enabled)
    }

    pub fn enabled_exchanges(&self) -> Vec<String> {
        self.threshold_participating_exchanges()
            .into_iter()
            .filter(|exchange| self.exchange_enabled(exchange))
            .collect()
    }

    pub fn active_exchange_count(&self) -> usize {
        self.enabled_exchanges().len()
    }

    pub fn disabled_exchanges(&self) -> Vec<String> {
        self.threshold_participating_exchanges()
            .into_iter()
            .filter(|exchange| !self.exchange_enabled(exchange))
            .collect()
    }

    pub fn enabled_exchange_set(&self) -> BTreeSet<String> {
        self.enabled_exchanges().into_iter().collect()
    }

    pub fn threshold_profile(&self) -> ContractWhaleThresholdProfile {
        let enabled = self.enabled_exchange_set();
        let binance_bitfinex = BTreeSet::from(["binance".to_string(), "bitfinex".to_string()]);
        let binance_bitfinex_coinbase = BTreeSet::from([
            "binance".to_string(),
            "bitfinex".to_string(),
            "coinbase".to_string(),
        ]);
        if enabled == binance_bitfinex {
            ContractWhaleThresholdProfile::BinanceBitfinex
        } else if enabled == binance_bitfinex_coinbase {
            ContractWhaleThresholdProfile::BinanceBitfinexCoinbase
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

    pub fn exchange_platform(&self, exchange: &str) -> Option<&ContractWhalePlatformConfig> {
        match exchange.to_ascii_lowercase().as_str() {
            "binance" => Some(&self.exchanges.binance),
            "okx" => Some(&self.exchanges.okx),
            "bitfinex" => Some(&self.exchanges.bitfinex),
            "coinbase" => Some(&self.exchanges.coinbase),
            _ => None,
        }
    }

    pub fn market_enabled(&self, exchange: &str, market: ContractWhaleMarketType) -> bool {
        self.exchange_platform(exchange)
            .is_some_and(|platform| platform.market_enabled(market))
    }

    pub fn platform_keys(&self) -> Vec<String> {
        ["binance", "bitfinex", "coinbase", "okx"]
            .into_iter()
            .map(ToString::to_string)
            .collect()
    }

    pub fn primary_contract_exchanges(&self) -> Vec<String> {
        self.platform_keys()
            .into_iter()
            .filter(|exchange| {
                self.exchange_platform(exchange).is_some_and(|platform| {
                    platform.market_enabled(ContractWhaleMarketType::Perp)
                        && platform.market_role(ContractWhaleMarketType::Perp)
                            == ContractWhaleSourceRole::Primary
                })
            })
            .collect()
    }

    fn threshold_participating_exchanges(&self) -> Vec<String> {
        let mut exchanges = vec![
            "binance".to_string(),
            "okx".to_string(),
            "bitfinex".to_string(),
        ];
        if self.exchange_platform("coinbase").is_some_and(|platform| {
            platform.market_enabled(ContractWhaleMarketType::Perp)
                || platform.market_enabled(ContractWhaleMarketType::Level2)
                || platform.market_enabled(ContractWhaleMarketType::Funding)
                || platform.market_enabled(ContractWhaleMarketType::Oi)
                || platform.market_enabled(ContractWhaleMarketType::Liquidation)
        }) {
            exchanges.push("coinbase".to_string());
        }
        exchanges
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
    BinanceBitfinexCoinbase,
}

impl ContractWhaleThresholdProfile {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::ThreeExchange => "three_exchange",
            Self::BinanceBitfinex => "binance_bitfinex",
            Self::BinanceBitfinexCoinbase => "binance_bitfinex_coinbase",
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
            Self::BinanceBitfinexCoinbase => ContractWhaleNotionalThresholds {
                high: 50_000_000.0,
                critical: 120_000_000.0,
                s: 250_000_000.0,
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
    pub binance: ContractWhalePlatformConfig,
    pub okx: ContractWhalePlatformConfig,
    pub bitfinex: ContractWhalePlatformConfig,
    pub coinbase: ContractWhalePlatformConfig,
}

impl Default for ContractWhaleExchangeConfig {
    fn default() -> Self {
        Self {
            binance: ContractWhalePlatformConfig::binance_default(),
            okx: ContractWhalePlatformConfig::okx_default(),
            bitfinex: ContractWhalePlatformConfig::bitfinex_default(),
            coinbase: ContractWhalePlatformConfig::coinbase_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhalePlatformConfig {
    pub enabled: bool,
    pub spot: ContractWhaleSourceConfig,
    pub perp: ContractWhaleSourceConfig,
    pub level2: ContractWhaleSourceConfig,
    pub funding: ContractWhaleSourceConfig,
    pub oi: ContractWhaleSourceConfig,
    pub liquidation: ContractWhaleSourceConfig,
}

impl ContractWhalePlatformConfig {
    pub fn any_market_enabled(&self) -> bool {
        self.enabled
            && [
                self.spot.enabled,
                self.perp.enabled,
                self.level2.enabled,
                self.funding.enabled,
                self.oi.enabled,
                self.liquidation.enabled,
            ]
            .into_iter()
            .any(|enabled| enabled)
    }

    pub fn contract_markets_enabled(&self) -> bool {
        self.enabled
            && [
                self.perp.enabled,
                self.level2.enabled,
                self.funding.enabled,
                self.oi.enabled,
                self.liquidation.enabled,
            ]
            .into_iter()
            .any(|enabled| enabled)
    }

    pub fn market_enabled(&self, market: ContractWhaleMarketType) -> bool {
        self.enabled && self.source_for_market(market).enabled
    }

    pub fn market_role(&self, market: ContractWhaleMarketType) -> ContractWhaleSourceRole {
        if !self.enabled {
            ContractWhaleSourceRole::Disabled
        } else {
            self.source_for_market(market).role
        }
    }

    pub fn enabled_markets(&self) -> Vec<String> {
        [
            ContractWhaleMarketType::Spot,
            ContractWhaleMarketType::Perp,
            ContractWhaleMarketType::Level2,
            ContractWhaleMarketType::Funding,
            ContractWhaleMarketType::Oi,
            ContractWhaleMarketType::Liquidation,
        ]
        .into_iter()
        .filter(|market| self.market_enabled(*market))
        .map(|market| market.as_key().to_string())
        .collect()
    }

    pub fn enabled_market_roles(&self) -> BTreeMap<String, String> {
        [
            ContractWhaleMarketType::Spot,
            ContractWhaleMarketType::Perp,
            ContractWhaleMarketType::Level2,
            ContractWhaleMarketType::Funding,
            ContractWhaleMarketType::Oi,
            ContractWhaleMarketType::Liquidation,
        ]
        .into_iter()
        .filter(|market| self.market_enabled(*market))
        .map(|market| {
            (
                market.as_key().to_string(),
                self.market_role(market).as_key().to_string(),
            )
        })
        .collect()
    }

    fn source_for_market(&self, market: ContractWhaleMarketType) -> &ContractWhaleSourceConfig {
        match market {
            ContractWhaleMarketType::Spot => &self.spot,
            ContractWhaleMarketType::Perp => &self.perp,
            ContractWhaleMarketType::Level2 => &self.level2,
            ContractWhaleMarketType::Funding => &self.funding,
            ContractWhaleMarketType::Oi => &self.oi,
            ContractWhaleMarketType::Liquidation => &self.liquidation,
        }
    }

    fn binance_default() -> Self {
        Self {
            enabled: true,
            spot: ContractWhaleSourceConfig::new(true, ContractWhaleSourceRole::Primary),
            perp: ContractWhaleSourceConfig::new(true, ContractWhaleSourceRole::Primary),
            level2: ContractWhaleSourceConfig::new(false, ContractWhaleSourceRole::Optional),
            funding: ContractWhaleSourceConfig::new(true, ContractWhaleSourceRole::Primary),
            oi: ContractWhaleSourceConfig::new(true, ContractWhaleSourceRole::Primary),
            liquidation: ContractWhaleSourceConfig::new(true, ContractWhaleSourceRole::Primary),
        }
    }

    fn okx_default() -> Self {
        Self {
            enabled: false,
            spot: ContractWhaleSourceConfig::disabled(),
            perp: ContractWhaleSourceConfig::disabled(),
            level2: ContractWhaleSourceConfig::disabled(),
            funding: ContractWhaleSourceConfig::disabled(),
            oi: ContractWhaleSourceConfig::disabled(),
            liquidation: ContractWhaleSourceConfig::disabled(),
        }
    }

    fn bitfinex_default() -> Self {
        Self {
            enabled: true,
            spot: ContractWhaleSourceConfig::new(true, ContractWhaleSourceRole::Confirmation),
            perp: ContractWhaleSourceConfig::new(true, ContractWhaleSourceRole::Confirmation),
            level2: ContractWhaleSourceConfig::new(false, ContractWhaleSourceRole::Optional),
            funding: ContractWhaleSourceConfig::disabled(),
            oi: ContractWhaleSourceConfig::disabled(),
            liquidation: ContractWhaleSourceConfig::disabled(),
        }
    }

    fn coinbase_default() -> Self {
        Self {
            enabled: true,
            spot: ContractWhaleSourceConfig::new(true, ContractWhaleSourceRole::Primary),
            perp: ContractWhaleSourceConfig::new(false, ContractWhaleSourceRole::Optional),
            level2: ContractWhaleSourceConfig::new(false, ContractWhaleSourceRole::Optional),
            funding: ContractWhaleSourceConfig::disabled(),
            oi: ContractWhaleSourceConfig::disabled(),
            liquidation: ContractWhaleSourceConfig::disabled(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ContractWhaleSourceConfig {
    pub enabled: bool,
    pub role: ContractWhaleSourceRole,
}

impl ContractWhaleSourceConfig {
    pub const fn new(enabled: bool, role: ContractWhaleSourceRole) -> Self {
        Self { enabled, role }
    }

    pub const fn disabled() -> Self {
        Self::new(false, ContractWhaleSourceRole::Disabled)
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
        binance: load_platform_config(
            settings,
            "binance",
            &ContractWhalePlatformConfig::binance_default(),
            Some("CONTRACT_WHALE_BINANCE_ENABLED"),
            bool_setting(settings, "ENABLE_BINANCE", "enable_binance", true),
        ),
        okx: load_platform_config(
            settings,
            "okx",
            &ContractWhalePlatformConfig::okx_default(),
            Some("CONTRACT_WHALE_OKX_ENABLED"),
            bool_setting(settings, "ENABLE_OKX", "enable_okx", false),
        ),
        bitfinex: load_platform_config(
            settings,
            "bitfinex",
            &ContractWhalePlatformConfig::bitfinex_default(),
            Some("CONTRACT_WHALE_BITFINEX_ENABLED"),
            true,
        ),
        coinbase: load_platform_config(
            settings,
            "coinbase",
            &ContractWhalePlatformConfig::coinbase_default(),
            Some("CONTRACT_WHALE_COINBASE_ENABLED"),
            true,
        ),
    }
}

fn load_platform_config(
    settings: &::config::Config,
    exchange: &str,
    default: &ContractWhalePlatformConfig,
    enabled_env_key: Option<&str>,
    enabled_default: bool,
) -> ContractWhalePlatformConfig {
    let exchange_upper = exchange.to_ascii_uppercase();
    let enabled = enabled_env_key
        .and_then(|env_key| std::env::var(env_key).ok())
        .map(|value| value.eq_ignore_ascii_case("true"))
        .or_else(|| {
            settings
                .get_bool(&format!(
                    "contract_whale_monitor.exchanges.{exchange}.enabled"
                ))
                .ok()
        })
        .unwrap_or(enabled_default);
    ContractWhalePlatformConfig {
        enabled,
        spot: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Spot,
            default.spot,
        ),
        perp: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Perp,
            default.perp,
        ),
        level2: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Level2,
            default.level2,
        ),
        funding: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Funding,
            default.funding,
        ),
        oi: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Oi,
            default.oi,
        ),
        liquidation: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Liquidation,
            default.liquidation,
        ),
    }
}

fn load_source_config(
    settings: &::config::Config,
    exchange: &str,
    exchange_upper: &str,
    market: ContractWhaleMarketType,
    default: ContractWhaleSourceConfig,
) -> ContractWhaleSourceConfig {
    let market_key = market.as_key();
    let enabled = bool_setting(
        settings,
        &format!(
            "CONTRACT_WHALE_{exchange_upper}_{}_ENABLED",
            market.as_env_key()
        ),
        &format!("contract_whale_monitor.exchanges.{exchange}.{market_key}.enabled"),
        default.enabled,
    );
    let role = source_role_setting(
        settings,
        &format!("contract_whale_monitor.exchanges.{exchange}.{market_key}.role"),
        default.role,
    );
    ContractWhaleSourceConfig { enabled, role }
}

fn source_role_setting(
    settings: &::config::Config,
    path: &str,
    default: ContractWhaleSourceRole,
) -> ContractWhaleSourceRole {
    settings
        .get_string(path)
        .ok()
        .and_then(|value| parse_source_role(&value))
        .unwrap_or(default)
}

fn parse_source_role(value: &str) -> Option<ContractWhaleSourceRole> {
    match value.trim().to_ascii_lowercase().as_str() {
        "primary" => Some(ContractWhaleSourceRole::Primary),
        "confirmation" => Some(ContractWhaleSourceRole::Confirmation),
        "optional" => Some(ContractWhaleSourceRole::Optional),
        "disabled" => Some(ContractWhaleSourceRole::Disabled),
        _ => None,
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
