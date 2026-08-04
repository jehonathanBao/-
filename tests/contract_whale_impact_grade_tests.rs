use btc_toxic_flow_monitor_rs::contract_whale_monitor::discord_gate::impact_grade_v3_discord_eligible;
use btc_toxic_flow_monitor_rs::contract_whale_monitor::impact_baseline::{
    build_robust_impact_baseline, score_event_impact, ImpactBaselineKey,
};
use btc_toxic_flow_monitor_rs::contract_whale_monitor::impact_episode::{
    aggregate_shock_episodes, ImpactBucketContribution, ImpactEventFragment,
};
use btc_toxic_flow_monitor_rs::contract_whale_monitor::impact_grade::{
    assess_contract_impact_episode, ContractEventImpactGrade, ContractImpactEpisode,
    ImpactGradeState,
};
use btc_toxic_flow_monitor_rs::contract_whale_monitor::ContractWhaleRuntimeConfig;

fn base_episode() -> ContractImpactEpisode {
    ContractImpactEpisode {
        episode_id: "episode-test".to_string(),
        symbol: "BTC".to_string(),
        start_time_ms: 1_700_000_000_000,
        end_time_ms: 1_700_000_060_000,
        source_event_ids: vec!["source-test".to_string()],
        total_volume_btc: 975.407,
        total_notional_usd: 62_540_360.75,
        net_volume_btc: 537.175,
        unique_turnover_btc: None,
        unique_turnover_notional_usd: None,
        live_liquidation_btc: None,
        live_liquidation_notional_usd: None,
        peak_abs_price_move_pct: Some(0.2077),
        peak_abs_oi_change_pct: Some(0.1425),
        confirmed_sources: vec!["binance".to_string(), "bitfinex".to_string()],
        data_quality: 85,
        robust_percentile: Some(99.5),
        robust_z: Some(4.5),
        baseline_sample_count: 20_000,
    }
}

#[test]
fn ordinary_relative_burst_without_liquidation_or_turnover_is_never_s() {
    let assessment = assess_contract_impact_episode(
        &base_episode(),
        &ContractWhaleRuntimeConfig::default(),
        1_700_000_100_000,
    );
    assert_ne!(assessment.grade, ContractEventImpactGrade::S);
    assert!(assessment
        .reason_codes
        .iter()
        .any(|reason| reason == "s_hard_evidence_missing"));
}

#[test]
fn missing_live_liquidation_and_unique_turnover_fail_s_closed() {
    let mut episode = base_episode();
    episode.total_volume_btc = 25_000.0;
    episode.total_notional_usd = 1_500_000_000.0;
    episode.peak_abs_price_move_pct = Some(3.0);
    episode.robust_percentile = Some(99.99);
    episode.robust_z = Some(8.0);

    let assessment = assess_contract_impact_episode(
        &episode,
        &ContractWhaleRuntimeConfig::default(),
        1_700_000_100_000,
    );
    assert_ne!(assessment.grade, ContractEventImpactGrade::S);
    assert_ne!(assessment.state, ImpactGradeState::Confirmed);
}

#[test]
fn extreme_live_liquidation_confirms_s() {
    let mut episode = base_episode();
    episode.total_volume_btc = 8_000.0;
    episode.total_notional_usd = 500_000_000.0;
    episode.live_liquidation_btc = Some(2_500.0);
    episode.live_liquidation_notional_usd = Some(250_000_000.0);
    episode.peak_abs_price_move_pct = Some(2.0);
    episode.robust_percentile = Some(99.95);
    episode.robust_z = Some(6.0);

    let assessment = assess_contract_impact_episode(
        &episode,
        &ContractWhaleRuntimeConfig::default(),
        1_700_000_100_000,
    );
    assert_eq!(assessment.grade, ContractEventImpactGrade::S);
    assert_eq!(assessment.state, ImpactGradeState::Confirmed);
    assert!(assessment
        .reason_codes
        .iter()
        .any(|reason| reason == "s_live_liquidation_extreme"));
}

