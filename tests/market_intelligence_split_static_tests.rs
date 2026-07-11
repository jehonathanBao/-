use std::{fs, path::PathBuf};

fn project_file(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(path)).unwrap_or_else(|err| {
        panic!("failed to read {path}: {err}");
    })
}

#[test]
fn btc_engine_does_not_expose_altcoin_manipulation_score() {
    let btc_engine = project_file("src/btc_structure_engine.rs");

    assert!(
        !btc_engine.contains("manipulation_score"),
        "BTC structure engine must not expose or calculate manipulation_score"
    );
}

#[test]
fn altcoin_manipulation_engine_and_routes_are_removed() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(!root.join("src/altcoin_manipulation_engine.rs").exists());
    assert!(!root.join("src/api/altcoin_routes.rs").exists());

    let api_mod = project_file("src/api/mod.rs");
    let server = project_file("src/api/server.rs");
    let lib = project_file("src/lib.rs");

    assert!(!lib.contains("pub mod altcoin_manipulation_engine;"));
    assert!(!api_mod.contains("pub mod altcoin_routes;"));
    assert!(!server.contains("/api/altcoin/"));
    assert!(server.contains("/api/binance-alt-contract/summary"));
    assert!(server.contains("/api/new-token-watch/list"));
}

#[test]
fn btc_structure_and_remaining_altcoin_monitors_stay_registered() {
    let api_mod = project_file("src/api/mod.rs");
    let server = project_file("src/api/server.rs");
    let lib = project_file("src/lib.rs");

    assert!(lib.contains("pub mod btc_structure_engine;"));
    assert!(lib.contains("pub mod binance_alt_contract_monitor;"));
    assert!(api_mod.contains("pub mod btc_structure_routes;"));
    assert!(api_mod.contains("pub mod binance_alt_contract_routes;"));
    assert!(api_mod.contains("pub mod new_token_watch_routes;"));
    assert!(server.contains("/api/btc/structure"));
    assert!(server.contains("/api/btc/regime"));
    assert!(server.contains("/api/btc/liquidation"));
    assert!(server.contains("/api/binance-alt-contract/summary"));
    assert!(server.contains("/api/new-token-watch/list"));
}

#[test]
fn fusion_core_has_no_dedicated_altcoin_manipulation_domain() {
    let fusion_core = project_file("src/multi_timeframe_orderflow_fusion.rs");

    assert!(fusion_core.contains("market_domain: MarketDomain"));
    assert!(fusion_core.contains("MarketDomain::BtcStructure"));
    assert!(!fusion_core.contains("MarketDomain::AltcoinManipulation"));
    assert!(!fusion_core.contains("FAKE_BREAKOUT_OR_MANIPULATION"));
    assert!(!fusion_core.contains("manipulation_score"));
    assert!(fusion_core.contains("OI_STRUCTURE_SHIFT"));
}
