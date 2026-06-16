use btc_toxic_flow_monitor_rs::{
    binance_alt_contract_monitor::config::load_binance_alt_contract_runtime_config_from_settings,
    config::system_mode::{
        load_system_mode_config_from_settings, MarketSystemMode, SystemModeConfig,
    },
};
use std::sync::{Mutex, OnceLock};

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn settings_from_toml(input: &str) -> config::Config {
    config::Config::builder()
        .add_source(config::File::from_str(input, config::FileFormat::Toml))
        .build()
        .expect("test config should parse")
}

#[test]
fn bear_market_mode_disables_altcoin_monitoring_even_if_bacm_is_enabled() {
    let settings = settings_from_toml(
        r#"
        [system]
        mode = "bear_market"

        [features]
        altcoin_monitoring = true
        memecoin_tracking = true
        low_cap_scanner = true

        [binance_alt_contract_monitor]
        enabled = true
        dry_run = false

        [binance_alt_contract_monitor.exchanges.binance]
        enabled = true

        [binance_alt_contract_monitor.oi_scheduler]
        enabled = true

        [binance_alt_contract_monitor.discord]
        enabled = true
        dry_run = false
        "#,
    );

    let system_mode = load_system_mode_config_from_settings(&settings);
    assert_eq!(system_mode.mode, MarketSystemMode::BearMarket);
    assert_eq!(
        system_mode.altcoin_disabled_reason(),
        Some("bear_market_mode")
    );
    assert!(!system_mode.altcoin_monitoring_enabled());

    let mut bacm = load_binance_alt_contract_runtime_config_from_settings(&settings);
    assert!(
        bacm.enabled,
        "raw BACM config should be enabled before mode gate"
    );
    bacm.apply_system_mode(system_mode);

    assert!(!bacm.enabled);
    assert!(bacm.dry_run);
    assert!(!bacm.exchange.binance_enabled);
    assert!(!bacm.oi_scheduler.enabled);
    assert!(!bacm.discord.enabled);
    assert!(bacm.discord.dry_run);
}

#[test]
fn normal_mode_can_restore_altcoin_monitoring_when_feature_is_enabled() {
    let settings = settings_from_toml(
        r#"
        [system]
        mode = "normal"

        [features]
        altcoin_monitoring = true

        [binance_alt_contract_monitor]
        enabled = true
        dry_run = true

        [binance_alt_contract_monitor.exchanges.binance]
        enabled = true
        "#,
    );

    let system_mode = load_system_mode_config_from_settings(&settings);
    assert_eq!(system_mode.mode, MarketSystemMode::Normal);
    assert!(system_mode.altcoin_monitoring_enabled());

    let mut bacm = load_binance_alt_contract_runtime_config_from_settings(&settings);
    bacm.apply_system_mode(system_mode);

    assert!(bacm.enabled);
    assert!(bacm.exchange.binance_enabled);
}

#[test]
fn normal_mode_still_requires_explicit_altcoin_feature() {
    let system_mode = SystemModeConfig {
        mode: MarketSystemMode::Normal,
        ..SystemModeConfig::default()
    };

    assert_eq!(
        system_mode.altcoin_disabled_reason(),
        Some("altcoin_monitoring_feature_disabled")
    );
    assert!(!system_mode.altcoin_monitoring_enabled());
}

#[test]
fn bacm_storage_retention_defaults_to_seven_days_and_allows_env_override() {
    let _guard = env_guard();
    for key in [
        "BINANCE_ALT_CONTRACT_HOT_1S_RETENTION_HOURS",
        "BINANCE_ALT_CONTRACT_FLOW_1M_RETENTION_DAYS",
        "BINANCE_ALT_CONTRACT_SIGNALS_RETENTION_DAYS",
        "BINANCE_ALT_CONTRACT_CLEANUP_INTERVAL_SEC",
    ] {
        std::env::remove_var(key);
    }

    let settings = settings_from_toml(
        r#"
        [binance_alt_contract_monitor.storage]
        hot_1s_retention_hours = 24
        flow_1m_retention_days = 7
        signals_retention_days = 7
        cleanup_interval_sec = 3600
        "#,
    );
    let config = load_binance_alt_contract_runtime_config_from_settings(&settings);
    assert_eq!(config.storage.hot_1s_retention_hours, 24);
    assert_eq!(config.storage.flow_1m_retention_days, 7);
    assert_eq!(config.storage.signals_retention_days, 7);
    assert_eq!(config.storage.cleanup_interval_sec, 3600);

    std::env::set_var("BINANCE_ALT_CONTRACT_HOT_1S_RETENTION_HOURS", "12");
    std::env::set_var("BINANCE_ALT_CONTRACT_FLOW_1M_RETENTION_DAYS", "3");
    std::env::set_var("BINANCE_ALT_CONTRACT_SIGNALS_RETENTION_DAYS", "7");
    std::env::set_var("BINANCE_ALT_CONTRACT_CLEANUP_INTERVAL_SEC", "900");
    let overridden = load_binance_alt_contract_runtime_config_from_settings(&settings);
    assert_eq!(overridden.storage.hot_1s_retention_hours, 12);
    assert_eq!(overridden.storage.flow_1m_retention_days, 3);
    assert_eq!(overridden.storage.signals_retention_days, 7);
    assert_eq!(overridden.storage.cleanup_interval_sec, 900);

    for key in [
        "BINANCE_ALT_CONTRACT_HOT_1S_RETENTION_HOURS",
        "BINANCE_ALT_CONTRACT_FLOW_1M_RETENTION_DAYS",
        "BINANCE_ALT_CONTRACT_SIGNALS_RETENTION_DAYS",
        "BINANCE_ALT_CONTRACT_CLEANUP_INTERVAL_SEC",
    ] {
        std::env::remove_var(key);
    }
}
