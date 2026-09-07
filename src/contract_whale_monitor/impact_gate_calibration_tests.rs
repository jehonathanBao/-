use super::super::impact_forecast::calibration_tests::{prior, signal};
use super::super::impact_v4_2::{
    build_hybrid_forecast, CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
};
use super::*;

struct TestStore(SqliteStore);
impl TestStore {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("cwm-calibration-{}.sqlite", uuid::Uuid::new_v4()));
        let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
        store.migrate().unwrap();
        Self(store)
    }
}
impl Drop for TestStore {
    fn drop(&mut self) {
        // Only the unique database created by this fixture, never runtime data.
        let _ = std::fs::remove_file(self.0.path());
    }
}

fn forecast(id: &str, ts: i64) -> ContractWhaleMultiHorizonImpactForecast {
    let mut forecast = build_hybrid_forecast(&signal(id, ts, true), &[], &[], ts);
    forecast.horizons.truncate(1);
    forecast.dominant_horizon = "15m".into();
    forecast.source_policy = "binance_only".into();
    forecast.binance_evidence_complete = true;
    forecast
}

fn outcome(id: &str, ts: i64) -> ContractWhaleHorizonOutcome {
    let mut outcome = prior(id, ts, true);
    outcome.outcome_version = CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION.into();
    outcome
}

#[test]
fn calibration_signed_success_rejects_unknown_and_nonfinite() {
    for direction in ["bullish", "bearish", "buy", "sell"] {
        assert!(aligned_success(direction, 17.942055752), "{direction}");
        assert!(!aligned_success(direction, -17.942055752));
        assert!(!aligned_success(direction, f64::INFINITY));
        assert!(!aligned_success(direction, f64::NAN));
        assert!(!aligned_success(direction, 0.0));
    }
    assert!(!aligned_success("unknown", 1.0));
    assert!(!aligned_success("neutral", 1.0));
}

#[test]
fn calibration_gate_counts_missing_expected_outcomes() {
    let store = TestStore::new();
    store
        .0
        .upsert_contract_whale_impact_forecasts(&[forecast("a", 60_000), forecast("b", 120_000)])
        .unwrap();
    store
        .0
        .upsert_contract_whale_horizon_outcomes(&[outcome("a", 60_000)])
        .unwrap();
    let current = forecast("current", 2_000_000);
    let config = super::super::config::ContractWhaleImpactV4HybridConfig::default();
    let decision = evaluate_forecast_gate(&store.0, &current, &config, current.event_ts).unwrap();
    assert_eq!(decision.raw_sample_count, 1);
    assert_eq!(decision.accuracy, 1.0);
    assert_eq!(decision.coverage, 0.5);
    assert!(!decision.allowed);
    assert_eq!(decision.reason, "missing_or_invalid_outcomes");
}

#[test]
fn calibration_gate_excludes_unmatured_and_duplicate_episode_outcomes() {
    let store = TestStore::new();
    let a = outcome("a", 60_000);
    let mut duplicate = a.clone();
    duplicate.event_id = "duplicate".into();
    let mut duplicate_forecast = forecast("duplicate", 60_000);
    duplicate_forecast.episode_id = "a".into();
    store
        .0
        .upsert_contract_whale_impact_forecasts(&[
            forecast("a", 60_000),
            duplicate_forecast,
            forecast("future", 1_900_000),
        ])
        .unwrap();
    store
        .0
        .upsert_contract_whale_horizon_outcomes(&[
            a.clone(),
            duplicate,
            outcome("future", 1_900_000),
        ])
        .unwrap();
    let current = forecast("current", 2_000_000);
    let config = super::super::config::ContractWhaleImpactV4HybridConfig::default();
    let first = evaluate_forecast_gate(&store.0, &current, &config, current.event_ts).unwrap();
    assert_eq!(first.raw_sample_count, 1);
    assert_eq!(first.unique_episode_count, 1);
    assert_eq!(first.effective_sample_count, 1.0);
    assert_eq!(first.coverage, 1.0);
    store
        .0
        .upsert_contract_whale_horizon_outcomes(&[a])
        .unwrap();
    let repeated = evaluate_forecast_gate(&store.0, &current, &config, current.event_ts).unwrap();
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(repeated).unwrap()
    );
}

