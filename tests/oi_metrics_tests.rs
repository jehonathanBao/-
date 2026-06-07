use std::sync::Mutex;

use btc_toxic_flow_monitor_rs::runtime::{
    perp_tof_metrics::{
        build_perp_tof_metrics, classify_open_interest, merge_spot_perp_candidate, PerpTofInput,
        PerpTofMetrics,
    },
    tof_metrics::TofDirection,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn open_interest_direction_follows_price_and_oi_change() {
    assert_eq!(
        classify_open_interest(10.0, 120_000.0),
        ("long_increase".to_string(), TofDirection::Bullish)
    );
    assert_eq!(
        classify_open_interest(10.0, -120_000.0),
        ("short_decrease".to_string(), TofDirection::Bullish)
    );
    assert_eq!(
        classify_open_interest(-10.0, 120_000.0),
        ("short_increase".to_string(), TofDirection::Bearish)
    );
    assert_eq!(
        classify_open_interest(-10.0, -120_000.0),
        ("long_decrease".to_string(), TofDirection::Bearish)
    );
}

#[test]
fn spot_and_perp_merge_boosts_aligned_high_risk_candidate() {
    let perp = perp_metrics("OpenInterestCandidate", TofDirection::Bullish, 86);
    let merged = merge_spot_perp_candidate(
        "SpoofingCandidate",
        TofDirection::Bullish,
        84,
        &["high_vpin_proxy".to_string()],
        &perp,
    );

    assert!(merged.risk_score >= 86);
    assert_eq!(merged.metrics_direction, TofDirection::Bullish);
    assert!(merged.final_candidate_type.contains("Bullish Candidate"));
    assert!(merged
        .explain_tags
        .contains(&"SpoofingCandidate".to_string()));
    assert!(merged
        .explain_tags
        .contains(&"OI long increase".to_string()));
}

#[test]
fn perp_tof_disabled_returns_neutral_safe_summary() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var("PERP_TOF_ENABLED", "false");

    let metrics = build_perp_tof_metrics(&PerpTofInput {
        symbol: "BTC-PERP",
        spot_candidate_type: "SpoofingCandidate",
        spot_direction: TofDirection::Bullish,
        spot_risk_score: 88,
        spot_data_quality: 86.0,
        spot_confidence: 0.9,
        summary: "safe summary",
    });

    std::env::remove_var("PERP_TOF_ENABLED");

    assert_eq!(metrics.candidate_type, "PerpTofDisabled");
    assert_eq!(metrics.metrics_direction, TofDirection::Neutral);
    assert_eq!(metrics.risk_score, 0);
    assert_eq!(metrics.data_quality, 86.0);
}

fn perp_metrics(candidate_type: &str, direction: TofDirection, risk_score: u8) -> PerpTofMetrics {
    PerpTofMetrics {
        oi_change: 140_000.0,
        oi_direction: "long_increase".to_string(),
        funding_rate: -0.06,
        funding_side: "short".to_string(),
        liquidation_pressure: 82.0,
        squeeze_side: "short".to_string(),
        agg_buy_volume: 1_400_000.0,
        agg_sell_volume: 400_000.0,
        direction_bias: direction,
        metrics_direction: direction,
        risk_score,
        data_quality: 88.0,
        candidate_type: candidate_type.to_string(),
        explain_tags: vec!["OI long increase".to_string()],
        confidence: 86.0,
    }
}
