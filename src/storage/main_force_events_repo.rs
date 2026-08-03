use anyhow::Context;
use rusqlite::{params, OptionalExtension};

use crate::types::main_force_event::{
    MainForceEvent, MainForceEventObservation, MainForceEventQuery,
};

use super::sqlite::SqliteStore;

pub trait MainForceEventsRepo {
    fn observe_main_force_event(
        &self,
        symbol: &str,
        observation: Option<&MainForceEventObservation>,
        now_ms: i64,
    ) -> anyhow::Result<Option<MainForceEvent>>;
    fn list_main_force_events(
        &self,
        query: &MainForceEventQuery,
    ) -> anyhow::Result<Vec<MainForceEvent>>;
}

impl MainForceEventsRepo for SqliteStore {
    fn observe_main_force_event(
        &self,
        symbol: &str,
        observation: Option<&MainForceEventObservation>,
        now_ms: i64,
    ) -> anyhow::Result<Option<MainForceEvent>> {
        self.with_write_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let open_event = latest_open_event(&tx, symbol)?;
            let mut result = None;

            if let Some(observation) = observation {
                if let Some(existing) = open_event {
                    if should_rollover(&existing, observation) {
                        close_event(&tx, existing.id, observation.observed_at)?;
                        if observation.start_triggered() {
                            let event = insert_event(&tx, observation, now_ms)?;
                            result = Some(event);
                        }
                    } else if observation.keeps_event_open() {
                        let event = update_open_event(&tx, &existing, observation)?;
                        result = Some(event);
                    } else {
                        let event = mark_or_close_inactive(&tx, &existing, now_ms)?;
                        result = event;
                    }
                } else if observation.start_triggered() {
                    let event = insert_event(&tx, observation, now_ms)?;
                    result = Some(event);
                }
            } else if let Some(existing) = open_event {
                result = mark_or_close_inactive(&tx, &existing, now_ms)?;
            }

            tx.commit()?;
            Ok(result)
        })
    }

    fn list_main_force_events(
        &self,
        query: &MainForceEventQuery,
    ) -> anyhow::Result<Vec<MainForceEvent>> {
        let regime_type = query
            .regime_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        let severity = query
            .severity
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        let active_only = query.active_only.map(bool_to_int);
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, symbol, started_at, ended_at, peak_at, last_observed_at, inactive_since,
                       regime_type, severity, peak_main_force_score, peak_extreme_impact_score,
                       peak_structure_bias, confidence, spot_score, contract_score,
                       cross_confirm_score, cwm_score, oi_score, liquidation_score,
                       funding_crowding_score, main_force_confirmed, extreme_impact_confirmed,
                       liquidation_driven, reasons_json, created_at
                FROM main_force_events
                WHERE (?1 IS NULL OR symbol = ?1)
                  AND (?2 IS NULL OR lower(regime_type) = ?2)
                  AND (?3 IS NULL OR lower(severity) = ?3)
                  AND (?4 IS NULL OR (CASE WHEN ended_at IS NULL THEN 1 ELSE 0 END) = ?4)
                  AND (?5 IS NULL OR started_at >= ?5)
                  AND (?6 IS NULL OR COALESCE(ended_at, last_observed_at) <= ?6)
                ORDER BY started_at DESC, id DESC
                LIMIT ?7 OFFSET ?8
                "#,
            )?;
            let rows = stmt.query_map(
                params![
                    query.symbol.as_deref(),
                    regime_type.as_deref(),
                    severity.as_deref(),
                    active_only,
                    query.from_ts,
                    query.to_ts,
                    query.limit as i64,
                    query.offset as i64,
                ],
                decode_main_force_event_row,
            )?;
            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }
}

