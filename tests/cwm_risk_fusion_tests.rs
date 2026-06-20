use btc_toxic_flow_monitor_rs::{
    contract_whale_monitor::types::{
        ContractWhaleActiveSources, ContractWhaleDirection, ContractWhaleLiquidationForce,
        ContractWhaleMarketType, ContractWhalePriceResponseType, ContractWhaleScoreBreakdown,
        ContractWhaleSeverity, ContractWhaleSignal, ContractWhaleSignalType,
        ContractWhaleSourceRole, ContractWhaleSpotConfirmationContext,
    },
    runtime::{
        cwm_risk_fusion::{
            build_cwm_risk_contribution, build_split_risk_systems, decayed_toxic_score,
            SplitRiskSystemsInput,
        },
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
        tof_metrics: &tof_metrics,
        advanced_score: 89,
        perp_score: 87,
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: contribution,
    });

    assert_eq!(systems.short_term_toxic.toxic_score, 84);
    assert_eq!(systems.short_term_toxic.short_pressure, -84);
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
    assert_eq!(systems.short_term_toxic.reasons.len(), 7);
    assert_eq!(
        systems.short_term_toxic.reasons[0].reason_type,
        "ToxicOrderCluster"
    );
    assert_eq!(systems.short_term_toxic.reasons[0].weight, 0.25);
    assert_eq!(systems.short_term_toxic.reasons[0].window_sec, 5);
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
    assert_eq!(systems.main_force_structure.main_force_score, 83);
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
    assert_eq!(systems.main_force_structure.structure_bias, 72);
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
    assert_eq!(systems.main_force_structure.spot_score, 75);
    assert_eq!(systems.main_force_structure.spot_cvd_score, 84);
    assert_eq!(systems.main_force_structure.spot_volume_anomaly, 72);
    assert_eq!(systems.main_force_structure.spot_absorption, 64);
    assert_eq!(systems.main_force_structure.spot_liquidity_shift, 73);
    assert_eq!(systems.main_force_structure.spot_price_response, 85);
    assert_eq!(systems.main_force_structure.contract_score, 86);
    assert_eq!(systems.main_force_structure.cwm_aggressive_flow, 94);
    assert_eq!(systems.main_force_structure.oi_impulse, 71);
    assert_eq!(systems.main_force_structure.liquidation_context, 93);
    assert_eq!(systems.main_force_structure.funding_crowding, 88);
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
    assert_eq!(systems.main_force_structure.structure_raw, 83.4);
    assert_eq!(systems.main_force_structure.spot_contract_floor, 75);
    assert_eq!(systems.main_force_structure.duration_score, 100);
    assert_eq!(systems.main_force_structure.liquidation_penalty, 0.0);
    assert_eq!(systems.main_force_structure.crowding_penalty, 0.0);
    assert_eq!(systems.main_force_structure.oi_score, 71);
    assert_eq!(systems.main_force_structure.liquidation_score, 93);
    assert_eq!(systems.main_force_structure.funding_crowding_score, 88);
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
        tof_metrics: &tof_metrics,
        advanced_score: 80,
        perp_score: 80,
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
    }
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
    assert!(!json.to_ascii_lowercase().contains("evidence"));
    assert!(!json.to_ascii_lowercase().contains("markout"));
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
        tof_metrics: &tof_metrics,
        advanced_score: 91,
        perp_score: 90,
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
    assert!(systems
        .main_force_structure
        .reasons
        .iter()
        .any(|reason| reason.reason_type == "LiquidationContext"
            && reason
                .description
                .contains("not automatically main-force builds")));
}

#[test]
fn market_structure_labels_contract_only_dislocation_as_contract_flow_shock() {
    let mut signal = sample_cwm_signal();
    signal.signal_type = ContractWhaleSignalType::AggressiveSell;
    signal.direction = ContractWhaleDirection::Sell;
    signal.score = 95;
    signal.oi_change_pct = Some(-0.3);
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
        tof_metrics: &tof_metrics,
        advanced_score: 84,
        perp_score: 92,
        metrics_direction: TofDirection::Neutral,
        cwm_contribution: contribution,
    });

    assert_eq!(
        systems.main_force_structure.regime_type,
        "contract_flow_shock"
    );
    assert!(!systems.main_force_structure.main_force_confirmed);
    assert!(
        systems.main_force_structure.main_force_score <= 68,
        "contract-only surges should stay below main-force confirmation territory"
    );
    assert!(systems.main_force_structure.contract_score >= 78);
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
        tof_metrics: &tof_metrics,
        advanced_score: 93,
        perp_score: 95,
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
    assert!(systems
        .main_force_structure
        .reasons
        .iter()
        .any(|reason| reason.reason_type == "LiquidationContext"
            && reason
                .description
                .contains("not automatically main-force builds")));
}

#[test]
fn downside_absorption_keeps_a_slightly_bullish_structure_bias() {
    let mut signal = sample_cwm_signal();
    signal.signal_type = ContractWhaleSignalType::DownsideAbsorption;
    signal.direction = ContractWhaleDirection::Absorption;
    signal.score = 90;
    signal.price_move_pct = Some(-0.01);
    signal.oi_change_pct = Some(0.4);
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
        tof_metrics: &tof_metrics,
        advanced_score: 86,
        perp_score: 88,
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
        tof_metrics: &tof_metrics,
        advanced_score: 95,
        perp_score: 95,
        metrics_direction: TofDirection::Bullish,
        cwm_contribution: contribution,
    });

    assert!(systems.main_force_structure.spot_score <= 30);
    assert!(systems.main_force_structure.contract_score >= 85);
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
        tof_metrics: &tof_metrics,
        advanced_score: 95,
        perp_score: 95,
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
        tof_metrics: &tof_metrics,
        advanced_score: 95,
        perp_score: 95,
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
    }
}

fn sample_cwm_signal() -> ContractWhaleSignal {
    ContractWhaleSignal {
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
        main_exchange: Some("binance".to_string()),
        market_type: ContractWhaleMarketType::Perp,
        source_role: ContractWhaleSourceRole::Primary,
        exchanges: Vec::new(),
        dominant_venue_net_contribution_share: Some(0.72),
        dynamic_multiple: Some(9.4),
        dynamic_baseline_btc: Some(512.0),
        dynamic_threshold_level: "critical".to_string(),
        percentile_level: Some(99.9),
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
        funding_rate: None,
        funding_bias: None,
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
    }
}
