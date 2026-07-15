use btc_toxic_flow_monitor_rs::{
    contract_whale_monitor::types::{
        ContractWhaleActiveSources, ContractWhaleDirection, ContractWhaleEvidenceState,
        ContractWhaleLiquidationForce, ContractWhaleMarketType, ContractWhalePriceResponseType,
        ContractWhaleScoreBreakdown, ContractWhaleSeverity, ContractWhaleSignal,
        ContractWhaleSignalType, ContractWhaleSourceRole, ContractWhaleSpotConfirmationContext,
    },
    runtime::{
        cwm_risk_fusion::{
            build_cwm_risk_contribution, build_cwm_risk_contribution_for_candidate,
            build_split_risk_systems, decayed_toxic_score, SplitRiskSystemsInput,
        },
        metric_provenance::MetricLineage,
        perp_tof_metrics::observed_perp_snapshot_from_cwm,
        tof_metrics::{TofDirection, TofMetrics},
    },
};

#[test]
fn split_risk_systems_keep_toxic_score_independent_from_cwm() {
    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&sample_cwm_signal()));
    let tof_metrics = sample_tof_metrics();
    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 84,
        short_tof_score: 88.4,
        short_direction: TofDirection::Bearish,
        toxic_type: "SpoofingCandidate",
        data_quality: 86.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(89),
        perp_score: Some(87),
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: contribution,
    });
    assert_eq!(systems.short_term_toxic.toxic_score, 84);
    assert_eq!(systems.short_term_toxic.toxicity_hazard_score, Some(88.4));
    assert_eq!(systems.short_term_toxic.short_pressure, -84);
    assert_eq!(systems.short_term_toxic.confidence, 82.0);
    assert_eq!(systems.short_term_toxic.confidence_source, "detector");
    assert_eq!(
        systems.short_term_toxic.direction_context.direction,
        "bearish"
    );
    assert_eq!(
        systems.short_term_toxic.direction_context.signed_pressure,
        -84
    );
    assert_eq!(
        systems.short_term_toxic.direction_context.source,
        "detector"
    );
    assert_eq!(systems.short_term_toxic.ts, 1_700_000_000_000);
    assert_eq!(systems.short_term_toxic.symbol, "BTC-PERP");
    assert_eq!(systems.short_term_toxic.data_quality, 86.0);
    assert_eq!(systems.short_term_toxic.severity, "Critical");
    assert_eq!(systems.short_term_toxic.toxic_type, "spoofing");
    assert_eq!(systems.short_term_toxic.ttl_sec, 300);
    assert_eq!(systems.short_term_toxic.expires_at, 1_700_000_300_000);
    assert_eq!(systems.short_term_toxic.half_life_sec, 45);
    assert_eq!(systems.short_term_toxic.max_ttl_sec, 300);
    assert_eq!(systems.short_term_toxic.decayed_score, 84.0);
    assert!(systems
        .short_term_toxic
        .decay_formula
        .contains("exp(-elapsedSec / halfLifeSec)"));
    assert_eq!(systems.short_term_toxic.reasons.len(), 4);
    assert_eq!(
        systems.short_term_toxic.reasons[0].reason_type,
        "DetectorSignal"
    );
    assert_eq!(systems.short_term_toxic.reasons[0].weight, 1.0);
    assert_eq!(systems.short_term_toxic.reasons[0].window_sec, 0);
    assert_eq!(systems.short_term_toxic.reasons[0].direction, "bearish");
    assert_eq!(
        systems.short_term_toxic.timeframes,
        vec![
            "1s".to_string(),
            "5s".to_string(),
            "15s".to_string(),
            "60s".to_string()
        ]
    );
    assert!(systems.short_term_toxic.formula.contains("AggressiveSweep"));
    assert!(systems
        .short_term_toxic
        .discord_gate
        .contains("toxicScore>=85"));
    assert!(systems
        .short_term_toxic
        .discord_gate
        .contains("confidence>=70"));
    assert_eq!(systems.main_force_structure.main_force_score, 79);
    assert!(systems.main_force_structure.main_force_confirmed);
    assert_eq!(
        systems.main_force_structure.main_force_confirmation_count,
        7
    );
    assert_eq!(
        systems.main_force_structure.main_force_confirmation_total,
        7
    );
    assert_eq!(
        systems
            .main_force_structure
            .main_force_confirmation_threshold,
        3
    );
    assert_eq!(systems.main_force_structure.structure_bias, 69);
    assert_eq!(systems.main_force_structure.extreme_impact_score, 94);
    assert!(systems.main_force_structure.extreme_impact_confirmed);
    assert_eq!(systems.main_force_structure.ts, 1_700_000_000_000);
    assert_eq!(systems.main_force_structure.symbol, "BTC-PERP");
    assert_eq!(systems.main_force_structure.data_quality, 92.6);
    assert_eq!(systems.main_force_structure.confidence, 95.01);
    assert_eq!(systems.main_force_structure.severity, "Major");
    assert_eq!(
        systems.main_force_structure.regime_type,
        "main_force_long_build"
    );
    assert_eq!(systems.main_force_structure.spot_score, 76);
    assert_eq!(systems.main_force_structure.spot_cvd_score, 84);
    assert_eq!(systems.main_force_structure.spot_volume_anomaly, 73);
    assert_eq!(systems.main_force_structure.spot_absorption, 64);
    assert_eq!(systems.main_force_structure.spot_liquidity_shift, 73);
    assert_eq!(systems.main_force_structure.spot_price_response, 85);
    assert_eq!(systems.main_force_structure.contract_score, 73);
    assert_eq!(systems.main_force_structure.cwm_aggressive_flow, 94);
    assert_eq!(systems.main_force_structure.oi_impulse, 71);
    assert_eq!(systems.main_force_structure.liquidation_context, 0);
    assert_eq!(systems.main_force_structure.funding_crowding, 90);
    assert_eq!(systems.main_force_structure.basis_premium, 74);
    assert_eq!(
        systems.main_force_structure.active_exchange_confirmation,
        92
    );
    assert!(systems.main_force_structure.cross_confirm_score >= 92);
    assert_eq!(
        systems
            .main_force_structure
            .spot_contract_direction_consistency,
        94
    );
    assert_eq!(systems.main_force_structure.multi_window_consistency, 96);
    assert_eq!(systems.main_force_structure.price_response_consistency, 90);
    assert_eq!(systems.main_force_structure.source_coverage, 100);
    assert_eq!(systems.main_force_structure.structure_raw, 78.6);
    assert_eq!(systems.main_force_structure.spot_contract_floor, 73);
    assert_eq!(systems.main_force_structure.duration_score, 100);
    assert_eq!(systems.main_force_structure.liquidation_penalty, 0.0);
    assert_eq!(systems.main_force_structure.crowding_penalty, 0.0);
    assert_eq!(systems.main_force_structure.oi_score, 71);
    assert_eq!(systems.main_force_structure.liquidation_score, 0);
    assert_eq!(systems.main_force_structure.funding_crowding_score, 90);
    assert_eq!(systems.main_force_structure.cwm_score, 94);
    assert_eq!(systems.main_force_structure.reasons.len(), 28);
    assert_eq!(
        systems.main_force_structure.reasons[0].reason_type,
        "SpotScore"
    );
    assert_eq!(
        systems.market_structure_score.regime_type,
        systems.main_force_structure.regime_type
    );
    assert_eq!(
        systems.main_force_structure.timeframes,
        vec![
            "5m".to_string(),
            "15m".to_string(),
            "1h".to_string(),
            "4h".to_string()
        ]
    );
    assert!(systems
        .main_force_structure
        .formula
        .contains("MarketStructureScore"));
    assert!(systems
        .main_force_structure
        .formula
        .contains("min(spotScore, contractScore)"));
}

