use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{OnceLock, RwLock},
};

use super::{
    types::{
        ContractExchange, ContractWhaleExchangeStatus, ContractWhaleMarketType,
        ContractWhaleSourceRole, ContractWhaleThresholds,
    },
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
    pub classification: ContractWhaleClassificationConfig,
    pub toxic_order: ContractWhaleToxicOrderConfig,
    pub discord: ContractWhaleDiscordGateConfig,
    pub producer: ContractWhaleProducerConfig,
    pub discord_outbox: ContractWhaleDiscordOutboxConfig,
    pub emission: ContractWhaleEmissionConfig,
    pub lifecycle: ContractWhaleLifecycleConfig,
    pub okx_instruments: ContractWhaleOkxInstrumentConfig,
    pub symbols: BTreeMap<String, ContractWhaleSymbolConfig>,
    pub threshold_profiles: BTreeMap<String, ContractWhaleThresholdProfileConfig>,
    pub data_quality: ContractWhaleDataQualityConfig,
    pub retention: ContractWhaleRetentionConfig,
}

const CONTRACT_SOURCE_ORDER: [ContractExchange; 4] = [
    ContractExchange::Binance,
    ContractExchange::Bitfinex,
    ContractExchange::Coinbase,
    ContractExchange::Okx,
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractSourceSet {
    pub sources: BTreeSet<ContractExchange>,
}

impl ContractSourceSet {
    pub fn from_exchanges(exchanges: impl IntoIterator<Item = ContractExchange>) -> Self {
        Self {
            sources: exchanges.into_iter().collect(),
        }
    }

    pub fn from_keys(keys: impl IntoIterator<Item = String>) -> Self {
        Self {
            sources: keys
                .into_iter()
                .filter_map(|key| contract_exchange_from_key(&key))
                .collect(),
        }
    }

    pub fn keys(&self) -> Vec<String> {
        ordered_contract_sources(&self.sources)
            .into_iter()
            .map(|exchange| exchange.as_key().to_string())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThresholdProfileResolution {
    pub profile: ContractWhaleThresholdProfile,
    pub profile_name: String,
    pub reason: String,
    pub configured_contract_sources: Vec<ContractExchange>,
    pub eligible_contract_sources: Vec<ContractExchange>,
    pub active_contract_sources: Vec<ContractExchange>,
}

impl ThresholdProfileResolution {
    pub fn configured_keys(&self) -> Vec<String> {
        self.configured_contract_sources
            .iter()
            .map(|exchange| exchange.as_key().to_string())
            .collect()
    }

    pub fn eligible_keys(&self) -> Vec<String> {
        self.eligible_contract_sources
            .iter()
            .map(|exchange| exchange.as_key().to_string())
            .collect()
    }

    pub fn active_keys(&self) -> Vec<String> {
        self.active_contract_sources
            .iter()
            .map(|exchange| exchange.as_key().to_string())
            .collect()
    }
}

impl ContractWhaleRuntimeConfig {
    pub fn thresholds_for_symbol_window(
        &self,
        symbol: &str,
        window_sec: u64,
    ) -> ContractWhaleThresholds {
        self.thresholds_for_symbol_window_with_profile(symbol, window_sec, self.threshold_profile())
    }

    pub fn thresholds_for_symbol_window_with_profile(
        &self,
        symbol: &str,
        window_sec: u64,
        profile: ContractWhaleThresholdProfile,
    ) -> ContractWhaleThresholds {
        if profile == ContractWhaleThresholdProfile::NoContractSources {
            return ContractWhaleThresholds {
                high_btc: f64::INFINITY,
                critical_btc: f64::INFINITY,
                s_btc: f64::INFINITY,
            };
        }
        if normalize_symbol(symbol) == "BTC" {
            if let Some(override_thresholds) = self.btc_symbol_threshold_override(window_sec) {
                return override_thresholds;
            }
            if let Some(thresholds) = self
                .threshold_profiles
                .get(profile.as_key())
                .and_then(|profile| profile.thresholds_btc.get(&window_sec))
                .copied()
            {
                return thresholds;
            }
        }
        self.symbols
            .get(&normalize_symbol(symbol))
            .and_then(|symbol_config| symbol_config.thresholds_btc.get(&window_sec).copied())
            .unwrap_or_else(|| ContractWhaleThresholds::for_window(window_sec))
    }

    fn btc_symbol_threshold_override(&self, window_sec: u64) -> Option<ContractWhaleThresholds> {
        let configured = self
            .symbols
            .get("BTC")?
            .thresholds_btc
            .get(&window_sec)
            .copied()?;
        let default = ContractWhaleSymbolConfig::btc_default(true)
            .thresholds_btc
            .get(&window_sec)
            .copied()
            .unwrap_or_else(|| ContractWhaleThresholds::for_window(window_sec));
        if threshold_differs(configured, default) {
            Some(configured)
        } else {
            None
        }
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
        self.threshold_profile_resolution().active_keys()
    }

    pub fn active_exchange_count(&self) -> usize {
        self.enabled_exchanges().len()
    }

    pub fn disabled_exchanges(&self) -> Vec<String> {
        self.threshold_participating_exchanges()
            .into_iter()
            .filter(|exchange| {
                !self
                    .threshold_profile_resolution()
                    .active_keys()
                    .iter()
                    .any(|active| active == exchange)
            })
            .collect()
    }

    pub fn enabled_exchange_set(&self) -> BTreeSet<String> {
        self.enabled_exchanges().into_iter().collect()
    }

    pub fn threshold_profile(&self) -> ContractWhaleThresholdProfile {
        self.threshold_profile_resolution().profile
    }

    pub fn threshold_profile_resolution(&self) -> ThresholdProfileResolution {
        self.threshold_profile_resolution_from_active_override(None)
    }

    pub fn threshold_profile_resolution_for_observed_sources(
        &self,
        observed_sources: impl IntoIterator<Item = String>,
    ) -> ThresholdProfileResolution {
        let observed = ContractSourceSet::from_keys(observed_sources);
        self.threshold_profile_resolution_from_active_override(Some(&observed))
    }

    pub fn threshold_profile_resolution_with_statuses(
        &self,
        statuses: &BTreeMap<String, ContractWhaleExchangeStatus>,
        now: i64,
    ) -> ThresholdProfileResolution {
        let configured = self.configured_contract_source_set();
        let eligible = self.eligible_contract_source_set(&configured);
        let active =
            ContractSourceSet::from_exchanges(eligible.sources.iter().copied().filter(
                |exchange| self.contract_source_active_with_status(*exchange, statuses, now),
            ));
        self.build_threshold_profile_resolution(configured, eligible, active)
    }

    fn threshold_profile_resolution_from_active_override(
        &self,
        active_override: Option<&ContractSourceSet>,
    ) -> ThresholdProfileResolution {
        let configured = self.configured_contract_source_set();
        let eligible = self.eligible_contract_source_set(&configured);
        let active = ContractSourceSet::from_exchanges(eligible.sources.iter().copied().filter(
            |exchange| {
                if let Some(override_set) = active_override {
                    return override_set.sources.contains(exchange);
                }
                if matches!(exchange, ContractExchange::Coinbase) {
                    return false;
                }
                true
            },
        ));
        self.build_threshold_profile_resolution(configured, eligible, active)
    }

    fn build_threshold_profile_resolution(
        &self,
        configured: ContractSourceSet,
        eligible: ContractSourceSet,
        active: ContractSourceSet,
    ) -> ThresholdProfileResolution {
        let profile = threshold_profile_for_active_sources(&active);
        let mut reason = if active.is_empty() {
            "no_contract_sources".to_string()
        } else {
            format!("active_contract_sources={}", active.keys().join(","))
        };
        if configured.sources.contains(&ContractExchange::Coinbase)
            && !eligible.sources.contains(&ContractExchange::Coinbase)
        {
            reason = "coinbase_perp_auth_missing".to_string();
        } else if eligible.sources.contains(&ContractExchange::Coinbase)
            && !active.sources.contains(&ContractExchange::Coinbase)
        {
            reason = "coinbase_perp_not_active".to_string();
        }
        ThresholdProfileResolution {
            profile,
            profile_name: profile.as_key().to_string(),
            reason,
            configured_contract_sources: ordered_contract_sources(&configured.sources),
            eligible_contract_sources: ordered_contract_sources(&eligible.sources),
            active_contract_sources: ordered_contract_sources(&active.sources),
        }
    }

    fn configured_contract_source_set(&self) -> ContractSourceSet {
        ContractSourceSet::from_exchanges(CONTRACT_SOURCE_ORDER.into_iter().filter(|exchange| {
            self.exchange_platform(exchange.as_key())
                .is_some_and(|platform| platform.market_enabled(ContractWhaleMarketType::Perp))
        }))
    }

    fn eligible_contract_source_set(&self, configured: &ContractSourceSet) -> ContractSourceSet {
        ContractSourceSet::from_exchanges(configured.sources.iter().copied().filter(|exchange| {
            self.exchange_platform(exchange.as_key())
                .is_some_and(|platform| platform.perp.eligible_for_contract_threshold())
        }))
    }

    fn contract_source_active_with_status(
        &self,
        exchange: ContractExchange,
        statuses: &BTreeMap<String, ContractWhaleExchangeStatus>,
        now: i64,
    ) -> bool {
        if !self
            .exchange_platform(exchange.as_key())
            .is_some_and(|platform| platform.perp.eligible_for_contract_threshold())
        {
            return false;
        }
        if !matches!(exchange, ContractExchange::Coinbase) {
            return true;
        }
        statuses.get(exchange.as_key()).is_some_and(|status| {
            status.connected && exchange_recent(status.last_trade_at, now, 30_000)
        })
    }

    pub fn threshold_profile_for_active_sources(
        &self,
        active_sources: &ContractSourceSet,
    ) -> ContractWhaleThresholdProfile {
        threshold_profile_for_active_sources(active_sources)
    }

    pub fn threshold_profile_for_observed_sources(
        &self,
        observed_sources: impl IntoIterator<Item = String>,
    ) -> ContractWhaleThresholdProfile {
        self.threshold_profile_resolution_for_observed_sources(observed_sources)
            .profile
    }

    pub fn threshold_profile_key(&self) -> &'static str {
        self.threshold_profile().as_key()
    }

    pub fn notional_thresholds_usd_for_profile(
        &self,
        profile: ContractWhaleThresholdProfile,
    ) -> ContractWhaleNotionalThresholds {
        self.threshold_profiles
            .get(profile.as_key())
            .map(|profile| profile.notional_usd)
            .unwrap_or_else(|| profile.notional_thresholds_usd())
    }

    pub fn notional_thresholds_usd(&self) -> ContractWhaleNotionalThresholds {
        self.notional_thresholds_usd_for_profile(self.threshold_profile())
    }

    pub fn legacy_threshold_profile(&self) -> ContractWhaleThresholdProfile {
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

fn threshold_profile_for_active_sources(
    active_sources: &ContractSourceSet,
) -> ContractWhaleThresholdProfile {
    let keys = active_sources.keys();
    if keys.is_empty() {
        ContractWhaleThresholdProfile::NoContractSources
    } else if keys.iter().any(|key| key == "okx") {
        ContractWhaleThresholdProfile::ThreeExchange
    } else if keys.iter().any(|key| key == "coinbase") {
        ContractWhaleThresholdProfile::BinanceBitfinexCoinbase
    } else {
        ContractWhaleThresholdProfile::BinanceBitfinex
    }
}

fn ordered_contract_sources(sources: &BTreeSet<ContractExchange>) -> Vec<ContractExchange> {
    CONTRACT_SOURCE_ORDER
        .into_iter()
        .filter(|exchange| sources.contains(exchange))
        .collect()
}

fn contract_exchange_from_key(value: &str) -> Option<ContractExchange> {
    match value.to_ascii_lowercase().as_str() {
        "binance" => Some(ContractExchange::Binance),
        "bitfinex" => Some(ContractExchange::Bitfinex),
        "coinbase" => Some(ContractExchange::Coinbase),
        "okx" => Some(ContractExchange::Okx),
        _ => None,
    }
}

fn exchange_recent(last_trade_at: Option<i64>, now: i64, max_silence_ms: i64) -> bool {
    last_trade_at.is_some_and(|last_trade_at| now.saturating_sub(last_trade_at) <= max_silence_ms)
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
            classification: ContractWhaleClassificationConfig::default(),
            toxic_order: ContractWhaleToxicOrderConfig::default(),
            discord: ContractWhaleDiscordGateConfig::default(),
            producer: ContractWhaleProducerConfig::default(),
            discord_outbox: ContractWhaleDiscordOutboxConfig::default(),
            emission: ContractWhaleEmissionConfig::default(),
            lifecycle: ContractWhaleLifecycleConfig::default(),
            okx_instruments: ContractWhaleOkxInstrumentConfig::default(),
            symbols,
            threshold_profiles: default_threshold_profiles(),
            data_quality: ContractWhaleDataQualityConfig::default(),
            retention: ContractWhaleRetentionConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleThresholdProfileConfig {
    pub active_contract_sources: Vec<String>,
    pub thresholds_btc: BTreeMap<u64, ContractWhaleThresholds>,
    pub notional_usd: ContractWhaleNotionalThresholds,
}

fn default_threshold_profiles() -> BTreeMap<String, ContractWhaleThresholdProfileConfig> {
    [
        (
            "binance_bitfinex",
            ContractWhaleThresholdProfileConfig {
                active_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
                thresholds_btc: threshold_map([
                    (5, (650.0, 1_200.0, 2_000.0)),
                    (15, (1_200.0, 2_200.0, 3_600.0)),
                    (60, (2_800.0, 5_200.0, 8_000.0)),
                ]),
                notional_usd: ContractWhaleNotionalThresholds {
                    high: 40_000_000.0,
                    critical: 95_000_000.0,
                    s: 200_000_000.0,
                },
            },
        ),
        (
            "binance_bitfinex_coinbase",
            ContractWhaleThresholdProfileConfig {
                active_contract_sources: vec![
                    "binance".to_string(),
                    "bitfinex".to_string(),
                    "coinbase".to_string(),
                ],
                thresholds_btc: threshold_map([
                    (5, (750.0, 1_400.0, 2_300.0)),
                    (15, (1_400.0, 2_600.0, 4_200.0)),
                    (60, (3_200.0, 6_000.0, 9_200.0)),
                ]),
                notional_usd: ContractWhaleNotionalThresholds {
                    high: 50_000_000.0,
                    critical: 115_000_000.0,
                    s: 230_000_000.0,
                },
            },
        ),
        (
            "three_exchange",
            ContractWhaleThresholdProfileConfig {
                active_contract_sources: vec![
                    "binance".to_string(),
                    "bitfinex".to_string(),
                    "okx".to_string(),
                ],
                thresholds_btc: threshold_map([
                    (5, (800.0, 1_500.0, 2_500.0)),
                    (15, (1_500.0, 2_800.0, 4_500.0)),
                    (60, (3_500.0, 6_500.0, 10_000.0)),
                ]),
                notional_usd: ContractWhaleNotionalThresholds {
                    high: 50_000_000.0,
                    critical: 120_000_000.0,
                    s: 250_000_000.0,
                },
            },
        ),
    ]
    .into_iter()
    .map(|(key, profile)| (key.to_string(), profile))
    .collect()
}

fn threshold_map(
    entries: impl IntoIterator<Item = (u64, (f64, f64, f64))>,
) -> BTreeMap<u64, ContractWhaleThresholds> {
    entries
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
        .collect()
}

fn threshold_differs(left: ContractWhaleThresholds, right: ContractWhaleThresholds) -> bool {
    (left.high_btc - right.high_btc).abs() > f64::EPSILON
        || (left.critical_btc - right.critical_btc).abs() > f64::EPSILON
        || (left.s_btc - right.s_btc).abs() > f64::EPSILON
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractWhaleThresholdProfile {
    NoContractSources,
    ThreeExchange,
    BinanceBitfinex,
    BinanceBitfinexCoinbase,
}

impl ContractWhaleThresholdProfile {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::NoContractSources => "no_contract_sources",
            Self::ThreeExchange => "three_exchange",
            Self::BinanceBitfinex => "binance_bitfinex",
            Self::BinanceBitfinexCoinbase => "binance_bitfinex_coinbase",
        }
    }

    pub fn notional_thresholds_usd(self) -> ContractWhaleNotionalThresholds {
        match self {
            Self::NoContractSources => ContractWhaleNotionalThresholds {
                high: f64::INFINITY,
                critical: f64::INFINITY,
                s: f64::INFINITY,
            },
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

    pub fn source_for_market(&self, market: ContractWhaleMarketType) -> &ContractWhaleSourceConfig {
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
            perp: ContractWhaleSourceConfig {
                enabled: false,
                role: ContractWhaleSourceRole::Confirmation,
                product: Some("BTC-PERP".to_string()),
                source: Some("coinbase_intx_match".to_string()),
                requires_auth: true,
                market_data_only: true,
                auth: ContractWhaleSourceAuthConfig {
                    key_env: Some("COINBASE_INTX_KEY".to_string()),
                    secret_env: Some("COINBASE_INTX_SECRET".to_string()),
                    passphrase_env: Some("COINBASE_INTX_PASSPHRASE".to_string()),
                },
            },
            level2: ContractWhaleSourceConfig::new(false, ContractWhaleSourceRole::Optional),
            funding: ContractWhaleSourceConfig::disabled(),
            oi: ContractWhaleSourceConfig::disabled(),
            liquidation: ContractWhaleSourceConfig::disabled(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleSourceConfig {
    pub enabled: bool,
    pub role: ContractWhaleSourceRole,
    pub product: Option<String>,
    pub source: Option<String>,
    pub requires_auth: bool,
    pub market_data_only: bool,
    pub auth: ContractWhaleSourceAuthConfig,
}

impl ContractWhaleSourceConfig {
    pub fn new(enabled: bool, role: ContractWhaleSourceRole) -> Self {
        Self {
            enabled,
            role,
            product: None,
            source: None,
            requires_auth: false,
            market_data_only: true,
            auth: ContractWhaleSourceAuthConfig::default(),
        }
    }

    pub fn disabled() -> Self {
        Self::new(false, ContractWhaleSourceRole::Disabled)
    }

    pub fn auth_configured(&self) -> bool {
        if !self.requires_auth {
            return true;
        }
        let required = [
            self.auth.key_env.as_deref(),
            self.auth.secret_env.as_deref(),
            self.auth.passphrase_env.as_deref(),
        ];
        required.into_iter().all(|env_key| {
            env_key
                .filter(|key| !key.trim().is_empty())
                .and_then(|key| std::env::var(key).ok())
                .is_some_and(|value| !value.trim().is_empty())
        })
    }

    pub fn eligible_for_contract_threshold(&self) -> bool {
        self.enabled && self.market_data_only && self.auth_configured()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ContractWhaleSourceAuthConfig {
    pub key_env: Option<String>,
    pub secret_env: Option<String>,
    pub passphrase_env: Option<String>,
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

#[derive(Debug, Clone)]
pub struct ContractWhaleToxicOrderConfig {
    pub max_price_deviation_pct: f64,
    pub enable_spot_score: bool,
    pub enable_contract_score: bool,
}

impl Default for ContractWhaleToxicOrderConfig {
    fn default() -> Self {
        Self {
            max_price_deviation_pct: 5.0,
            enable_spot_score: true,
            enable_contract_score: true,
        }
    }
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
pub struct ContractWhaleClassificationConfig {
    pub enabled: bool,
    pub oi_context_enabled: bool,
    pub oi_batch_resolver_enabled: bool,
    pub oi_consensus_guard_enabled: bool,
    pub evidence_fail_closed_enabled: bool,
    pub flow_direction_dominance_min: f64,
    pub strong_intent_dominance_min: f64,
    pub absorption_dominance_min: f64,
    pub no_follow_pct: f64,
    pub follow_pct: f64,
    pub strong_follow_pct: f64,
    pub follow_same_direction_min_pct: f64,
    pub absorption_min_notional_usd: f64,
    pub low_price_efficiency_max: f64,
    pub normalized_price_efficiency_enabled: bool,
    pub low_price_efficiency_max_bps_per_million: f64,
    pub micro_volatility_enabled: bool,
    pub micro_volatility_min_samples: usize,
    pub micro_volatility_ewma_alpha: f64,
    pub micro_volatility_no_follow_multiplier: f64,
    pub micro_volatility_follow_multiplier: f64,
    pub micro_volatility_strong_follow_multiplier: f64,
    pub micro_volatility_max_staleness_seconds: i64,
    pub min_data_quality_for_strong_intent: u8,
    pub min_data_quality_for_absorption: u8,
    pub require_multi_exchange_for_strong_intent: bool,
    pub require_multi_exchange_for_absorption: bool,
    pub oi_lookup_max_gap_seconds: i64,
    pub oi_delta_min_pct: f64,
    pub oi_flat_max_abs_pct: f64,
    pub oi_context_change_pct: f64,
}

#[derive(Debug, Clone)]
pub struct ContractWhaleEmissionConfig {
    pub enabled: bool,
    pub score_delta_min: u8,
    pub volume_delta_ratio_min: f64,
    pub force_refresh_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct ContractWhaleProducerConfig {
    pub interval_ms: u64,
    pub skip_missed_ticks: bool,
    pub prevent_overlap: bool,
}

#[derive(Debug, Clone)]
pub struct ContractWhaleDiscordOutboxConfig {
    pub enabled: bool,
    pub poll_interval_ms: u64,
    pub batch_size: usize,
    pub max_attempts: usize,
    pub base_retry_seconds: i64,
    pub max_retry_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct ContractWhaleLifecycleConfig {
    pub update_window_seconds: i64,
    pub close_after_seconds: i64,
    pub unique_turnover_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ContractWhaleOkxInstrumentConfig {
    pub metadata_enabled: bool,
    pub refresh_minutes: i64,
    pub fallback_quality_penalty: u8,
    pub fallback_ct_val_base: BTreeMap<String, f64>,
}

impl Default for ContractWhaleOkxInstrumentConfig {
    fn default() -> Self {
        Self {
            metadata_enabled: true,
            refresh_minutes: 60,
            fallback_quality_penalty: 10,
            fallback_ct_val_base: BTreeMap::new(),
        }
    }
}

impl ContractWhaleOkxInstrumentConfig {
    pub fn fallback_ct_val_base(&self, symbol: &str) -> Option<f64> {
        self.fallback_ct_val_base
            .get(&normalize_symbol(symbol))
            .copied()
            .filter(|value| value.is_finite() && *value > 0.0)
    }
}

impl Default for ContractWhaleLifecycleConfig {
    fn default() -> Self {
        Self {
            update_window_seconds: 30,
            close_after_seconds: 120,
            unique_turnover_enabled: true,
        }
    }
}

impl Default for ContractWhaleProducerConfig {
    fn default() -> Self {
        Self {
            interval_ms: 2_000,
            skip_missed_ticks: true,
            prevent_overlap: true,
        }
    }
}

impl Default for ContractWhaleDiscordOutboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_ms: 1_000,
            batch_size: 20,
            max_attempts: 6,
            base_retry_seconds: 2,
            max_retry_seconds: 300,
        }
    }
}

impl Default for ContractWhaleEmissionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            score_delta_min: 5,
            volume_delta_ratio_min: 0.10,
            force_refresh_seconds: 15,
        }
    }
}

impl Default for ContractWhaleClassificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            oi_context_enabled: true,
            oi_batch_resolver_enabled: true,
            oi_consensus_guard_enabled: true,
            evidence_fail_closed_enabled: true,
            flow_direction_dominance_min: 0.55,
            strong_intent_dominance_min: 0.60,
            absorption_dominance_min: 0.65,
            no_follow_pct: 0.05,
            follow_pct: 0.12,
            strong_follow_pct: 0.20,
            follow_same_direction_min_pct: 0.20,
            absorption_min_notional_usd: 10_000_000.0,
            low_price_efficiency_max: 0.25,
            normalized_price_efficiency_enabled: true,
            low_price_efficiency_max_bps_per_million: 2.5,
            micro_volatility_enabled: true,
            micro_volatility_min_samples: 60,
            micro_volatility_ewma_alpha: 0.08,
            micro_volatility_no_follow_multiplier: 0.20,
            micro_volatility_follow_multiplier: 0.35,
            micro_volatility_strong_follow_multiplier: 0.60,
            micro_volatility_max_staleness_seconds: 30,
            min_data_quality_for_strong_intent: 70,
            min_data_quality_for_absorption: 70,
            require_multi_exchange_for_strong_intent: true,
            require_multi_exchange_for_absorption: true,
            oi_lookup_max_gap_seconds: 90,
            oi_delta_min_pct: 0.10,
            oi_flat_max_abs_pct: 0.05,
            oi_context_change_pct: 0.20,
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

#[derive(Debug, Clone)]
pub struct ContractWhaleDiscordGateConfig {
    pub impact_level_push_enabled: bool,
    pub push_impact_levels: Vec<String>,
    pub impact_level_min_data_quality: u8,
}

impl ContractWhaleDiscordGateConfig {
    pub fn allows_impact_level(&self, impact_level: Option<&str>, data_quality: u8) -> bool {
        if !self.impact_level_push_enabled || data_quality < self.impact_level_min_data_quality {
            return false;
        }
        let Some(level) = impact_level else {
            return false;
        };
        self.push_impact_levels
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(level.trim()))
    }
}

impl Default for ContractWhaleDiscordGateConfig {
    fn default() -> Self {
        Self {
            impact_level_push_enabled: true,
            push_impact_levels: vec!["B".to_string(), "A".to_string(), "S".to_string()],
            impact_level_min_data_quality: 70,
        }
    }
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
        classification: load_classification_config(settings),
        toxic_order: load_toxic_order_config(settings),
        discord: load_discord_gate_config(settings),
        producer: load_producer_config(settings),
        discord_outbox: load_discord_outbox_config(settings),
        emission: load_emission_config(settings),
        lifecycle: load_lifecycle_config(settings),
        okx_instruments: load_okx_instrument_config(settings),
        data_quality: load_data_quality_config(settings),
        symbols: load_symbol_configs(settings),
        threshold_profiles: load_threshold_profiles(settings),
        retention: load_retention_config(settings),
    }
}

fn load_producer_config(settings: &::config::Config) -> ContractWhaleProducerConfig {
    let defaults = ContractWhaleProducerConfig::default();
    ContractWhaleProducerConfig {
        interval_ms: u64_setting(
            settings,
            "CONTRACT_WHALE_AUTO_PUSH_INTERVAL_MS",
            "contract_whale_monitor.producer.interval_ms",
            defaults.interval_ms,
        )
        .clamp(1_000, 60_000),
        skip_missed_ticks: bool_setting(
            settings,
            "CONTRACT_WHALE_PRODUCER_SKIP_MISSED_TICKS",
            "contract_whale_monitor.producer.skip_missed_ticks",
            defaults.skip_missed_ticks,
        ),
        prevent_overlap: bool_setting(
            settings,
            "CONTRACT_WHALE_PRODUCER_PREVENT_OVERLAP",
            "contract_whale_monitor.producer.prevent_overlap",
            defaults.prevent_overlap,
        ),
    }
}

fn load_discord_outbox_config(settings: &::config::Config) -> ContractWhaleDiscordOutboxConfig {
    let defaults = ContractWhaleDiscordOutboxConfig::default();
    ContractWhaleDiscordOutboxConfig {
        enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_DISCORD_OUTBOX_ENABLED",
            "contract_whale_monitor.discord_outbox.enabled",
            defaults.enabled,
        ),
        poll_interval_ms: u64_setting(
            settings,
            "CONTRACT_WHALE_DISCORD_OUTBOX_POLL_INTERVAL_MS",
            "contract_whale_monitor.discord_outbox.poll_interval_ms",
            defaults.poll_interval_ms,
        )
        .clamp(100, 60_000),
        batch_size: usize_setting_with_env(
            settings,
            "CONTRACT_WHALE_DISCORD_OUTBOX_BATCH_SIZE",
            "contract_whale_monitor.discord_outbox.batch_size",
            defaults.batch_size,
        )
        .clamp(1, 100),
        max_attempts: usize_setting_with_env(
            settings,
            "CONTRACT_WHALE_DISCORD_MAX_ATTEMPTS",
            "contract_whale_monitor.discord_outbox.max_attempts",
            defaults.max_attempts,
        )
        .clamp(1, 6),
        base_retry_seconds: i64_setting(
            settings,
            "contract_whale_monitor.discord_outbox.base_retry_seconds",
            defaults.base_retry_seconds,
        )
        .clamp(1, 60),
        max_retry_seconds: i64_setting(
            settings,
            "contract_whale_monitor.discord_outbox.max_retry_seconds",
            defaults.max_retry_seconds,
        )
        .clamp(1, 3_600),
    }
}

fn load_okx_instrument_config(settings: &::config::Config) -> ContractWhaleOkxInstrumentConfig {
    let defaults = ContractWhaleOkxInstrumentConfig::default();
    let fallback_ct_val_base = settings
        .get::<BTreeMap<String, f64>>("contract_whale_monitor.okx_instruments.fallback_ct_val_base")
        .unwrap_or_default()
        .into_iter()
        .map(|(symbol, value)| (normalize_symbol(&symbol), value))
        .filter(|(_, value)| value.is_finite() && *value > 0.0)
        .collect();
    ContractWhaleOkxInstrumentConfig {
        metadata_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_OKX_METADATA_ENABLED",
            "contract_whale_monitor.okx_instruments.metadata_enabled",
            defaults.metadata_enabled,
        ),
        refresh_minutes: i64_setting(
            settings,
            "contract_whale_monitor.okx_instruments.refresh_minutes",
            defaults.refresh_minutes,
        )
        .clamp(1, 1_440),
        fallback_quality_penalty: u8_setting(
            settings,
            "contract_whale_monitor.okx_instruments.fallback_quality_penalty",
            defaults.fallback_quality_penalty,
        )
        .min(100),
        fallback_ct_val_base,
    }
}

fn load_lifecycle_config(settings: &::config::Config) -> ContractWhaleLifecycleConfig {
    let defaults = ContractWhaleLifecycleConfig::default();
    ContractWhaleLifecycleConfig {
        update_window_seconds: i64_setting(
            settings,
            "contract_whale_monitor.lifecycle.update_window_seconds",
            defaults.update_window_seconds,
        )
        .clamp(1, 300),
        close_after_seconds: i64_setting(
            settings,
            "contract_whale_monitor.lifecycle.close_after_seconds",
            defaults.close_after_seconds,
        )
        .clamp(1, 3_600),
        unique_turnover_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_UNIQUE_TURNOVER_ENABLED",
            "contract_whale_monitor.lifecycle.unique_turnover_enabled",
            defaults.unique_turnover_enabled,
        ),
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
            default.spot.clone(),
        ),
        perp: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Perp,
            default.perp.clone(),
        ),
        level2: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Level2,
            default.level2.clone(),
        ),
        funding: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Funding,
            default.funding.clone(),
        ),
        oi: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Oi,
            default.oi.clone(),
        ),
        liquidation: load_source_config(
            settings,
            exchange,
            &exchange_upper,
            ContractWhaleMarketType::Liquidation,
            default.liquidation.clone(),
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
    let path = format!("contract_whale_monitor.exchanges.{exchange}.{market_key}");
    ContractWhaleSourceConfig {
        enabled,
        role,
        product: optional_string_setting(settings, &format!("{path}.product"), default.product),
        source: optional_string_setting(settings, &format!("{path}.source"), default.source),
        requires_auth: bool_setting(
            settings,
            &format!(
                "CONTRACT_WHALE_{exchange_upper}_{}_REQUIRES_AUTH",
                market.as_env_key()
            ),
            &format!("{path}.requires_auth"),
            default.requires_auth,
        ),
        market_data_only: bool_setting(
            settings,
            &format!(
                "CONTRACT_WHALE_{exchange_upper}_{}_MARKET_DATA_ONLY",
                market.as_env_key()
            ),
            &format!("{path}.market_data_only"),
            default.market_data_only,
        ),
        auth: ContractWhaleSourceAuthConfig {
            key_env: optional_string_setting(
                settings,
                &format!("{path}.auth.key_env"),
                default.auth.key_env,
            ),
            secret_env: optional_string_setting(
                settings,
                &format!("{path}.auth.secret_env"),
                default.auth.secret_env,
            ),
            passphrase_env: optional_string_setting(
                settings,
                &format!("{path}.auth.passphrase_env"),
                default.auth.passphrase_env,
            ),
        },
    }
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

fn load_emission_config(settings: &::config::Config) -> ContractWhaleEmissionConfig {
    let defaults = ContractWhaleEmissionConfig::default();
    ContractWhaleEmissionConfig {
        enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_EMISSION_ENABLED",
            "contract_whale_monitor.emission.enabled",
            defaults.enabled,
        ),
        score_delta_min: u8_setting(
            settings,
            "contract_whale_monitor.emission.score_delta_min",
            defaults.score_delta_min,
        )
        .clamp(1, 100),
        volume_delta_ratio_min: positive_float_setting(
            settings,
            "contract_whale_monitor.emission.volume_delta_ratio_min",
            defaults.volume_delta_ratio_min,
        )
        .clamp(0.01, 1.0),
        force_refresh_seconds: i64_setting(
            settings,
            "contract_whale_monitor.emission.force_refresh_seconds",
            defaults.force_refresh_seconds,
        )
        .clamp(1, 300),
    }
}

fn load_classification_config(settings: &::config::Config) -> ContractWhaleClassificationConfig {
    let defaults = ContractWhaleClassificationConfig::default();
    ContractWhaleClassificationConfig {
        enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_CLASSIFICATION_V2_ENABLED",
            "contract_whale_monitor.classification.enabled",
            defaults.enabled,
        ),
        oi_context_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_OI_CONTEXT_ENABLED",
            "contract_whale_monitor.classification.oi_context_enabled",
            defaults.oi_context_enabled,
        ),
        oi_batch_resolver_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_OI_BATCH_RESOLVER_ENABLED",
            "contract_whale_monitor.classification.oi_batch_resolver_enabled",
            defaults.oi_batch_resolver_enabled,
        ),
        oi_consensus_guard_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_OI_CONSENSUS_GUARD_ENABLED",
            "contract_whale_monitor.classification.oi_consensus_guard_enabled",
            defaults.oi_consensus_guard_enabled,
        ),
        evidence_fail_closed_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_EVIDENCE_FAIL_CLOSED_ENABLED",
            "contract_whale_monitor.classification.evidence_fail_closed_enabled",
            defaults.evidence_fail_closed_enabled,
        ),
        flow_direction_dominance_min: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.flow_direction_dominance_min",
            defaults.flow_direction_dominance_min,
        ),
        strong_intent_dominance_min: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.strong_intent_dominance_min",
            defaults.strong_intent_dominance_min,
        ),
        absorption_dominance_min: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.absorption_dominance_min",
            defaults.absorption_dominance_min,
        ),
        no_follow_pct: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.no_follow_pct",
            defaults.no_follow_pct,
        ),
        follow_pct: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.follow_pct",
            defaults.follow_pct,
        ),
        strong_follow_pct: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.strong_follow_pct",
            defaults.strong_follow_pct,
        ),
        follow_same_direction_min_pct: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.follow_same_direction_min_pct",
            defaults.follow_same_direction_min_pct,
        ),
        absorption_min_notional_usd: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.absorption_min_notional_usd",
            defaults.absorption_min_notional_usd,
        ),
        low_price_efficiency_max: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.low_price_efficiency_max",
            defaults.low_price_efficiency_max,
        ),
        normalized_price_efficiency_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_NORMALIZED_PRICE_EFFICIENCY_ENABLED",
            "contract_whale_monitor.classification.normalized_price_efficiency_enabled",
            defaults.normalized_price_efficiency_enabled,
        ),
        low_price_efficiency_max_bps_per_million: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.low_price_efficiency_max_bps_per_million",
            defaults.low_price_efficiency_max_bps_per_million,
        ),
        micro_volatility_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_MICRO_VOLATILITY_ENABLED",
            "contract_whale_monitor.classification.volatility.enabled",
            defaults.micro_volatility_enabled,
        ),
        micro_volatility_min_samples: usize_setting(
            settings,
            "contract_whale_monitor.classification.volatility.min_samples",
            defaults.micro_volatility_min_samples,
        ),
        micro_volatility_ewma_alpha: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.volatility.ewma_alpha",
            defaults.micro_volatility_ewma_alpha,
        )
        .clamp(0.001, 1.0),
        micro_volatility_no_follow_multiplier: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.volatility.no_follow_multiplier",
            defaults.micro_volatility_no_follow_multiplier,
        ),
        micro_volatility_follow_multiplier: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.volatility.follow_multiplier",
            defaults.micro_volatility_follow_multiplier,
        ),
        micro_volatility_strong_follow_multiplier: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.volatility.strong_follow_multiplier",
            defaults.micro_volatility_strong_follow_multiplier,
        ),
        micro_volatility_max_staleness_seconds: i64_setting(
            settings,
            "contract_whale_monitor.classification.volatility.max_staleness_seconds",
            defaults.micro_volatility_max_staleness_seconds,
        )
        .clamp(1, 300),
        min_data_quality_for_strong_intent: u8_setting(
            settings,
            "contract_whale_monitor.classification.min_data_quality_for_strong_intent",
            defaults.min_data_quality_for_strong_intent,
        ),
        min_data_quality_for_absorption: u8_setting(
            settings,
            "contract_whale_monitor.classification.min_data_quality_for_absorption",
            defaults.min_data_quality_for_absorption,
        ),
        require_multi_exchange_for_strong_intent: bool_setting(
            settings,
            "CONTRACT_WHALE_CLASSIFICATION_REQUIRE_MULTI_EXCHANGE_STRONG",
            "contract_whale_monitor.classification.require_multi_exchange_for_strong_intent",
            defaults.require_multi_exchange_for_strong_intent,
        ),
        require_multi_exchange_for_absorption: bool_setting(
            settings,
            "CONTRACT_WHALE_CLASSIFICATION_REQUIRE_MULTI_EXCHANGE_ABSORPTION",
            "contract_whale_monitor.classification.require_multi_exchange_for_absorption",
            defaults.require_multi_exchange_for_absorption,
        ),
        oi_lookup_max_gap_seconds: i64_setting(
            settings,
            "contract_whale_monitor.classification.oi_lookup_max_gap_seconds",
            defaults.oi_lookup_max_gap_seconds,
        ),
        oi_delta_min_pct: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.oi_delta_min_pct",
            defaults.oi_delta_min_pct,
        ),
        oi_flat_max_abs_pct: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.oi_flat_max_abs_pct",
            defaults.oi_flat_max_abs_pct,
        ),
        oi_context_change_pct: positive_float_setting(
            settings,
            "contract_whale_monitor.classification.oi_context_change_pct",
            defaults.oi_context_change_pct,
        ),
    }
}

