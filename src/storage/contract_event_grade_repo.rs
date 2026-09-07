use anyhow::Context;
use rusqlite::{params, OptionalExtension};

use crate::contract_whale_monitor::{
    impact_grade::{AssessmentStatus, ContractEventImpactAssessment},
    types::ContractWhaleSignal,
};

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

    /// Atomically persist one canonical assessment and every lookup alias.
    /// Legacy grade rows are retained for compatibility; the alias table is
    /// the authoritative episode/source/projection identity map.
    pub fn upsert_assessment_with_aliases(
        &self,
        assessment: &ContractEventImpactAssessment,
        aliases: &[(&str, Option<&str>, Option<&str>)],
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
        self.store.with_transaction(|tx| {
            // Keep exactly one authoritative grade row per episode.  The
            // alias table below is the fan-out index for source/projection
            // identifiers; duplicating grades for every alias would inflate
            // health metrics and allow alert state to diverge.
            tx.execute(
                "INSERT INTO contract_event_impact_grades
                 (event_id, grade_version, episode_id, symbol, grade, state, reason_codes_json,
                  evidence_json, assessed_at_ms, discord_sent_at_ms, created_at_ms, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?10)
                 ON CONFLICT(event_id, grade_version) DO UPDATE SET
                   episode_id=excluded.episode_id, symbol=excluded.symbol, grade=excluded.grade,
                   state=excluded.state, reason_codes_json=excluded.reason_codes_json,
                   evidence_json=excluded.evidence_json, assessed_at_ms=excluded.assessed_at_ms,
                   updated_at_ms=excluded.updated_at_ms",
                params![assessment.event_id, assessment.grade_version, assessment.episode_id,
                    assessment.symbol, grade, state, reason_codes_json, evidence_json,
                    assessment.assessed_at_ms, now_ms],
            )?;
            for (alias, projection_event_id, source_event_id) in aliases {
                let alias = alias.trim();
                if alias.is_empty() { continue; }
                tx.execute(
                    "INSERT INTO contract_event_impact_aliases
                     (alias_id, grade_version, episode_id, projection_event_id, source_event_id, updated_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(alias_id, grade_version) DO UPDATE SET
                       episode_id=excluded.episode_id, projection_event_id=excluded.projection_event_id,
                       source_event_id=excluded.source_event_id, updated_at_ms=excluded.updated_at_ms",
                    params![alias, assessment.grade_version, assessment.episode_id,
                        projection_event_id, source_event_id, now_ms],
                )?;
            }
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
                    let reason_codes: Vec<String> = serde_json::from_str(&reason_codes_json)
                        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                    Ok(ContractEventImpactAssessment {
                        event_id: row.get(0)?,
                        grade_version: row.get(1)?,
                        episode_id: row.get(2)?,
                        symbol: row.get(3)?,
                        grade: serde_json::from_str(&format!("\"{grade}\""))
                            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                        state: serde_json::from_str(&format!("\"{state}\""))
                            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                        reason_codes: reason_codes.clone(),
                        assessed_at_ms: row.get(8)?,
                        evidence: serde_json::from_str(&evidence_json)
                            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                        status: AssessmentStatus::from_parts(&state, &reason_codes),
                    })
                },
            )
            .optional()
            .context("failed to load contract impact grade")
        })
    }

    /// Resolve an assessment through the event aliases used by the lifecycle
    /// projection.  A displayed lifecycle row can have a newly-computed event
    /// id while its source rows retain the persisted V3 ids.  Looking up the
    /// source payloads here keeps every read path on the same episode grade.
    pub fn get_assessment_for_aliases(
        &self,
        aliases: &[&str],
        grade_version: &str,
    ) -> anyhow::Result<Option<ContractEventImpactAssessment>> {
        let mut candidate_ids = Vec::<String>::new();
        for alias in aliases
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            if !candidate_ids.iter().any(|candidate| candidate == alias) {
                candidate_ids.push(alias.to_string());
            }
        }

        // Resolve episode, source and projection identifiers through the
        // persisted V3.2 alias map before falling back to legacy payload
        // inspection. `json_each` keeps this a single indexed SQL query.
        let alias_json = serde_json::to_string(&candidate_ids)?;
        let mapped_ids = self.store.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT episode_id, alias_id
                   FROM contract_event_impact_aliases
                  WHERE grade_version = ?1
                    AND (alias_id IN (SELECT value FROM json_each(?2))
                      OR episode_id IN (SELECT value FROM json_each(?2))
                      OR projection_event_id IN (SELECT value FROM json_each(?2))
                      OR source_event_id IN (SELECT value FROM json_each(?2)))",
            )?;
            let rows = stmt.query_map(params![grade_version, alias_json], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut values = Vec::new();
            for row in rows {
                let (episode_id, alias_id) = row?;
                values.push(episode_id);
                values.push(alias_id);
            }
            Ok(values)
        })?;
        candidate_ids.extend(mapped_ids);

        // Lifecycle projections expose source signal ids in mergedFrom. Map
        // those ids back to the source event id stored in their payload.
        let source_event_ids = self.store.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT COALESCE(NULLIF(json_extract(payload_json, '$.eventLifecycle.eventId'), ''), signal_id)
                   FROM contract_whale_signals_history
                  WHERE signal_id = ?1
                  LIMIT 1",
            )?;
            let mut mapped = Vec::new();
            for alias in &candidate_ids {
                if let Some(event_id) = stmt
                    .query_row([alias], |row| row.get::<_, String>(0))
                    .optional()?
                {
                    if !event_id.trim().is_empty()
                        && !candidate_ids.iter().any(|candidate| candidate == &event_id)
                        && !mapped.iter().any(|candidate: &String| candidate == &event_id)
                    {
                        mapped.push(event_id);
                    }
                }
            }
            Ok(mapped)
        })?;
        candidate_ids.extend(source_event_ids);

        let mut best: Option<ContractEventImpactAssessment> = None;
        for event_id in candidate_ids {
            let Some(assessment) = self.get_assessment(&event_id, grade_version)? else {
                continue;
            };
            let rank = (
                assessment.assessed_at_ms,
                grade_rank(&assessment),
                state_rank(&assessment),
            );
            if best.as_ref().map_or(true, |current| {
                rank > (
                    current.assessed_at_ms,
                    grade_rank(current),
                    state_rank(current),
                )
            }) {
                best = Some(assessment);
            }
        }
        Ok(best)
    }

    pub fn get_assessment_for_signal(
        &self,
        signal: &ContractWhaleSignal,
        grade_version: &str,
    ) -> anyhow::Result<Option<ContractEventImpactAssessment>> {
        let mut aliases = Vec::with_capacity(signal.merged_from.len() + 2);
        aliases.push(signal.event_lifecycle.event_id.as_str());
        aliases.push(signal.id.as_str());
        aliases.extend(signal.merged_from.iter().map(String::as_str));
        self.get_assessment_for_aliases(&aliases, grade_version)
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

    pub fn mark_episode_alert_sent(
        &self,
        episode_id: &str,
        grade_version: &str,
        sent_at_ms: i64,
    ) -> anyhow::Result<bool> {
        self.store.with_write_connection(|conn| {
            let changed = conn.execute(
                "UPDATE contract_event_impact_grades
                    SET discord_sent_at_ms = COALESCE(discord_sent_at_ms, ?3), updated_at_ms = ?3
                  WHERE episode_id = ?1 AND grade_version = ?2 AND discord_sent_at_ms IS NULL",
                params![episode_id, grade_version, sent_at_ms],
            )?;
            Ok(changed > 0)
        })
    }

    pub fn episode_alert_already_sent(
        &self,
        episode_id: &str,
        _grade_version: &str,
    ) -> anyhow::Result<bool> {
        self.store.with_connection(|conn| {
            conn.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM contract_event_impact_grades
                     WHERE episode_id = ?1
                       AND discord_sent_at_ms IS NOT NULL
                 )",
                params![episode_id],
                |row| row.get(0),
            )
            .context("failed to check contract impact episode alert status")
        })
    }
}

fn grade_rank(assessment: &ContractEventImpactAssessment) -> u8 {
    match assessment.grade {
        crate::contract_whale_monitor::impact_grade::ContractEventImpactGrade::C => 0,
        crate::contract_whale_monitor::impact_grade::ContractEventImpactGrade::B => 1,
        crate::contract_whale_monitor::impact_grade::ContractEventImpactGrade::A => 2,
        crate::contract_whale_monitor::impact_grade::ContractEventImpactGrade::S => 3,
    }
}

fn state_rank(assessment: &ContractEventImpactAssessment) -> u8 {
    match assessment.state {
        crate::contract_whale_monitor::impact_grade::ImpactGradeState::EvidenceInsufficient => 0,
        crate::contract_whale_monitor::impact_grade::ImpactGradeState::Provisional => 1,
        crate::contract_whale_monitor::impact_grade::ImpactGradeState::Confirmed => 2,
    }
}
