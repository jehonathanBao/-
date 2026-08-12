use btc_toxic_flow_monitor_rs::contract_whale_monitor::behavior::{
    assess_contract_whale_behavior, BehaviorAssessmentInput, BehaviorState, BehaviorType,
};
use btc_toxic_flow_monitor_rs::contract_whale_monitor::types::{
    ContractWhaleOiContextTag, ContractWhalePriceResponseType, ContractWhaleSignalType,
};

fn input(signal_type: ContractWhaleSignalType) -> BehaviorAssessmentInput {
    BehaviorAssessmentInput {
        signal_type,
        oi_context: ContractWhaleOiContextTag::OiUnavailable,
        oi_available: false,
        oi_evidence_degraded: false,
        price_move_pct: Some(0.0),
        price_response_type: ContractWhalePriceResponseType::NoClearResponse,
        multi_exchange_confirmed: true,
        data_quality: 90,
        dominance: 0.75,
        liquidation_suspected: false,
        liquidation_total_btc: 0.0,
    }
}

#[test]
fn ordinary_directional_volume_without_oi_stays_insufficient() {
    let assessment = assess_contract_whale_behavior(&input(ContractWhaleSignalType::AggressiveBuy));

    assert_eq!(assessment.behavior_type, BehaviorType::InsufficientEvidence);
    assert_eq!(assessment.state, BehaviorState::Insufficient);
    assert!(!assessment.main_force_confirmed);
}

#[test]
fn new_long_build_requires_oi_and_price_follow_through() {
    let mut candidate = input(ContractWhaleSignalType::AggressiveBuy);
    candidate.oi_context = ContractWhaleOiContextTag::NewLongBuild;
    candidate.oi_available = true;
    candidate.price_move_pct = Some(0.24);
    candidate.price_response_type = ContractWhalePriceResponseType::TrendFollowUp;

    let assessment = assess_contract_whale_behavior(&candidate);

    assert_eq!(assessment.behavior_type, BehaviorType::NewLongBuild);
    assert_eq!(assessment.state, BehaviorState::Confirmed);
    assert!(assessment.main_force_confirmed);
}

#[test]
fn short_covering_is_not_accumulation() {
    let mut candidate = input(ContractWhaleSignalType::AggressiveBuy);
    candidate.oi_context = ContractWhaleOiContextTag::ShortCovering;
    candidate.oi_available = true;
    candidate.price_move_pct = Some(0.24);
    candidate.price_response_type = ContractWhalePriceResponseType::TrendFollowUp;

    let assessment = assess_contract_whale_behavior(&candidate);

    assert_eq!(assessment.behavior_type, BehaviorType::ShortCovering);
    assert_eq!(assessment.state, BehaviorState::Provisional);
    assert!(!assessment.main_force_confirmed);
    assert!(assessment
        .counter_evidence
        .iter()
        .any(|item| item.contains("OI")));
}

#[test]
fn liquidation_sweep_stays_in_impact_lane() {
    let mut candidate = input(ContractWhaleSignalType::AggressiveSell);
    candidate.liquidation_suspected = true;
    candidate.liquidation_total_btc = 1_200.0;
    candidate.oi_available = true;
    candidate.oi_context = ContractWhaleOiContextTag::NewShortBuild;

    let assessment = assess_contract_whale_behavior(&candidate);

    assert_eq!(assessment.behavior_type, BehaviorType::LiquidationSweep);
    assert!(!assessment.main_force_confirmed);
    assert!(assessment.state != BehaviorState::Confirmed);
}