#[test]
fn calibration_gate_closed_and_invalid_outcomes_cannot_open() {
    let store = TestStore::new();
    store
        .0
        .upsert_contract_whale_impact_forecasts(&[
            forecast("closed", 60_000),
            forecast("invalid", 120_000),
        ])
        .unwrap();
    let mut closed = outcome("closed", 60_000);
    closed.state = "closed".into();
    let mut invalid = outcome("invalid", 120_000);
    invalid.price_coverage = 0.1;
    store
        .0
        .upsert_contract_whale_horizon_outcomes(&[closed, invalid])
        .unwrap();
    let current = forecast("current", 2_000_000);
    let mut config = super::super::config::ContractWhaleImpactV4HybridConfig::default();
    config.auto_gate_min_open_samples = 1;
    config.auto_gate_min_open_effective_samples = 0.5;
    let decision = evaluate_forecast_gate(&store.0, &current, &config, current.event_ts).unwrap();
    assert!(!decision.allowed);
    assert_eq!(decision.coverage, 0.0);
    assert_eq!(decision.raw_sample_count, 0);
    assert!(!current.production_ready);
}

#[test]
fn calibration_gate_legacy_forecast_and_missing_candidates_fail_closed() {
    let store = TestStore::new();
    store
        .0
        .upsert_contract_whale_horizon_outcomes(&[outcome("orphan", 60_000)])
        .unwrap();
    let mut current = forecast("current", 2_000_000);
    let config = super::super::config::ContractWhaleImpactV4HybridConfig::default();
    let decision = evaluate_forecast_gate(&store.0, &current, &config, current.event_ts).unwrap();
    assert_eq!(decision.coverage, 0.0);
    assert_eq!(decision.raw_sample_count, 0);
    assert_eq!(decision.reason, "expected_candidates_unavailable");
    current.forecast_version = "cwm_impact_v4_2_hybrid".into();
    let decision = evaluate_forecast_gate(&store.0, &current, &config, current.event_ts).unwrap();
    assert!(!decision.allowed);
    assert_eq!(decision.reason, "forecast_version_ineligible");
}

#[test]
fn calibration_gate_uses_kish_effective_size_and_weighted_accuracy() {
    let store = TestStore::new();
    store
        .0
        .upsert_contract_whale_impact_forecasts(&[forecast("a", 60_000), forecast("b", 120_000)])
        .unwrap();
    let mut fallback = outcome("b", 120_000);
    fallback.behavior = "other".into();
    fallback.signed_markout_bps = Some(-10.0);
    store
        .0
        .upsert_contract_whale_horizon_outcomes(&[outcome("a", 60_000), fallback])
        .unwrap();
    let current = forecast("current", 2_000_000);
    let config = super::super::config::ContractWhaleImpactV4HybridConfig::default();
    let decision = evaluate_forecast_gate(&store.0, &current, &config, current.event_ts).unwrap();
    let expected = super::super::impact_v4_2::effective_sample_size(&[1.0, 0.60]);
    assert!((decision.effective_sample_count - expected).abs() < 1e-9);
    assert!((decision.accuracy - 1.0 / 1.60).abs() < 1e-9);
}

#[test]
fn calibration_version_bump_preserves_daily_alert_budget() {
    let store = TestStore::new();
    store
        .0
        .with_write_connection(|conn| {
            conn.execute(
                "INSERT INTO contract_whale_v42_gate_evaluations
            (gate_version, cohort_key, evaluated_at_ms, state, allowed, reason,
             raw_sample_count, effective_sample_count, accuracy, coverage, payload_json)
            VALUES (?1, 'BTC:15m:bearish', 1000000, 'OPEN', 1, 'fixture', 1, 1, 1, 1, '{}')",
                ["cwm_v4_2_auto_gate_v1"],
            )?;
            Ok(())
        })
        .unwrap();
    let mut config = super::super::config::ContractWhaleImpactV4HybridConfig::default();
    config.auto_gate_max_alerts_per_day = 1;
    assert!(!canary_slot_available(&store.0, &config, 2_000_000).unwrap());
}