#[test]
fn short_toxic_score_uses_expected_severity_bands_and_exponential_decay() {
    assert_eq!(short_system_for_score(39).short_term_toxic.severity, "Calm");
    assert_eq!(
        short_system_for_score(40).short_term_toxic.severity,
        "Watch"
    );
    assert_eq!(short_system_for_score(60).short_term_toxic.severity, "High");
    assert_eq!(
        short_system_for_score(75).short_term_toxic.severity,
        "Critical"
    );
    assert_eq!(short_system_for_score(90).short_term_toxic.severity, "S");
    assert_eq!(short_system_for_score(90).short_term_toxic.ttl_sec, 300);
    assert_eq!(short_system_for_score(75).short_term_toxic.ttl_sec, 300);
    assert_eq!(short_system_for_score(60).short_term_toxic.ttl_sec, 300);
    assert_eq!(decayed_toxic_score(90, 45.0, 45), 33.11);
    assert_eq!(decayed_toxic_score(90, 0.0, 45), 90.0);
}

fn short_system_for_score(
    score: u8,
) -> btc_toxic_flow_monitor_rs::runtime::cwm_risk_fusion::SplitRiskSystems {
    let tof_metrics = weak_spot_tof_metrics();
    build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: score,
        short_tof_score: score as f64,
        short_direction: TofDirection::Bearish,
        toxic_type: "SpoofingCandidate",
        data_quality: 86.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(80),
        perp_score: Some(80),
        metrics_direction: TofDirection::Bearish,
        cwm_contribution: build_cwm_risk_contribution("BTC-PERP", None),
    })
}

fn sample_tof_metrics() -> TofMetrics {
    TofMetrics {
        trade_imbalance: -0.43,
        trade_imbalance_score: 73.0,
        vpin_proxy: 89.0,
        vpin_bucket_count: 8,
        vpin_window_volume: 12_000.0,
        bid_depth_withdrawal: 58.0,
        ask_depth_withdrawal: 12.0,
        depth_withdrawal_score: 71.0,
        spread_bps: 8.4,
        spread_widening_score: 68.0,
        order_churn_score: 76.0,
        book_update_rate: 64.0,
        trade_rate: 52.0,
        liquidity_vacuum_score: 79.0,
        thin_side: "bid".to_string(),
        metrics_direction: TofDirection::Bearish,
        metrics_confidence: 82.0,
        tof_score: 88.4,
        final_risk_score: 84,
        metrics_completeness: 91.0,
        vpin_zscore: Some(2.4),
        vpin_percentile: Some(0.98),
        per_venue_vpin: Default::default(),
        lineage: observed_tof_lineage(),
        metric_lineage: observed_metric_lineage(),
    }
}

