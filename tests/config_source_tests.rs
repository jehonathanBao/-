use std::{fs, sync::Mutex};

use btc_toxic_flow_monitor_rs::config::AppConfig;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn toml_enable_binance_true_is_authoritative_without_env_override() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    let config_path = write_config("toml-binance-true", "enable_binance = true\n");

    let config = AppConfig::from_env_with_config_file(&config_path).expect("config");

    assert!(config.venues.binance.enabled);
    assert_eq!(
        config.config_source_label(),
        "env_overrides_toml_overrides_defaults"
    );
    clear_config_env();
}

#[test]
fn env_false_overrides_toml_true() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    std::env::set_var("ENABLE_BINANCE", "false");
    let config_path = write_config("env-false-overrides", "enable_binance = true\n");

    let config = AppConfig::from_env_with_config_file(&config_path).expect("config");

    assert!(!config.venues.binance.enabled);
    clear_config_env();
}

#[test]
fn env_true_overrides_toml_false() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    std::env::set_var("ENABLE_BINANCE", "true");
    let config_path = write_config("env-true-overrides", "enable_binance = false\n");

    let config = AppConfig::from_env_with_config_file(&config_path).expect("config");

    assert!(config.venues.binance.enabled);
    clear_config_env();
}

#[test]
fn toml_covers_core_startup_fields() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    let config_path = write_config(
        "core-fields",
        r#"
api_host = "127.0.0.1"
api_port = 3011
symbol = "ETH-PERP"
toxic_volume_alert_btc = 42.5
enable_binance = true
enable_bybit = true
enable_okx = false
"#,
    );

    let config = AppConfig::from_env_with_config_file(&config_path).expect("config");

    assert_eq!(config.api_host.to_string(), "127.0.0.1");
    assert_eq!(config.api_port, 3011);
    assert_eq!(config.symbol, "ETH-PERP");
    assert_eq!(config.toxic_volume_alert_btc, 42.5);
    assert!(config.venues.binance.enabled);
    assert!(config.venues.bybit.enabled);
    assert!(!config.venues.okx.enabled);
    clear_config_env();
}

fn write_config(name: &str, content: &str) -> String {
    let base = std::env::temp_dir().join(format!(
        "btc-toxic-flow-monitor-rs-{name}-{}",
        std::process::id()
    ));
    let path = base.with_extension("toml");
    fs::write(&path, content).expect("write config");
    base.display().to_string()
}

fn clear_config_env() {
    for key in [
        "APP_ENV",
        "READ_ONLY",
        "API_HOST",
        "API_PORT",
        "SYMBOL",
        "TOXIC_VOLUME_ALERT_BTC",
        "ENABLE_BINANCE",
        "ENABLE_BYBIT",
        "ENABLE_OKX",
    ] {
        std::env::remove_var(key);
    }
}
