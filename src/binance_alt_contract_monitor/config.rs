use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{OnceLock, RwLock},
};

use super::types::{AltContractSeverity, AltContractSymbolTier, AltContractTierThresholds};

static GLOBAL_CONFIG: OnceLock<RwLock<BinanceAltContractRuntimeConfig>> = OnceLock::new();
const DISABLED_NOTIONAL_USD: f64 = 999_999_999_999.0;

#[derive(Debug, Clone)]
pub struct BinanceAltContractRuntimeConfig {
    pub enabled: bool,
    pub dry_run: bool,
    pub exchange: BinanceAltExchangeConfig,
    pub symbol_universe: BinanceAltSymbolUniverseConfig,
    pub windows_sec: Vec<u64>,
    pub dynamic: BinanceAltDynamicConfig,
    pub data_quality: BinanceAltDataQualityConfig,
    pub oi_scheduler: BinanceAltOiSchedulerConfig,
    pub storage: BinanceAltStorageConfig,
    pub display: BinanceAltDisplayConfig,
    pub discord: BinanceAltDiscordConfig,
    pub tier_d_rules: BinanceAltTierDRulesConfig,
    pub tier_e_rules: BinanceAltTierERulesConfig,
    pub persistence_path: PathBuf,
    pub thresholds: BTreeMap<AltContractSymbolTier, AltContractTierThresholds>,
    pub tier_d_min_signal_score: u8,
}

impl BinanceAltContractRuntimeConfig {
    pub fn effective_universe_mode(&self) -> BinanceAltUniverseMode {
        if !self.symbol_universe.whitelist.is_empty() {
            BinanceAltUniverseMode::WhitelistOnly
        } else {
            self.symbol_universe.universe_mode
        }
    }

    pub fn enabled_symbols(&self) -> Vec<String> {
        let mut symbols = match self.effective_universe_mode() {
            BinanceAltUniverseMode::AllBinanceUsdtPerp => {
                if self.symbol_universe.whitelist.is_empty() {
                    default_alt_symbols()
                } else {
                    self.symbol_universe.whitelist.clone()
                }
            }
            BinanceAltUniverseMode::TopN | BinanceAltUniverseMode::WhitelistOnly => {
                if self.symbol_universe.whitelist.is_empty() {
                    default_alt_symbols()
                } else {
                    self.symbol_universe.whitelist.clone()
                }
            }
        };
        let excludes = self
            .symbol_universe
            .exclude_symbols
            .iter()
            .map(|symbol| normalize_product_id(symbol))
            .collect::<BTreeSet<_>>();
        let blacklist = self
            .symbol_universe
            .blacklist
            .iter()
            .map(|symbol| normalize_product_id(symbol))
            .collect::<BTreeSet<_>>();
        symbols = symbols
            .into_iter()
            .map(|symbol| normalize_product_id(&symbol))
            .filter(|symbol| !excludes.contains(symbol) && !blacklist.contains(symbol))
            .collect();
        if self.symbol_universe.symbol_limit > 0
            && !matches!(
                self.effective_universe_mode(),
                BinanceAltUniverseMode::AllBinanceUsdtPerp
            )
        {
            symbols.truncate(self.symbol_universe.symbol_limit);
        }
        symbols.sort();
        symbols.dedup();
        symbols
    }

    pub fn symbol_enabled(&self, product_id: &str) -> bool {
        let product_id = normalize_product_id(product_id);
        if self
            .symbol_universe
            .exclude_symbols
            .iter()
            .map(|symbol| normalize_product_id(symbol))
            .any(|symbol| symbol == product_id)
            || self
                .symbol_universe
                .blacklist
                .iter()
                .map(|symbol| normalize_product_id(symbol))
                .any(|symbol| symbol == product_id)
        {
            return false;
        }
        match self.effective_universe_mode() {
            BinanceAltUniverseMode::AllBinanceUsdtPerp => product_id.ends_with("USDT"),
            BinanceAltUniverseMode::TopN | BinanceAltUniverseMode::WhitelistOnly => self
                .enabled_symbols()
                .iter()
                .any(|symbol| symbol == &product_id),
        }
    }

    pub fn thresholds_for_tier(&self, tier: AltContractSymbolTier) -> AltContractTierThresholds {
        self.thresholds
            .get(&tier)
            .copied()
            .unwrap_or_else(|| default_thresholds()[&AltContractSymbolTier::B])
    }
}