#[test]
fn toxic_hazard_is_direction_independent_and_unavailable_evidence_stays_fail_closed() {
    let observed = sample_tof_metrics();
    let build = |direction| {
        build_split_risk_systems(SplitRiskSystemsInput {
            ts_ms: 1_700_000_000_000,
            symbol: "BTC-PERP",
            short_toxic_score: 84,
            short_tof_score: 88.4,
            short_direction: direction,
            toxic_type: "WhaleFlowCandidate",
            data_quality: 86.0,
            detector_confidence: 82.0,
            direction_confidence: 77.0,
            direction_source: "detector",
            tof_metrics: &observed,
            advanced_score: Some(84),
            perp_score: Some(80),
            metrics_direction: direction,
            cwm_contribution: build_cwm_risk_contribution("BTC-PERP", None),
        })
        .short_term_toxic
    };

    let bullish = build(TofDirection::Bullish);
    let bearish = build(TofDirection::Bearish);
    assert_eq!(bullish.toxicity_hazard_score, bearish.toxicity_hazard_score);
    assert_eq!(bullish.toxic_score, bearish.toxic_score);
    assert_eq!(bullish.direction_context.signed_pressure, 84);
    assert_eq!(bearish.direction_context.signed_pressure, -84);

    let mut unavailable = observed;
    unavailable.lineage = MetricLineage::unavailable("observed_tof_missing");
    unavailable.metric_lineage.clear();
    let closed_systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 84,
        short_tof_score: 0.0,
        short_direction: TofDirection::Neutral,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 86.0,
        detector_confidence: 63.0,
        direction_confidence: 35.0,
        direction_source: "detector",
        tof_metrics: &unavailable,
        advanced_score: None,
        perp_score: None,
        metrics_direction: TofDirection::Neutral,
        cwm_contribution: build_cwm_risk_contribution("BTC-PERP", None),
    });
    let closed = &closed_systems.short_term_toxic;

    assert_eq!(closed.toxicity_hazard_score, None);
    assert_eq!(closed.confidence, 63.0);
    assert_eq!(closed.reasons.len(), 1);
    assert_eq!(closed.reasons[0].reason_type, "DetectorSignal");
    assert_eq!(closed_systems.main_force_structure.spot_score, 0);
    assert_eq!(closed_systems.main_force_structure.spot_cvd_score, 0);
    assert_eq!(closed_systems.main_force_structure.spot_volume_anomaly, 0);
    assert_eq!(closed_systems.main_force_structure.spot_absorption, 0);
    assert_eq!(closed_systems.main_force_structure.spot_liquidity_shift, 0);
    assert_eq!(closed_systems.main_force_structure.spot_price_response, 0);
    assert_eq!(closed_systems.main_force_structure.extreme_impact_score, 0);
    assert!(!closed_systems.main_force_structure.extreme_impact_confirmed);
}

#[test]
fn cwm_spot_volume_anomaly_uses_relative_vpin_context() {
    let mut low_relative = sample_tof_metrics();
    low_relative.vpin_proxy = 90.0;
    low_relative.vpin_zscore = Some(-2.0);
    low_relative.vpin_percentile = Some(0.05);
    let mut high_relative = low_relative.clone();
    high_relative.vpin_zscore = Some(3.0);
    high_relative.vpin_percentile = Some(0.99);

    let low = split_systems_for_tof(&low_relative);
    let high = split_systems_for_tof(&high_relative);

    assert!(
        high.main_force_structure.spot_volume_anomaly
            > low.main_force_structure.spot_volume_anomaly,
        "relative VPIN anomaly must raise the CWM spot anomaly component"
    );
}

#[test]
fn cwm_discord_gating_scores_ignore_absolute_vpin_level() {
    let mut low_raw = sample_tof_metrics();
    low_raw.vpin_proxy = 10.0;
    low_raw.vpin_zscore = Some(-2.0);
    low_raw.vpin_percentile = Some(0.05);
    let mut high_raw = low_raw.clone();
    high_raw.vpin_proxy = 90.0;

    let low = split_systems_for_tof(&low_raw);
    let high = split_systems_for_tof(&high_raw);

    assert_eq!(
        low.main_force_structure.spot_volume_anomaly,
        high.main_force_structure.spot_volume_anomaly
    );
    assert_eq!(
        low.main_force_structure.main_force_score,
        high.main_force_structure.main_force_score
    );
    assert_eq!(
        low.main_force_structure.extreme_impact_score,
        high.main_force_structure.extreme_impact_score
    );
    assert_eq!(
        low.main_force_structure.main_force_confirmed,
        high.main_force_structure.main_force_confirmed
    );
}

#[test]
fn observed_perp_snapshot_accepts_canonical_contract_aliases() {
    let signal = sample_cwm_signal();

    for alias in ["BTCUSDT", "BTCPERP", "BTC-USD-SWAP", "tBTCF0:USTF0"] {
        let snapshot = observed_perp_snapshot_from_cwm(alias, &signal);
        assert!(snapshot.is_some(), "expected {alias} to resolve to BTC");
    }
}

fn split_systems_for_tof(
    tof_metrics: &TofMetrics,
) -> btc_toxic_flow_monitor_rs::runtime::cwm_risk_fusion::SplitRiskSystems {
    build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 84,
        short_tof_score: 70.0,
        short_direction: TofDirection::Bearish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 86.0,
        detector_confidence: 82.0,
        direction_confidence: 77.0,
        direction_source: "detector",
        tof_metrics,
        advanced_score: Some(84),
        perp_score: Some(80),
        metrics_direction: TofDirection::Bearish,
        cwm_contribution: build_cwm_risk_contribution("BTC-PERP", None),
    })
}

#[test]
fn toxic_reasons_only_claim_metrics_with_alert_eligible_lineage() {
    let mut partial = sample_tof_metrics();
    partial.metric_lineage.remove("sweep");
    partial.metric_lineage.remove("spread");
    partial.metric_lineage.remove("bookUpdateRate");
    partial.metric_lineage.remove("markout");
    partial.metric_lineage.remove("microVolatility");
    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 84,
        short_tof_score: 88.4,
        short_direction: TofDirection::Bullish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 10.0,
        detector_confidence: 73.0,
        direction_confidence: 67.0,
        direction_source: "detector",
        tof_metrics: &partial,
        advanced_score: Some(84),
        perp_score: Some(80),
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: build_cwm_risk_contribution("BTC-PERP", None),
    });
    let reason_types = systems
        .short_term_toxic
        .reasons
        .iter()
        .map(|reason| reason.reason_type.as_str())
        .collect::<Vec<_>>();

    assert_eq!(systems.short_term_toxic.confidence, 73.0);
    assert_eq!(systems.short_term_toxic.toxic_type, "adverse_selection");
    assert_eq!(
        reason_types,
        vec!["DetectorSignal", "OrderbookDeformation", "LiquidityGap"]
    );
}

