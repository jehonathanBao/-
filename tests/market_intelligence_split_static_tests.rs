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
    let altcoin_engine = project_file("src/altcoin_manipulation_engine.rs");

    assert!(
        !btc_engine.contains("manipulation_score"),
        "BTC structure engine must not expose or calculate manipulation_score"
    );
    assert!(
        altcoin_engine.contains("manipulation_score"),
        "Altcoin engine should retain manipulation scoring"
    );
}

#[test]
fn market_intelligence_routes_are_domain_split() {
    let api_mod = project_file("src/api/mod.rs");
    let server = project_file("src/api/server.rs");
    let lib = project_file("src/lib.rs");

    assert!(lib.contains("pub mod btc_structure_engine;"));
    assert!(lib.contains("pub mod altcoin_manipulation_engine;"));
    assert!(lib.contains("pub mod market_domain;"));

    assert!(api_mod.contains("pub mod btc_structure_routes;"));
    assert!(api_mod.contains("pub mod altcoin_routes;"));

    assert!(server.contains("\"/api/btc/structure\""));
    assert!(server.contains("\"/api/btc/regime\""));
    assert!(server.contains("\"/api/btc/liquidation\""));
    assert!(server.contains("\"/api/altcoin/manipulation\""));
    assert!(server.contains("\"/api/altcoin/regime\""));
    assert!(server.contains("\"/api/altcoin/fusion\""));
    assert!(server.contains("\"/api/altcoin/signals\""));
}

#[test]
fn fusion_core_is_domain_aware() {
    let fusion_core = project_file("src/multi_timeframe_orderflow_fusion.rs");

    assert!(fusion_core.contains("market_domain: MarketDomain"));
    assert!(fusion_core.contains("MarketDomain::BtcStructure"));
    assert!(fusion_core.contains("MarketDomain::AltcoinManipulation"));
    assert!(fusion_core.contains("OI_STRUCTURE_SHIFT"));
    assert!(fusion_core.contains("OI_DIVERGENCE"));
}
