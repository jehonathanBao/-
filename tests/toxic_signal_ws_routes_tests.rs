use std::{fs, path::Path};

#[test]
fn ws_signals_route_is_read_only_static_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source =
        fs::read_to_string(root.join("src/api/toxic_signal_ws_routes.rs")).expect("ws source");

    assert!(source.contains("read_only: true"));
    assert!(source.contains("runtime_modified: false"));
    assert!(source.contains("execution_enabled: false"));
    assert!(!source.contains("discord_notification_proxy"));
    assert!(!source.contains("telegram"));
    assert!(!source.contains("clearSignalInbox"));
    assert!(!source.contains("run_production_replay"));
}

#[test]
fn ws_logs_do_not_include_operator_token_static_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source =
        fs::read_to_string(root.join("src/api/toxic_signal_ws_routes.rs")).expect("ws source");

    assert!(!source.contains("OPERATOR_TOKEN"));
    assert!(!source.contains("authorization"));
    assert!(!source.contains("webhook"));
    assert!(!source.contains("raw evidence"));
}