#[test]
fn observed_zero_sweep_score_does_not_claim_an_aggressive_sweep() {
    let mut metrics = sample_tof_metrics();
    metrics.liquidity_vacuum_score = 0.0;
    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 84,
        short_tof_score: 70.0,
        short_direction: TofDirection::Bearish,
        toxic_type: "SweepCandidate",
        data_quality: 86.0,
        detector_confidence: 82.0,
        direction_confidence: 72.0,
        direction_source: "detector",
        tof_metrics: &metrics,
        advanced_score: Some(84),
        perp_score: Some(80),
        metrics_direction: TofDirection::Bearish,
        cwm_contribution: build_cwm_risk_contribution("BTC-PERP", None),
    });

    assert_eq!(systems.short_term_toxic.toxic_type, "adverse_selection");
    assert!(!systems
        .short_term_toxic
        .reasons
        .iter()
        .any(|reason| reason.reason_type == "AggressiveSweep"));
}

#[test]
fn cwm_contribution_is_safe_and_marks_discord_gate_independent() {
    let signal = sample_cwm_signal();
    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let json = serde_json::to_string(&contribution).expect("json");

    assert!(contribution.available);
    assert_eq!(contribution.score, Some(94));
    assert_eq!(contribution.contribution_weight, 0.25);
    assert_eq!(contribution.weighted_contribution, 23.5);
    assert_eq!(contribution.liquidation_suspected, Some(false));
    assert!(contribution.discord_gate_independent);
    assert!(json.contains("contract_whale_monitor"));
    assert!(json.contains("MarketStructureScore"));
    assert!(json.contains("liquidationSuspected"));
    assert!(!json.contains("finalRiskScore"));
    assert!(!json.to_ascii_lowercase().contains("webhook"));
    assert!(!json.to_ascii_lowercase().contains("token"));
    assert!(!json.contains("rawPayload"));
    assert!(!json.contains("rawEvidence"));
    assert!(!json.to_ascii_lowercase().contains("markout"));
}

#[test]
fn inferred_liquidation_and_unavailable_derivatives_are_display_only() {
    let mut signal = sample_cwm_signal();
    signal.score = 40;
    signal.signal_type = ContractWhaleSignalType::AggressiveSell;
    signal.direction = ContractWhaleDirection::Sell;
    signal.price_move_pct = Some(-0.28);
    signal.liquidation_suspected = true;
    signal.liquidation_ratio = Some(0.82);
    signal.liquidation_long_btc = 1_900.0;
    signal.liquidation_short_btc = 90.0;
    signal.oi_change_pct = Some(-2.4);
    signal.funding_rate = Some(0.05);
    signal.classification_v2.evidence.liquidation_status = "inferred".to_string();
    signal.classification_v2.evidence.liquidation_reason =
        Some("price_volume_shape_only".to_string());
    signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Missing;
    signal.classification_v2.evidence.funding = ContractWhaleEvidenceState::QueryFailed;

    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let contribution_json =
        serde_json::to_value(&contribution).expect("inferred contribution json");

    assert_eq!(contribution.oi_change_pct, None);
    assert_eq!(contribution.funding_rate, None);
    assert_eq!(contribution.liquidation_suspected, Some(true));
    assert_eq!(contribution.liquidation_ratio, Some(0.82));
    assert_eq!(contribution_json["oiLineage"]["provenance"], "unavailable");
    assert_eq!(contribution_json["oiLineage"]["alertEligible"], false);
    assert_eq!(
        contribution_json["fundingLineage"]["provenance"],
        "unavailable"
    );
    assert_eq!(contribution_json["fundingLineage"]["alertEligible"], false);
    assert_eq!(
        contribution_json["liquidationLineage"]["provenance"],
        "inferred"
    );
    assert_eq!(
        contribution_json["liquidationLineage"]["alertEligible"],
        false
    );

    let tof_metrics = weak_spot_tof_metrics();
    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: signal.ts,
        symbol: "BTC-PERP",
        short_toxic_score: 40,
        short_tof_score: 25.0,
        short_direction: TofDirection::Bearish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 82.0,
        detector_confidence: 70.0,
        direction_confidence: 70.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: None,
        perp_score: None,
        metrics_direction: TofDirection::Bearish,
        cwm_contribution: contribution,
    });
    assert_eq!(systems.main_force_structure.oi_impulse, 0);
    assert_eq!(systems.main_force_structure.funding_crowding, 0);
    assert_eq!(systems.main_force_structure.liquidation_context, 0);
    assert!(
        systems.main_force_structure.extreme_impact_score < 85,
        "inferred liquidation must not force the 85-point extreme-impact floor"
    );
    assert_ne!(
        systems.main_force_structure.regime_type,
        "long_liquidation_cascade"
    );
    assert_ne!(
        systems.main_force_structure.regime_type,
        "contract_short_squeeze"
    );
}

#[test]
fn non_finite_available_oi_and_funding_fail_closed() {
    let mut signal = sample_cwm_signal();
    signal.oi_change_pct = Some(-4.2);
    signal.funding_rate = Some(0.08);
    signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Available(f64::NAN);
    signal.classification_v2.evidence.funding =
        ContractWhaleEvidenceState::Available(f64::INFINITY);

    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let contribution_json =
        serde_json::to_value(&contribution).expect("non-finite contribution json");

    assert_eq!(contribution.oi_change_pct, None);
    assert_eq!(contribution.funding_rate, None);
    assert_eq!(contribution_json["oiLineage"]["alertEligible"], false);
    assert_eq!(contribution_json["fundingLineage"]["alertEligible"], false);
}