fn load_toxic_order_config(settings: &::config::Config) -> ContractWhaleToxicOrderConfig {
    let defaults = ContractWhaleToxicOrderConfig::default();
    ContractWhaleToxicOrderConfig {
        max_price_deviation_pct: positive_float_setting(
            settings,
            "toxicOrder.max_price_deviation_pct",
            defaults.max_price_deviation_pct,
        ),
        enable_spot_score: bool_setting(
            settings,
            "TOXIC_ORDER_ENABLE_SPOT_SCORE",
            "toxicOrder.enable_spot_score",
            defaults.enable_spot_score,
        ),
        enable_contract_score: bool_setting(
            settings,
            "TOXIC_ORDER_ENABLE_CONTRACT_SCORE",
            "toxicOrder.enable_contract_score",
            defaults.enable_contract_score,
        ),
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

fn load_discord_gate_config(settings: &::config::Config) -> ContractWhaleDiscordGateConfig {
    let defaults = ContractWhaleDiscordGateConfig::default();
    ContractWhaleDiscordGateConfig {
        impact_level_push_enabled: bool_setting(
            settings,
            "CONTRACT_WHALE_IMPACT_LEVEL_PUSH_ENABLED",
            "contract_whale_monitor.discord.impact_level_push_enabled",
            defaults.impact_level_push_enabled,
        ),
        push_impact_levels: string_array_setting(
            settings,
            "contract_whale_monitor.discord.push_impact_levels",
            defaults.push_impact_levels,
        )
        .into_iter()
        .map(|level| level.to_ascii_uppercase())
        .collect(),
        impact_level_min_data_quality: u8_setting(
            settings,
            "contract_whale_monitor.discord.impact_level_min_data_quality",
            defaults.impact_level_min_data_quality,
        ),
    }
}

fn load_threshold_profiles(
    settings: &::config::Config,
) -> BTreeMap<String, ContractWhaleThresholdProfileConfig> {
    let mut profiles = default_threshold_profiles();
    for profile_key in [
        "binance_bitfinex",
        "binance_bitfinex_coinbase",
        "three_exchange",
    ] {
        let path = format!("contract_whale_monitor.threshold_profiles.{profile_key}");
        let fallback = profiles.get(profile_key).cloned().unwrap_or_else(|| {
            ContractWhaleThresholdProfileConfig {
                active_contract_sources: Vec::new(),
                thresholds_btc: BTreeMap::new(),
                notional_usd: ContractWhaleNotionalThresholds {
                    high: 0.0,
                    critical: 0.0,
                    s: 0.0,
                },
            }
        });
        profiles.insert(
            profile_key.to_string(),
            ContractWhaleThresholdProfileConfig {
                active_contract_sources: string_array_setting(
                    settings,
                    &format!("{path}.active_contract_sources"),
                    fallback.active_contract_sources.clone(),
                ),
                thresholds_btc: load_threshold_profile_windows(settings, &path, &fallback),
                notional_usd: ContractWhaleNotionalThresholds {
                    high: positive_float_setting(
                        settings,
                        &format!("{path}.notional_usd.high"),
                        fallback.notional_usd.high,
                    ),
                    critical: positive_float_setting(
                        settings,
                        &format!("{path}.notional_usd.critical"),
                        fallback.notional_usd.critical,
                    ),
                    s: positive_float_setting(
                        settings,
                        &format!("{path}.notional_usd.s"),
                        fallback.notional_usd.s,
                    ),
                },
            },
        );
    }
    profiles
}

fn load_threshold_profile_windows(
    settings: &::config::Config,
    profile_path: &str,
    fallback: &ContractWhaleThresholdProfileConfig,
) -> BTreeMap<u64, ContractWhaleThresholds> {
    [5_u64, 15, 60]
        .into_iter()
        .map(|window_sec| {
            let default = fallback
                .thresholds_btc
                .get(&window_sec)
                .copied()
                .unwrap_or_else(|| ContractWhaleThresholds::for_window(window_sec));
            let window_key = format!("window_{window_sec}s");
            (
                window_sec,
                ContractWhaleThresholds {
                    high_btc: positive_float_setting(
                        settings,
                        &format!("{profile_path}.high.{window_key}"),
                        default.high_btc,
                    ),
                    critical_btc: positive_float_setting(
                        settings,
                        &format!("{profile_path}.critical.{window_key}"),
                        default.critical_btc,
                    ),
                    s_btc: positive_float_setting(
                        settings,
                        &format!("{profile_path}.s.{window_key}"),
                        default.s_btc,
                    ),
                },
            )
        })
        .collect()
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

fn usize_setting_with_env(
    settings: &::config::Config,
    env_key: &str,
    path: &str,
    default: usize,
) -> usize {
    if let Ok(value) = std::env::var(env_key) {
        if let Ok(parsed) = value.parse::<usize>() {
            return parsed;
        }
        warn_invalid(path, value, default);
        return default;
    }
    usize_setting(settings, path, default)
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

fn bool_setting(settings: &::config::Config, env_key: &str, toml_key: &str, default: bool) -> bool {
    std::env::var(env_key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .or_else(|| settings.get_bool(toml_key).ok())
        .unwrap_or(default)
}

fn optional_string_setting(
    settings: &::config::Config,
    path: &str,
    default: Option<String>,
) -> Option<String> {
    settings
        .get_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or(default)
}

fn string_array_setting(
    settings: &::config::Config,
    path: &str,
    default: Vec<String>,
) -> Vec<String> {
    settings
        .get_array(path)
        .ok()
        .map(|items| {
            items
                .into_iter()
                .filter_map(|item| item.into_string().ok())
                .map(|item| item.trim().to_ascii_lowercase())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
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