#[test]
fn extraordinary_unique_turnover_confirms_s() {
    let mut episode = base_episode();
    episode.total_volume_btc = 25_000.0;
    episode.total_notional_usd = 1_500_000_000.0;
    episode.unique_turnover_btc = Some(20_000.0);
    episode.unique_turnover_notional_usd = Some(1_000_000_000.0);
    episode.peak_abs_price_move_pct = Some(2.5);
    episode.robust_percentile = Some(99.99);
    episode.robust_z = Some(7.0);

    let assessment = assess_contract_impact_episode(
        &episode,
        &ContractWhaleRuntimeConfig::default(),
        1_700_000_100_000,
    );
    assert_eq!(assessment.grade, ContractEventImpactGrade::S);
    assert_eq!(assessment.state, ImpactGradeState::Confirmed);
    assert!(assessment
        .reason_codes
        .iter()
        .any(|reason| reason == "s_unique_turnover_extreme"));
}

#[test]
fn major_non_systemic_event_can_be_a() {
    let mut episode = base_episode();
    episode.total_volume_btc = 3_000.0;
    episode.total_notional_usd = 190_000_000.0;
    episode.peak_abs_price_move_pct = Some(0.6);
    episode.robust_percentile = Some(99.5);
    episode.robust_z = Some(4.1);

    let assessment = assess_contract_impact_episode(
        &episode,
        &ContractWhaleRuntimeConfig::default(),
        1_700_000_100_000,
    );
    assert_eq!(assessment.grade, ContractEventImpactGrade::A);
    assert_eq!(assessment.state, ImpactGradeState::Confirmed);
}

#[test]
fn insufficient_baseline_is_evidence_insufficient_and_not_s() {
    let mut episode = base_episode();
    episode.baseline_sample_count = 10;
    episode.robust_percentile = None;
    episode.robust_z = None;
    episode.unique_turnover_btc = Some(50_000.0);
    episode.unique_turnover_notional_usd = Some(2_000_000_000.0);
    episode.peak_abs_price_move_pct = Some(4.0);

    let assessment = assess_contract_impact_episode(
        &episode,
        &ContractWhaleRuntimeConfig::default(),
        1_700_000_100_000,
    );
    assert_ne!(assessment.grade, ContractEventImpactGrade::S);
    assert_eq!(assessment.state, ImpactGradeState::EvidenceInsufficient);
}

#[test]
fn robust_baseline_ignores_non_positive_and_rejects_zero_mad() {
    let key = ImpactBaselineKey {
        symbol: "BTC".to_string(),
        window_sec: 3600,
        threshold_profile: "binance_bitfinex".to_string(),
    };
    assert!(build_robust_impact_baseline(key.clone(), [0.0, -1.0, 1.0, 1.0], 2).is_none());
    assert!(build_robust_impact_baseline(key, [1.0; 10], 2).is_none());
}

#[test]
fn robust_baseline_score_is_stable_when_one_outlier_is_added() {
    let key = ImpactBaselineKey {
        symbol: "BTC".to_string(),
        window_sec: 3600,
        threshold_profile: "binance_bitfinex".to_string(),
    };
    let base: Vec<f64> = (1..=10_000).map(|value| value as f64).collect();
    let baseline = build_robust_impact_baseline(key.clone(), base.clone(), 10_000).unwrap();
    let with_outlier =
        build_robust_impact_baseline(key, base.into_iter().chain([1_000_000_000.0]), 10_000)
            .unwrap();
    let first = score_event_impact(50_000.0, &baseline).unwrap();
    let second = score_event_impact(50_000.0, &with_outlier).unwrap();
    assert!((first.robust_z - second.robust_z).abs() < 0.02);
    assert!((first.percentile - second.percentile).abs() < 0.02);
}

