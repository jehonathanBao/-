use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard, OnceLock},
};

use btc_toxic_flow_monitor_rs::contract_whale_monitor::{
    config::{ContractWhaleRuntimeConfig, ContractWhaleThresholdProfile},
    detector::detect_contract_whale_signal_with_config,
    types::{
        ContractWhaleExchangeStatus, ContractWhaleLiquidationContext, ContractWhaleMarketContext,
        ContractWhaleWindowStats, ExchangeFlowContribution,
    },
};

const COINBASE_AUTH_ENVS: [&str; 3] = [
    "COINBASE_INTX_KEY",
    "COINBASE_INTX_SECRET",
    "COINBASE_INTX_PASSPHRASE",
];

fn threshold_profile_test_guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct EnvRestore {
    values: Vec<(&'static str, Option<String>)>,
}

impl EnvRestore {
    fn clear_coinbase_auth() -> Self {
        let values = COINBASE_AUTH_ENVS
            .into_iter()
            .map(|key| {
                let previous = std::env::var(key).ok();
                std::env::remove_var(key);
                (key, previous)
            })
            .collect();
        Self { values }
    }

    fn set_coinbase_auth() -> Self {
        let values = COINBASE_AUTH_ENVS
            .into_iter()
            .map(|key| {
                let previous = std::env::var(key).ok();
                std::env::set_var(key, "test-value");
                (key, previous)
            })
            .collect();
        Self { values }
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (key, value) in &self.values {
            match value {
                Some(previous) => std::env::set_var(key, previous),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[test]
fn default_profile_ignores_coinbase_spot_only_source() {
    let _guard = threshold_profile_test_guard();
    let _env = EnvRestore::clear_coinbase_auth();
    let config = ContractWhaleRuntimeConfig::default();

    let resolution = config.threshold_profile_resolution();

    assert_eq!(
        resolution.profile,
        ContractWhaleThresholdProfile::BinanceBitfinex
    );
    assert_eq!(resolution.profile_name, "binance_bitfinex");
    assert_eq!(resolution.configured_keys(), vec!["binance", "bitfinex"]);
    assert_eq!(resolution.eligible_keys(), vec!["binance", "bitfinex"]);
    assert_eq!(resolution.active_keys(), vec!["binance", "bitfinex"]);
    assert_eq!(
        resolution.reason,
        "active_contract_sources=binance,bitfinex"
    );
}

#[test]
fn coinbase_perp_enabled_without_auth_does_not_change_profile() {
    let _guard = threshold_profile_test_guard();
    let _env = EnvRestore::clear_coinbase_auth();
    let mut config = ContractWhaleRuntimeConfig::default();
    config.exchanges.coinbase.perp.enabled = true;

    let resolution = config.threshold_profile_resolution();

    assert_eq!(
        resolution.profile,
        ContractWhaleThresholdProfile::BinanceBitfinex
    );
    assert_eq!(resolution.profile_name, "binance_bitfinex");
    assert_eq!(
        resolution.configured_keys(),
        vec!["binance", "bitfinex", "coinbase"]
    );
    assert_eq!(resolution.eligible_keys(), vec!["binance", "bitfinex"]);
    assert_eq!(resolution.active_keys(), vec!["binance", "bitfinex"]);
    assert_eq!(resolution.reason, "coinbase_perp_auth_missing");
}

#[test]
fn coinbase_perp_auth_ready_but_not_active_does_not_change_profile() {
    let _guard = threshold_profile_test_guard();
    let _env = EnvRestore::set_coinbase_auth();
    let mut config = ContractWhaleRuntimeConfig::default();
    config.exchanges.coinbase.perp.enabled = true;

    let resolution = config.threshold_profile_resolution();

    assert_eq!(
        resolution.profile,
        ContractWhaleThresholdProfile::BinanceBitfinex
    );
    assert_eq!(
        resolution.eligible_keys(),
        vec!["binance", "bitfinex", "coinbase"]
    );
    assert_eq!(resolution.active_keys(), vec!["binance", "bitfinex"]);
    assert_eq!(resolution.reason, "coinbase_perp_not_active");
}

#[test]
fn connected_coinbase_perp_switches_to_coinbase_profile() {
    let _guard = threshold_profile_test_guard();
    let _env = EnvRestore::set_coinbase_auth();
    let mut config = ContractWhaleRuntimeConfig::default();
    config.exchanges.coinbase.perp.enabled = true;
    let now = 1_700_000_000_000;
    let statuses = BTreeMap::from([
        (
            "coinbase".to_string(),
            exchange_status("connected", true, Some(now - 500)),
        ),
        (
            "binance".to_string(),
            exchange_status("connected", true, Some(now - 200)),
        ),
        (
            "bitfinex".to_string(),
            exchange_status("connected", true, Some(now - 250)),
        ),
    ]);

    let resolution = config.threshold_profile_resolution_with_statuses(&statuses, now);

    assert_eq!(
        resolution.profile,
        ContractWhaleThresholdProfile::BinanceBitfinexCoinbase
    );
    assert_eq!(resolution.profile_name, "binance_bitfinex_coinbase");
    assert_eq!(
        resolution.active_keys(),
        vec!["binance", "bitfinex", "coinbase"]
    );
    assert_eq!(
        resolution.reason,
        "active_contract_sources=binance,bitfinex,coinbase"
    );
}

#[test]
fn okx_enabled_switches_to_three_exchange_profile() {
    let _guard = threshold_profile_test_guard();
    let _env = EnvRestore::clear_coinbase_auth();
    let mut config = ContractWhaleRuntimeConfig::default();
    config.exchanges.okx.enabled = true;
    config.exchanges.okx.perp.enabled = true;

    let resolution = config.threshold_profile_resolution();

    assert_eq!(
        resolution.profile,
        ContractWhaleThresholdProfile::ThreeExchange
    );
    assert_eq!(resolution.profile_name, "three_exchange");
    assert_eq!(resolution.active_keys(), vec!["binance", "bitfinex", "okx"]);
    assert_eq!(
        resolution.reason,
        "active_contract_sources=binance,bitfinex,okx"
    );
}

#[test]
fn no_perp_contract_sources_resolves_to_no_contract_sources_and_detector_is_silent() {
    let _guard = threshold_profile_test_guard();
    let _env = EnvRestore::clear_coinbase_auth();
    let mut config = ContractWhaleRuntimeConfig::default();
    config.exchanges.binance.perp.enabled = false;
    config.exchanges.bitfinex.perp.enabled = false;
    config.exchanges.coinbase.perp.enabled = false;
    config.exchanges.okx.perp.enabled = false;

    let resolution = config.threshold_profile_resolution();
    assert_eq!(
        resolution.profile,
        ContractWhaleThresholdProfile::NoContractSources
    );
    assert_eq!(resolution.profile_name, "no_contract_sources");
    assert_eq!(resolution.active_keys(), Vec::<String>::new());

    let stats = whale_stats(vec![
        contribution("binance", 2_500.0, 100.0),
        contribution("bitfinex", 800.0, 50.0),
    ]);

    assert!(detect_contract_whale_signal_with_config(&stats, &config).is_none());
}

#[test]
fn coinbase_spot_only_observation_does_not_create_contract_signal() {
    let _guard = threshold_profile_test_guard();
    let _env = EnvRestore::clear_coinbase_auth();
    let config = ContractWhaleRuntimeConfig::default();
    let stats = whale_stats(vec![contribution("coinbase", 4_500.0, 100.0)]);

    let resolution =
        config.threshold_profile_resolution_for_observed_sources(vec!["coinbase".to_string()]);

    assert_eq!(
        resolution.profile,
        ContractWhaleThresholdProfile::NoContractSources
    );
    assert_eq!(resolution.active_keys(), Vec::<String>::new());
    assert!(detect_contract_whale_signal_with_config(&stats, &config).is_none());
}

#[test]
fn signal_snapshot_records_active_threshold_sources() {
    let _guard = threshold_profile_test_guard();
    let _env = EnvRestore::clear_coinbase_auth();
    let config = ContractWhaleRuntimeConfig::default();
    let stats = whale_stats(vec![
        contribution("binance", 2_800.0, 100.0),
        contribution("bitfinex", 1_000.0, 100.0),
    ]);

    let signal =
        detect_contract_whale_signal_with_config(&stats, &config).expect("contract signal");

    assert_eq!(signal.threshold_profile, "binance_bitfinex");
    assert_eq!(
        signal.active_sources.threshold_profile_reason,
        "active_contract_sources=binance,bitfinex"
    );
    assert_eq!(
        signal.active_sources.configured_contract_sources,
        vec!["binance", "bitfinex"]
    );
    assert_eq!(
        signal.active_sources.eligible_contract_sources,
        vec!["binance", "bitfinex"]
    );
    assert_eq!(
        signal.active_sources.active_contract_sources,
        vec!["binance", "bitfinex"]
    );
}

fn exchange_status(
    status: &str,
    connected: bool,
    last_trade_at: Option<i64>,
) -> ContractWhaleExchangeStatus {
    ContractWhaleExchangeStatus {
        connected,
        status: status.to_string(),
        last_trade_at,
        latency_ms: Some(50),
        reconnect_count: 0,
        platform_enabled: true,
        contract_enabled: true,
        enabled_markets: vec!["perp".to_string()],
        market_roles: BTreeMap::from([("perp".to_string(), "primary".to_string())]),
    }
}

fn whale_stats(exchanges: Vec<ExchangeFlowContribution>) -> ContractWhaleWindowStats {
    let buy_volume_btc = exchanges
        .iter()
        .map(|item| item.buy_volume_btc)
        .sum::<f64>();
    let sell_volume_btc = exchanges
        .iter()
        .map(|item| item.sell_volume_btc)
        .sum::<f64>();
    let buy_notional_usd = exchanges
        .iter()
        .map(|item| item.buy_notional_usd)
        .sum::<f64>();
    let sell_notional_usd = exchanges
        .iter()
        .map(|item| item.sell_notional_usd)
        .sum::<f64>();
    let total_volume_btc = buy_volume_btc + sell_volume_btc;
    let net_volume_btc = buy_volume_btc - sell_volume_btc;
    let total_notional_usd = buy_notional_usd + sell_notional_usd;
    let exchange_count = exchanges.len();

    ContractWhaleWindowStats {
        symbol: "BTC".to_string(),
        window_sec: 15,
        ts: 1_700_000_000_000,
        buy_volume_btc,
        sell_volume_btc,
        total_volume_btc,
        net_volume_btc,
        dominance: net_volume_btc.abs() / total_volume_btc,
        buy_notional_usd,
        sell_notional_usd,
        total_notional_usd,
        price_move_pct: Some(0.30),
        exchange_count,
        main_exchange: Some("binance".to_string()),
        exchanges,
        dominant_venue_net_contribution_share: Some(0.80),
        dynamic_multiple: Some(12.0),
        percentile_level: Some(99.95),
        multi_exchange_confirmed: true,
        liquidation_context: ContractWhaleLiquidationContext::default(),
        market_context: ContractWhaleMarketContext::default(),
        price_reversal_ratio: Some(0.0),
        data_quality: 95,
        ws_latency_ms: Some(30),
        startup_age_ms: Some(120_000),
        liquidation_driven: false,
        price_jump_anomaly: false,
    }
}

fn contribution(
    exchange: &str,
    buy_volume_btc: f64,
    sell_volume_btc: f64,
) -> ExchangeFlowContribution {
    let price = 70_000.0;
    let total_volume_btc = buy_volume_btc + sell_volume_btc;
    let net_volume_btc = buy_volume_btc - sell_volume_btc;
    ExchangeFlowContribution {
        exchange: exchange.to_string(),
        buy_volume_btc,
        sell_volume_btc,
        total_volume_btc,
        buy_share: if total_volume_btc > 0.0 {
            buy_volume_btc / total_volume_btc
        } else {
            0.0
        },
        sell_share: if total_volume_btc > 0.0 {
            sell_volume_btc / total_volume_btc
        } else {
            0.0
        },
        buy_notional_usd: buy_volume_btc * price,
        sell_notional_usd: sell_volume_btc * price,
        total_notional_usd: total_volume_btc * price,
        net_volume_btc,
        dominance: if total_volume_btc > 0.0 {
            net_volume_btc.abs() / total_volume_btc
        } else {
            0.0
        },
        net_contribution_share: 0.10,
        trade_count: total_volume_btc.round() as u64,
    }
}
