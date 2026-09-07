use anyhow::Context;
use rusqlite::{params, OptionalExtension};

pub use crate::spot_whale_monitor::types::{
    SPOT_WHALE_BTC_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
    SPOT_WHALE_ETH_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
    SPOT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
};

use crate::spot_whale_monitor::types::{is_permanent_spot_whale_signal, SpotWhaleSignal};

use super::sqlite::SqliteStore;

#[derive(Debug, Clone, Default)]
pub struct SpotWhaleSignalQuery {
    pub symbol: Option<String>,
    pub severity: Option<String>,
    pub signal_type: Option<String>,
    pub discord_sent: Option<bool>,
    pub min_abs_net_volume_base: Option<f64>,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub permanent_only: Option<bool>,
    pub limit: usize,
    pub offset: usize,
    pub cursor_ts: Option<i64>,
    pub cursor_signal_id: Option<String>,
}

pub trait SpotWhaleRepo {
    fn upsert_spot_whale_signal(&self, signal: &SpotWhaleSignal) -> anyhow::Result<()>;
    fn query_spot_whale_signals(
        &self,
        query: &SpotWhaleSignalQuery,
    ) -> anyhow::Result<Vec<SpotWhaleSignal>>;
    fn count_spot_whale_signals_with_query(
        &self,
        query: &SpotWhaleSignalQuery,
    ) -> anyhow::Result<usize>;
    fn update_spot_whale_discord_status(
        &self,
        signal_id: &str,
        sent: bool,
        sent_at_ms: Option<i64>,
        reason: &str,
    ) -> anyhow::Result<usize>;
    fn count_spot_whale_signals(&self, symbol: &str) -> anyhow::Result<usize>;
    fn prune_spot_whale_signals_older_than(&self, cutoff_ts: i64) -> anyhow::Result<usize>;
}

