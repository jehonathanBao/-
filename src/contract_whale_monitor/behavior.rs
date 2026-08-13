//! Evidence-first contract-whale behavior assessment.
//!
//! This layer is intentionally separate from impact grading. A large print is
//! an impact observation; it is not, by itself, evidence of accumulation or
//! distribution. The assessment fails closed when OI/price evidence is absent
//! or degraded.

use serde::{Deserialize, Serialize};

use super::types::{
    ContractWhaleOiContextTag, ContractWhalePriceResponseType, ContractWhaleSignalType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorState {
    Insufficient,
    Provisional,
    Confirmed,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorType {
    NewLongBuild,
    NewShortBuild,
    ShortCovering,
    LongUnwind,
    DownsideAbsorption,
    UpsideSuppression,
    LiquidationSweep,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorAssessmentInput {
    pub signal_type: ContractWhaleSignalType,
    pub oi_context: ContractWhaleOiContextTag,
    pub oi_available: bool,
    pub oi_evidence_degraded: bool,
    pub price_move_pct: Option<f64>,
    pub price_response_type: ContractWhalePriceResponseType,
    pub multi_exchange_confirmed: bool,
    pub data_quality: u8,
    pub dominance: f64,
    pub liquidation_suspected: bool,
    pub liquidation_total_btc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorAssessment {
    pub behavior_type: BehaviorType,
    pub state: BehaviorState,
    pub confidence: u8,
    pub main_force_confirmed: bool,
    pub supporting_evidence: Vec<String>,
    pub counter_evidence: Vec<String>,
    pub rationale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalidation_reason: Option<String>,
    #[serde(default = "default_behavior_assessment_version")]
    pub assessment_version: String,
}

fn default_behavior_assessment_version() -> String {
    "behavior-v1".to_string()
}

impl Default for BehaviorAssessment {
    fn default() -> Self {
        Self {
            behavior_type: BehaviorType::InsufficientEvidence,
            state: BehaviorState::Insufficient,
            confidence: 0,
            main_force_confirmed: false,
            supporting_evidence: Vec::new(),
            counter_evidence: vec!["证据不足，不能确认主力行为".to_string()],
            rationale: "仅观察到成交流，未形成可确认的主力行为证据链。".to_string(),
            invalidation_reason: None,
            assessment_version: default_behavior_assessment_version(),
        }
    }
}

pub fn assess_contract_whale_behavior(input: &BehaviorAssessmentInput) -> BehaviorAssessment {
    let mut assessment = BehaviorAssessment::default();

    if input.liquidation_suspected {
        assessment.behavior_type = BehaviorType::LiquidationSweep;
        assessment.state = BehaviorState::Provisional;
        assessment.confidence = 35;
        assessment
            .supporting_evidence
            .push("清算证据存在".to_string());
        assessment
            .counter_evidence
            .push("清算驱动事件进入市场影响通道，不直接认定为主力主动建仓或派发".to_string());
        assessment.rationale = "价格行为可能由强平推动，暂不确认主力主动行为。".to_string();
        return assessment;
    }

    if input.oi_evidence_degraded || !input.oi_available {
        assessment
            .counter_evidence
            .push(if input.oi_evidence_degraded {
                "OI 证据降级".to_string()
            } else {
                "OI 证据不可用".to_string()
            });
        return assessment;
    }

    if input.data_quality < 70 || input.dominance < 0.55 || !input.multi_exchange_confirmed {
        assessment
            .counter_evidence
            .push("数据质量、方向占比或跨市场确认不足".to_string());
        return assessment;
    }

    let price_follow = matches!(
        input.price_response_type,
        ContractWhalePriceResponseType::TrendFollowUp
            | ContractWhalePriceResponseType::TrendFollowDown
    ) && input
        .price_move_pct
        .is_some_and(|value| value.abs() >= 0.12);

    match input.oi_context {
        ContractWhaleOiContextTag::NewLongBuild
            if input.signal_type == ContractWhaleSignalType::AggressiveBuy && price_follow =>
        {
            confirmed(
                &mut assessment,
                BehaviorType::NewLongBuild,
                "主动买入 + OI 上升 + 价格跟随，确认新多头建仓概率较高。",
            );
        }
        ContractWhaleOiContextTag::NewShortBuild
            if input.signal_type == ContractWhaleSignalType::AggressiveSell && price_follow =>
        {
            confirmed(
                &mut assessment,
                BehaviorType::NewShortBuild,
                "主动卖出 + OI 上升 + 价格跟随，确认新空头建仓概率较高。",
            );
        }
        ContractWhaleOiContextTag::ShortCovering
            if input.signal_type == ContractWhaleSignalType::AggressiveBuy && price_follow =>
        {
            provisional(
                &mut assessment,
                BehaviorType::ShortCovering,
                "主动买入伴随 OI 下降，更像空头回补而非主力吸筹。",
                "OI 下降反对新多头建仓".to_string(),
            );
        }
        ContractWhaleOiContextTag::LongUnwind
            if input.signal_type == ContractWhaleSignalType::AggressiveSell && price_follow =>
        {
            provisional(
                &mut assessment,
                BehaviorType::LongUnwind,
                "主动卖出伴随 OI 下降，更像多头平仓而非主力派发。",
                "OI 下降反对新空头建仓".to_string(),
            );
        }
        ContractWhaleOiContextTag::NewShortBuild
            if input.signal_type == ContractWhaleSignalType::DownsideAbsorption
                && !price_follow =>
        {
            provisional(
                &mut assessment,
                BehaviorType::DownsideAbsorption,
                "卖压被承接且价格未有效下行，形成下方吸收候选。",
                "仍需更长窗口和现货承接证据".to_string(),
            );
        }
        ContractWhaleOiContextTag::NewLongBuild
            if input.signal_type == ContractWhaleSignalType::UpsideSuppression && !price_follow =>
        {
            provisional(
                &mut assessment,
                BehaviorType::UpsideSuppression,
                "买压未能推动价格上行，形成上方压制候选。",
                "仍需更长窗口和现货阻力证据".to_string(),
            );
        }
        _ => {
            assessment
                .counter_evidence
                .push("OI 方向与成交行为不构成确认矩阵".to_string());
        }
    }

    assessment
}

/// Returns whether the behavior lane has enough evidence for a main-force
/// notification. Market-impact and liquidation alerts intentionally use a
/// separate gate and must not call this helper as a proxy for impact.
pub fn is_behavior_alert_eligible(assessment: &BehaviorAssessment) -> bool {
    assessment.state == BehaviorState::Confirmed
        && assessment.main_force_confirmed
        && assessment.confidence >= 80
        && !matches!(
            assessment.behavior_type,
            BehaviorType::InsufficientEvidence | BehaviorType::LiquidationSweep
        )
}

/// Reconciles a fresh observation with the last persisted assessment. A
/// previously confirmed behavior cannot silently become an ordinary event;
/// it must carry an explicit invalidation state for the operator and audit log.
pub fn transition_behavior_assessment(
    previous: &BehaviorAssessment,
    mut next: BehaviorAssessment,
) -> BehaviorAssessment {
    if previous.state == BehaviorState::Confirmed && !matches!(next.state, BehaviorState::Confirmed)
    {
        next.state = BehaviorState::Invalidated;
        next.main_force_confirmed = false;
        next.invalidation_reason =
            Some("后续 OI、价格或数据质量证据不足，原确认已失效".to_string());
        next.counter_evidence
            .push("确认后的后续证据未能维持原行为结论".to_string());
        next.rationale = "原主力行为确认已被后续证据撤销，回到只读观察。".to_string();
    }
    next
}

pub fn behavior_input_from_signal(
    signal: &super::types::ContractWhaleSignal,
) -> BehaviorAssessmentInput {
    BehaviorAssessmentInput {
        signal_type: signal.signal_type,
        oi_context: signal.classification_v2.oi_context,
        oi_available: signal.classification_v2.oi_available,
        oi_evidence_degraded: signal.classification_v2.oi_evidence_degraded,
        price_move_pct: signal.price_move_pct,
        price_response_type: signal.price_response_type,
        multi_exchange_confirmed: signal.multi_exchange_confirmed,
        data_quality: signal.data_quality,
        dominance: signal.dominance,
        liquidation_suspected: signal.liquidation_suspected,
        liquidation_total_btc: signal.liquidation_long_btc + signal.liquidation_short_btc,
    }
}

fn confirmed(assessment: &mut BehaviorAssessment, behavior_type: BehaviorType, rationale: &str) {
    assessment.behavior_type = behavior_type;
    assessment.state = BehaviorState::Confirmed;
    assessment.confidence = 86;
    assessment.main_force_confirmed = true;
    assessment.supporting_evidence.extend([
        "OI 方向一致".to_string(),
        "价格响应明确".to_string(),
        "跨市场方向确认".to_string(),
    ]);
    assessment.counter_evidence.clear();
    assessment.rationale = rationale.to_string();
}

fn provisional(
    assessment: &mut BehaviorAssessment,
    behavior_type: BehaviorType,
    rationale: &str,
    counter_evidence: String,
) {
    assessment.behavior_type = behavior_type;
    assessment.state = BehaviorState::Provisional;
    assessment.confidence = 62;
    assessment.main_force_confirmed = false;
    assessment
        .supporting_evidence
        .push("方向与 OI 初步匹配".to_string());
    assessment.counter_evidence.push(counter_evidence);
    assessment.rationale = rationale.to_string();
}
