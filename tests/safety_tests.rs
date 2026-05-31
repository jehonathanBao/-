use std::sync::Mutex;

use btc_toxic_flow_monitor_rs::safety::{
    read_only_guard::assert_read_only_runtime, secret_scanner::scan_forbidden_secrets,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn allows_read_only_runtime_without_secrets() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_forbidden();

    assert!(assert_read_only_runtime().is_ok());
}

#[test]
fn detects_private_key_and_exchange_secret() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_forbidden();
    std::env::set_var("PRIVATE_KEY", "0xabc");
    std::env::set_var("BINANCE_SECRET", "secret");

    let findings = scan_forbidden_secrets();
    let keys = findings
        .into_iter()
        .map(|finding| finding.key)
        .collect::<Vec<_>>();

    assert_eq!(keys, vec!["BINANCE_SECRET", "PRIVATE_KEY"]);
    clear_forbidden();
}

#[test]
fn rejects_common_exchange_api_keys_without_leaking_values() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_forbidden();
    std::env::set_var("BINANCE_API_KEY", "binance-secret-value");
    std::env::set_var("BYBIT_API_KEY", "bybit-secret-value");
    std::env::set_var("OKX_API_KEY", "okx-secret-value");

    let err = assert_read_only_runtime().expect_err("api keys must fail");
    let message = err.to_string();

    assert!(message.contains("BINANCE_API_KEY"));
    assert!(message.contains("BYBIT_API_KEY"));
    assert!(message.contains("OKX_API_KEY"));
    assert!(!message.contains("binance-secret-value"));
    assert!(!message.contains("bybit-secret-value"));
    assert!(!message.contains("okx-secret-value"));
    clear_forbidden();
}

#[test]
fn runtime_flags_are_not_misclassified_as_api_keys() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_forbidden();
    std::env::set_var("ENABLE_BINANCE", "true");
    std::env::set_var("API_HOST", "127.0.0.1");
    std::env::set_var("API_PORT", "3000");

    assert!(scan_forbidden_secrets().is_empty());
    clear_forbidden();
    std::env::remove_var("ENABLE_BINANCE");
    std::env::remove_var("API_HOST");
    std::env::remove_var("API_PORT");
}

#[test]
fn rejects_live_trading_flags() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_forbidden();
    std::env::set_var("LIVE_TRADING_ENABLED", "true");

    let err = assert_read_only_runtime().expect_err("live trading flag must fail");

    assert!(err.to_string().contains("LIVE_TRADING_ENABLED"));
    clear_forbidden();
}

fn clear_forbidden() {
    for key in [
        "PRIVATE_KEY",
        "WALLET_KEY",
        "MNEMONIC",
        "EXCHANGE_SECRET",
        "EXCHANGE_API_KEY",
        "EXCHANGE_API_SECRET",
        "CCXT_API_KEY",
        "CCXT_SECRET",
        "BINANCE_API_KEY",
        "BINANCE_API_SECRET",
        "BINANCE_SECRET",
        "BYBIT_API_KEY",
        "BYBIT_API_SECRET",
        "BYBIT_SECRET",
        "OKX_API_KEY",
        "OKX_API_SECRET",
        "OKX_PASSPHRASE",
        "OKX_SECRET",
        "ORDER_EXECUTION_ENABLED",
        "LIVE_TRADING_ENABLED",
    ] {
        std::env::remove_var(key);
    }
}
