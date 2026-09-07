//! Versioned, fail-closed impact grading for contract-whale episodes.
//!
//! The grade is deliberately based on absolute evidence as well as robust
//! relative statistics.  A page cohort or a one-off relative burst can never
//! promote an event to S without liquidation or extraordinary unique-turnover
//! evidence.

use serde::{Deserialize, Serialize};

use super::{config::ContractWhaleRuntimeConfig, types::ContractWhaleSignal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ContractEventImpactGrade {
    C,
    B,
    A,
    S,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactGradeState {
    EvidenceInsufficient,
    Provisional,
    Confirmed,
}

/// Public V3.2 assessment lifecycle.  `ImpactGradeState` is retained for
/// backwards compatibility with Discord gating and older clients; this field
/// is the authoritative reason why a row is or is not graded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    AssessmentPending,
    BaselineWarmingUp,
    HistoricalBaselineUnavailable,
    BaselineInsufficient,
    EvidenceMissing,
    AssessmentFailed,
    Graded,
}

impl AssessmentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AssessmentPending => "assessment_pending",
            Self::BaselineWarmingUp => "baseline_warming_up",
            Self::HistoricalBaselineUnavailable => "historical_baseline_unavailable",
            Self::BaselineInsufficient => "baseline_insufficient",
            Self::EvidenceMissing => "evidence_missing",
            Self::AssessmentFailed => "assessment_failed",
            Self::Graded => "graded",
        }
    }

    pub fn from_parts(state: &str, reason_codes: &[String]) -> Self {
        if reason_codes
            .iter()
            .any(|code| code == "v3_assessment_failed")
        {
            Self::AssessmentFailed
        } else if reason_codes.iter().any(|code| code == "evidence_missing") {
            Self::EvidenceMissing
        } else if reason_codes
            .iter()
            .any(|code| code == "historical_baseline_unavailable")
        {
            Self::HistoricalBaselineUnavailable
        } else if reason_codes
            .iter()
            .any(|code| code == "baseline_warming_up")
        {
            Self::BaselineWarmingUp
        } else if state.eq_ignore_ascii_case("confirmed") {
            Self::Graded
        } else if state.eq_ignore_ascii_case("provisional")
            || reason_codes
                .iter()
                .any(|code| code == "v3_assessment_unavailable")
        {
            Self::AssessmentPending
        } else {
            Self::BaselineInsufficient
        }
    }
}

const UNRATED_GRADE: &str = "UNRATED";
const UNRATED_SIGNAL_LEVEL: &str = "N/A";
const UNRATED_STRENGTH: &str = "PENDING";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractImpactEpisode {
    pub episode_id: String,
    pub symbol: String,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub source_event_ids: Vec<String>,
    /// Peak lifecycle/window volume. This is the quantity ranked against the
    /// historical per-window baseline; episode turnover remains separate.
    #[serde(default)]
    pub peak_window_volume_btc: f64,
    pub total_volume_btc: f64,
    pub total_notional_usd: f64,
    pub net_volume_btc: f64,
    pub unique_turnover_btc: Option<f64>,
    pub unique_turnover_notional_usd: Option<f64>,
    pub live_liquidation_btc: Option<f64>,
    pub live_liquidation_notional_usd: Option<f64>,
    pub peak_abs_price_move_pct: Option<f64>,
    pub peak_abs_oi_change_pct: Option<f64>,
    pub confirmed_sources: Vec<String>,
    pub data_quality: u8,
    pub robust_percentile: Option<f64>,
    pub robust_z: Option<f64>,
    pub baseline_sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactGradeEvidence {
    pub data_quality: u8,
    #[serde(default)]
    pub peak_window_volume_btc: Option<f64>,
    pub robust_percentile: Option<f64>,
    pub robust_z: Option<f64>,
    pub abs_price_move_pct: Option<f64>,
    pub oi_change_pct: Option<f64>,
    pub live_liquidation_btc: Option<f64>,
    pub live_liquidation_notional_usd: Option<f64>,
    pub unique_turnover_btc: Option<f64>,
    pub unique_turnover_notional_usd: Option<f64>,
    pub confirmed_source_count: usize,
    pub baseline_sample_count: usize,
    /// V3.2 dual-axis scores in the normalized 0..100 range.
    #[serde(default)]
    pub flow_anomaly_score: Option<f64>,
    #[serde(default)]
    pub market_impact_score: Option<f64>,
    #[serde(default)]
    pub confidence_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractEventImpactAssessment {
    pub event_id: String,
    pub episode_id: String,
    pub symbol: String,
    pub grade_version: String,
    pub grade: ContractEventImpactGrade,
    pub state: ImpactGradeState,
    #[serde(default = "default_assessment_status")]
    pub status: AssessmentStatus,
    pub reason_codes: Vec<String>,
    pub assessed_at_ms: i64,
    pub evidence: ImpactGradeEvidence,
}

const fn default_assessment_status() -> AssessmentStatus {
    AssessmentStatus::AssessmentPending
}

impl ContractEventImpactGrade {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::B => "B",
            Self::A => "A",
            Self::S => "S",
        }
    }

    pub const fn signal_level(self) -> &'static str {
        match self {
            Self::C => "L1",
            Self::B => "L2",
            Self::A => "L3",
            Self::S => "S",
        }
    }

    pub const fn signal_label(self) -> &'static str {
        match self {
            Self::C => "LOW IMPACT EVENT",
            Self::B => "MEDIUM IMPACT EVENT",
            Self::A => "HIGH IMPACT EVENT",
            Self::S => "SHOCK IMPACT EVENT",
        }
    }

    pub const fn normalized_strength(self) -> &'static str {
        match self {
            Self::C => "LOW",
            Self::B => "MEDIUM",
            Self::A => "HIGH",
            Self::S => "EXTREME",
        }
    }
}