impl SpotWhaleRepo for SqliteStore {
    fn upsert_spot_whale_signal(&self, signal: &SpotWhaleSignal) -> anyhow::Result<()> {
        let signal = canonicalize_signal(signal);
        let signal_type = format!("{:?}", signal.signal_type);
        let direction = format!("{:?}", signal.direction);
        let severity = format!("{:?}", signal.severity);
        let exchanges_json = serde_json::to_string(&signal.exchanges)?;
        let payload_json = serde_json::to_string(&signal)?;
        self.with_write_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO spot_whale_signals (
                  signal_id, ts, symbol, window_sec, signal_type, direction, severity, score,
                  total_volume_base, net_volume_base, total_notional_usd, dominance,
                  price_move_pct, coinbase_premium_pct, main_exchange, exchanges_json,
                  dynamic_multiple, multi_exchange_confirmed, data_quality, discord_eligible,
                  discord_sent, discord_sent_at, discord_reason, is_permanent, payload_json,
                  created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                          ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                          ?26)
                ON CONFLICT(signal_id) DO UPDATE SET
                  ts = excluded.ts,
                  symbol = excluded.symbol,
                  window_sec = excluded.window_sec,
                  signal_type = excluded.signal_type,
                  direction = excluded.direction,
                  severity = excluded.severity,
                  score = excluded.score,
                  total_volume_base = excluded.total_volume_base,
                  net_volume_base = excluded.net_volume_base,
                  total_notional_usd = excluded.total_notional_usd,
                  dominance = excluded.dominance,
                  price_move_pct = excluded.price_move_pct,
                  coinbase_premium_pct = excluded.coinbase_premium_pct,
                  main_exchange = excluded.main_exchange,
                  exchanges_json = excluded.exchanges_json,
                  dynamic_multiple = excluded.dynamic_multiple,
                  multi_exchange_confirmed = excluded.multi_exchange_confirmed,
                  data_quality = excluded.data_quality,
                  discord_eligible = excluded.discord_eligible,
                  discord_sent = excluded.discord_sent,
                  discord_sent_at = excluded.discord_sent_at,
                  discord_reason = excluded.discord_reason,
                  is_permanent = excluded.is_permanent,
                  payload_json = excluded.payload_json,
                  created_at = excluded.created_at
                "#,
                params![
                    signal.id,
                    signal.ts,
                    signal.symbol,
                    signal.window_sec as i64,
                    signal_type,
                    direction,
                    severity,
                    signal.score as i64,
                    signal.total_volume_base,
                    signal.net_volume_base,
                    signal.total_notional_usd,
                    signal.dominance,
                    signal.price_move_pct,
                    signal.coinbase_premium_pct,
                    signal.main_exchange,
                    exchanges_json,
                    signal.dynamic_multiple,
                    bool_to_int(signal.multi_exchange_confirmed),
                    signal.data_quality as i64,
                    bool_to_int(signal.discord_eligible),
                    bool_to_int(signal.discord_sent),
                    signal.discord_sent_at,
                    signal.discord_reason,
                    bool_to_int(signal.is_permanent),
                    payload_json,
                    crate::normalizers::trade::now_ms(),
                ],
            )
            .context("failed to upsert spot whale signal")?;
            Ok(())
        })
    }

    fn query_spot_whale_signals(
        &self,
        query: &SpotWhaleSignalQuery,
    ) -> anyhow::Result<Vec<SpotWhaleSignal>> {
        let severity = query.severity.as_deref().map(compact_filter_value);
        let signal_type = query.signal_type.as_deref().map(compact_filter_value);
        let discord_sent = query.discord_sent.map(bool_to_int);
        let min_abs_net_volume_base = query
            .min_abs_net_volume_base
            .filter(|value| value.is_finite() && *value > 0.0);
        let from_ts = query.from_ts.filter(|value| *value > 0);
        let to_ts = query.to_ts.filter(|value| *value > 0);
        let permanent_only = query.permanent_only.map(bool_to_int);
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT payload_json, discord_eligible, discord_sent, discord_sent_at,
                       discord_reason, is_permanent
                FROM spot_whale_signals
                WHERE (?1 IS NULL OR symbol = ?1)
                  AND (?2 IS NULL OR LOWER(REPLACE(severity, '_', '')) = ?2)
                  AND (?3 IS NULL OR LOWER(REPLACE(signal_type, '_', '')) = ?3)
                  AND (?4 IS NULL OR discord_sent = ?4)
                  AND (?5 IS NULL OR ABS(net_volume_base) >= ?5)
                  AND (?6 IS NULL OR ts >= ?6)
                  AND (?7 IS NULL OR ts < ?7)
                  AND (?8 IS NULL OR is_permanent = ?8)
                  AND (?9 IS NULL OR ts < ?9 OR (ts = ?9 AND signal_id < ?10))
                ORDER BY ts DESC, signal_id DESC
                LIMIT ?11 OFFSET ?12
                "#,
            )?;
            let rows = stmt.query_map(
                params![
                    query.symbol.as_deref(),
                    severity.as_deref(),
                    signal_type.as_deref(),
                    discord_sent,
                    min_abs_net_volume_base,
                    from_ts,
                    to_ts,
                    permanent_only,
                    query.cursor_ts,
                    query.cursor_signal_id.as_deref(),
                    query.limit as i64,
                    query.offset as i64,
                ],
                decode_signal_row,
            )?;
            let mut signals = Vec::new();
            for row in rows {
                signals.push(row?);
            }
            Ok(signals)
        })
    }

    fn count_spot_whale_signals_with_query(
        &self,
        query: &SpotWhaleSignalQuery,
    ) -> anyhow::Result<usize> {
        let severity = query.severity.as_deref().map(compact_filter_value);
        let signal_type = query.signal_type.as_deref().map(compact_filter_value);
        let discord_sent = query.discord_sent.map(bool_to_int);
        let min_abs_net_volume_base = query
            .min_abs_net_volume_base
            .filter(|value| value.is_finite() && *value > 0.0);
        let from_ts = query.from_ts.filter(|value| *value > 0);
        let to_ts = query.to_ts.filter(|value| *value > 0);
        let permanent_only = query.permanent_only.map(bool_to_int);
        self.with_connection(|conn| {
            let count = conn.query_row(
                r#"
                SELECT COUNT(*)
                FROM spot_whale_signals
                WHERE (?1 IS NULL OR symbol = ?1)
                  AND (?2 IS NULL OR LOWER(REPLACE(severity, '_', '')) = ?2)
                  AND (?3 IS NULL OR LOWER(REPLACE(signal_type, '_', '')) = ?3)
                  AND (?4 IS NULL OR discord_sent = ?4)
                  AND (?5 IS NULL OR ABS(net_volume_base) >= ?5)
                  AND (?6 IS NULL OR ts >= ?6)
                  AND (?7 IS NULL OR ts < ?7)
                  AND (?8 IS NULL OR is_permanent = ?8)
                "#,
                params![
                    query.symbol.as_deref(),
                    severity.as_deref(),
                    signal_type.as_deref(),
                    discord_sent,
                    min_abs_net_volume_base,
                    from_ts,
                    to_ts,
                    permanent_only,
                ],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(count.max(0) as usize)
        })
    }

    fn update_spot_whale_discord_status(
        &self,
        signal_id: &str,
        sent: bool,
        sent_at_ms: Option<i64>,
        reason: &str,
    ) -> anyhow::Result<usize> {
        self.with_write_connection(|conn| {
            let payload = conn
                .query_row(
                    "SELECT payload_json, is_permanent FROM spot_whale_signals WHERE signal_id = ?1",
                    params![signal_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?;
            let updated_payload = payload
                .map(|(json, is_permanent)| {
                    let mut signal: SpotWhaleSignal = serde_json::from_str(&json)?;
                    signal.is_permanent = is_permanent != 0;
                    signal.discord_sent = sent;
                    signal.discord_sent_at = sent_at_ms;
                    signal.discord_reason = reason.to_string();
                    serde_json::to_string(&signal)
                })
                .transpose()?;
            let changed = conn
                .execute(
                    r#"
                    UPDATE spot_whale_signals
                    SET discord_sent = ?2,
                        discord_sent_at = ?3,
                        discord_reason = ?4,
                        payload_json = COALESCE(?5, payload_json)
                    WHERE signal_id = ?1
                    "#,
                    params![
                        signal_id,
                        bool_to_int(sent),
                        sent_at_ms,
                        reason,
                        updated_payload,
                    ],
                )
                .context("failed to update spot whale discord status")?;
            Ok(changed)
        })
    }

    fn count_spot_whale_signals(&self, symbol: &str) -> anyhow::Result<usize> {
        self.with_connection(|conn| {
            let count = conn.query_row(
                "SELECT COUNT(*) FROM spot_whale_signals WHERE symbol = ?1",
                params![symbol],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(count.max(0) as usize)
        })
    }

    fn prune_spot_whale_signals_older_than(&self, cutoff_ts: i64) -> anyhow::Result<usize> {
        self.with_write_connection(|conn| {
            let changed = conn
                .execute(
                    r#"
                    DELETE FROM spot_whale_signals
                    WHERE ts < ?1
                      AND CASE
                        WHEN UPPER(TRIM(symbol)) = 'ETH'
                          THEN ABS(net_volume_base) < ?2
                        ELSE ABS(net_volume_base) < ?3
                      END
                    "#,
                    params![
                        cutoff_ts,
                        SPOT_WHALE_ETH_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
                        SPOT_WHALE_BTC_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
                    ],
                )
                .context("failed to prune spot whale signals")?;
            Ok(changed)
        })
    }
}

fn decode_signal_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SpotWhaleSignal> {
    let payload_json: String = row.get(0)?;
    let mut signal = serde_json::from_str::<SpotWhaleSignal>(&payload_json)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    signal.discord_eligible = row.get::<_, i64>(1)? != 0;
    signal.discord_sent = row.get::<_, i64>(2)? != 0;
    signal.discord_sent_at = row.get(3)?;
    signal.discord_reason = row.get(4)?;
    signal.is_permanent = row.get::<_, i64>(5)? != 0;
    Ok(signal)
}

fn canonicalize_signal(signal: &SpotWhaleSignal) -> SpotWhaleSignal {
    let mut signal = signal.clone();
    signal.is_permanent = is_permanent_spot_whale_signal(&signal.symbol, signal.net_volume_base);
    signal
}

fn compact_filter_value(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != '_')
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
