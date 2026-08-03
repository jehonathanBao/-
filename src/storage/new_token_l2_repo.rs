//! Compact persistence for new-token L2 session metrics.
//!
//! Raw depth frames are intentionally not stored. This table only captures a
//! low-frequency, redacted session summary suitable for diagnostics and later
//! shadow evaluation.

use anyhow::Context;
use rusqlite::params;

use crate::toxic_v3::new_token_watch::{
    session::L2SessionSnapshot,
    shadow::{ShadowOutcomeLabel, ShadowOutcomeObservation},
};

use super::sqlite::SqliteStore;

pub const NEW_TOKEN_L2_METRIC_RETENTION_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
pub const NEW_TOKEN_L2_OUTCOME_RETENTION_MS: i64 = 365 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedNewTokenL2Outcome {
    pub event_id: String,
    pub symbol: String,
    pub observed_at_ms: i64,
    pub horizon_seconds: u32,
    pub intent_state: String,
    pub entry_price: f64,
    pub observed_price: f64,
    pub price_move_bps: f64,
    pub outcome_label: ShadowOutcomeLabel,
    pub shadow_only: bool,
    pub discord_eligible: bool,
    pub execution_enabled: bool,
    pub outcome_reason: String,
}

pub trait NewTokenL2Repo {
    fn insert_new_token_l2_metric(
        &self,
        ts: i64,
        session: &L2SessionSnapshot,
    ) -> anyhow::Result<()>;
    fn count_new_token_l2_metrics(&self, symbol: &str) -> anyhow::Result<usize>;
    fn prune_new_token_l2_metrics_older_than(&self, cutoff_ts: i64) -> anyhow::Result<usize>;
    fn upsert_new_token_l2_shadow_outcomes(
        &self,
        outcomes: &[ShadowOutcomeObservation],
    ) -> anyhow::Result<usize>;
    fn list_new_token_l2_shadow_outcomes(
        &self,
        symbol: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<PersistedNewTokenL2Outcome>>;
    fn prune_new_token_l2_outcomes_older_than(&self, cutoff_ts: i64) -> anyhow::Result<usize>;
}

impl NewTokenL2Repo for SqliteStore {
    fn insert_new_token_l2_metric(
        &self,
        ts: i64,
        session: &L2SessionSnapshot,
    ) -> anyhow::Result<()> {
        let payload_json = serde_json::to_string(session)?;
        self.with_write_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO new_token_l2_metrics (
                    ts, symbol, readiness, evidence_mode, spread_bps, imbalance,
                    visible_cancel_to_add_ratio, intent_state, intent_confidence,
                    intent_available, wall_count, payload_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(symbol, ts) DO UPDATE SET
                    readiness = excluded.readiness,
                    evidence_mode = excluded.evidence_mode,
                    spread_bps = excluded.spread_bps,
                    imbalance = excluded.imbalance,
                    visible_cancel_to_add_ratio = excluded.visible_cancel_to_add_ratio,
                    intent_state = excluded.intent_state,
                    intent_confidence = excluded.intent_confidence,
                    intent_available = excluded.intent_available,
                    wall_count = excluded.wall_count,
                    payload_json = excluded.payload_json
                "#,
                params![
                    ts,
                    session.symbol,
                    format!("{:?}", session.orderbook.readiness).to_ascii_lowercase(),
                    session.evidence_mode,
                    session.orderbook.spread_bps,
                    session.orderbook.imbalance,
                    session.orderbook.visible_cancel_to_add_ratio,
                    format!("{:?}", session.intent.state).to_ascii_lowercase(),
                    session.intent.confidence,
                    i64::from(session.intent_assessment_available),
                    session.wall_evidence.len() as i64,
                    payload_json,
                ],
            )
            .context("failed to insert compact new-token L2 metric")?;
            Ok(())
        })
    }

    fn count_new_token_l2_metrics(&self, symbol: &str) -> anyhow::Result<usize> {
        self.with_connection(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM new_token_l2_metrics WHERE symbol = ?1",
                [symbol],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count.max(0) as usize)
            .context("failed to count new-token L2 metrics")
        })
    }

    fn prune_new_token_l2_metrics_older_than(&self, cutoff_ts: i64) -> anyhow::Result<usize> {
        self.with_write_connection(|conn| {
            conn.execute(
                "DELETE FROM new_token_l2_metrics WHERE ts < ?1",
                [cutoff_ts],
            )
            .map(|deleted| deleted as usize)
            .context("failed to prune new-token L2 metrics")
        })
    }

    fn upsert_new_token_l2_shadow_outcomes(
        &self,
        outcomes: &[ShadowOutcomeObservation],
    ) -> anyhow::Result<usize> {
        if outcomes.is_empty() {
            return Ok(0);
        }
        self.with_write_connection(|conn| {
            let transaction = conn.unchecked_transaction()?;
            let mut written = 0;
            for observation in outcomes {
                written += transaction.execute(
                    r#"
                    INSERT INTO new_token_l2_outcomes (
                        event_id, symbol, observed_at, horizon_sec, intent_state,
                        entry_price, observed_price, price_move_bps, outcome_label,
                        shadow_only, discord_eligible, execution_enabled, outcome_reason
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                    ON CONFLICT(event_id, horizon_sec) DO UPDATE SET
                        observed_price = excluded.observed_price,
                        price_move_bps = excluded.price_move_bps,
                        outcome_label = excluded.outcome_label,
                        shadow_only = excluded.shadow_only,
                        discord_eligible = excluded.discord_eligible,
                        execution_enabled = excluded.execution_enabled,
                        outcome_reason = excluded.outcome_reason
                    "#,
                    params![
                        observation.event_id,
                        observation.symbol,
                        observation.observed_at_ms,
                        observation.horizon_seconds as i64,
                        format!("{:?}", observation.intent_state).to_ascii_lowercase(),
                        observation.entry_price,
                        observation.observed_price,
                        observation.price_move_bps,
                        format!("{:?}", observation.outcome.label).to_ascii_lowercase(),
                        i64::from(observation.outcome.shadow_only),
                        i64::from(observation.outcome.discord_eligible),
                        i64::from(observation.outcome.execution_enabled),
                        observation.outcome.reason,
                    ],
                )?;
            }
            transaction.commit()?;
            Ok(written)
        })
        .context("failed to persist new-token L2 shadow outcomes")
    }

    fn list_new_token_l2_shadow_outcomes(
        &self,
        symbol: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<PersistedNewTokenL2Outcome>> {
        self.with_connection(|conn| {
            let mut statement = conn.prepare(
                r#"
                SELECT event_id, symbol, observed_at, horizon_sec, intent_state,
                       entry_price, observed_price, price_move_bps, outcome_label,
                       shadow_only, discord_eligible, execution_enabled, outcome_reason
                FROM new_token_l2_outcomes
                WHERE symbol = ?1
                ORDER BY observed_at DESC, horizon_sec ASC, event_id ASC
                LIMIT ?2
                "#,
            )?;
            let rows = statement.query_map(params![symbol, limit.clamp(1, 500) as i64], |row| {
                Ok(PersistedNewTokenL2Outcome {
                    event_id: row.get(0)?,
                    symbol: row.get(1)?,
                    observed_at_ms: row.get(2)?,
                    horizon_seconds: row.get::<_, i64>(3)? as u32,
                    intent_state: row.get(4)?,
                    entry_price: row.get(5)?,
                    observed_price: row.get(6)?,
                    price_move_bps: row.get(7)?,
                    outcome_label: match row.get::<_, String>(8)?.as_str() {
                        "aligned" => ShadowOutcomeLabel::Aligned,
                        "conflicted" => ShadowOutcomeLabel::Conflicted,
                        "neutral" => ShadowOutcomeLabel::Neutral,
                        _ => ShadowOutcomeLabel::InsufficientEvidence,
                    },
                    shadow_only: row.get::<_, i64>(9)? != 0,
                    discord_eligible: row.get::<_, i64>(10)? != 0,
                    execution_enabled: row.get::<_, i64>(11)? != 0,
                    outcome_reason: row.get(12)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()
                .context("failed to read new-token L2 outcomes")
        })
    }

    fn prune_new_token_l2_outcomes_older_than(&self, cutoff_ts: i64) -> anyhow::Result<usize> {
        self.with_write_connection(|conn| {
            conn.execute(
                "DELETE FROM new_token_l2_outcomes WHERE observed_at < ?1",
                [cutoff_ts],
            )
            .map(|deleted| deleted as usize)
            .context("failed to prune new-token L2 outcomes")
        })
    }
}
