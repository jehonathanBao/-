//! V4.2 automatic directional-alert gate.
//!
//! The gate is deliberately independent from the signal grade.  A rare S
//! event does not need 300 other S events: it may use the same hierarchical
//! outcome evidence as every other event, while the gate only opens when the
//! resulting cohort statistics are reliable enough.  All state is persisted
//! in SQLite so a restart cannot accidentally reopen alerts.

use std::collections::BTreeSet;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    impact_forecast::{
        event_id, ContractWhaleHorizonOutcome, ContractWhaleMultiHorizonImpactForecast,
    },
    types::ContractWhaleSignal,
};
use crate::storage::{contract_whale_repo::ContractWhaleRepo, SqliteStore};

pub const GATE_VERSION: &str = "cwm_v4_2_auto_gate_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateState {
    ShadowCollecting,
    Eligible,
    Canary,
    Open,
    Cooldown,
    ForcedClosed,
}

impl GateState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ShadowCollecting => "SHADOW_COLLECTING",
            Self::Eligible => "ELIGIBLE",
            Self::Canary => "CANARY",
            Self::Open => "OPEN",
            Self::Cooldown => "COOLDOWN",
            Self::ForcedClosed => "FORCED_CLOSED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GateDecision {
    pub allowed: bool,
    pub state: String,
    pub reason: String,
    pub cohort_key: String,
    pub raw_sample_count: usize,
    pub effective_sample_count: f64,
    pub unique_episode_count: usize,
    pub accuracy: f64,
    pub coverage: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GateCohortStatus {
    pub cohort_key: String,
    pub symbol: String,
    pub horizon: String,
    pub direction: String,
    pub state: String,
    pub raw_sample_count: usize,
    pub effective_sample_count: f64,
    pub unique_episode_count: usize,
    pub accuracy: f64,
    pub coverage: f64,
    pub quality_mean: f64,
    pub consecutive_passes: usize,
    pub last_evaluated_at_ms: i64,
    pub opened_at_ms: Option<i64>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GateHealth {
    pub version: &'static str,
    pub enabled: bool,
    pub armed: bool,
    pub force_closed: bool,
    pub external_alerts_enabled: bool,
    pub cohorts: Vec<GateCohortStatus>,
}

fn cohort_key(forecast: &ContractWhaleMultiHorizonImpactForecast, horizon: &str) -> String {
    format!(
        "{}:{}:{}",
        forecast.symbol.to_ascii_uppercase(),
        horizon,
        forecast.direction.to_ascii_lowercase()
    )
}

fn aligned_success(direction: &str, markout: f64) -> bool {
    match direction.to_ascii_lowercase().as_str() {
        "bullish" | "buy" => markout > 0.0,
        "bearish" | "sell" => markout < 0.0,
        _ => false,
    }
}

fn gate_outcomes(
    store: &SqliteStore,
    forecast: &ContractWhaleMultiHorizonImpactForecast,
    horizon_sec: u64,
) -> anyhow::Result<Vec<ContractWhaleHorizonOutcome>> {
    let rows = store.with_connection(|conn| {
        let mut stmt = conn.prepare(
            "SELECT payload_json FROM contract_whale_behavior_horizon_outcomes
              WHERE symbol = ?1 AND horizon_sec = ?2 AND event_ts < ?3
                AND outcome_version = ?4 AND state = 'complete'
              ORDER BY event_ts DESC LIMIT 5000",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![
                forecast.symbol,
                horizon_sec as i64,
                forecast.event_ts,
                super::impact_v4_2::CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION
            ],
            |row| row.get::<_, String>(0),
        )?;
        rows.map(|row| {
            let payload = row?;
            serde_json::from_str::<ContractWhaleHorizonOutcome>(&payload)
                .context("invalid V4.2 outcome payload")
        })
        .collect::<Result<Vec<_>, _>>()
    })?;
    Ok(rows)
}