impl ImpactGradeState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EvidenceInsufficient => "evidence_insufficient",
            Self::Provisional => "provisional",
            Self::Confirmed => "confirmed",
        }
    }
}

/// Mirror the canonical event assessment into compatibility/display fields.
/// The assessment row remains authoritative; this snapshot prevents nested
/// signal payloads and Discord cards from showing a legacy detector grade.
pub fn apply_impact_assessment_to_signal(
    signal: &mut ContractWhaleSignal,
    assessment: &ContractEventImpactAssessment,
) {
    if assessment.status != AssessmentStatus::Graded
        && assessment.state != ImpactGradeState::Provisional
    {
        apply_unavailable_impact_assessment_to_signal(
            signal,
            &assessment.grade_version,
            assessment
                .reason_codes
                .first()
                .map(String::as_str)
                .unwrap_or("baseline_insufficient"),
        );
        return;
    }
    signal.impact_level = Some(assessment.grade.as_str().to_string());
    signal.signal_level = Some(assessment.grade.signal_level().to_string());
    signal.signal_label = Some(if assessment.state == ImpactGradeState::Provisional {
        "PROVISIONAL · PENDING HARD EVIDENCE".to_string()
    } else {
        assessment.grade.signal_label().to_string()
    });
    signal.normalized_strength = Some(assessment.grade.normalized_strength().to_string());
    signal.impact_z_score = assessment.evidence.robust_z;
    signal.percentile_level = assessment.evidence.robust_percentile;
    signal.impact_grade_state = Some(assessment.state.as_str().to_string());
    signal.impact_grade_version = Some(assessment.grade_version.clone());
    signal.impact_reason_codes = assessment.reason_codes.clone();
    // Keep the combined score available to legacy consumers while the API
    // exposes the two component scores in `impactEvidence`.
    signal.impact_score = assessment
        .evidence
        .flow_anomaly_score
        .zip(assessment.evidence.market_impact_score)
        .map(|(flow, impact)| (flow * 0.5 + impact * 0.5) / 100.0);
}

pub fn apply_unavailable_impact_assessment_to_signal(
    signal: &mut ContractWhaleSignal,
    grade_version: &str,
    reason_code: &str,
) {
    let signal_label = match reason_code {
        "v3_assessment_unavailable" => "RATING PENDING",
        "v3_assessment_failed" => "RATING ERROR",
        "baseline_warming_up" => "BASELINE WARMING UP",
        "historical_baseline_unavailable" => "HISTORICAL BASELINE UNAVAILABLE",
        "evidence_missing" => "EVIDENCE MISSING",
        "baseline_insufficient" => "BASELINE INSUFFICIENT",
        _ => "RATING UNAVAILABLE",
    };
    signal.impact_level = Some(UNRATED_GRADE.to_string());
    signal.signal_level = Some(UNRATED_SIGNAL_LEVEL.to_string());
    signal.signal_label = Some(signal_label.to_string());
    signal.normalized_strength = Some(UNRATED_STRENGTH.to_string());
    signal.impact_z_score = None;
    signal.percentile_level = None;
    signal.impact_grade_state = Some(ImpactGradeState::EvidenceInsufficient.as_str().to_string());
    signal.impact_grade_version = Some(grade_version.to_string());
    signal.impact_reason_codes = vec![reason_code.to_string()];
}