#[test]
fn adjacent_fragments_merge_and_overlapping_buckets_are_counted_once() {
    let fragment = |event_id: &str, start_time_ms: i64| ImpactEventFragment {
        event_id: event_id.to_string(),
        symbol: "BTC".to_string(),
        start_time_ms,
        end_time_ms: start_time_ms + 60_000,
        total_volume_btc: 999.0,
        total_notional_usd: 99_000_000.0,
        net_volume_btc: 100.0,
        unique_turnover_btc: Some(100.0),
        unique_turnover_notional_usd: Some(10_000_000.0),
        live_liquidation_btc: None,
        live_liquidation_notional_usd: None,
        peak_abs_price_move_pct: Some(0.4),
        peak_abs_oi_change_pct: Some(0.2),
        confirmed_sources: vec!["binance".to_string()],
        data_quality: 90,
        robust_percentile: Some(99.0),
        robust_z: Some(3.0),
        baseline_sample_count: 20_000,
        flow_buckets: vec![ImpactBucketContribution {
            identity: "bucket-1".to_string(),
            volume_btc: 500.0,
            notional_usd: 50_000_000.0,
        }],
        liquidation_buckets: Vec::new(),
    };
    let episodes =
        aggregate_shock_episodes(vec![fragment("e1", 1_000), fragment("e2", 30_000)], 900);
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0].source_event_ids, vec!["e1", "e2"]);
    assert_eq!(episodes[0].total_volume_btc, 500.0);
    assert!(episodes[0].episode_id.starts_with("episode-"));
}

#[test]
fn impact_grade_defaults_are_conservative_and_validation_rejects_inverted_thresholds() {
    let defaults = ContractWhaleRuntimeConfig::default();
    assert_eq!(defaults.impact_grade_v3.grade_version, "cwm_impact_v3");
    assert_eq!(defaults.impact_grade_v3.s.min_robust_percentile, 99.95);
    assert!(defaults.impact_grade_v3.validate().is_ok());
    let mut invalid = defaults.impact_grade_v3.clone();
    invalid.a.min_robust_percentile = 100.0;
    assert!(invalid.validate().is_err());
    invalid = defaults.impact_grade_v3.clone();
    invalid.s.min_abs_price_move_pct = f64::NAN;
    assert!(invalid.validate().is_err());
}

#[test]
fn historical_impact_fixtures_keep_ordinary_flow_below_s_and_confirm_systemic_anchors() {
    let config = ContractWhaleRuntimeConfig::default();
    let ordinary: ContractImpactEpisode = serde_json::from_str(include_str!(
        "fixtures/contract_whale_impact/ordinary-2026-07.json"
    ))
    .unwrap();
    let march_2020: ContractImpactEpisode = serde_json::from_str(include_str!(
        "fixtures/contract_whale_impact/btc-2020-03-12.json"
    ))
    .unwrap();
    let october_2025: ContractImpactEpisode = serde_json::from_str(include_str!(
        "fixtures/contract_whale_impact/btc-2025-10-10.json"
    ))
    .unwrap();

    let ordinary_assessment = assess_contract_impact_episode(&ordinary, &config, 1_800_000_000_000);
    assert_ne!(ordinary_assessment.grade, ContractEventImpactGrade::S);
    assert_eq!(ordinary_assessment.state, ImpactGradeState::Confirmed);

    for anchor in [march_2020, october_2025] {
        let assessment = assess_contract_impact_episode(&anchor, &config, 1_800_000_000_000);
        assert_eq!(assessment.grade, ContractEventImpactGrade::S);
        assert_eq!(assessment.state, ImpactGradeState::Confirmed);
        assert!(impact_grade_v3_discord_eligible(&assessment));
    }
}

#[test]
fn provisional_a_and_confirmed_c_never_enter_v3_discord_gate() {
    let mut episode = base_episode();
    episode.total_volume_btc = 25_000.0;
    episode.total_notional_usd = 1_500_000_000.0;
    episode.peak_abs_price_move_pct = Some(2.5);
    episode.robust_percentile = Some(99.99);
    episode.robust_z = Some(8.0);
    let mut assessment = assess_contract_impact_episode(
        &episode,
        &ContractWhaleRuntimeConfig::default(),
        1_700_000_100_000,
    );
    assert_eq!(assessment.grade, ContractEventImpactGrade::A);
    assert_eq!(assessment.state, ImpactGradeState::Provisional);
    assert!(!impact_grade_v3_discord_eligible(&assessment));
    assessment.grade = ContractEventImpactGrade::C;
    assessment.state = ImpactGradeState::Confirmed;
    assert!(!impact_grade_v3_discord_eligible(&assessment));
}