#[test]
fn live_liquidation_and_finite_available_derivatives_remain_alert_eligible() {
    let mut signal = sample_cwm_signal();
    signal.score = 42;
    signal.signal_type = ContractWhaleSignalType::AggressiveSell;
    signal.direction = ContractWhaleDirection::Sell;
    signal.price_move_pct = Some(-0.24);
    signal.liquidation_suspected = true;
    signal.liquidation_ratio = Some(0.74);
    signal.liquidation_long_btc = 1_620.0;
    signal.liquidation_short_btc = 140.0;
    signal.liquidation_notional_usd = 123_000_000.0;
    signal.oi_change_pct = Some(99.0);
    signal.funding_rate = Some(0.2);
    signal.classification_v2.evidence.liquidation_status = "live".to_string();
    signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Available(-1.5);
    signal.classification_v2.evidence.funding = ContractWhaleEvidenceState::Available(0.00076);

    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let contribution_json = serde_json::to_value(&contribution).expect("live contribution json");

    assert_eq!(contribution.oi_change_pct, Some(-1.5));
    assert_eq!(contribution.funding_rate, Some(0.0008));
    assert_eq!(contribution_json["oiLineage"]["alertEligible"], true);
    assert_eq!(contribution_json["fundingLineage"]["alertEligible"], true);
    assert_eq!(
        contribution_json["liquidationLineage"]["provenance"],
        "observed"
    );
    assert_eq!(
        contribution_json["liquidationLineage"]["alertEligible"],
        true
    );

    let tof_metrics = weak_spot_tof_metrics();
    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: signal.ts,
        symbol: "BTC-PERP",
        short_toxic_score: 40,
        short_tof_score: 25.0,
        short_direction: TofDirection::Bearish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 82.0,
        detector_confidence: 70.0,
        direction_confidence: 70.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: None,
        perp_score: None,
        metrics_direction: TofDirection::Bearish,
        cwm_contribution: contribution,
    });
    assert!(systems.main_force_structure.oi_impulse > 0);
    assert!(systems.main_force_structure.funding_crowding > 0);
    assert!(systems.main_force_structure.liquidation_context > 0);
    assert_eq!(
        systems.main_force_structure.regime_type,
        "long_liquidation_cascade"
    );
    assert!(systems.main_force_structure.extreme_impact_score >= 85);
}

#[test]
fn missing_cwm_does_not_synthesize_contract_component_scores_or_quality() {
    let tof_metrics = sample_tof_metrics();
    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 72,
        short_tof_score: 76.0,
        short_direction: TofDirection::Bullish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 20.0,
        detector_confidence: 70.0,
        direction_confidence: 70.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(96),
        perp_score: Some(95),
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: build_cwm_risk_contribution("BTC-PERP", None),
    });

    assert_eq!(systems.main_force_structure.cwm_aggressive_flow, 0);
    assert_eq!(systems.main_force_structure.oi_impulse, 0);
    assert_eq!(systems.main_force_structure.liquidation_context, 0);
    assert_eq!(systems.main_force_structure.funding_crowding, 0);
    assert_eq!(systems.main_force_structure.basis_premium, 0);
    assert_eq!(systems.main_force_structure.active_exchange_confirmation, 0);
    assert_eq!(systems.main_force_structure.duration_score, 0);
    assert_eq!(systems.main_force_structure.contract_score, 0);
    assert!(systems.main_force_structure.data_quality <= 20.0);
}

#[test]
fn candidate_bound_cwm_rejects_signal_beyond_future_clock_skew() {
    let mut signal = sample_cwm_signal();
    let candidate_at_ms = signal.ts;
    signal.ts = candidate_at_ms + 5_001;

    let contribution = build_cwm_risk_contribution_for_candidate(
        "BTC-PERP",
        Some(&signal),
        candidate_at_ms,
        signal.ts,
    );

    assert!(!contribution.available);
    assert!(!contribution.fresh);
    assert_eq!(contribution.observed_at_ms, Some(signal.ts));
    assert_eq!(
        contribution.unavailable_reason.as_deref(),
        Some("candidate_time_mismatch")
    );
    assert_eq!(contribution.score, None);
    assert!(contribution.summary.contains("candidate time"));
}

#[test]
fn candidate_bound_cwm_rejects_signal_after_ttl_expires() {
    let signal = sample_cwm_signal();

    let contribution = build_cwm_risk_contribution_for_candidate(
        "BTC-PERP",
        Some(&signal),
        signal.ts,
        signal.ts + 120_001,
    );

    assert!(!contribution.available);
    assert!(!contribution.fresh);
    assert_eq!(contribution.observed_at_ms, Some(signal.ts));
    assert_eq!(
        contribution.unavailable_reason.as_deref(),
        Some("ttl_expired")
    );
    assert_eq!(contribution.score, None);
    assert!(contribution.summary.contains("TTL"));
}

#[test]
fn candidate_bound_cwm_rejects_symbol_mismatch() {
    let mut signal = sample_cwm_signal();
    signal.symbol = "ETHUSDT".to_string();

    let contribution =
        build_cwm_risk_contribution_for_candidate("BTC-PERP", Some(&signal), signal.ts, signal.ts);

    assert!(!contribution.available);
    assert!(!contribution.fresh);
    assert_eq!(contribution.observed_at_ms, Some(signal.ts));
    assert_eq!(
        contribution.unavailable_reason.as_deref(),
        Some("symbol_mismatch")
    );
    assert!(contribution.summary.contains("symbol"));
}

#[test]
fn candidate_bound_cwm_accepts_fresh_signal_at_candidate_time() {
    let signal = sample_cwm_signal();

    let contribution = build_cwm_risk_contribution_for_candidate(
        "BTC-PERP",
        Some(&signal),
        signal.ts,
        signal.ts + 5_000,
    );

    let json = serde_json::to_value(&contribution).expect("candidate contribution json");

    assert!(contribution.available);
    assert!(contribution.fresh);
    assert_eq!(contribution.observed_at_ms, Some(signal.ts));
    assert_eq!(contribution.unavailable_reason, None);
    assert_eq!(contribution.signal_id.as_deref(), Some(signal.id.as_str()));
    assert_eq!(json["observedAtMs"], signal.ts);
    assert_eq!(json["fresh"], true);
    assert_eq!(json["unavailableReason"], serde_json::Value::Null);
}

