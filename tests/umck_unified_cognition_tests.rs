use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::umck::{
    cognize_market_state, AssetProjection, MarketStateVector,
};

#[test]
fn btc_driven_market_is_classified_as_btc_led_regime() {
    let cognition = cognize_market_state(&[
        projection("BTC", 1.25, 74.0, 72.0, 68.0, 60.0, 18.0),
        projection("ETH", 1.05, 34.0, 31.0, 24.0, 24.0, 8.0),
        projection("WIF", 0.70, 52.0, 48.0, 42.0, 35.0, 12.0),
    ]);

    assert_eq!(cognition.unified_signal.global_regime, "BtcLedRegime");
    assert_eq!(cognition.dominant_asset, "BTC");
    assert_eq!(cognition.liquidity_direction, "bullish");
    assert!(cognition.cross_asset_field.btc_influence >= 70.0);
    assert!(cognition.read_only);
    assert!(!cognition.direct_discord_gate);
}

#[test]
fn isolated_alt_spike_is_local_liquidity_event_not_global_regime() {
    let cognition = cognize_market_state(&[
        projection("BTC", 1.25, 10.0, 6.0, 8.0, 15.0, 4.0),
        projection("ETH", 1.05, 12.0, 5.0, 9.0, 14.0, 4.0),
        projection("PEPE", 0.70, 88.0, 82.0, 75.0, 72.0, 18.0),
    ]);

    assert_eq!(
        cognition.unified_signal.global_regime,
        "LocalLiquidityEvent"
    );
    assert_eq!(cognition.dominant_asset, "PEPE");
    assert!(cognition.unified_signal.cross_asset_alignment < 45.0);
    assert!(cognition.cross_asset_field.alt_amplification >= 80.0);
}

#[test]
fn cross_asset_reversal_spikes_raise_manipulation_field() {
    let cognition = cognize_market_state(&[
        projection("BTC", 1.25, 76.0, 70.0, 58.0, 92.0, 84.0),
        projection("ETH", 1.05, 72.0, -68.0, 62.0, 90.0, 88.0),
        projection("DOGE", 0.70, 80.0, 74.0, 71.0, 94.0, 90.0),
    ]);

    assert_eq!(
        cognition.unified_signal.global_regime,
        "CrossAssetManipulationRisk"
    );
    assert_eq!(cognition.risk_regime, "manipulation_watch");
    assert!(cognition.manipulation_field >= 65.0);
    assert!(cognition.unified_signal.confidence < 80.0);
}

#[test]
fn synchronized_major_assets_form_global_accumulation() {
    let cognition = cognize_market_state(&[
        projection("BTC", 1.25, 58.0, 54.0, 45.0, 36.0, 10.0),
        projection("ETH", 1.05, 55.0, 50.0, 42.0, 34.0, 8.0),
        projection("SOL", 1.00, 52.0, 47.0, 39.0, 32.0, 8.0),
        projection("XRP", 0.85, 48.0, 45.0, 35.0, 30.0, 6.0),
    ]);

    assert_eq!(cognition.unified_signal.global_regime, "GlobalAccumulation");
    assert!(cognition.unified_signal.cross_asset_alignment >= 70.0);
    assert!(cognition.market_state.flow_intensity >= 45.0);
    assert_eq!(cognition.dominant_force, "buy_side_liquidity");
}

fn projection(
    asset: &str,
    weight: f64,
    flow_intensity: f64,
    directional_bias: f64,
    risk_pressure: f64,
    volatility: f64,
    manipulation: f64,
) -> AssetProjection {
    AssetProjection {
        asset: asset.to_string(),
        contribution_vector: MarketStateVector {
            global_liquidity: flow_intensity * 0.85,
            risk_pressure,
            directional_bias,
            volatility_regime: volatility,
            cross_asset_correlation: 0.0,
            manipulation_index: manipulation,
            flow_intensity,
        },
        weight,
    }
}
