use btc_toxic_flow_monitor_rs::{
    binance_alt_contract_monitor::config::load_binance_alt_contract_runtime_config_from_settings,
    config::system_mode::{
        load_system_mode_config_from_settings, MarketSystemMode, SystemModeConfig,
    },
};

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