#[test]
fn cwm_contribution_missing_signal_keeps_independent_gate_visible() {
    let contribution = build_cwm_risk_contribution("ETH-PERP", None);

    assert!(!contribution.available);
    assert_eq!(contribution.score, None);
    assert_eq!(contribution.weighted_contribution, 0.0);
    assert!(contribution.discord_gate_independent);
    assert!(contribution
        .summary
        .contains("main-force structure uses spot/perp context only"));
}

#[test]
fn market_structure_marks_liquidation_cascade_without_treating_it_as_main_force_build() {
    let mut signal = sample_cwm_signal();
    signal.signal_type = ContractWhaleSignalType::AggressiveSell;
    signal.direction = ContractWhaleDirection::Sell;
    signal.score = 92;
    signal.price_move_pct = Some(-0.24);
    signal.liquidation_suspected = true;
    signal.liquidation_ratio = Some(0.74);
    signal.liquidation_long_btc = 1_620.0;
    signal.liquidation_short_btc = 140.0;
    signal.oi_change_pct = Some(-1.5);
    signal.classification_v2.evidence.liquidation_status = "live".to_string();
    signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Available(-1.5);
    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let tof_metrics = weak_spot_tof_metrics();

    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 88,
        short_tof_score: 84.0,
        short_direction: TofDirection::Bearish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 82.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(91),
        perp_score: Some(90),
        metrics_direction: TofDirection::Bearish,
        cwm_contribution: contribution,
    });

    assert_eq!(
        systems.main_force_structure.regime_type,
        "long_liquidation_cascade"
    );
    assert!(systems.main_force_structure.extreme_impact_score >= 92);
    assert!(systems.main_force_structure.extreme_impact_confirmed);
    assert!(!systems.main_force_structure.main_force_confirmed);
    assert!(
        systems.main_force_structure.main_force_score <= 64,
        "liquidation-driven extremes should raise extremeImpactScore and actively cap mainForceScore"
    );
    assert!(systems.main_force_structure.reasons.iter().any(|reason| {
        reason.reason_type == "LiquidationContext"
            && reason
                .description
                .contains("not automatically main-force builds")
    }));
}

#[test]
fn contract_only_dislocation_without_live_liquidation_stays_fail_closed() {
    let mut signal = sample_cwm_signal();
    signal.signal_type = ContractWhaleSignalType::AggressiveSell;
    signal.direction = ContractWhaleDirection::Sell;
    signal.score = 95;
    signal.oi_change_pct = Some(-0.3);
    signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Available(-0.3);
    signal.price_move_pct = Some(0.02);
    signal.multi_exchange_confirmed = false;
    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let tof_metrics = dislocated_spot_tof_metrics();

    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 71,
        short_tof_score: 39.0,
        short_direction: TofDirection::Bullish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 84.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(84),
        perp_score: Some(92),
        metrics_direction: TofDirection::Neutral,
        cwm_contribution: contribution,
    });

    assert_eq!(systems.main_force_structure.regime_type, "unclear");
    assert!(!systems.main_force_structure.main_force_confirmed);
    assert!(
        systems.main_force_structure.main_force_score <= 68,
        "contract-only surges should stay below main-force confirmation territory"
    );
    assert!(systems.main_force_structure.contract_score >= 65);
    assert!(systems.main_force_structure.spot_score < 60);
    assert!(
        systems
            .main_force_structure
            .spot_contract_direction_consistency
            < 65
    );
    assert!(systems.main_force_structure.price_response_consistency < 70);
    assert!(systems.main_force_structure.signal_agreement < 70);
}

#[test]
fn market_structure_marks_short_squeeze_as_extreme_impact_without_main_force_confirmation() {
    let mut signal = sample_cwm_signal();
    signal.signal_type = ContractWhaleSignalType::AggressiveBuy;
    signal.direction = ContractWhaleDirection::Buy;
    signal.score = 96;
    signal.price_move_pct = Some(0.28);
    signal.liquidation_suspected = true;
    signal.liquidation_ratio = Some(0.68);
    signal.liquidation_long_btc = 120.0;
    signal.liquidation_short_btc = 1_840.0;
    signal.oi_change_pct = Some(-1.8);
    signal.classification_v2.evidence.liquidation_status = "live".to_string();
    signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Available(-1.8);
    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let tof_metrics = weak_spot_tof_metrics();

    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 84,
        short_tof_score: 72.0,
        short_direction: TofDirection::Bullish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 84.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(93),
        perp_score: Some(95),
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: contribution,
    });

    assert_eq!(
        systems.main_force_structure.regime_type,
        "contract_short_squeeze"
    );
    assert!(systems.main_force_structure.extreme_impact_confirmed);
    assert!(!systems.main_force_structure.main_force_confirmed);
    assert!(systems.main_force_structure.extreme_impact_score >= 95);
    assert!(systems.main_force_structure.reasons.iter().any(|reason| {
        reason.reason_type == "LiquidationContext"
            && reason
                .description
                .contains("not automatically main-force builds")
    }));
}

#[test]
fn downside_absorption_keeps_a_slightly_bullish_structure_bias() {
    let mut signal = sample_cwm_signal();
    signal.signal_type = ContractWhaleSignalType::DownsideAbsorption;
    signal.direction = ContractWhaleDirection::Absorption;
    signal.score = 90;
    signal.price_move_pct = Some(-0.01);
    signal.oi_change_pct = Some(0.4);
    signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Available(0.4);
    signal.final_result = "主动卖出很大，但价格跌不动，下方承接明显".to_string();
    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let tof_metrics = absorption_spot_tof_metrics();

    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 77,
        short_tof_score: 74.0,
        short_direction: TofDirection::Bearish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 88.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(86),
        perp_score: Some(88),
        metrics_direction: TofDirection::Bearish,
        cwm_contribution: contribution,
    });

    assert_eq!(
        systems.main_force_structure.regime_type,
        "downside_absorption"
    );
    assert!(
        systems.main_force_structure.structure_bias > 0,
        "downside absorption should keep a slightly bullish structure bias instead of falling back to neutral/bearish"
    );
    assert!(
        systems.main_force_structure.main_force_score >= 70,
        "clear absorption can still be a strong structure event even before full trend expansion"
    );
}

