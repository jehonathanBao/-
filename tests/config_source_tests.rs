use std::{fs, sync::Mutex};

use btc_toxic_flow_monitor_rs::{
    config::AppConfig,
    contract_whale_monitor::config::{
        contract_whale_runtime_config, reset_contract_whale_runtime_config,
    },
    runtime::score_config::{reset_score_runtime_config, score_runtime_config},
};

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

#[test]
fn toml_contract_whale_monitor_flags_default_to_disabled_dry_run() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    let config_path = write_config("cwm-defaults", "");

    let config = AppConfig::from_env_with_config_file(&config_path).expect("config");

    assert!(!config.contract_whale_monitor.enabled);
    assert!(config.contract_whale_monitor.dry_run);
    clear_config_env();
}

#[test]
fn env_example_documents_contract_whale_enabled_by_default() {
    let contents =
        fs::read_to_string(".env.example").expect("read .env.example for contract whale default");

    assert!(
        contents
            .lines()
            .any(|line| line.trim() == "CONTRACT_WHALE_ENABLED=true"),
        ".env.example should document enabled contract whale monitoring by default"
    );
}

#[test]
fn env_contract_whale_monitor_flags_override_toml() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    std::env::set_var("CONTRACT_WHALE_ENABLED", "false");
    std::env::set_var("CONTRACT_WHALE_DRY_RUN", "true");
    let config_path = write_config(
        "cwm-env-overrides",
        r#"
[contract_whale_monitor]
enabled = true
dry_run = false
"#,
    );

    let config = AppConfig::from_env_with_config_file(&config_path).expect("config");

    assert!(!config.contract_whale_monitor.enabled);
    assert!(config.contract_whale_monitor.dry_run);
    clear_config_env();
}

#[test]
fn env_contract_whale_monitor_true_overrides_disabled_toml() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    std::env::set_var("CONTRACT_WHALE_ENABLED", "true");
    let config_path = write_config(
        "cwm-env-true",
        r#"
[contract_whale_monitor]
enabled = false
dry_run = false
"#,
    );

    let config = AppConfig::from_env_with_config_file(&config_path).expect("config");

    assert!(config.contract_whale_monitor.enabled);
    assert!(!config.contract_whale_monitor.dry_run);
    clear_config_env();
}

#[test]
fn app_config_file_env_loads_overlay_for_personal_cwm_runtime() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    let config_path = write_config(
        "app-config-file-overlay",
        r#"
[contract_whale_monitor]
enabled = true
dry_run = true

[spot_whale_monitor]
enabled = true
dry_run = true
"#,
    );
    std::env::set_var("APP_CONFIG_FILE", config_path);

    let config = AppConfig::from_env().expect("config");

    assert!(config.contract_whale_monitor.enabled);
    assert!(config.contract_whale_monitor.dry_run);
    assert!(config.spot_whale_monitor.enabled);
    assert!(config.spot_whale_monitor.dry_run);
    clear_config_env();
}

#[test]
fn personal_cwm_example_enables_dry_run_binance_bitfinex_with_coinbase_spot_only() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    reset_contract_whale_runtime_config();

    let config = AppConfig::from_env_with_config_file("config/cwm.personal.example")
        .expect("personal CWM config");
    let cwm = contract_whale_runtime_config();
    let resolution = cwm.threshold_profile_resolution();

    assert!(config.contract_whale_monitor.enabled);
    assert!(config.contract_whale_monitor.dry_run);
    assert!(config.spot_whale_monitor.enabled);
    assert!(config.spot_whale_monitor.dry_run);
    assert_eq!(resolution.profile_name, "binance_bitfinex");
    assert_eq!(resolution.active_keys(), vec!["binance", "bitfinex"]);
    assert!(cwm.exchanges.coinbase.spot.enabled);
    assert!(!cwm.exchanges.coinbase.perp.enabled);
    assert!(!cwm.exchanges.okx.perp.enabled);

    reset_contract_whale_runtime_config();
    clear_config_env();
}