pub fn assess_contract_impact_episode(
    episode: &ContractImpactEpisode,
    config: &ContractWhaleRuntimeConfig,
    assessed_at_ms: i64,
) -> ContractEventImpactAssessment {
    let evidence = ImpactGradeEvidence {
        data_quality: episode.data_quality,
        peak_window_volume_btc: Some(episode.peak_window_volume_btc),
        robust_percentile: episode.robust_percentile,
        robust_z: episode.robust_z,
        abs_price_move_pct: episode.peak_abs_price_move_pct,
        oi_change_pct: episode.peak_abs_oi_change_pct,
        live_liquidation_btc: episode.live_liquidation_btc,
        live_liquidation_notional_usd: episode.live_liquidation_notional_usd,
        unique_turnover_btc: episode.unique_turnover_btc,
        unique_turnover_notional_usd: episode.unique_turnover_notional_usd,
        confirmed_source_count: episode.confirmed_sources.len(),
        baseline_sample_count: episode.baseline_sample_count,
        flow_anomaly_score: None,
        market_impact_score: None,
        confidence_score: None,
    };
    let mut reason_codes = Vec::new();

    let grade_config = &config.impact_grade_v3;
    if episode.baseline_sample_count < grade_config.baseline_min_samples
        || episode.robust_percentile.is_none()
        || episode.robust_z.is_none()
    {
        reason_codes.push("baseline_insufficient".to_string());
        return assessment(
            episode,
            ContractEventImpactGrade::C,
            ImpactGradeState::EvidenceInsufficient,
            reason_codes,
            evidence,
            &grade_config.grade_version,
            assessed_at_ms,
        );
    }

    // Price response and multi-source confirmation are hard evidence for V3.
    // Missing values must never be coerced to zero (which previously made an
    // absent price move look like perfect absorption), and a single venue
    // cannot promote an event into A/S.
    if episode.peak_abs_price_move_pct.is_none() {
        reason_codes.push("evidence_missing".to_string());
        reason_codes.push("price_response_missing".to_string());
        return assessment(
            episode,
            ContractEventImpactGrade::C,
            ImpactGradeState::EvidenceInsufficient,
            reason_codes,
            evidence,
            &grade_config.grade_version,
            assessed_at_ms,
        );
    }
    if episode.confirmed_sources.len() < grade_config.min_confirmed_sources {
        reason_codes.push("evidence_missing".to_string());
        reason_codes.push("confirmed_sources_insufficient".to_string());
        return assessment(
            episode,
            ContractEventImpactGrade::C,
            ImpactGradeState::EvidenceInsufficient,
            reason_codes,
            evidence,
            &grade_config.grade_version,
            assessed_at_ms,
        );
    }

    let percentile = episode.robust_percentile.unwrap_or_default();
    let robust_z = episode.robust_z.unwrap_or_default();
    let liquidation_btc = episode.live_liquidation_btc.unwrap_or_default();
    let liquidation_usd = episode.live_liquidation_notional_usd.unwrap_or_default();
    let unique_turnover_btc = episode.unique_turnover_btc.unwrap_or_default();
    let unique_turnover_usd = episode.unique_turnover_notional_usd.unwrap_or_default();
    let price_move_pct = episode
        .peak_abs_price_move_pct
        .expect("price evidence checked above");
    let flow_anomaly_score = dual_axis_flow_score(
        percentile,
        robust_z,
        episode.net_volume_btc,
        episode.total_volume_btc,
    );
    let market_impact_score = dual_axis_market_score(
        price_move_pct,
        episode.peak_abs_oi_change_pct,
        liquidation_btc,
        unique_turnover_btc,
    );
    let confidence_score = ((episode.data_quality as f64) * 0.7
        + (episode.confirmed_sources.len().min(2) as f64 / 2.0) * 30.0)
        .clamp(0.0, 100.0);
    let mut evidence = evidence;
    evidence.flow_anomaly_score = Some(flow_anomaly_score);
    evidence.market_impact_score = Some(market_impact_score);
    evidence.confidence_score = Some(confidence_score);
    // High flow with low price efficiency is an absorption/suppression setup;
    // it must not be mechanically downgraded by the trend confirmation floor.
    let absorption_confirmed = flow_anomaly_score >= 80.0 && price_move_pct < 0.10;
    let s = &grade_config.s;
    let a = &grade_config.a;
    let b = &grade_config.b;
    // Base-unit thresholds are only meaningful for BTC.  Other symbols use
    // canonical USD thresholds so ETH/alt contracts are not compared as if
    // their native units were BTC.
    let is_btc = episode.symbol.eq_ignore_ascii_case("BTC");
    let has_s_hard_evidence = liquidation_usd
        >= s.min_live_liquidation_notional_usd.unwrap_or(f64::INFINITY)
        || unique_turnover_usd >= s.min_unique_turnover_notional_usd.unwrap_or(f64::INFINITY)
        || (is_btc
            && (liquidation_btc >= s.min_live_liquidation_btc.unwrap_or(f64::INFINITY)
                || unique_turnover_btc >= s.min_unique_turnover_btc.unwrap_or(f64::INFINITY)));

    let s_eligible = has_s_hard_evidence
        && episode.data_quality >= s.min_data_quality
        && percentile >= s.min_robust_percentile
        && (price_move_pct >= s.min_abs_price_move_pct || absorption_confirmed);
    if s_eligible {
        let reason = if liquidation_btc >= s.min_live_liquidation_btc.unwrap_or(f64::INFINITY)
            || liquidation_usd >= s.min_live_liquidation_notional_usd.unwrap_or(f64::INFINITY)
        {
            "s_live_liquidation_extreme"
        } else {
            "s_unique_turnover_extreme"
        };
        reason_codes.push(reason.to_string());
        reason_codes.push(
            if absorption_confirmed {
                "s_absorption_low_price_efficiency"
            } else {
                "s_price_confirmation"
            }
            .to_string(),
        );
        if episode.confirmed_sources.len() < grade_config.min_confirmed_sources {
            reason_codes.push("s_single_source_low_confidence".to_string());
        }
        return assessment(
            episode,
            ContractEventImpactGrade::S,
            ImpactGradeState::Confirmed,
            reason_codes,
            evidence,
            &grade_config.grade_version,
            assessed_at_ms,
        );
    }
    if !has_s_hard_evidence {
        reason_codes.push("s_hard_evidence_missing".to_string());
    } else {
        reason_codes.push("s_confirmation_requirements_missing".to_string());
    }

    let a_eligible = episode.data_quality >= a.min_data_quality
        && percentile >= a.min_robust_percentile
        && robust_z >= a.min_robust_z.unwrap_or(f64::INFINITY)
        && (price_move_pct >= a.min_abs_price_move_pct || absorption_confirmed)
        && materiality_gate(
            is_btc,
            episode.total_volume_btc,
            episode.total_notional_usd,
            a.min_event_volume_btc,
            a.min_event_notional_usd,
        );
    if a_eligible {
        reason_codes.push("a_historical_outlier".to_string());
        reason_codes.push("a_major_confirmed_event".to_string());
        if absorption_confirmed {
            reason_codes.push("absorption_low_price_efficiency".to_string());
        }
        if episode.confirmed_sources.len() < grade_config.min_confirmed_sources {
            reason_codes.push("single_source_low_confidence".to_string());
        }
        let state = if !has_s_hard_evidence
            && percentile >= s.min_robust_percentile
            && (price_move_pct >= s.min_abs_price_move_pct || absorption_confirmed)
            && (episode.total_volume_btc >= s.min_unique_turnover_btc.unwrap_or(f64::INFINITY)
                || episode.total_notional_usd
                    >= s.min_unique_turnover_notional_usd.unwrap_or(f64::INFINITY))
        {
            reason_codes.push("provisional_pending_s_hard_evidence".to_string());
            ImpactGradeState::Provisional
        } else {
            ImpactGradeState::Confirmed
        };
        return assessment(
            episode,
            ContractEventImpactGrade::A,
            state,
            reason_codes,
            evidence,
            &grade_config.grade_version,
            assessed_at_ms,
        );
    }

    let b_eligible = episode.data_quality >= b.min_data_quality
        && percentile >= b.min_robust_percentile
        && robust_z >= b.min_robust_z.unwrap_or(f64::INFINITY)
        && (price_move_pct >= b.min_abs_price_move_pct || absorption_confirmed)
        && materiality_gate(
            is_btc,
            episode.total_volume_btc,
            episode.total_notional_usd,
            b.min_event_volume_btc,
            b.min_event_notional_usd,
        );
    let grade = if b_eligible {
        reason_codes.push("b_material_confirmed_event".to_string());
        ContractEventImpactGrade::B
    } else {
        reason_codes.push("c_below_materiality_floor".to_string());
        ContractEventImpactGrade::C
    };
    assessment(
        episode,
        grade,
        ImpactGradeState::Confirmed,
        reason_codes,
        evidence,
        &grade_config.grade_version,
        assessed_at_ms,
    )
}