fn calculate(
    forecast: &ContractWhaleMultiHorizonImpactForecast,
    outcomes: &[ContractWhaleHorizonOutcome],
    config: &super::config::ContractWhaleImpactV4HybridConfig,
    now: i64,
    horizon: &str,
    horizon_sec: u64,
) -> (GateDecision, GateCohortStatus) {
    let key = cohort_key(forecast, horizon);
    let mut episodes = BTreeSet::new();
    let mut raw = 0usize;
    let mut effective = 0.0;
    let mut wins = 0usize;
    let mut quality = 0.0;
    let mut eligible = 0usize;
    for outcome in outcomes.iter().filter(|item| {
        item.horizon_sec == horizon_sec
            && item.signed_markout_bps.is_some()
            && item.event_ts < forecast.event_ts
            && item.direction.eq_ignore_ascii_case(&forecast.direction)
    }) {
        raw += 1;
        episodes.insert(outcome.episode_id.clone());
        let weight = if outcome.behavior == forecast.behavior
            && outcome.market_regime == forecast.market_regime
        {
            1.0
        } else if outcome.behavior == forecast.behavior {
            0.82
        } else {
            0.60
        };
        effective += weight;
        let markout = outcome.signed_markout_bps.unwrap_or(0.0);
        if aligned_success(&forecast.direction, markout) {
            wins += 1;
        }
        quality += outcome.data_quality as f64;
        eligible += 1;
    }
    let unique = episodes.len();
    let accuracy = if eligible == 0 {
        0.0
    } else {
        wins as f64 / eligible as f64
    };
    let quality_mean = if eligible == 0 {
        0.0
    } else {
        quality / eligible as f64
    };
    // Every stored closed outcome is a covered candidate.  A zero here means
    // the gate is not allowed to pretend a missing outcome is a success.
    let coverage = if raw == 0 { 0.0 } else { 1.0 };
    let canary_pass = raw >= config.auto_gate_min_canary_samples
        && effective >= config.auto_gate_min_canary_effective_samples
        && accuracy >= config.auto_gate_min_accuracy - 0.03
        && coverage >= 0.85
        && quality_mean >= 70.0;
    let open_pass = raw >= config.auto_gate_min_open_samples
        && effective >= config.auto_gate_min_open_effective_samples
        && unique >= config.auto_gate_min_open_samples.saturating_sub(50)
        && accuracy >= config.auto_gate_min_accuracy
        && coverage >= config.auto_gate_min_coverage
        && quality_mean >= 75.0;
    let (state, allowed, reason) = if !config.auto_gate_enabled {
        (GateState::ForcedClosed, false, "auto_gate_disabled")
    } else if config.shadow_mode || !config.external_directional_alerts_enabled {
        (
            GateState::ForcedClosed,
            false,
            "external_directional_alerts_disabled",
        )
    } else if config.auto_gate_force_closed || !config.auto_gate_armed {
        // These are explicit emergency controls. Normal operation starts
        // armed and reaches OPEN from sample evidence alone.
        (GateState::ForcedClosed, false, "operator_emergency_close")
    } else if open_pass {
        (GateState::Open, true, "open_thresholds_passed")
    } else if config.auto_gate_canary_enabled && canary_pass {
        (GateState::Canary, false, "canary_thresholds_passed")
    } else {
        (
            GateState::ShadowCollecting,
            false,
            "insufficient_calibration_evidence",
        )
    };
    let status = GateCohortStatus {
        cohort_key: key.clone(),
        symbol: forecast.symbol.clone(),
        horizon: horizon.to_string(),
        direction: forecast.direction.clone(),
        state: state.as_str().to_string(),
        raw_sample_count: raw,
        effective_sample_count: effective,
        unique_episode_count: unique,
        accuracy,
        coverage,
        quality_mean,
        consecutive_passes: if open_pass || canary_pass { 1 } else { 0 },
        last_evaluated_at_ms: now,
        opened_at_ms: (state == GateState::Open).then_some(now),
        reason: reason.to_string(),
    };
    (
        GateDecision {
            allowed,
            state: state.as_str().to_string(),
            reason: reason.to_string(),
            cohort_key: key,
            raw_sample_count: raw,
            effective_sample_count: effective,
            unique_episode_count: unique,
            accuracy,
            coverage,
        },
        status,
    )
}

