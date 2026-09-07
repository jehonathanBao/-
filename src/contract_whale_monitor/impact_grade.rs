//! Versioned, fail-closed impact grading for contract-whale episodes.
//!
//! The grade is deliberately based on absolute evidence as well as robust
//! relative statistics.  A page cohort or a one-off relative burst can never
//! promote an event to S without liquidation or extraordinary unique-turnover
//! evidence.

use serde::{Deserialize, Serialize};

use super::{config::ContractWhaleRuntimeConfig, types::ContractWhaleSignal};

pub const CONTRACT_EVENT_IMPACT_GRADE_VERSION: &str = "cwm_impact_v3_3";

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
    /// Distinct perp venues with actual fresh trade evidence at the episode
    /// cutoff. Producers must exclude stale/missing trades; stream names (spot,
    /// OI, funding) are not venue confirmations. Historical cutoffs are causal.
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
    signal.impact_score = None;
}

pub fn assess_contract_impact_episode(
    episode: &ContractImpactEpisode,
    config: &ContractWhaleRuntimeConfig,
    assessed_at_ms: i64,
) -> ContractEventImpactAssessment {
    let eligible = config.threshold_profile_resolution().eligible_keys();
    let confirmed = episode
        .confirmed_sources
        .iter()
        .map(|source| source.trim().to_ascii_lowercase())
        .filter(|source| eligible.contains(source))
        .collect::<std::collections::BTreeSet<_>>();
    let required_sources = config
        .impact_grade_v3
        .min_confirmed_sources
        .min(eligible.len())
        .max(1);
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
        confirmed_source_count: confirmed.len(),
        baseline_sample_count: episode.baseline_sample_count,
        flow_anomaly_score: None,
        market_impact_score: None,
        confidence_score: None,
    };
    let mut reason_codes = Vec::new();

    let grade_config = &config.impact_grade_v3;
    if episode.baseline_sample_count < grade_config.baseline_min_samples
        || !episode
            .robust_percentile
            .is_some_and(|value| value.is_finite() && (0.0..=100.0).contains(&value))
        || !episode.robust_z.is_some_and(f64::is_finite)
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

    // Price response and configured-source confirmation are hard evidence.
    // Missing values must never be coerced to zero (which previously made an
    // absent price move look like perfect absorption).
    if !episode
        .peak_abs_price_move_pct
        .is_some_and(|value| value.is_finite() && value >= 0.0)
    {
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
    if confirmed.len() < required_sources {
        reason_codes.push("evidence_missing".to_string());
        reason_codes.push("confirmed_sources_insufficient".to_string());
        reason_codes.push(format!(
            "configured_eligible_sources:{}",
            eligible.join(",")
        ));
        reason_codes.push(format!("required_source_count:{required_sources}"));
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

    if episode.data_quality < grade_config.b.min_data_quality.max(70)
        || !episode.total_volume_btc.is_finite()
        || episode.total_volume_btc <= 0.0
        || !episode.total_notional_usd.is_finite()
        || episode.total_notional_usd <= 0.0
        || !episode.net_volume_btc.is_finite()
    {
        return assessment(
            episode,
            ContractEventImpactGrade::C,
            ImpactGradeState::EvidenceInsufficient,
            vec![
                "evidence_missing".into(),
                "market_evidence_quality_insufficient".into(),
            ],
            evidence,
            &grade_config.grade_version,
            assessed_at_ms,
        );
    }
    reason_codes.push(format!(
        "configured_eligible_sources:{}",
        eligible.join(",")
    ));
    reason_codes.push(format!("required_source_count:{required_sources}"));

    let percentile = episode.robust_percentile.unwrap_or_default();
    let robust_z = episode.robust_z.unwrap_or_default();
    let finite_positive = |value: Option<f64>| {
        value
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or_default()
    };
    let liquidation_btc = finite_positive(episode.live_liquidation_btc);
    let liquidation_usd = finite_positive(episode.live_liquidation_notional_usd);
    let unique_turnover_btc = finite_positive(episode.unique_turnover_btc);
    let unique_turnover_usd = finite_positive(episode.unique_turnover_notional_usd);
    let price_move_pct = episode
        .peak_abs_price_move_pct
        .expect("price evidence checked above");
    let flow_anomaly_score = dual_axis_flow_score(
        percentile,
        robust_z,
        episode.net_volume_btc,
        episode.total_volume_btc,
    );
    let s = &grade_config.s;
    let is_btc = episode.symbol.eq_ignore_ascii_case("BTC");
    // USD thresholds give equal economic evidence equal weight across symbols.
    // BTC native-unit fallback is retained only for historical evidence lacking USD.
    let hard_score = if episode.live_liquidation_notional_usd.is_some()
        || episode.unique_turnover_notional_usd.is_some()
        || !is_btc
    {
        (liquidation_usd / s.min_live_liquidation_notional_usd.unwrap_or(f64::INFINITY))
            .max(unique_turnover_usd / s.min_unique_turnover_notional_usd.unwrap_or(f64::INFINITY))
    } else {
        (liquidation_btc / s.min_live_liquidation_btc.unwrap_or(f64::INFINITY))
            .max(unique_turnover_btc / s.min_unique_turnover_btc.unwrap_or(f64::INFINITY))
    } * 100.0;
    let market_impact_score = dual_axis_market_score(
        price_move_pct,
        episode
            .peak_abs_oi_change_pct
            .filter(|value| value.is_finite()),
        hard_score,
    );
    let confidence_score = ((episode.data_quality as f64) * 0.7
        + (confirmed.len() as f64 / eligible.len().max(1) as f64) * 30.0)
        .clamp(0.0, 100.0);
    let mut evidence = evidence;
    evidence.flow_anomaly_score = Some(flow_anomaly_score);
    evidence.market_impact_score = Some(market_impact_score);
    evidence.confidence_score = Some(confidence_score);
    // High flow with low price efficiency is an absorption/suppression setup;
    // it must not be mechanically downgraded by the trend confirmation floor.
    let absorption_confirmed = flow_anomaly_score >= 80.0 && price_move_pct < 0.10;
    let a = &grade_config.a;
    let b = &grade_config.b;
    // Base-unit thresholds are only meaningful for BTC.  Other symbols use
    // canonical USD thresholds so ETH/alt contracts are not compared as if
    // their native units were BTC.
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
        let reason = if (is_btc
            && liquidation_btc >= s.min_live_liquidation_btc.unwrap_or(f64::INFINITY))
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
        let state = if !has_s_hard_evidence
            && percentile >= s.min_robust_percentile
            && (price_move_pct >= s.min_abs_price_move_pct || absorption_confirmed)
            && ((is_btc
                && episode.total_volume_btc >= s.min_unique_turnover_btc.unwrap_or(f64::INFINITY))
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

fn dual_axis_market_score(price_move_pct: f64, oi_change_pct: Option<f64>, hard_score: f64) -> f64 {
    let price = (price_move_pct.abs() / 1.0 * 100.0).clamp(0.0, 100.0);
    let oi = (oi_change_pct.unwrap_or_default().abs() / 1.0 * 100.0).clamp(0.0, 100.0);
    let hard = hard_score.clamp(0.0, 100.0);
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

#[cfg(test)]
mod canonical_grade_tests {
    use super::*;

    fn config() -> ContractWhaleRuntimeConfig {
        let mut config = ContractWhaleRuntimeConfig::default();
        config.exchanges.bitfinex.enabled = false;
        config
    }

    fn episode() -> ContractImpactEpisode {
        ContractImpactEpisode {
            episode_id: "canonical-grade-episode".into(),
            symbol: "BTC".into(),
            start_time_ms: 1_700_000_000_000,
            end_time_ms: 1_700_000_060_000,
            source_event_ids: vec!["canonical-grade-source".into()],
            peak_window_volume_btc: 8_000.0,
            total_volume_btc: 8_000.0,
            total_notional_usd: 500_000_000.0,
            net_volume_btc: 6_000.0,
            unique_turnover_btc: None,
            unique_turnover_notional_usd: None,
            live_liquidation_btc: Some(2_500.0),
            live_liquidation_notional_usd: Some(250_000_000.0),
            peak_abs_price_move_pct: Some(2.0),
            peak_abs_oi_change_pct: Some(0.4),
            confirmed_sources: vec!["binance".into()],
            data_quality: 90,
            robust_percentile: Some(99.95),
            robust_z: Some(6.0),
            baseline_sample_count: 20_000,
        }
    }

    #[test]
    fn canonical_grade_binance_only_reaches_s_without_lowering_hard_floors() {
        let config = config();
        let value = assess_contract_impact_episode(&episode(), &config, 1_700_000_060_000);
        assert_eq!(value.grade, ContractEventImpactGrade::S);
        assert_eq!(value.status, AssessmentStatus::Graded);
        assert_eq!(value.grade_version, "cwm_impact_v3_3");
        assert_eq!(value.evidence.confirmed_source_count, 1);
        assert_eq!(value.evidence.confidence_score, Some(93.0));
        let mut ordinary = episode();
        ordinary.live_liquidation_btc = None;
        ordinary.live_liquidation_notional_usd = None;
        assert_ne!(
            assess_contract_impact_episode(&ordinary, &config, 0).grade,
            ContractEventImpactGrade::S
        );
    }

    #[test]
    fn canonical_grade_extra_expected_venue_cannot_be_replaced_by_duplicate_or_spot_oi() {
        let config = ContractWhaleRuntimeConfig::default();
        let mut episode = episode();
        episode.confirmed_sources = vec![
            "binance".into(),
            "BINANCE".into(),
            "binance_spot".into(),
            "binance_oi".into(),
            "coinbase".into(),
        ];
        let value = assess_contract_impact_episode(&episode, &config, 0);
        assert_eq!(value.status, AssessmentStatus::EvidenceMissing);
        assert_eq!(value.evidence.confirmed_source_count, 1);
        assert!(value
            .reason_codes
            .contains(&"configured_eligible_sources:binance,bitfinex".into()));
    }

    #[test]
    fn canonical_grade_missing_stale_or_invalid_evidence_stays_ungraded() {
        let config = config();
        for case in 0..7 {
            let mut episode = episode();
            // Stale trades must be excluded from confirmed_sources by the producer.
            match case {
                0 => episode.confirmed_sources.clear(),
                1 => episode.data_quality = 0,
                2 => episode.peak_abs_price_move_pct = None,
                3 => episode.peak_abs_price_move_pct = Some(f64::NAN),
                4 => episode.baseline_sample_count = 0,
                5 => episode.robust_percentile = Some(f64::NAN),
                _ => episode.robust_z = Some(f64::INFINITY),
            }
            let value = assess_contract_impact_episode(&episode, &config, 0);
            assert_ne!(value.status, AssessmentStatus::Graded, "case {case}");
            assert_eq!(
                value.state,
                ImpactGradeState::EvidenceInsufficient,
                "case {case}"
            );
        }
    }

    #[test]
    fn canonical_grade_uses_usd_for_non_btc_market_axis_and_materiality() {
        let config = config();
        let mut first = episode();
        first.symbol = "ETH".into();
        first.peak_abs_price_move_pct = Some(0.6);
        first.live_liquidation_notional_usd = Some(25_000_000.0);
        first.live_liquidation_btc = Some(20_000.0);
        let mut second = first.clone();
        second.symbol = "SOL".into();
        second.live_liquidation_btc = Some(200.0);
        let a = assess_contract_impact_episode(&first, &config, 0);
        let b = assess_contract_impact_episode(&second, &config, 0);
        assert_eq!(a.grade, ContractEventImpactGrade::A);
        assert_eq!(a.grade, b.grade);
        assert_eq!(
            a.evidence.market_impact_score,
            b.evidence.market_impact_score
        );
        first.total_notional_usd = 10_000_000.0;
        assert_eq!(
            assess_contract_impact_episode(&first, &config, 0).grade,
            ContractEventImpactGrade::C
        );
    }
}
