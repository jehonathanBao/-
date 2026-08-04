use anyhow::Context;
use rusqlite::{params, OptionalExtension};

use crate::contract_whale_monitor::impact_grade::ContractEventImpactAssessment;

use super::SqliteStore;

#[derive(Debug, Clone)]
pub struct ContractEventGradeRepo {
    store: SqliteStore,
}

impl ContractEventGradeRepo {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    pub fn upsert_assessment(
        &self,
        assessment: &ContractEventImpactAssessment,
        now_ms: i64,
    ) -> anyhow::Result<()> {
        let reason_codes_json = serde_json::to_string(&assessment.reason_codes)?;
        let evidence_json = serde_json::to_string(&assessment.evidence)?;
        let grade = serde_json::to_string(&assessment.grade)?
            .trim_matches('"')
            .to_string();
        let state = serde_json::to_string(&assessment.state)?
            .trim_matches('"')
            .to_string();
        self.store.with_write_connection(|conn| {
            conn.execute(
                "INSERT INTO contract_event_impact_grades
                 (event_id, grade_version, episode_id, symbol, grade, state, reason_codes_json,
                  evidence_json, assessed_at_ms, discord_sent_at_ms, created_at_ms, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?10)
                 ON CONFLICT(event_id, grade_version) DO UPDATE SET
                   episode_id = excluded.episode_id,
                   symbol = excluded.symbol,
                   grade = excluded.grade,
                   state = excluded.state,
                   reason_codes_json = excluded.reason_codes_json,
                   evidence_json = excluded.evidence_json,
                   assessed_at_ms = excluded.assessed_at_ms,
                   discord_sent_at_ms = COALESCE(contract_event_impact_grades.discord_sent_at_ms, excluded.discord_sent_at_ms),
                   updated_at_ms = excluded.updated_at_ms",
                params![
                    assessment.event_id,
                    assessment.grade_version,
                    assessment.episode_id,
                    assessment.symbol,
                    grade,
                    state,
                    reason_codes_json,
                    evidence_json,
                    assessment.assessed_at_ms,
                    now_ms,
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_assessment(
        &self,
        event_id: &str,
        grade_version: &str,
    ) -> anyhow::Result<Option<ContractEventImpactAssessment>> {
        self.store.with_connection(|conn| {
            conn.query_row(
                "SELECT event_id, grade_version, episode_id, symbol, grade, state, reason_codes_json,
                        evidence_json, assessed_at_ms
                   FROM contract_event_impact_grades
                  WHERE event_id = ?1 AND grade_version = ?2",
                params![event_id, grade_version],
                |row| {
                    let grade: String = row.get(4)?;
                    let state: String = row.get(5)?;
                    let reason_codes_json: String = row.get(6)?;
                    let evidence_json: String = row.get(7)?;
                    Ok(ContractEventImpactAssessment {
                        event_id: row.get(0)?,
                        grade_version: row.get(1)?,
                        episode_id: row.get(2)?,
                        symbol: row.get(3)?,
                        grade: serde_json::from_str(&format!("\"{grade}\""))
                            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                        state: serde_json::from_str(&format!("\"{state}\""))
                            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                        reason_codes: serde_json::from_str(&reason_codes_json)
                            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                        assessed_at_ms: row.get(8)?,
                        evidence: serde_json::from_str(&evidence_json)
                            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                    })
                },
            )
            .optional()
            .context("failed to load contract impact grade")
        })
    }

    pub fn mark_alert_sent(
        &self,
        event_id: &str,
        grade_version: &str,
        sent_at_ms: i64,
    ) -> anyhow::Result<bool> {
        self.store.with_write_connection(|conn| {
            let changed = conn.execute(
                "UPDATE contract_event_impact_grades
                    SET discord_sent_at_ms = COALESCE(discord_sent_at_ms, ?3), updated_at_ms = ?3
                  WHERE event_id = ?1 AND grade_version = ?2 AND discord_sent_at_ms IS NULL",
                params![event_id, grade_version, sent_at_ms],
            )?;
            Ok(changed == 1)
        })
    }

    pub fn alert_already_sent(&self, event_id: &str, grade_version: &str) -> anyhow::Result<bool> {
        self.store.with_connection(|conn| {
            let sent: Option<i64> = conn
                .query_row(
                    "SELECT discord_sent_at_ms FROM contract_event_impact_grades
                      WHERE event_id = ?1 AND grade_version = ?2",
                    params![event_id, grade_version],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(sent.is_some())
        })
    }
}
