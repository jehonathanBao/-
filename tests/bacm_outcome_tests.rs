use std::sync::{Mutex, MutexGuard, OnceLock};

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::{
        reset_binance_alt_contract_runtime_config, set_binance_alt_contract_runtime_config,
        BinanceAltContractRuntimeConfig, BinanceAltOutcomeConfig,
    },
    service::{AltContractOutcomeFilter, BinanceAltContractService},
    types::{
        AltContractExposureTier, AltContractSeverity, AltContractSignalOutcome,
        AltContractStructureConfidence, AltContractSymbolTier,
    },
};

fn guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn outcome(id: &str, signal_ts: i64, markout_1h_bps: Option<f64>) -> AltContractSignalOutcome {
    AltContractSignalOutcome {
        signal_id: id.to_string(),
        product_id: "ALTUSDT".to_string(),
        tier: AltContractSymbolTier::C,
        signal_ts,
        window_sec: 60,
        signal_type: "MainForceLongBuild".to_string(),
        anomaly_severity: AltContractSeverity::High,
        structure_confidence: AltContractStructureConfidence::Medium,
        exposure_tier: AltContractExposureTier::Highlight,
        ais_score: 82.0,
        abnormal_score: 70,
        build_score: 76,
        regime: "Accumulation".to_string(),
        oi_context: "1".to_string(),
        liquidation_context: "not_liquidation_driven".to_string(),
        entry_price: Some(100.0),
        markout_1h_bps,
        follow_through_1h: markout_1h_bps.map(|value| value > 0.0),
        ..AltContractSignalOutcome::default()
    }
}

#[test]
fn mark_price_evaluator_records_real_1h_outcome_and_summary_filters() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let mut config = BinanceAltContractRuntimeConfig {
        enabled: true,
        outcomes: BinanceAltOutcomeConfig {
            enabled: true,
            min_samples_for_reporting: 1,
        },
        ..BinanceAltContractRuntimeConfig::default()
    };
    set_binance_alt_contract_runtime_config(config.clone());

    let now = 1_700_000_000_000_i64;
    let service = BinanceAltContractService::new(true, true, now - 120_000);
    service.insert_outcome_for_tests(outcome("outcome-1", now - 60 * 60_000, None));
    service.update_outcomes_for_tests(
        "ALTUSDT",
        btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::types::AltContractDirection::Buy,
        102.0,
        now,
    );

    let summary = service.outcome_summary(AltContractOutcomeFilter {
        tier: Some("C".to_string()),
        window_sec: Some(60),
        signal_type: Some("MainForceLongBuild".to_string()),
        severity: Some("High".to_string()),
        ais_min: Some(80.0),
        regime: Some("Accumulation".to_string()),
        oi_context: Some("1".to_string()),
        ..AltContractOutcomeFilter::default()
    });

    assert_eq!(summary.sample_count, 1);
    assert!(!summary.insufficient_samples);
    assert_eq!(summary.follow_through_rate, Some(100.0));
    assert!(summary
        .median_markout_bps
        .is_some_and(|markout| (markout - 200.0).abs() < 1e-6));

    config.outcomes.min_samples_for_reporting = 2;
    set_binance_alt_contract_runtime_config(config);
    let insufficient = service.outcome_summary(AltContractOutcomeFilter::default());
    assert_eq!(insufficient.sample_count, 1);
    assert!(insufficient.insufficient_samples);
    reset_binance_alt_contract_runtime_config();
}