#[test]
fn toml_contract_whale_scoring_and_symbol_thresholds_are_loaded() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    reset_contract_whale_runtime_config();
    let config_path = write_config(
        "cwm-scoring-thresholds",
        r#"
[contract_whale_monitor.scoring]
volume_strength_weight = 40
dynamic_multiple_weight = 10

[contract_whale_monitor.scoring.penalties]
warmup_period = 12

[contract_whale_monitor.data_quality]
min_dynamic_samples = 7

[contract_whale_monitor.retention]
flow_1s_days = 10
signals_days = 180

[contract_whale_monitor.symbols.BTC.thresholds_btc.high]
15 = 2222

[contract_whale_monitor.symbols.ETH]
enabled = false
"#,
    );

    let _config = AppConfig::from_env_with_config_file(&config_path).expect("config");
    let cwm = contract_whale_runtime_config();

    assert_eq!(cwm.scoring.volume_strength_weight, 40.0);
    assert_eq!(cwm.scoring.dynamic_multiple_weight, 10.0);
    assert_eq!(cwm.scoring.penalties.warmup_period, 12.0);
    assert_eq!(cwm.data_quality.min_dynamic_samples, 7);
    assert_eq!(cwm.retention.flow_1s_days, 10);
    assert_eq!(cwm.retention.signals_days, 180);
    assert_eq!(cwm.thresholds_for_symbol_window("BTC", 15).high_btc, 2222.0);
    assert!(cwm.symbol_enabled("BTC"));
    assert!(!cwm.symbol_enabled("ETH"));
    assert!(!cwm.symbol_enabled("SOL"));
    assert_ne!(
        cwm.thresholds_for_symbol_window("SOL", 15).high_btc,
        cwm.thresholds_for_symbol_window("BTC", 15).high_btc
    );
    reset_contract_whale_runtime_config();
    clear_config_env();
}

#[test]
fn toml_score_runtime_config_is_loaded_for_toxic_and_market_structure() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    reset_score_runtime_config();
    let config_path = write_config(
        "score-runtime-config",
        r#"
[scoring.toxic_short]
half_life_sec = 55
max_ttl_sec = 420
windows_sec = [2, 10, 30]

[scoring.toxic_short.weights]
aggressive_sweep = 0.33

[scoring.toxic_short.discord]
min_score = 88
cooldown_sec = 75

[scoring.market_structure]
windows_min = [10, 30, 120]
event_end_hold_minutes = 21

[scoring.market_structure.contract_weights]
oi_impulse = 0.27

[scoring.market_structure.confirmation]
min_confirm_conditions = 4

[scoring.market_structure.discord]
min_main_force_score = 83
cooldown_sec = 1500
"#,
    );

    let _config = AppConfig::from_env_with_config_file(&config_path).expect("config");
    let score_config = score_runtime_config();

    assert_eq!(score_config.toxic_short.half_life_sec, 55);
    assert_eq!(score_config.toxic_short.max_ttl_sec, 420);
    assert_eq!(score_config.toxic_short.windows_sec, vec![2, 10, 30]);
    assert_eq!(score_config.toxic_short.weights.aggressive_sweep, 0.33);
    assert_eq!(score_config.toxic_short.discord.min_score, 88);
    assert_eq!(score_config.toxic_short.discord.cooldown_sec, 75);
    assert_eq!(score_config.market_structure.windows_min, vec![10, 30, 120]);
    assert_eq!(score_config.market_structure.event_end_hold_minutes, 21);
    assert_eq!(
        score_config.market_structure.contract_weights.oi_impulse,
        0.27
    );
    assert_eq!(
        score_config
            .market_structure
            .confirmation
            .min_confirm_conditions,
        4
    );
    assert_eq!(
        score_config.market_structure.discord.min_main_force_score,
        83
    );
    assert_eq!(score_config.market_structure.discord.cooldown_sec, 1500);
    reset_score_runtime_config();
    clear_config_env();
}

#[test]
fn invalid_contract_whale_config_values_fall_back_to_defaults() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_config_env();
    reset_contract_whale_runtime_config();
    let config_path = write_config(
        "cwm-invalid-fallbacks",
        r#"
[contract_whale_monitor.scoring]
volume_strength_weight = -1

[contract_whale_monitor.data_quality]
min_discord_quality = 777
min_dynamic_samples = 0

[contract_whale_monitor.symbols.BTC.thresholds_btc.high]
15 = -3
"#,
    );

    let _config = AppConfig::from_env_with_config_file(&config_path).expect("config");
    let cwm = contract_whale_runtime_config();

    assert_eq!(cwm.scoring.volume_strength_weight, 35.0);
    assert_eq!(cwm.data_quality.min_discord_quality, 70);
    assert_eq!(cwm.data_quality.min_dynamic_samples, 20);
    assert_eq!(cwm.thresholds_for_symbol_window("BTC", 15).high_btc, 1200.0);
    reset_contract_whale_runtime_config();
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
        "CONTRACT_WHALE_ENABLED",
        "CONTRACT_WHALE_DRY_RUN",
        "SPOT_WHALE_ENABLED",
        "SPOT_WHALE_DRY_RUN",
        "APP_CONFIG_FILE",
    ] {
        std::env::remove_var(key);
    }
}
