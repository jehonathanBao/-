use anyhow::Context;
use rusqlite::params;

use crate::{
    binance_alt_contract_monitor::types::{AltContractSignal, AltContractSignalOutcome},
    storage::sqlite::SqliteStore,
};

pub trait BinanceAltContractRepo {
    fn upsert_alt_contract_signal(&self, signal: &AltContractSignal) -> anyhow::Result<()>;
    fn upsert_alt_contract_signals(&self, signals: &[AltContractSignal]) -> anyhow::Result<()>;
    fn load_alt_contract_signals(&self, limit: usize) -> anyhow::Result<Vec<AltContractSignal>>;
    fn prune_alt_contract_signals(&self, older_than: i64) -> anyhow::Result<usize>;
    fn upsert_alt_contract_outcome(&self, outcome: &AltContractSignalOutcome)
        -> anyhow::Result<()>;
    fn load_alt_contract_outcomes(
        &self,
        limit: usize,
    ) -> anyhow::Result<Vec<AltContractSignalOutcome>>;
}

impl BinanceAltContractRepo for SqliteStore {
    fn upsert_alt_contract_signal(&self, signal: &AltContractSignal) -> anyhow::Result<()> {
        self.upsert_alt_contract_signals(std::slice::from_ref(signal))
    }

    fn upsert_alt_contract_signals(&self, signals: &[AltContractSignal]) -> anyhow::Result<()> {
        if signals.is_empty() {
            return Ok(());
        }
        let rows = signals
            .iter()
            .map(|signal| {
                Ok((
                    signal.id.clone(),
                    signal.product_id.clone(),
                    signal.ts,
                    format!("{:?}", signal.signal_type),
                    format!("{:?}", signal.severity),
                    format!("{:?}", signal.direction),
                    serde_json::to_string(signal).context("failed to serialize BACM signal")?,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.with_transaction(|tx| {
            let mut statement = tx
                .prepare(
                    r#"
                    INSERT INTO alt_contract_signals (
                      signal_id, product_id, ts, signal_type, severity, direction, payload_json
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    ON CONFLICT(signal_id) DO UPDATE SET
                      product_id = excluded.product_id,
                      ts = excluded.ts,
                      signal_type = excluded.signal_type,
                      severity = excluded.severity,
                      direction = excluded.direction,
                      payload_json = excluded.payload_json
                    "#,
                )
                .context("failed to prepare BACM signal upsert")?;
            for row in &rows {
                statement
                    .execute(params![row.0, row.1, row.2, row.3, row.4, row.5, row.6])
                    .context("failed to upsert BACM signal")?;
            }
            Ok(())
        })
    }

    fn prune_alt_contract_signals(&self, older_than: i64) -> anyhow::Result<usize> {
        self.with_connection(|conn| {
            conn.execute(
                "DELETE FROM alt_contract_signals WHERE severity != 'S' AND ts < ?1",
                [older_than],
            )
            .context("failed to prune BACM signals")
        })
    }

    fn load_alt_contract_signals(&self, limit: usize) -> anyhow::Result<Vec<AltContractSignal>> {
        self.with_connection(|conn| {
            let mut statement = conn
                .prepare("SELECT payload_json FROM alt_contract_signals ORDER BY ts DESC LIMIT ?1")
                .context("failed to prepare BACM signal load")?;
            let rows = statement
                .query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
                    row.get::<_, String>(0)
                })
                .context("failed to load BACM signals")?;
            let signals = rows
                .filter_map(Result::ok)
                .filter_map(|payload| serde_json::from_str(&payload).ok())
                .collect::<Vec<_>>();
            Ok(signals)
        })
    }

    fn upsert_alt_contract_outcome(
        &self,
        outcome: &AltContractSignalOutcome,
    ) -> anyhow::Result<()> {
        let payload = serde_json::to_string(outcome).context("failed to serialize BACM outcome")?;
        self.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO alt_contract_signal_outcomes (
                  signal_id, product_id, tier, signal_ts, window_sec, signal_type,
                  anomaly_severity, structure_confidence, exposure_tier, ais_score,
                  abnormal_score, build_score, regime, oi_context, liquidation_context,
                  entry_price, markout_5m_bps, markout_15m_bps, markout_1h_bps,
                  mfe_1h_bps, mae_1h_bps, follow_through_5m, follow_through_15m,
                  follow_through_1h, evaluated_5m_at, evaluated_15m_at, evaluated_1h_at,
                  outcome_version, payload_json
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                  ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29
                ) ON CONFLICT(signal_id) DO UPDATE SET
                  markout_5m_bps = excluded.markout_5m_bps,
                  markout_15m_bps = excluded.markout_15m_bps,
                  markout_1h_bps = excluded.markout_1h_bps,
                  mfe_1h_bps = excluded.mfe_1h_bps,
                  mae_1h_bps = excluded.mae_1h_bps,
                  follow_through_5m = excluded.follow_through_5m,
                  follow_through_15m = excluded.follow_through_15m,
                  follow_through_1h = excluded.follow_through_1h,
                  evaluated_5m_at = excluded.evaluated_5m_at,
                  evaluated_15m_at = excluded.evaluated_15m_at,
                  evaluated_1h_at = excluded.evaluated_1h_at,
                  payload_json = excluded.payload_json
                "#,
                params![
                    outcome.signal_id,
                    outcome.product_id,
                    format!("{:?}", outcome.tier),
                    outcome.signal_ts,
                    outcome.window_sec,
                    outcome.signal_type,
                    format!("{:?}", outcome.anomaly_severity),
                    format!("{:?}", outcome.structure_confidence),
                    format!("{:?}", outcome.exposure_tier),
                    outcome.ais_score,
                    outcome.abnormal_score,
                    outcome.build_score,
                    outcome.regime,
                    outcome.oi_context,
                    outcome.liquidation_context,
                    outcome.entry_price,
                    outcome.markout_5m_bps,
                    outcome.markout_15m_bps,
                    outcome.markout_1h_bps,
                    outcome.mfe_1h_bps,
                    outcome.mae_1h_bps,
                    outcome.follow_through_5m.map(i64::from),
                    outcome.follow_through_15m.map(i64::from),
                    outcome.follow_through_1h.map(i64::from),
                    outcome.evaluated_5m_at,
                    outcome.evaluated_15m_at,
                    outcome.evaluated_1h_at,
                    outcome.outcome_version,
                    payload,
                ],
            )
            .context("failed to upsert BACM outcome")?;
            Ok(())
        })
    }

    fn load_alt_contract_outcomes(
        &self,
        limit: usize,
    ) -> anyhow::Result<Vec<AltContractSignalOutcome>> {
        self.with_connection(|conn| {
            let mut statement = conn
                .prepare("SELECT payload_json FROM alt_contract_signal_outcomes ORDER BY signal_ts DESC LIMIT ?1")
                .context("failed to prepare BACM outcome load")?;
            let rows = statement
                .query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| row.get::<_, String>(0))
                .context("failed to load BACM outcomes")?;
            Ok(rows
                .filter_map(Result::ok)
                .filter_map(|payload| serde_json::from_str(&payload).ok())
                .collect())
        })
    }
}