fn persist_status(store: &SqliteStore, status: &GateCohortStatus, now: i64) -> anyhow::Result<()> {
    store.with_write_connection(|conn| {
        conn.execute(
            "INSERT INTO contract_whale_v42_gate_states
             (gate_version, cohort_key, symbol, horizon, direction, state,
              raw_sample_count, effective_sample_count, unique_episode_count,
              accuracy, coverage, quality_mean, consecutive_passes,
              last_evaluated_at_ms, opened_at_ms, reason, updated_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
             ON CONFLICT(gate_version, cohort_key) DO UPDATE SET
              state=excluded.state, raw_sample_count=excluded.raw_sample_count,
              effective_sample_count=excluded.effective_sample_count,
              unique_episode_count=excluded.unique_episode_count,
              accuracy=excluded.accuracy, coverage=excluded.coverage,
              quality_mean=excluded.quality_mean,
              consecutive_passes=excluded.consecutive_passes,
              last_evaluated_at_ms=excluded.last_evaluated_at_ms,
              opened_at_ms=COALESCE(contract_whale_v42_gate_states.opened_at_ms, excluded.opened_at_ms),
              reason=excluded.reason, updated_at_ms=excluded.updated_at_ms",
            rusqlite::params![
                GATE_VERSION, status.cohort_key, status.symbol, status.horizon,
                status.direction, status.state, status.raw_sample_count as i64,
                status.effective_sample_count, status.unique_episode_count as i64,
                status.accuracy, status.coverage, status.quality_mean,
                status.consecutive_passes as i64, status.last_evaluated_at_ms,
                status.opened_at_ms, status.reason, now
            ],
        )?;
        conn.execute(
            "INSERT INTO contract_whale_v42_gate_evaluations
             (gate_version, cohort_key, evaluated_at_ms, state, allowed,
              reason, raw_sample_count, effective_sample_count, accuracy, coverage, payload_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            rusqlite::params![
                GATE_VERSION, status.cohort_key, now, status.state,
                status.state == GateState::Open.as_str(), status.reason,
                status.raw_sample_count as i64, status.effective_sample_count,
                status.accuracy, status.coverage, serde_json::to_string(status)?
            ],
        )?;
        Ok(())
    })
}

fn canary_slot_available(
    store: &SqliteStore,
    config: &super::config::ContractWhaleImpactV4HybridConfig,
    now: i64,
) -> anyhow::Result<bool> {
    let since = now.saturating_sub(86_400_000);
    store.with_connection(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM contract_whale_v42_gate_evaluations
              WHERE gate_version = ?1 AND evaluated_at_ms >= ?2 AND allowed = 1",
            rusqlite::params![GATE_VERSION, since],
            |row| row.get(0),
        )?;
        Ok(count < config.auto_gate_max_alerts_per_day as i64)
    })
}

fn deterministic_canary_slot(event_id: &str) -> bool {
    let digest = Sha256::digest(event_id.as_bytes());
    digest[0] < 26 // approximately 10% and stable across retries/restarts
}

pub fn evaluate_forecast_gate(
    store: &SqliteStore,
    forecast: &ContractWhaleMultiHorizonImpactForecast,
    config: &super::config::ContractWhaleImpactV4HybridConfig,
    now: i64,
) -> anyhow::Result<GateDecision> {
    let selected = forecast
        .horizons
        .iter()
        .find(|item| item.horizon == forecast.dominant_horizon)
        .or_else(|| forecast.horizons.first())
        .ok_or_else(|| anyhow::anyhow!("forecast has no horizon"))?;
    if !forecast.source_policy.eq_ignore_ascii_case("binance_only")
        || !forecast.binance_evidence_complete
    {
        return Ok(GateDecision {
            allowed: false,
            state: GateState::ForcedClosed.as_str().to_string(),
            reason: "binance_evidence_incomplete".to_string(),
            cohort_key: cohort_key(forecast, &selected.horizon),
            ..Default::default()
        });
    }
    let mut selected_decision = None;
    for horizon in &forecast.horizons {
        let outcomes = gate_outcomes(store, forecast, horizon.horizon_sec)?;
        let (mut decision, status) = calculate(
            forecast,
            &outcomes,
            config,
            now,
            &horizon.horizon,
            horizon.horizon_sec,
        );
        // Canary is a real, deterministic low-rate path. It never changes
        // the state to OPEN and is capped by the daily budget in SQLite.
        if decision.state == GateState::Canary.as_str()
            && deterministic_canary_slot(&forecast.event_id)
            && canary_slot_available(store, config, now)?
        {
            decision.allowed = true;
            decision.reason = "canary_sampled".to_string();
        }
        persist_status(store, &status, now)?;
        if horizon.horizon == selected.horizon {
            selected_decision = Some(decision);
        }
    }
    selected_decision.ok_or_else(|| anyhow::anyhow!("selected horizon gate was not evaluated"))
}