fn materiality_gate(
    is_btc: bool,
    volume: f64,
    notional: f64,
    min_volume: Option<f64>,
    min_notional: Option<f64>,
) -> bool {
    let notional_ok = min_notional
        .map(|threshold| notional >= threshold)
        .unwrap_or(false);
    if !is_btc {
        return notional_ok;
    }
    volume >= min_volume.unwrap_or(f64::INFINITY) || notional_ok
}

fn dual_axis_flow_score(percentile: f64, robust_z: f64, net_volume: f64, total_volume: f64) -> f64 {
    let percentile_score = ((percentile - 50.0) * 2.0).clamp(0.0, 100.0);
    let z_score = (robust_z.max(0.0) / 6.0 * 100.0).clamp(0.0, 100.0);
    let dominance = if total_volume > f64::EPSILON {
        (net_volume.abs() / total_volume).clamp(0.0, 1.0) * 100.0
    } else {
        0.0
    };
    (percentile_score * 0.5 + z_score * 0.3 + dominance * 0.2).clamp(0.0, 100.0)
}

fn dual_axis_market_score(
    price_move_pct: f64,
    oi_change_pct: Option<f64>,
    liquidation_btc: f64,
    turnover_btc: f64,
) -> f64 {
    let price = (price_move_pct.abs() / 1.0 * 100.0).clamp(0.0, 100.0);
    let oi = (oi_change_pct.unwrap_or_default().abs() / 1.0 * 100.0).clamp(0.0, 100.0);
    let hard = ((liquidation_btc.max(turnover_btc) / 1_000.0) * 100.0).clamp(0.0, 100.0);
    (price * 0.45 + oi * 0.2 + hard * 0.35).clamp(0.0, 100.0)
}

