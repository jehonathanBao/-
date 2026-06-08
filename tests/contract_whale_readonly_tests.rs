use btc_toxic_flow_monitor_rs::contract_whale_monitor::{log_events, LOG_PREFIX, LOG_TARGET};

#[test]
fn contract_whale_monitor_logging_namespace_is_stable() {
    assert_eq!(LOG_TARGET, "contract_whale_monitor");
    assert_eq!(LOG_PREFIX, "[cwm]");
    assert_eq!(log_events::CONFIG_LOADED, "cwm.config.loaded");
    assert_eq!(log_events::RUNTIME_DISABLED, "cwm.runtime.disabled");
    assert_eq!(log_events::SIGNAL_GENERATED, "cwm.signal.generated");
    assert_eq!(log_events::DISCORD_SKIPPED, "cwm.discord.skipped");
}

#[test]
fn contract_whale_monitor_contains_no_private_execution_capability_terms() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/contract_whale_monitor");
    let forbidden = [
        "place_order",
        "cancel_order",
        "transfer_funds",
        "withdraw_funds",
        "api_secret",
        "private_key",
        "signer",
    ];

    for path in rust_files(&root) {
        let body = std::fs::read_to_string(&path).expect("read cwm source file");
        for needle in forbidden {
            assert!(
                !body.contains(needle),
                "forbidden private execution term `{needle}` found in {}",
                path.display()
            );
        }
    }
}

fn rust_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(root).expect("read contract whale monitor dir");
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files
}