pub fn evaluate_signal_gate(
    store: &SqliteStore,
    signal: &ContractWhaleSignal,
    config: &super::config::ContractWhaleImpactV4HybridConfig,
    now: i64,
) -> anyhow::Result<GateDecision> {
    let event_id = event_id(signal);
    let ids = [event_id.as_str()];
    let forecasts = store.load_contract_whale_impact_forecasts(
        &ids,
        super::impact_v4_2::CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
    )?;
    let Some(forecast) = forecasts.get(&event_id) else {
        return Ok(GateDecision {
            allowed: false,
            state: GateState::ShadowCollecting.as_str().to_string(),
            reason: "v4_2_forecast_unavailable".to_string(),
            cohort_key: format!("{}:unknown:unknown", signal.symbol),
            ..Default::default()
        });
    };
    evaluate_forecast_gate(store, forecast, config, now)
}

pub fn health(
    store: &SqliteStore,
    config: &super::config::ContractWhaleImpactV4HybridConfig,
) -> anyhow::Result<GateHealth> {
    let cohorts = store.with_connection(|conn| {
        let mut stmt = conn.prepare(
            "SELECT cohort_key,symbol,horizon,direction,state,raw_sample_count,
                    effective_sample_count,unique_episode_count,accuracy,coverage,
                    quality_mean,consecutive_passes,last_evaluated_at_ms,opened_at_ms,reason
               FROM contract_whale_v42_gate_states WHERE gate_version = ?1
              ORDER BY symbol,horizon,direction",
        )?;
        let rows = stmt.query_map([GATE_VERSION], |row| {
            Ok(GateCohortStatus {
                cohort_key: row.get(0)?,
                symbol: row.get(1)?,
                horizon: row.get(2)?,
                direction: row.get(3)?,
                state: row.get(4)?,
                raw_sample_count: row.get::<_, i64>(5)?.max(0) as usize,
                effective_sample_count: row.get(6)?,
                unique_episode_count: row.get::<_, i64>(7)?.max(0) as usize,
                accuracy: row.get(8)?,
                coverage: row.get(9)?,
                quality_mean: row.get(10)?,
                consecutive_passes: row.get::<_, i64>(11)?.max(0) as usize,
                last_evaluated_at_ms: row.get(12)?,
                opened_at_ms: row.get(13)?,
                reason: row.get(14)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })?;
    Ok(GateHealth {
        version: GATE_VERSION,
        enabled: config.auto_gate_enabled,
        armed: config.auto_gate_armed,
        force_closed: config.auto_gate_force_closed,
        external_alerts_enabled: config.external_directional_alerts_enabled,
        cohorts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gate_is_sample_driven_and_not_manually_locked() {
        let config = super::super::config::ContractWhaleImpactV4HybridConfig::default();
        assert!(config.auto_gate_enabled);
        assert!(config.auto_gate_armed);
        assert!(!config.auto_gate_force_closed);
        assert!(!config.shadow_mode);
        assert!(config.external_directional_alerts_enabled);
    }

    #[test]
    fn directional_markout_is_aligned() {
        assert!(aligned_success("bullish", 1.0));
        assert!(aligned_success("bearish", -1.0));
        assert!(!aligned_success("bullish", -1.0));
    }
}