/// Assess an aggregated episode while retaining the source lifecycle event as
/// the lookup key. One episode can span several adjacent lifecycle fragments;
/// every source event must still resolve to the same persisted assessment.
pub fn assess_contract_impact_episode_for_event(
    episode: &ContractImpactEpisode,
    event_id: &str,
    config: &ContractWhaleRuntimeConfig,
    assessed_at_ms: i64,
) -> ContractEventImpactAssessment {
    let mut assessment = assess_contract_impact_episode(episode, config, assessed_at_ms);
    assessment.event_id = event_id.to_string();
    assessment
}

fn assessment(
    episode: &ContractImpactEpisode,
    grade: ContractEventImpactGrade,
    state: ImpactGradeState,
    reason_codes: Vec<String>,
    evidence: ImpactGradeEvidence,
    grade_version: &str,
    assessed_at_ms: i64,
) -> ContractEventImpactAssessment {
    let status = match state {
        ImpactGradeState::Confirmed => AssessmentStatus::Graded,
        ImpactGradeState::Provisional => AssessmentStatus::AssessmentPending,
        ImpactGradeState::EvidenceInsufficient => {
            if reason_codes.iter().any(|code| code == "evidence_missing") {
                AssessmentStatus::EvidenceMissing
            } else {
                AssessmentStatus::BaselineInsufficient
            }
        }
    };
    ContractEventImpactAssessment {
        event_id: episode.episode_id.clone(),
        episode_id: episode.episode_id.clone(),
        symbol: episode.symbol.clone(),
        grade_version: grade_version.to_string(),
        grade,
        state,
        status,
        reason_codes,
        assessed_at_ms,
        evidence,
    }
}