impl Default for BinanceAltContractRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            dry_run: true,
            exchange: BinanceAltExchangeConfig::default(),
            symbol_universe: BinanceAltSymbolUniverseConfig::default(),
            windows_sec: vec![15, 60, 300],
            dynamic: BinanceAltDynamicConfig::default(),
            data_quality: BinanceAltDataQualityConfig::default(),
            oi_scheduler: BinanceAltOiSchedulerConfig::default(),
            storage: BinanceAltStorageConfig::default(),
            display: BinanceAltDisplayConfig::default(),
            discord: BinanceAltDiscordConfig::default(),
            tier_d_rules: BinanceAltTierDRulesConfig::default(),
            tier_e_rules: BinanceAltTierERulesConfig::default(),
            persistence_path: PathBuf::from(".runtime/binance-alt-contract-signals.jsonl"),
            thresholds: default_thresholds(),
            tier_d_min_signal_score: 88,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltExchangeConfig {
    pub binance_enabled: bool,
}

impl Default for BinanceAltExchangeConfig {
    fn default() -> Self {
        Self {
            binance_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinanceAltUniverseMode {
    AllBinanceUsdtPerp,
    TopN,
    WhitelistOnly,
}

impl BinanceAltUniverseMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllBinanceUsdtPerp => "all_binance_usdt_perp",
            Self::TopN => "top_n",
            Self::WhitelistOnly => "whitelist_only",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltSymbolUniverseConfig {
    pub universe_mode: BinanceAltUniverseMode,
    pub quote_asset: String,
    pub contract_type: String,
    pub status: String,
    pub symbol_limit: usize,
    pub min_24h_quote_volume_usd: f64,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
    pub exclude_symbols: Vec<String>,
}

impl Default for BinanceAltSymbolUniverseConfig {
    fn default() -> Self {
        Self {
            universe_mode: BinanceAltUniverseMode::AllBinanceUsdtPerp,
            quote_asset: "USDT".to_string(),
            contract_type: "PERPETUAL".to_string(),
            status: "TRADING".to_string(),
            symbol_limit: 0,
            min_24h_quote_volume_usd: 0.0,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
            exclude_symbols: vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltDynamicConfig {
    pub enabled: bool,
    pub lookback_minutes: u64,
    pub min_samples: usize,
    pub high_multiple: f64,
    pub critical_multiple: f64,
    pub s_multiple: f64,
}

impl Default for BinanceAltDynamicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            lookback_minutes: 60,
            min_samples: 20,
            high_multiple: 4.0,
            critical_multiple: 6.0,
            s_multiple: 9.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltDataQualityConfig {
    pub min_discord_quality: u8,
    pub warmup_ms: i64,
    pub heartbeat_stale_ms: i64,
}

impl Default for BinanceAltDataQualityConfig {
    fn default() -> Self {
        Self {
            min_discord_quality: 70,
            warmup_ms: 60_000,
            heartbeat_stale_ms: 45_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltOiSchedulerConfig {
    pub enabled: bool,
    pub all_symbols_interval_sec: u64,
    pub hot_symbols_interval_sec: u64,
    pub candidate_ttl_sec: u64,
    pub max_oi_requests_per_sec: u64,
    pub immediate_fetch_on_candidate: bool,
}

impl Default for BinanceAltOiSchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            all_symbols_interval_sec: 300,
            hot_symbols_interval_sec: 15,
            candidate_ttl_sec: 600,
            max_oi_requests_per_sec: 5,
            immediate_fetch_on_candidate: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltStorageConfig {
    pub persist_all_1s: bool,
    pub persist_all_1m: bool,
    pub persist_hot_1s: bool,
    pub hot_1s_retention_hours: u64,
    pub flow_1m_retention_days: u64,
    pub signals_retention_days: u64,
}

impl Default for BinanceAltStorageConfig {
    fn default() -> Self {
        Self {
            persist_all_1s: false,
            persist_all_1m: true,
            persist_hot_1s: true,
            hot_1s_retention_hours: 24,
            flow_1m_retention_days: 14,
            signals_retention_days: 180,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltDisplayConfig {
    pub min_notional_usd: f64,
}

impl Default for BinanceAltDisplayConfig {
    fn default() -> Self {
        Self {
            min_notional_usd: 500_000.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltDiscordConfig {
    pub enabled: bool,
    pub dry_run: bool,
    pub webhook_env: String,
    pub cooldown_sec: i64,
    pub global_hourly_cap: usize,
    pub min_data_quality: u8,
    pub min_display_notional_usd: f64,
    pub push_build_score: u8,
    pub push_abnormal_score: u8,
    pub push_main_force_confidence: u8,
    pub push_min_evidence_count: u8,
    pub allow_liquidation_alerts: bool,
    pub push_liquidation_abnormal_score: u8,
    pub min_abnormal_score: u8,
    pub min_build_score: u8,
    pub tier_thresholds: BTreeMap<AltContractSymbolTier, BinanceAltDiscordTierConfig>,
    pub market_wide_symbol_count: usize,
    pub market_wide_ratio: f64,
    pub market_wide_top_n: u32,
}

impl Default for BinanceAltDiscordConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dry_run: true,
            webhook_env: "BACM_DISCORD_WEBHOOK_URL".to_string(),
            cooldown_sec: 900,
            global_hourly_cap: 12,
            min_data_quality: 70,
            min_display_notional_usd: 500_000.0,
            push_build_score: 80,
            push_abnormal_score: 90,
            push_main_force_confidence: 75,
            push_min_evidence_count: 4,
            allow_liquidation_alerts: true,
            push_liquidation_abnormal_score: 92,
            min_abnormal_score: 85,
            min_build_score: 80,
            tier_thresholds: default_discord_tier_thresholds(),
            market_wide_symbol_count: 15,
            market_wide_ratio: 0.12,
            market_wide_top_n: 5,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BinanceAltDiscordTierConfig {
    pub enabled: bool,
    pub min_notional_usd: f64,
    pub critical_notional_usd: f64,
    pub s_notional_usd: f64,
    pub s_enabled: bool,
    pub require_build_score: u8,
    pub require_abnormal_score: u8,
    pub require_non_liquidation: bool,
}

#[derive(Debug, Clone)]
pub struct BinanceAltTierDRulesConfig {
    pub discord_min_abnormal_score: u8,
    pub discord_min_build_score: u8,
    pub require_non_liquidation: bool,
    pub max_severity_without_build_confirmation: AltContractSeverity,
}

impl Default for BinanceAltTierDRulesConfig {
    fn default() -> Self {
        Self {
            discord_min_abnormal_score: 85,
            discord_min_build_score: 85,
            require_non_liquidation: true,
            max_severity_without_build_confirmation: AltContractSeverity::High,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceAltTierERulesConfig {
    pub discord_min_abnormal_score: u8,
    pub discord_min_build_score: u8,
    pub min_dynamic_multiple: f64,
    pub require_oi_confirmation: bool,
    pub require_non_liquidation: bool,
    pub max_severity_without_build_confirmation: AltContractSeverity,
}

impl Default for BinanceAltTierERulesConfig {
    fn default() -> Self {
        Self {
            discord_min_abnormal_score: 92,
            discord_min_build_score: 90,
            min_dynamic_multiple: 10.0,
            require_oi_confirmation: true,
            require_non_liquidation: true,
            max_severity_without_build_confirmation: AltContractSeverity::High,
        }
    }
}

pub fn binance_alt_contract_runtime_config() -> BinanceAltContractRuntimeConfig {
    global_config()
        .read()
        .expect("binance alt contract config lock poisoned")
        .clone()
}

pub fn set_binance_alt_contract_runtime_config(config: BinanceAltContractRuntimeConfig) {
    *global_config()
        .write()
        .expect("binance alt contract config lock poisoned") = config;
}

pub fn reset_binance_alt_contract_runtime_config() {
    set_binance_alt_contract_runtime_config(BinanceAltContractRuntimeConfig::default());
}

pub fn load_binance_alt_contract_runtime_config_from_settings(
    settings: &::config::Config,
) -> BinanceAltContractRuntimeConfig {
    let fallback = BinanceAltContractRuntimeConfig::default();
    let dry_run = bool_setting(
        settings,
        "BINANCE_ALT_CONTRACT_DRY_RUN",
        "binance_alt_contract_monitor.dry_run",
        fallback.dry_run,
    );
    BinanceAltContractRuntimeConfig {
        enabled: bool_setting(
            settings,
            "BINANCE_ALT_CONTRACT_ENABLED",
            "binance_alt_contract_monitor.enabled",
            fallback.enabled,
        ),
        dry_run,
        exchange: BinanceAltExchangeConfig {
            binance_enabled: bool_setting(
                settings,
                "BINANCE_ALT_CONTRACT_BINANCE_ENABLED",
                "binance_alt_contract_monitor.exchanges.binance.enabled",
                fallback.exchange.binance_enabled,
            ),
        },
        symbol_universe: BinanceAltSymbolUniverseConfig {
            universe_mode: universe_mode_setting(
                settings,
                "BINANCE_ALT_CONTRACT_UNIVERSE_MODE",
                "binance_alt_contract_monitor.universe_mode",
                fallback.symbol_universe.universe_mode,
            ),
            quote_asset: string_setting(
                settings,
                "binance_alt_contract_monitor.symbol_filter.quote_asset",
                &fallback.symbol_universe.quote_asset,
            ),
            contract_type: string_setting(
                settings,
                "binance_alt_contract_monitor.symbol_filter.contract_type",
                &fallback.symbol_universe.contract_type,
            ),
            status: string_setting(
                settings,
                "binance_alt_contract_monitor.symbol_filter.status",
                &fallback.symbol_universe.status,
            ),
            symbol_limit: usize_setting(
                settings,
                "binance_alt_contract_monitor.symbol_limit",
                fallback.symbol_universe.symbol_limit,
            ),
            min_24h_quote_volume_usd: nonnegative_f64_setting(
                settings,
                "binance_alt_contract_monitor.symbol_filter.min_24h_quote_volume_usd",
                fallback.symbol_universe.min_24h_quote_volume_usd,
            ),
            whitelist: string_vec_setting(
                settings,
                "BINANCE_ALT_CONTRACT_WHITELIST",
                "binance_alt_contract_monitor.whitelist",
                fallback.symbol_universe.whitelist.clone(),
            ),
            blacklist: string_vec_setting(
                settings,
                "BINANCE_ALT_CONTRACT_BLACKLIST",
                "binance_alt_contract_monitor.blacklist",
                fallback.symbol_universe.blacklist.clone(),
            ),
            exclude_symbols: string_vec_setting(
                settings,
                "BINANCE_ALT_CONTRACT_EXCLUDE_SYMBOLS",
                "binance_alt_contract_monitor.exclude_symbols",
                fallback.symbol_universe.exclude_symbols.clone(),
            ),
        },
        windows_sec: u64_vec_setting(
            settings,
            "binance_alt_contract_monitor.windows.trade_windows_sec",
            fallback.windows_sec.clone(),
        ),
        dynamic: BinanceAltDynamicConfig {
            enabled: settings
                .get_bool("binance_alt_contract_monitor.dynamic_threshold.enabled")
                .unwrap_or(fallback.dynamic.enabled),
            lookback_minutes: u64_setting(
                settings,
                "binance_alt_contract_monitor.dynamic_threshold.lookback_minutes",
                fallback.dynamic.lookback_minutes,
            ),
            min_samples: usize_setting(
                settings,
                "binance_alt_contract_monitor.dynamic_threshold.min_samples",
                fallback.dynamic.min_samples,
            ),
            high_multiple: f64_setting(
                settings,
                "binance_alt_contract_monitor.dynamic_threshold.high_multiple",
                fallback.dynamic.high_multiple,
            ),
            critical_multiple: f64_setting(
                settings,
                "binance_alt_contract_monitor.dynamic_threshold.critical_multiple",
                fallback.dynamic.critical_multiple,
            ),
            s_multiple: f64_setting(
                settings,
                "binance_alt_contract_monitor.dynamic_threshold.s_multiple",
                fallback.dynamic.s_multiple,
            ),
        },
        data_quality: BinanceAltDataQualityConfig {
            min_discord_quality: u8_setting(
                settings,
                "binance_alt_contract_monitor.data_quality.min_discord_quality",
                fallback.data_quality.min_discord_quality,
            ),
            warmup_ms: i64_setting(
                settings,
                "binance_alt_contract_monitor.data_quality.warmup_ms",
                fallback.data_quality.warmup_ms,
            ),
            heartbeat_stale_ms: i64_setting(
                settings,
                "binance_alt_contract_monitor.data_quality.heartbeat_stale_ms",
                fallback.data_quality.heartbeat_stale_ms,
            ),
        },
        oi_scheduler: BinanceAltOiSchedulerConfig {
            enabled: settings
                .get_bool("binance_alt_contract_monitor.oi_scheduler.enabled")
                .unwrap_or(fallback.oi_scheduler.enabled),
            all_symbols_interval_sec: u64_setting(
                settings,
                "binance_alt_contract_monitor.oi_scheduler.all_symbols_interval_sec",
                fallback.oi_scheduler.all_symbols_interval_sec,
            ),
            hot_symbols_interval_sec: u64_setting(
                settings,
                "binance_alt_contract_monitor.oi_scheduler.hot_symbols_interval_sec",
                fallback.oi_scheduler.hot_symbols_interval_sec,
            ),
            candidate_ttl_sec: u64_setting(
                settings,
                "binance_alt_contract_monitor.oi_scheduler.candidate_ttl_sec",
                fallback.oi_scheduler.candidate_ttl_sec,
            ),
            max_oi_requests_per_sec: u64_setting(
                settings,
                "binance_alt_contract_monitor.oi_scheduler.max_oi_requests_per_sec",
                fallback.oi_scheduler.max_oi_requests_per_sec,
            ),
            immediate_fetch_on_candidate: settings
                .get_bool("binance_alt_contract_monitor.oi_scheduler.immediate_fetch_on_candidate")
                .unwrap_or(fallback.oi_scheduler.immediate_fetch_on_candidate),
        },
        storage: BinanceAltStorageConfig {
            persist_all_1s: settings
                .get_bool("binance_alt_contract_monitor.storage.persist_all_1s")
                .unwrap_or(fallback.storage.persist_all_1s),
            persist_all_1m: settings
                .get_bool("binance_alt_contract_monitor.storage.persist_all_1m")
                .unwrap_or(fallback.storage.persist_all_1m),
            persist_hot_1s: settings
                .get_bool("binance_alt_contract_monitor.storage.persist_hot_1s")
                .unwrap_or(fallback.storage.persist_hot_1s),
            hot_1s_retention_hours: u64_setting(
                settings,
                "binance_alt_contract_monitor.storage.hot_1s_retention_hours",
                fallback.storage.hot_1s_retention_hours,
            ),
            flow_1m_retention_days: u64_setting(
                settings,
                "binance_alt_contract_monitor.storage.flow_1m_retention_days",
                fallback.storage.flow_1m_retention_days,
            ),
            signals_retention_days: u64_setting(
                settings,
                "binance_alt_contract_monitor.storage.signals_retention_days",
                fallback.storage.signals_retention_days,
            ),
        },
        display: BinanceAltDisplayConfig {
            min_notional_usd: nonnegative_f64_setting(
                settings,
                "binance_alt_contract_monitor.display.min_notional_usd",
                fallback.display.min_notional_usd,
            ),
        },
        discord: BinanceAltDiscordConfig {
            enabled: settings
                .get_bool("binance_alt_contract_monitor.discord.enabled")
                .unwrap_or(fallback.discord.enabled),
            dry_run: settings
                .get_bool("binance_alt_contract_monitor.discord.dry_run")
                .unwrap_or(dry_run),
            webhook_env: string_setting(
                settings,
                "binance_alt_contract_monitor.discord.webhook_env",
                &fallback.discord.webhook_env,
            ),
            cooldown_sec: i64_setting(
                settings,
                "binance_alt_contract_monitor.discord.cooldown_sec",
                fallback.discord.cooldown_sec,
            ),
            global_hourly_cap: usize_setting(
                settings,
                "binance_alt_contract_monitor.discord.global_hourly_cap",
                fallback.discord.global_hourly_cap,
            ),
            min_data_quality: u8_setting(
                settings,
                "binance_alt_contract_monitor.discord.min_data_quality",
                fallback.discord.min_data_quality,
            ),
            min_display_notional_usd: nonnegative_f64_setting(
                settings,
                "binance_alt_contract_monitor.discord.min_display_notional_usd",
                fallback.discord.min_display_notional_usd,
            ),
            push_build_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.discord.push_build_score",
                fallback.discord.push_build_score,
            ),
            push_abnormal_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.discord.push_abnormal_score",
                fallback.discord.push_abnormal_score,
            ),
            push_main_force_confidence: u8_setting(
                settings,
                "binance_alt_contract_monitor.discord.push_main_force_confidence",
                fallback.discord.push_main_force_confidence,
            ),
            push_min_evidence_count: u8_setting(
                settings,
                "binance_alt_contract_monitor.discord.push_min_evidence_count",
                fallback.discord.push_min_evidence_count,
            ),
            allow_liquidation_alerts: settings
                .get_bool("binance_alt_contract_monitor.discord.allow_liquidation_alerts")
                .unwrap_or(fallback.discord.allow_liquidation_alerts),
            push_liquidation_abnormal_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.discord.push_liquidation_abnormal_score",
                fallback.discord.push_liquidation_abnormal_score,
            ),
            min_abnormal_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.discord.min_abnormal_score",
                fallback.discord.min_abnormal_score,
            ),
            min_build_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.discord.min_build_score",
                fallback.discord.min_build_score,
            ),
            tier_thresholds: load_discord_tier_thresholds(
                settings,
                &fallback.discord.tier_thresholds,
            ),
            market_wide_symbol_count: usize_setting(
                settings,
                "binance_alt_contract_monitor.discord.market_wide_symbol_count",
                fallback.discord.market_wide_symbol_count,
            ),
            market_wide_ratio: f64_setting(
                settings,
                "binance_alt_contract_monitor.discord.market_wide_ratio",
                fallback.discord.market_wide_ratio,
            ),
            market_wide_top_n: u32_setting(
                settings,
                "binance_alt_contract_monitor.discord.market_wide_top_n",
                fallback.discord.market_wide_top_n,
            ),
        },
        tier_d_rules: BinanceAltTierDRulesConfig {
            discord_min_abnormal_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.tier_rules.tier_d.discord_min_abnormal_score",
                fallback.tier_d_rules.discord_min_abnormal_score,
            ),
            discord_min_build_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.tier_rules.tier_d.discord_min_build_score",
                fallback.tier_d_rules.discord_min_build_score,
            ),
            require_non_liquidation: settings
                .get_bool("binance_alt_contract_monitor.tier_rules.tier_d.require_non_liquidation")
                .unwrap_or(fallback.tier_d_rules.require_non_liquidation),
            max_severity_without_build_confirmation: severity_setting(
                settings,
                "binance_alt_contract_monitor.tier_rules.tier_d.max_severity_without_build_confirmation",
                fallback
                    .tier_d_rules
                    .max_severity_without_build_confirmation,
            ),
        },
        tier_e_rules: BinanceAltTierERulesConfig {
            discord_min_abnormal_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.tier_rules.tier_e.discord_min_abnormal_score",
                fallback.tier_e_rules.discord_min_abnormal_score,
            ),
            discord_min_build_score: u8_setting(
                settings,
                "binance_alt_contract_monitor.tier_rules.tier_e.discord_min_build_score",
                fallback.tier_e_rules.discord_min_build_score,
            ),
            min_dynamic_multiple: f64_setting(
                settings,
                "binance_alt_contract_monitor.tier_rules.tier_e.min_dynamic_multiple",
                fallback.tier_e_rules.min_dynamic_multiple,
            ),
            require_oi_confirmation: settings
                .get_bool("binance_alt_contract_monitor.tier_rules.tier_e.require_oi_confirmation")
                .unwrap_or(fallback.tier_e_rules.require_oi_confirmation),
            require_non_liquidation: settings
                .get_bool("binance_alt_contract_monitor.tier_rules.tier_e.require_non_liquidation")
                .unwrap_or(fallback.tier_e_rules.require_non_liquidation),
            max_severity_without_build_confirmation: severity_setting(
                settings,
                "binance_alt_contract_monitor.tier_rules.tier_e.max_severity_without_build_confirmation",
                fallback
                    .tier_e_rules
                    .max_severity_without_build_confirmation,
            ),
        },
        persistence_path: PathBuf::from(string_setting(
            settings,
            "binance_alt_contract_monitor.persistence_path",
            fallback.persistence_path.to_string_lossy().as_ref(),
        )),
        thresholds: load_thresholds(settings, &fallback.thresholds),
        tier_d_min_signal_score: u8_setting(
            settings,
            "binance_alt_contract_monitor.tier_d_min_signal_score",
            fallback.tier_d_min_signal_score,
        ),
    }
}

fn global_config() -> &'static RwLock<BinanceAltContractRuntimeConfig> {
    GLOBAL_CONFIG.get_or_init(|| RwLock::new(BinanceAltContractRuntimeConfig::default()))
}

fn default_thresholds() -> BTreeMap<AltContractSymbolTier, AltContractTierThresholds> {
    BTreeMap::from([
        (
            AltContractSymbolTier::A,
            AltContractTierThresholds {
                high_notional_usd: 20_000_000.0,
                critical_notional_usd: 50_000_000.0,
                s_notional_usd: 120_000_000.0,
            },
        ),
        (
            AltContractSymbolTier::B,
            AltContractTierThresholds {
                high_notional_usd: 10_000_000.0,
                critical_notional_usd: 25_000_000.0,
                s_notional_usd: 60_000_000.0,
            },
        ),
        (
            AltContractSymbolTier::C,
            AltContractTierThresholds {
                high_notional_usd: 5_000_000.0,
                critical_notional_usd: 15_000_000.0,
                s_notional_usd: 35_000_000.0,
            },
        ),
        (
            AltContractSymbolTier::D,
            AltContractTierThresholds {
                high_notional_usd: 2_000_000.0,
                critical_notional_usd: 6_000_000.0,
                s_notional_usd: 15_000_000.0,
            },
        ),
        (
            AltContractSymbolTier::E,
            AltContractTierThresholds {
                high_notional_usd: 1_000_000.0,
                critical_notional_usd: 3_000_000.0,
                s_notional_usd: 8_000_000.0,
            },
        ),
    ])
}

fn default_discord_tier_thresholds() -> BTreeMap<AltContractSymbolTier, BinanceAltDiscordTierConfig>
{
    BTreeMap::from([
        (
            AltContractSymbolTier::A,
            BinanceAltDiscordTierConfig {
                enabled: true,
                min_notional_usd: 3_000_000.0,
                critical_notional_usd: 8_000_000.0,
                s_notional_usd: 20_000_000.0,
                s_enabled: true,
                require_build_score: 80,
                require_abnormal_score: 90,
                require_non_liquidation: false,
            },
        ),
        (
            AltContractSymbolTier::B,
            BinanceAltDiscordTierConfig {
                enabled: true,
                min_notional_usd: 1_500_000.0,
                critical_notional_usd: 3_000_000.0,
                s_notional_usd: 8_000_000.0,
                s_enabled: true,
                require_build_score: 80,
                require_abnormal_score: 90,
                require_non_liquidation: false,
            },
        ),
        (
            AltContractSymbolTier::C,
            BinanceAltDiscordTierConfig {
                enabled: true,
                min_notional_usd: 500_000.0,
                critical_notional_usd: 1_500_000.0,
                s_notional_usd: 4_000_000.0,
                s_enabled: true,
                require_build_score: 80,
                require_abnormal_score: 90,
                require_non_liquidation: false,
            },
        ),
        (
            AltContractSymbolTier::D,
            BinanceAltDiscordTierConfig {
                enabled: true,
                min_notional_usd: 500_000.0,
                critical_notional_usd: 1_200_000.0,
                s_notional_usd: DISABLED_NOTIONAL_USD,
                s_enabled: false,
                require_build_score: 85,
                require_abnormal_score: 85,
                require_non_liquidation: true,
            },
        ),
        (
            AltContractSymbolTier::E,
            BinanceAltDiscordTierConfig {
                enabled: false,
                min_notional_usd: DISABLED_NOTIONAL_USD,
                critical_notional_usd: DISABLED_NOTIONAL_USD,
                s_notional_usd: DISABLED_NOTIONAL_USD,
                s_enabled: false,
                require_build_score: 90,
                require_abnormal_score: 92,
                require_non_liquidation: true,
            },
        ),
    ])
}

fn load_thresholds(
    settings: &::config::Config,
    fallback: &BTreeMap<AltContractSymbolTier, AltContractTierThresholds>,
) -> BTreeMap<AltContractSymbolTier, AltContractTierThresholds> {
    [
        AltContractSymbolTier::A,
        AltContractSymbolTier::B,
        AltContractSymbolTier::C,
        AltContractSymbolTier::D,
        AltContractSymbolTier::E,
    ]
    .into_iter()
    .map(|tier| {
        let key = match tier {
            AltContractSymbolTier::A => "a",
            AltContractSymbolTier::B => "b",
            AltContractSymbolTier::C => "c",
            AltContractSymbolTier::D => "d",
            AltContractSymbolTier::E => "e",
        };
        let default = fallback[&tier];
        (
            tier,
            AltContractTierThresholds {
                high_notional_usd: f64_setting(
                    settings,
                    &format!("binance_alt_contract_monitor.thresholds.{key}.high_notional_usd"),
                    default.high_notional_usd,
                ),
                critical_notional_usd: f64_setting(
                    settings,
                    &format!("binance_alt_contract_monitor.thresholds.{key}.critical_notional_usd"),
                    default.critical_notional_usd,
                ),
                s_notional_usd: f64_setting(
                    settings,
                    &format!("binance_alt_contract_monitor.thresholds.{key}.s_notional_usd"),
                    default.s_notional_usd,
                ),
            },
        )
    })
    .collect()
}

fn load_discord_tier_thresholds(
    settings: &::config::Config,
    fallback: &BTreeMap<AltContractSymbolTier, BinanceAltDiscordTierConfig>,
) -> BTreeMap<AltContractSymbolTier, BinanceAltDiscordTierConfig> {
    [
        AltContractSymbolTier::A,
        AltContractSymbolTier::B,
        AltContractSymbolTier::C,
        AltContractSymbolTier::D,
        AltContractSymbolTier::E,
    ]
    .into_iter()
    .map(|tier| {
        let key = match tier {
            AltContractSymbolTier::A => "tier_a",
            AltContractSymbolTier::B => "tier_b",
            AltContractSymbolTier::C => "tier_c",
            AltContractSymbolTier::D => "tier_d",
            AltContractSymbolTier::E => "tier_e",
        };
        let default = fallback[&tier];
        (
            tier,
            BinanceAltDiscordTierConfig {
                enabled: settings
                    .get_bool(&format!(
                        "binance_alt_contract_monitor.discord.{key}.enabled"
                    ))
                    .unwrap_or(default.enabled),
                min_notional_usd: f64_setting(
                    settings,
                    &format!("binance_alt_contract_monitor.discord.{key}.min_notional_usd"),
                    default.min_notional_usd,
                ),
                critical_notional_usd: f64_setting(
                    settings,
                    &format!("binance_alt_contract_monitor.discord.{key}.critical_notional_usd"),
                    default.critical_notional_usd,
                ),
                s_notional_usd: f64_setting(
                    settings,
                    &format!("binance_alt_contract_monitor.discord.{key}.s_notional_usd"),
                    default.s_notional_usd,
                ),
                s_enabled: settings
                    .get_bool(&format!(
                        "binance_alt_contract_monitor.discord.{key}.s_enabled"
                    ))
                    .unwrap_or(default.s_enabled),
                require_build_score: u8_setting(
                    settings,
                    &format!("binance_alt_contract_monitor.discord.{key}.require_build_score"),
                    default.require_build_score,
                ),
                require_abnormal_score: u8_setting(
                    settings,
                    &format!("binance_alt_contract_monitor.discord.{key}.require_abnormal_score"),
                    default.require_abnormal_score,
                ),
                require_non_liquidation: settings
                    .get_bool(&format!(
                        "binance_alt_contract_monitor.discord.{key}.require_non_liquidation"
                    ))
                    .unwrap_or(default.require_non_liquidation),
            },
        )
    })
    .collect()
}

fn default_alt_symbols() -> Vec<String> {
    [
        "SOLUSDT", "BNBUSDT", "XRPUSDT", "DOGEUSDT", "ADAUSDT", "LINKUSDT", "AVAXUSDT", "SUIUSDT",
        "LTCUSDT", "TRXUSDT", "DOTUSDT", "BCHUSDT",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn normalize_product_id(value: &str) -> String {
    value.trim().to_ascii_uppercase()
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
        .unwrap_or_else(|_| default.to_string())
}

fn string_vec_setting(
    settings: &::config::Config,
    env_key: &str,
    path: &str,
    default: Vec<String>,
) -> Vec<String> {
    if let Ok(value) = std::env::var(env_key) {
        let values = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !values.is_empty() {
            return values;
        }
    }
    settings
        .get_array(path)
        .ok()
        .map(|values| {
            values
                .into_iter()
                .filter_map(|value| value.into_string().ok())
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or(default)
}

fn u64_vec_setting(settings: &::config::Config, path: &str, default: Vec<u64>) -> Vec<u64> {
    settings
        .get_array(path)
        .ok()
        .map(|values| {
            values
                .into_iter()
                .filter_map(|value| value.into_int().ok())
                .filter_map(|value| u64::try_from(value).ok())
                .filter(|value| *value > 0)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or(default)
}

fn f64_setting(settings: &::config::Config, path: &str, default: f64) -> f64 {
    settings
        .get_float(path)
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
}

fn nonnegative_f64_setting(settings: &::config::Config, path: &str, default: f64) -> f64 {
    settings
        .get_float(path)
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default)
}

fn i64_setting(settings: &::config::Config, path: &str, default: i64) -> i64 {
    settings
        .get_int(path)
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn u64_setting(settings: &::config::Config, path: &str, default: u64) -> u64 {
    settings
        .get_int(path)
        .ok()
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn usize_setting(settings: &::config::Config, path: &str, default: usize) -> usize {
    settings
        .get_int(path)
        .ok()
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(default)
}

fn u32_setting(settings: &::config::Config, path: &str, default: u32) -> u32 {
    settings
        .get_int(path)
        .ok()
        .and_then(|value| u32::try_from(value).ok())
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

fn severity_setting(
    settings: &::config::Config,
    path: &str,
    default: AltContractSeverity,
) -> AltContractSeverity {
    match settings
        .get_string(path)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "s" => AltContractSeverity::S,
        "critical" => AltContractSeverity::Critical,
        "high" => AltContractSeverity::High,
        "medium" => AltContractSeverity::Medium,
        "calm" => AltContractSeverity::Calm,
        _ => default,
    }
}

fn universe_mode_setting(
    settings: &::config::Config,
    env_key: &str,
    toml_key: &str,
    default: BinanceAltUniverseMode,
) -> BinanceAltUniverseMode {
    let value = std::env::var(env_key)
        .ok()
        .or_else(|| settings.get_string(toml_key).ok());
    match value
        .unwrap_or_else(|| default.as_str().to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "all" | "all_binance_usdt_perp" | "all_usdt_perp" => {
            BinanceAltUniverseMode::AllBinanceUsdtPerp
        }
        "top" | "top_n" | "topn" => BinanceAltUniverseMode::TopN,
        "whitelist" | "whitelist_only" => BinanceAltUniverseMode::WhitelistOnly,
        _ => default,
    }
}