#[test]
fn market_structure_requires_spot_and_contract_confirmation_for_major_main_force_score() {
    let mut signal = sample_cwm_signal();
    signal.score = 95;
    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let tof_metrics = weak_spot_tof_metrics();

    let systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 55,
        short_tof_score: 25.0,
        short_direction: TofDirection::Neutral,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 88.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(95),
        perp_score: Some(95),
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: contribution,
    });

    assert!(systems.main_force_structure.spot_score <= 30);
    assert!(systems.main_force_structure.contract_score >= 70);
    assert_eq!(systems.main_force_structure.cwm_aggressive_flow, 95);
    assert_eq!(
        systems.main_force_structure.active_exchange_confirmation,
        92
    );
    assert!(systems.main_force_structure.cross_confirm_score >= 92);
    assert_eq!(
        systems.main_force_structure.spot_contract_floor,
        systems.main_force_structure.spot_score
    );
    assert!(!systems.main_force_structure.main_force_confirmed);
    assert!(
        systems.main_force_structure.main_force_score < 75,
        "contract-only strength must not become Major without spot confirmation"
    );
    assert_ne!(systems.main_force_structure.severity, "Major");
    assert_ne!(systems.main_force_structure.severity, "Extreme");
}

#[test]
fn contract_structure_caps_single_platform_cwm_strength_by_active_exchange_role() {
    let tof_metrics = sample_tof_metrics();

    let mut binance_only = sample_cwm_signal();
    binance_only.score = 96;
    binance_only.multi_exchange_confirmed = false;
    binance_only.main_exchange = Some("binance".to_string());
    let binance_systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 70,
        short_tof_score: 78.0,
        short_direction: TofDirection::Bullish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 90.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(95),
        perp_score: Some(95),
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: build_cwm_risk_contribution("BTC-PERP", Some(&binance_only)),
    });

    assert_eq!(binance_systems.main_force_structure.cwm_aggressive_flow, 89);
    assert_eq!(
        binance_systems
            .main_force_structure
            .active_exchange_confirmation,
        70
    );
    assert!(
        binance_systems.main_force_structure.contract_score < 90,
        "Binance-only extremes can be High/Critical evidence, but should not create S-like contract evidence"
    );

    let mut bitfinex_only = sample_cwm_signal();
    bitfinex_only.score = 96;
    bitfinex_only.multi_exchange_confirmed = false;
    bitfinex_only.main_exchange = Some("bitfinex".to_string());
    let bitfinex_systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: 1_700_000_000_000,
        symbol: "BTC-PERP",
        short_toxic_score: 70,
        short_tof_score: 78.0,
        short_direction: TofDirection::Bullish,
        toxic_type: "WhaleFlowCandidate",
        data_quality: 90.0,
        detector_confidence: 82.0,
        direction_confidence: 82.0,
        direction_source: "detector",
        tof_metrics: &tof_metrics,
        advanced_score: Some(95),
        perp_score: Some(95),
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: build_cwm_risk_contribution("BTC-PERP", Some(&bitfinex_only)),
    });

    assert_eq!(
        bitfinex_systems.main_force_structure.cwm_aggressive_flow,
        74
    );
    assert_eq!(
        bitfinex_systems
            .main_force_structure
            .active_exchange_confirmation,
        55
    );
    assert!(
        bitfinex_systems.main_force_structure.contract_score
            < binance_systems.main_force_structure.contract_score,
        "Bitfinex-only flow is treated as confirmation-source evidence, not an S-grade primary signal"
    );
}

fn weak_spot_tof_metrics() -> TofMetrics {
    TofMetrics {
        trade_imbalance: 0.03,
        trade_imbalance_score: 18.0,
        vpin_proxy: 20.0,
        vpin_bucket_count: 1,
        vpin_window_volume: 500.0,
        bid_depth_withdrawal: 10.0,
        ask_depth_withdrawal: 8.0,
        depth_withdrawal_score: 12.0,
        spread_bps: 1.2,
        spread_widening_score: 10.0,
        order_churn_score: 12.0,
        book_update_rate: 14.0,
        trade_rate: 10.0,
        liquidity_vacuum_score: 8.0,
        thin_side: "none".to_string(),
        metrics_direction: TofDirection::Neutral,
        metrics_confidence: 20.0,
        tof_score: 25.0,
        final_risk_score: 25,
        metrics_completeness: 80.0,
        vpin_zscore: Some(0.1),
        vpin_percentile: Some(0.55),
        per_venue_vpin: Default::default(),
        lineage: observed_tof_lineage(),
        metric_lineage: observed_metric_lineage(),
    }
}

fn dislocated_spot_tof_metrics() -> TofMetrics {
    TofMetrics {
        trade_imbalance: -0.08,
        trade_imbalance_score: 24.0,
        vpin_proxy: 28.0,
        vpin_bucket_count: 1,
        vpin_window_volume: 900.0,
        bid_depth_withdrawal: 12.0,
        ask_depth_withdrawal: 11.0,
        depth_withdrawal_score: 18.0,
        spread_bps: 1.4,
        spread_widening_score: 14.0,
        order_churn_score: 16.0,
        book_update_rate: 15.0,
        trade_rate: 18.0,
        liquidity_vacuum_score: 10.0,
        thin_side: "none".to_string(),
        metrics_direction: TofDirection::Neutral,
        metrics_confidence: 18.0,
        tof_score: 22.0,
        final_risk_score: 22,
        metrics_completeness: 82.0,
        vpin_zscore: Some(0.2),
        vpin_percentile: Some(0.60),
        per_venue_vpin: Default::default(),
        lineage: observed_tof_lineage(),
        metric_lineage: observed_metric_lineage(),
    }
}

