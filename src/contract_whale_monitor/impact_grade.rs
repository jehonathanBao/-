//! Versioned, fail-closed impact grading for contract-whale episodes.
//!
//! The grade is deliberately based on absolute evidence as well as robust
//! relative statistics.  A page cohort or a one-off relative burst can never
//! promote an event to S without liquidation or extraordinary unique-turnover
//! evidence.

use serde::{Deserialize, Serialize};

use super::config::ContractWhaleRuntimeConfig;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractImpactEpisode {
    pub episode_id: String,
    pub symbol: String,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub source_event_ids: Vec<String>,
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
    pub reason_codes: Vec<String>,
    pub assessed_at_ms: i64,
    pub evidence: ImpactGradeEvidence,
}

pub fn assess_contract_impact_episode(
    episode: &ContractImpactEpisode,
    config: &ContractWhaleRuntimeConfig,
    assessed_at_ms: i64,
) -> ContractEventImpactAssessment {
    let evidence = ImpactGradeEvidence {
        data_quality: episode.data_quality,
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

    let percentile = episode.robust_percentile.unwrap_or_default();
    let robust_z = episode.robust_z.unwrap_or_default();
    let liquidation_btc = episode.live_liquidation_btc.unwrap_or_default();
    let liquidation_usd = episode.live_liquidation_notional_usd.unwrap_or_default();
    let unique_turnover_btc = episode.unique_turnover_btc.unwrap_or_default();
    let unique_turnover_usd = episode.unique_turnover_notional_usd.unwrap_or_default();
    let price_move_pct = episode.peak_abs_price_move_pct.unwrap_or_default();
    let s = &grade_config.s;
    let a = &grade_config.a;
    let b = &grade_config.b;
    let has_s_hard_evidence = liquidation_btc
        >= s.min_live_liquidation_btc.unwrap_or(f64::INFINITY)
        || liquidation_usd >= s.min_live_liquidation_notional_usd.unwrap_or(f64::INFINITY)
        || unique_turnover_btc >= s.min_unique_turnover_btc.unwrap_or(f64::INFINITY)
        || unique_turnover_usd >= s.min_unique_turnover_notional_usd.unwrap_or(f64::INFINITY);

    let s_eligible = has_s_hard_evidence
        && episode.data_quality >= s.min_data_quality
        && episode.confirmed_sources.len() >= grade_config.min_confirmed_sources
        && percentile >= s.min_robust_percentile
        && price_move_pct >= s.min_abs_price_move_pct;
    if s_eligible {
        let reason = if liquidation_btc >= s.min_live_liquidation_btc.unwrap_or(f64::INFINITY)
            || liquidation_usd >= s.min_live_liquidation_notional_usd.unwrap_or(f64::INFINITY)
        {
            "s_live_liquidation_extreme"
        } else {
            "s_unique_turnover_extreme"
        };
        reason_codes.push(reason.to_string());
        reason_codes.push("s_price_confirmation".to_string());
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
        && price_move_pct >= a.min_abs_price_move_pct
        && (episode.total_volume_btc >= a.min_event_volume_btc.unwrap_or(f64::INFINITY)
            || episode.total_notional_usd >= a.min_event_notional_usd.unwrap_or(f64::INFINITY));
    if a_eligible {
        reason_codes.push("a_historical_outlier".to_string());
        reason_codes.push("a_major_confirmed_event".to_string());
        let state = if !has_s_hard_evidence
            && percentile >= s.min_robust_percentile
            && price_move_pct >= s.min_abs_price_move_pct
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
        && price_move_pct >= b.min_abs_price_move_pct
        && (episode.total_volume_btc >= b.min_event_volume_btc.unwrap_or(f64::INFINITY)
            || episode.total_notional_usd >= b.min_event_notional_usd.unwrap_or(f64::INFINITY));
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

fn assessment(
    episode: &ContractImpactEpisode,
    grade: ContractEventImpactGrade,
    state: ImpactGradeState,
    reason_codes: Vec<String>,
    evidence: ImpactGradeEvidence,
    grade_version: &str,
    assessed_at_ms: i64,
) -> ContractEventImpactAssessment {
    ContractEventImpactAssessment {
        event_id: episode.episode_id.clone(),
        episode_id: episode.episode_id.clone(),
        symbol: episode.symbol.clone(),
        grade_version: grade_version.to_string(),
        grade,
        state,
        reason_codes,
        assessed_at_ms,
        evidence,
    }
}
