use std::fs;

#[test]
fn liquidation_cascade_routes_are_registered() {
    let server = fs::read_to_string("src/api/server.rs").expect("server source");
    let api_mod = fs::read_to_string("src/api/mod.rs").expect("api mod source");
    let lib = fs::read_to_string("src/lib.rs").expect("lib source");

    assert!(lib.contains("pub mod liquidation_cascade_predictor;"));
    assert!(api_mod.contains("pub mod liquidation_cascade_routes;"));
    assert!(server.contains("\"/api/liquidation/cascade\""));
    assert!(server.contains("liquidation_cascade_routes::liquidation_cascade_route"));
    assert!(server.contains("\"/api/liquidation/leverage-map\""));
    assert!(server.contains("liquidation_cascade_routes::liquidation_leverage_map_route"));
    assert!(server.contains("\"/api/liquidation/liquidity-gap\""));
    assert!(server.contains("liquidation_cascade_routes::liquidation_liquidity_gap_route"));
}