fn latest_open_event(
    conn: &rusqlite::Connection,
    symbol: &str,
) -> anyhow::Result<Option<MainForceEvent>> {
    conn.query_row(
        r#"
        SELECT id, symbol, started_at, ended_at, peak_at, last_observed_at, inactive_since,
               regime_type, severity, peak_main_force_score, peak_extreme_impact_score,
               peak_structure_bias, confidence, spot_score, contract_score,
               cross_confirm_score, cwm_score, oi_score, liquidation_score,
               funding_crowding_score, main_force_confirmed, extreme_impact_confirmed,
               liquidation_driven, reasons_json, created_at
        FROM main_force_events
        WHERE symbol = ?1
          AND ended_at IS NULL
        ORDER BY started_at DESC, id DESC
        LIMIT 1
        "#,
        params![symbol],
        decode_main_force_event_row,
    )
    .optional()
    .context("failed to load latest open main force event")
}

fn insert_event(
    conn: &rusqlite::Connection,
    observation: &MainForceEventObservation,
    now_ms: i64,
) -> anyhow::Result<MainForceEvent> {
    conn.execute(
        r#"
        INSERT INTO main_force_events (
          symbol, started_at, ended_at, peak_at, last_observed_at, inactive_since,
          regime_type, severity, peak_main_force_score, peak_extreme_impact_score,
          peak_structure_bias, confidence, spot_score, contract_score,
          cross_confirm_score, cwm_score, oi_score, liquidation_score,
          funding_crowding_score, main_force_confirmed, extreme_impact_confirmed,
          liquidation_driven, reasons_json, created_at
        ) VALUES (?1, ?2, NULL, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                  ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
        "#,
        params![
            observation.symbol,
            observation.observed_at,
            observation.observed_at,
            observation.observed_at,
            observation.regime_type,
            observation.severity,
            observation.main_force_score,
            observation.extreme_impact_score,
            observation.structure_bias,
            observation.confidence,
            observation.spot_score,
            observation.contract_score,
            observation.cross_confirm_score,
            observation.cwm_score,
            observation.oi_score,
            observation.liquidation_score,
            observation.funding_crowding_score,
            bool_to_int(observation.main_force_confirmed),
            bool_to_int(observation.extreme_impact_confirmed),
            bool_to_int(observation.liquidation_driven),
            serde_json::to_string(&observation.reasons_json)?,
            now_ms,
        ],
    )
    .context("failed to insert main force event")?;
    let id = conn.last_insert_rowid();
    Ok(MainForceEvent {
        id,
        symbol: observation.symbol.clone(),
        started_at: observation.observed_at,
        ended_at: None,
        peak_at: observation.observed_at,
        last_observed_at: observation.observed_at,
        inactive_since: None,
        regime_type: observation.regime_type.clone(),
        severity: observation.severity.clone(),
        peak_main_force_score: observation.main_force_score,
        peak_extreme_impact_score: observation.extreme_impact_score,
        peak_structure_bias: observation.structure_bias,
        confidence: observation.confidence,
        spot_score: observation.spot_score,
        contract_score: observation.contract_score,
        cross_confirm_score: observation.cross_confirm_score,
        cwm_score: observation.cwm_score,
        oi_score: observation.oi_score,
        liquidation_score: observation.liquidation_score,
        funding_crowding_score: observation.funding_crowding_score,
        main_force_confirmed: observation.main_force_confirmed,
        extreme_impact_confirmed: observation.extreme_impact_confirmed,
        liquidation_driven: observation.liquidation_driven,
        reasons_json: observation.reasons_json.clone(),
        created_at: now_ms,
    })
}

fn update_open_event(
    conn: &rusqlite::Connection,
    existing: &MainForceEvent,
    observation: &MainForceEventObservation,
) -> anyhow::Result<MainForceEvent> {
    let replace_peak = peak_score(
        observation.main_force_score,
        observation.extreme_impact_score,
    ) >= peak_score(
        existing.peak_main_force_score,
        existing.peak_extreme_impact_score,
    );
    let updated = MainForceEvent {
        id: existing.id,
        symbol: existing.symbol.clone(),
        started_at: existing.started_at,
        ended_at: None,
        peak_at: if replace_peak {
            observation.observed_at
        } else {
            existing.peak_at
        },
        last_observed_at: observation.observed_at,
        inactive_since: None,
        regime_type: if replace_peak {
            observation.regime_type.clone()
        } else {
            existing.regime_type.clone()
        },
        severity: if replace_peak {
            observation.severity.clone()
        } else {
            existing.severity.clone()
        },
        peak_main_force_score: if replace_peak {
            observation.main_force_score
        } else {
            existing.peak_main_force_score
        },
        peak_extreme_impact_score: if replace_peak {
            observation.extreme_impact_score
        } else {
            existing.peak_extreme_impact_score
        },
        peak_structure_bias: if replace_peak {
            observation.structure_bias
        } else {
            existing.peak_structure_bias
        },
        confidence: if replace_peak {
            observation.confidence
        } else {
            existing.confidence
        },
        spot_score: if replace_peak {
            observation.spot_score
        } else {
            existing.spot_score
        },
        contract_score: if replace_peak {
            observation.contract_score
        } else {
            existing.contract_score
        },
        cross_confirm_score: if replace_peak {
            observation.cross_confirm_score
        } else {
            existing.cross_confirm_score
        },
        cwm_score: if replace_peak {
            observation.cwm_score
        } else {
            existing.cwm_score
        },
        oi_score: if replace_peak {
            observation.oi_score
        } else {
            existing.oi_score
        },
        liquidation_score: if replace_peak {
            observation.liquidation_score
        } else {
            existing.liquidation_score
        },
        funding_crowding_score: if replace_peak {
            observation.funding_crowding_score
        } else {
            existing.funding_crowding_score
        },
        main_force_confirmed: existing.main_force_confirmed || observation.main_force_confirmed,
        extreme_impact_confirmed: existing.extreme_impact_confirmed
            || observation.extreme_impact_confirmed,
        liquidation_driven: observation.liquidation_driven,
        reasons_json: if replace_peak {
            observation.reasons_json.clone()
        } else {
            existing.reasons_json.clone()
        },
        created_at: existing.created_at,
    };
    conn.execute(
        r#"
        UPDATE main_force_events
        SET peak_at = ?2,
            last_observed_at = ?3,
            inactive_since = NULL,
            regime_type = ?4,
            severity = ?5,
            peak_main_force_score = ?6,
            peak_extreme_impact_score = ?7,
            peak_structure_bias = ?8,
            confidence = ?9,
            spot_score = ?10,
            contract_score = ?11,
            cross_confirm_score = ?12,
            cwm_score = ?13,
            oi_score = ?14,
            liquidation_score = ?15,
            funding_crowding_score = ?16,
            main_force_confirmed = ?17,
            extreme_impact_confirmed = ?18,
            liquidation_driven = ?19,
            reasons_json = ?20
        WHERE id = ?1
        "#,
        params![
            updated.id,
            updated.peak_at,
            updated.last_observed_at,
            updated.regime_type,
            updated.severity,
            updated.peak_main_force_score,
            updated.peak_extreme_impact_score,
            updated.peak_structure_bias,
            updated.confidence,
            updated.spot_score,
            updated.contract_score,
            updated.cross_confirm_score,
            updated.cwm_score,
            updated.oi_score,
            updated.liquidation_score,
            updated.funding_crowding_score,
            bool_to_int(updated.main_force_confirmed),
            bool_to_int(updated.extreme_impact_confirmed),
            bool_to_int(updated.liquidation_driven),
            serde_json::to_string(&updated.reasons_json)?,
        ],
    )
    .context("failed to update open main force event")?;
    Ok(updated)
}

fn mark_or_close_inactive(
    conn: &rusqlite::Connection,
    existing: &MainForceEvent,
    now_ms: i64,
) -> anyhow::Result<Option<MainForceEvent>> {
    let inactive_since = existing.inactive_since.unwrap_or(now_ms);
    if existing.inactive_since.is_none() {
        conn.execute(
            "UPDATE main_force_events SET last_observed_at = ?2, inactive_since = ?3 WHERE id = ?1",
            params![existing.id, now_ms, inactive_since],
        )
        .context("failed to mark main force event inactive")?;
        let mut next = existing.clone();
        next.last_observed_at = now_ms;
        next.inactive_since = Some(inactive_since);
        return Ok(Some(next));
    }
    if now_ms.saturating_sub(inactive_since) >= event_end_grace_ms() {
        close_event(conn, existing.id, inactive_since)?;
        return Ok(None);
    }
    conn.execute(
        "UPDATE main_force_events SET last_observed_at = ?2 WHERE id = ?1",
        params![existing.id, now_ms],
    )
    .context("failed to refresh inactive main force event")?;
    let mut next = existing.clone();
    next.last_observed_at = now_ms;
    next.inactive_since = Some(inactive_since);
    Ok(Some(next))
}

fn close_event(conn: &rusqlite::Connection, event_id: i64, ended_at: i64) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE main_force_events SET ended_at = ?2, inactive_since = COALESCE(inactive_since, ?2) WHERE id = ?1",
        params![event_id, ended_at],
    )
    .context("failed to close main force event")?;
    Ok(())
}