fn absorption_spot_tof_metrics() -> TofMetrics {
    TofMetrics {
        trade_imbalance: -0.32,
        trade_imbalance_score: 68.0,
        vpin_proxy: 54.0,
        vpin_bucket_count: 4,
        vpin_window_volume: 6_500.0,
        bid_depth_withdrawal: 18.0,
        ask_depth_withdrawal: 42.0,
        depth_withdrawal_score: 58.0,
        spread_bps: 3.6,
        spread_widening_score: 42.0,
        order_churn_score: 49.0,
        book_update_rate: 39.0,
        trade_rate: 46.0,
        liquidity_vacuum_score: 44.0,
        thin_side: "ask".to_string(),
        metrics_direction: TofDirection::Bearish,
        metrics_confidence: 72.0,
        tof_score: 74.0,
        final_risk_score: 74,
        metrics_completeness: 90.0,
        vpin_zscore: Some(1.8),
        vpin_percentile: Some(0.92),
        per_venue_vpin: Default::default(),
        lineage: observed_tof_lineage(),
        metric_lineage: observed_metric_lineage(),
    }
}

fn observed_tof_lineage() -> MetricLineage {
    MetricLineage::calculated("test_observed_market_data", 1_700_000_000_000, true)
}

fn observed_metric_lineage() -> std::collections::BTreeMap<String, MetricLineage> {
    [
        "tradeImbalance",
        "tradeRate",
        "vpin",
        "depth",
        "spread",
        "bookUpdateRate",
        "liquidityVacuum",
        "sweep",
        "hazard",
    ]
    .into_iter()
    .map(|key| (key.to_string(), observed_tof_lineage()))
    .collect()
}

fn sample_cwm_signal() -> ContractWhaleSignal {
    let mut signal = ContractWhaleSignal {
        id: "contract-whale:BTC:15:1700000000000:buy".to_string(),
        ts: 1_700_000_000_000,
        symbol: "BTC".to_string(),
        window_sec: 15,
        signal_type: ContractWhaleSignalType::AggressiveBuy,
        direction: ContractWhaleDirection::Buy,
        severity: ContractWhaleSeverity::S,
        score: 94,
        main_force_score: Some(86),
        spot_score: Some(78),
        contract_score: Some(94),
        base_asset: "BTC".to_string(),
        quantity_unit: "BTC".to_string(),
        total_volume: 4_820.0,
        net_volume: 3_260.0,
        total_volume_btc: 4_820.0,
        net_volume_btc: 3_260.0,
        total_notional_usd: 337_000_000.0,
        dominance: 0.676,
        order_price_usd: Some(337_000_000.0 / 4_820.0),
        current_market_price_usd: Some(337_000_000.0 / 4_820.0),
        price_deviation_pct: Some(0.0),
        price_deviation_filtered: false,
        price_move_pct: Some(0.31),
        price_move_5s_pct: None,
        price_move_15s_pct: Some(0.31),
        price_move_30s_pct: None,
        price_response_type: ContractWhalePriceResponseType::TrendFollowUp,
        classification_v2: Default::default(),
        main_exchange: Some("binance".to_string()),
        market_type: ContractWhaleMarketType::Perp,
        source_role: ContractWhaleSourceRole::Primary,
        exchanges: Vec::new(),
        dominant_venue_net_contribution_share: Some(0.72),
        dynamic_multiple: Some(9.4),
        dynamic_baseline_btc: Some(512.0),
        dynamic_threshold_level: "critical".to_string(),
        percentile_level: Some(99.9),
        impact_level: None,
        signal_level: None,
        signal_label: None,
        normalized_strength: None,
        impact_score: None,
        impact_z_score: None,
        multi_exchange_confirmed: true,
        liquidation_suspected: false,
        liquidation_long_btc: 0.0,
        liquidation_short_btc: 0.0,
        liquidation_notional_usd: 0.0,
        liquidation_ratio: None,
        price_reversal_ratio: None,
        oi_change_1m_btc: None,
        oi_change_5m_btc: None,
        oi_change_pct: Some(1.4),
        oi_bias: Some("long_increase".to_string()),
        funding_rate: Some(0.00076),
        funding_bias: Some("long".to_string()),
        data_quality: 91,
        score_breakdown: ContractWhaleScoreBreakdown::default(),
        threshold_profile: "binance_bitfinex".to_string(),
        threshold_profile_reason: "active_contract_sources=binance,bitfinex".to_string(),
        configured_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
        eligible_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
        active_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
        active_sources: ContractWhaleActiveSources::default(),
        spot_confirmation: ContractWhaleSpotConfirmationContext::default(),
        discord_eligible: true,
        discord_sent: false,
        discord_sent_at: None,
        discord_reason: "dry_run".to_string(),
        discord_would_send: true,
        final_result: "多平台主动买入爆发，疑似主力合约拉盘".to_string(),
        cluster: Default::default(),
        persistence: Default::default(),
        whale_action: Default::default(),
        trajectory: Default::default(),
        liquidation_force: ContractWhaleLiquidationForce::default(),
        market_driver: Default::default(),
        event_lifecycle: Default::default(),
        event_quality: Default::default(),
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        merged_from: Vec::new(),
    };
    signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Available(1.4);
    signal.classification_v2.evidence.funding = ContractWhaleEvidenceState::Available(0.00076);
    signal.classification_v2.evidence.liquidation_status = "unavailable".to_string();
    signal.classification_v2.evidence.liquidation_reason =
        Some("no_live_liquidation_samples".to_string());
    signal
}
