use std::fs;

#[test]
fn market_regime_engine_routes_are_registered() {
    let server = fs::read_to_string("src/api/server.rs").expect("server source");
    let api_mod = fs::read_to_string("src/api/mod.rs").expect("api mod source");

    assert!(api_mod.contains("pub mod market_regime_routes;"));
    assert!(server.contains("\"/api/regime/latest\""));
    assert!(server.contains("market_regime_routes::market_regime_latest_route"));
    assert!(server.contains("\"/api/manipulation/latest\""));
    assert!(server.contains("market_regime_routes::manipulation_latest_route"));
    assert!(server.contains("\"/api/signal/latest\""));
    assert!(server.contains("market_regime_routes::market_signal_latest_route"));
}