fn decode_main_force_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MainForceEvent> {
    let reasons_json: String = row.get(23)?;
    let reasons = serde_json::from_str::<serde_json::Value>(&reasons_json)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    Ok(MainForceEvent {
        id: row.get(0)?,
        symbol: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        peak_at: row.get(4)?,
        last_observed_at: row.get(5)?,
        inactive_since: row.get(6)?,
        regime_type: row.get(7)?,
        severity: row.get(8)?,
        peak_main_force_score: row.get(9)?,
        peak_extreme_impact_score: row.get(10)?,
        peak_structure_bias: row.get(11)?,
        confidence: row.get(12)?,
        spot_score: row.get(13)?,
        contract_score: row.get(14)?,
        cross_confirm_score: row.get(15)?,
        cwm_score: row.get(16)?,
        oi_score: row.get(17)?,
        liquidation_score: row.get(18)?,
        funding_crowding_score: row.get(19)?,
        main_force_confirmed: row.get::<_, i64>(20)? != 0,
        extreme_impact_confirmed: row.get::<_, i64>(21)? != 0,
        liquidation_driven: row.get::<_, i64>(22)? != 0,
        reasons_json: reasons,
        created_at: row.get(24)?,
    })
}

fn should_rollover(existing: &MainForceEvent, observation: &MainForceEventObservation) -> bool {
    if !observation.start_triggered() {
        return false;
    }
    let existing_polarity = event_polarity(&existing.regime_type, existing.peak_structure_bias);
    let next_polarity = event_polarity(&observation.regime_type, observation.structure_bias);
    (existing_polarity != 0 && next_polarity != 0 && existing_polarity != next_polarity)
        || (existing.regime_type != observation.regime_type
            && existing.liquidation_driven != observation.liquidation_driven)
}

fn event_polarity(regime_type: &str, structure_bias: f64) -> i8 {
    match regime_type {
        "main_force_long_build"
        | "spot_accumulation"
        | "contract_short_squeeze"
        | "downside_absorption" => 1,
        "main_force_short_build"
        | "spot_distribution"
        | "long_liquidation_cascade"
        | "upside_resistance" => -1,
        _ if structure_bias >= 15.0 => 1,
        _ if structure_bias <= -15.0 => -1,
        _ => 0,
    }
}

fn peak_score(main_force_score: f64, extreme_impact_score: f64) -> f64 {
    main_force_score.max(extreme_impact_score)
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn event_end_grace_ms() -> i64 {
    let config = crate::runtime::score_config::score_runtime_config();
    i64::try_from(config.market_structure.event_end_hold_minutes)
        .unwrap_or(15)
        .saturating_mul(60_000)
}
