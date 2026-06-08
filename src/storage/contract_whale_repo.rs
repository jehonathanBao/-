use anyhow::Context;
use rusqlite::{params, OptionalExtension};

use crate::contract_whale_monitor::types::{
    ContractExchange, ContractFlowBucket, ContractFundingSnapshot, ContractLiquidationBucket,
    ContractOiSnapshot, ContractWhaleDirection, ContractWhalePercentileThreshold,
    ContractWhaleSeverity, ContractWhaleSignal, ContractWhaleSignalType,
};

use super::sqlite::SqliteStore;

#[derive(Debug, Clone, Default)]
pub struct ContractWhaleSignalQuery {
    pub symbol: Option<String>,
    pub severity: Option<ContractWhaleSeverity>,
    pub signal_type: Option<ContractWhaleSignalType>,
    pub direction: Option<ContractWhaleDirection>,
    pub discord_sent: Option<bool>,
    pub window_sec: Option<u64>,
    pub exchange: Option<String>,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub limit: usize,
    pub offset: usize,
}

pub trait ContractWhaleRepo {
    fn upsert_contract_flow_buckets(&self, buckets: &[ContractFlowBucket])
        -> anyhow::Result<usize>;
    fn list_recent_contract_flow_buckets(
        &self,
        symbol: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<ContractFlowBucket>>;
    fn list_contract_flow_buckets_between(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractFlowBucket>>;
    fn upsert_contract_liquidation_buckets(
        &self,
        buckets: &[ContractLiquidationBucket],
    ) -> anyhow::Result<usize>;
    fn list_contract_liquidation_buckets_between(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractLiquidationBucket>>;
    fn upsert_contract_oi_snapshots(
        &self,
        snapshots: &[ContractOiSnapshot],
    ) -> anyhow::Result<usize>;
    fn list_contract_oi_snapshots_between(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractOiSnapshot>>;
    fn upsert_contract_funding_snapshots(
        &self,
        snapshots: &[ContractFundingSnapshot],
    ) -> anyhow::Result<usize>;
    fn list_contract_funding_snapshots_between(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractFundingSnapshot>>;
    fn upsert_contract_whale_signal(&self, signal: &ContractWhaleSignal) -> anyhow::Result<()>;
    fn list_contract_whale_signals(
        &self,
        symbol: &str,
        severity: Option<ContractWhaleSeverity>,
        limit: usize,
    ) -> anyhow::Result<Vec<ContractWhaleSignal>>;
    fn query_contract_whale_signals(
        &self,
        query: &ContractWhaleSignalQuery,
    ) -> anyhow::Result<Vec<ContractWhaleSignal>>;
    fn update_contract_whale_discord_status(
        &self,
        signal_id: &str,
        sent: bool,
        sent_at_ms: Option<i64>,
    ) -> anyhow::Result<usize>;
    fn upsert_contract_whale_percentiles(
        &self,
        thresholds: &[ContractWhalePercentileThreshold],
    ) -> anyhow::Result<usize>;
    fn latest_contract_whale_percentile(
        &self,
        symbol: &str,
        exchange: &str,
        window_sec: u64,
    ) -> anyhow::Result<Option<ContractWhalePercentileThreshold>>;
    fn prune_contract_whale_retention(
        &self,
        flow_cutoff_ts: i64,
        signal_cutoff_ts: i64,
    ) -> anyhow::Result<ContractWhaleRetentionPruneResult>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContractWhaleRetentionPruneResult {
    pub flow_1s_deleted: usize,
    pub signal_deleted: usize,
}

impl ContractWhaleRepo for SqliteStore {
    fn upsert_contract_flow_buckets(
        &self,
        buckets: &[ContractFlowBucket],
    ) -> anyhow::Result<usize> {
        if buckets.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut written = 0;
            {
                let mut stmt = tx.prepare(
                    r#"
                    INSERT INTO contract_flow_1s (
                      ts_bucket, exchange, symbol, buy_volume_btc, sell_volume_btc,
                      buy_notional_usd, sell_notional_usd, trade_count,
                      max_single_trade_btc, vwap, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                    ON CONFLICT(ts_bucket, exchange, symbol) DO UPDATE SET
                      buy_volume_btc = excluded.buy_volume_btc,
                      sell_volume_btc = excluded.sell_volume_btc,
                      buy_notional_usd = excluded.buy_notional_usd,
                      sell_notional_usd = excluded.sell_notional_usd,
                      trade_count = excluded.trade_count,
                      max_single_trade_btc = excluded.max_single_trade_btc,
                      vwap = excluded.vwap,
                      created_at = excluded.created_at
                    "#,
                )?;
                let now = crate::normalizers::trade::now_ms();
                for bucket in buckets {
                    stmt.execute(params![
                        bucket.ts_bucket,
                        bucket.exchange,
                        bucket.symbol,
                        bucket.buy_volume_btc,
                        bucket.sell_volume_btc,
                        bucket.buy_notional_usd,
                        bucket.sell_notional_usd,
                        bucket.trade_count as i64,
                        bucket.max_single_trade_btc,
                        bucket.vwap,
                        now,
                    ])
                    .context("failed to upsert contract flow 1s bucket")?;
                    written += 1;
                }
            }
            tx.commit()?;
            Ok(written)
        })
    }

    fn list_recent_contract_flow_buckets(
        &self,
        symbol: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<ContractFlowBucket>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT ts_bucket, exchange, symbol, buy_volume_btc, sell_volume_btc,
                       buy_notional_usd, sell_notional_usd, trade_count,
                       max_single_trade_btc, vwap
                FROM contract_flow_1s
                WHERE symbol = ?1
                ORDER BY ts_bucket DESC
                LIMIT ?2
                "#,
            )?;
            let rows = stmt.query_map(params![symbol, limit as i64], |row| {
                Ok(ContractFlowBucket {
                    ts_bucket: row.get(0)?,
                    exchange: row.get(1)?,
                    symbol: row.get(2)?,
                    buy_volume_btc: row.get(3)?,
                    sell_volume_btc: row.get(4)?,
                    buy_notional_usd: row.get(5)?,
                    sell_notional_usd: row.get(6)?,
                    trade_count: row.get::<_, i64>(7)?.max(0) as u64,
                    max_single_trade_btc: row.get::<_, Option<f64>>(8)?.unwrap_or(0.0),
                    vwap: row.get(9)?,
                })
            })?;
            let mut buckets = Vec::new();
            for row in rows {
                buckets.push(row?);
            }
            Ok(buckets)
        })
    }

    fn list_contract_flow_buckets_between(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractFlowBucket>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT ts_bucket, exchange, symbol, buy_volume_btc, sell_volume_btc,
                       buy_notional_usd, sell_notional_usd, trade_count,
                       max_single_trade_btc, vwap
                FROM contract_flow_1s
                WHERE symbol = ?1
                  AND ts_bucket >= ?2
                  AND ts_bucket <= ?3
                ORDER BY ts_bucket ASC
                "#,
            )?;
            let rows = stmt.query_map(params![symbol, from_ts, to_ts], |row| {
                Ok(ContractFlowBucket {
                    ts_bucket: row.get(0)?,
                    exchange: row.get(1)?,
                    symbol: row.get(2)?,
                    buy_volume_btc: row.get(3)?,
                    sell_volume_btc: row.get(4)?,
                    buy_notional_usd: row.get(5)?,
                    sell_notional_usd: row.get(6)?,
                    trade_count: row.get::<_, i64>(7)?.max(0) as u64,
                    max_single_trade_btc: row.get::<_, Option<f64>>(8)?.unwrap_or(0.0),
                    vwap: row.get(9)?,
                })
            })?;
            let mut buckets = Vec::new();
            for row in rows {
                buckets.push(row?);
            }
            Ok(buckets)
        })
    }

    fn upsert_contract_liquidation_buckets(
        &self,
        buckets: &[ContractLiquidationBucket],
    ) -> anyhow::Result<usize> {
        if buckets.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut written = 0;
            {
                let mut stmt = tx.prepare(
                    r#"
                    INSERT INTO contract_liquidation_1s (
                      ts_bucket, exchange, symbol, long_liq_btc, short_liq_btc,
                      liq_notional_usd, order_count, max_single_liq_btc, vwap, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    ON CONFLICT(ts_bucket, exchange, symbol) DO UPDATE SET
                      long_liq_btc = excluded.long_liq_btc,
                      short_liq_btc = excluded.short_liq_btc,
                      liq_notional_usd = excluded.liq_notional_usd,
                      order_count = excluded.order_count,
                      max_single_liq_btc = excluded.max_single_liq_btc,
                      vwap = excluded.vwap,
                      created_at = excluded.created_at
                    "#,
                )?;
                let now = crate::normalizers::trade::now_ms();
                for bucket in buckets {
                    stmt.execute(params![
                        bucket.ts_bucket,
                        bucket.exchange,
                        bucket.symbol,
                        bucket.long_liq_btc,
                        bucket.short_liq_btc,
                        bucket.liq_notional_usd,
                        bucket.order_count as i64,
                        bucket.max_single_liq_btc,
                        bucket.vwap,
                        now,
                    ])
                    .context("failed to upsert contract liquidation 1s bucket")?;
                    written += 1;
                }
            }
            tx.commit()?;
            Ok(written)
        })
    }

    fn list_contract_liquidation_buckets_between(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractLiquidationBucket>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT ts_bucket, exchange, symbol, long_liq_btc, short_liq_btc,
                       liq_notional_usd, order_count, max_single_liq_btc, vwap
                FROM contract_liquidation_1s
                WHERE symbol = ?1
                  AND ts_bucket >= ?2
                  AND ts_bucket <= ?3
                ORDER BY ts_bucket ASC
                "#,
            )?;
            let rows = stmt.query_map(params![symbol, from_ts, to_ts], |row| {
                Ok(ContractLiquidationBucket {
                    ts_bucket: row.get(0)?,
                    exchange: row.get(1)?,
                    symbol: row.get(2)?,
                    long_liq_btc: row.get(3)?,
                    short_liq_btc: row.get(4)?,
                    liq_notional_usd: row.get(5)?,
                    order_count: row.get::<_, i64>(6)?.max(0) as u64,
                    max_single_liq_btc: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
                    vwap: row.get(8)?,
                })
            })?;
            let mut buckets = Vec::new();
            for row in rows {
                buckets.push(row?);
            }
            Ok(buckets)
        })
    }

    fn upsert_contract_oi_snapshots(
        &self,
        snapshots: &[ContractOiSnapshot],
    ) -> anyhow::Result<usize> {
        if snapshots.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut written = 0;
            {
                let mut stmt = tx.prepare(
                    r#"
                    INSERT INTO contract_oi_snapshots (
                      ts, exchange, symbol, oi_btc, oi_notional_usd, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    ON CONFLICT(ts, exchange, symbol) DO UPDATE SET
                      oi_btc = excluded.oi_btc,
                      oi_notional_usd = excluded.oi_notional_usd,
                      created_at = excluded.created_at
                    "#,
                )?;
                let now = crate::normalizers::trade::now_ms();
                for snapshot in snapshots {
                    stmt.execute(params![
                        snapshot.ts,
                        snapshot.exchange.as_key(),
                        snapshot.symbol,
                        snapshot.oi_btc,
                        snapshot.oi_notional_usd,
                        now,
                    ])
                    .context("failed to upsert contract oi snapshot")?;
                    written += 1;
                }
            }
            tx.commit()?;
            Ok(written)
        })
    }

    fn list_contract_oi_snapshots_between(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractOiSnapshot>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT ts, exchange, symbol, oi_btc, oi_notional_usd
                FROM contract_oi_snapshots
                WHERE symbol = ?1
                  AND ts >= ?2
                  AND ts <= ?3
                ORDER BY ts ASC
                "#,
            )?;
            let rows = stmt.query_map(params![symbol, from_ts, to_ts], |row| {
                Ok(ContractOiSnapshot {
                    ts: row.get(0)?,
                    exchange: exchange_from_key(row.get::<_, String>(1)?.as_str()),
                    symbol: row.get(2)?,
                    oi_btc: row.get(3)?,
                    oi_notional_usd: row.get(4)?,
                })
            })?;
            let mut snapshots = Vec::new();
            for row in rows {
                snapshots.push(row?);
            }
            Ok(snapshots)
        })
    }

    fn upsert_contract_funding_snapshots(
        &self,
        snapshots: &[ContractFundingSnapshot],
    ) -> anyhow::Result<usize> {
        if snapshots.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut written = 0;
            {
                let mut stmt = tx.prepare(
                    r#"
                    INSERT INTO contract_funding_snapshots (
                      ts, exchange, symbol, funding_rate, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5)
                    ON CONFLICT(ts, exchange, symbol) DO UPDATE SET
                      funding_rate = excluded.funding_rate,
                      created_at = excluded.created_at
                    "#,
                )?;
                let now = crate::normalizers::trade::now_ms();
                for snapshot in snapshots {
                    stmt.execute(params![
                        snapshot.ts,
                        snapshot.exchange.as_key(),
                        snapshot.symbol,
                        snapshot.funding_rate,
                        now,
                    ])
                    .context("failed to upsert contract funding snapshot")?;
                    written += 1;
                }
            }
            tx.commit()?;
            Ok(written)
        })
    }

    fn list_contract_funding_snapshots_between(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractFundingSnapshot>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT ts, exchange, symbol, funding_rate
                FROM contract_funding_snapshots
                WHERE symbol = ?1
                  AND ts >= ?2
                  AND ts <= ?3
                ORDER BY ts ASC
                "#,
            )?;
            let rows = stmt.query_map(params![symbol, from_ts, to_ts], |row| {
                Ok(ContractFundingSnapshot {
                    ts: row.get(0)?,
                    exchange: exchange_from_key(row.get::<_, String>(1)?.as_str()),
                    symbol: row.get(2)?,
                    funding_rate: row.get(3)?,
                })
            })?;
            let mut snapshots = Vec::new();
            for row in rows {
                snapshots.push(row?);
            }
            Ok(snapshots)
        })
    }

    fn upsert_contract_whale_signal(&self, signal: &ContractWhaleSignal) -> anyhow::Result<()> {
        let signal_type = enum_value(signal.signal_type)?;
        let direction = enum_value(signal.direction)?;
        let severity = enum_value(signal.severity)?;
        let exchanges_json = serde_json::to_string(&signal.exchanges)?;
        let payload_json = serde_json::to_string(signal)?;
        self.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO contract_whale_signals (
                  signal_id, ts, symbol, window_sec, signal_type, direction, severity, score,
                  total_volume_btc, net_volume_btc, total_notional_usd, dominance,
                  price_start, price_end, price_move_pct, main_exchange, exchanges_json,
                  dynamic_multiple, data_quality, discord_eligible, discord_sent,
                  discord_sent_at, payload_json, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                          NULL, NULL, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
                ON CONFLICT(signal_id) DO UPDATE SET
                  ts = excluded.ts,
                  symbol = excluded.symbol,
                  window_sec = excluded.window_sec,
                  signal_type = excluded.signal_type,
                  direction = excluded.direction,
                  severity = excluded.severity,
                  score = excluded.score,
                  total_volume_btc = excluded.total_volume_btc,
                  net_volume_btc = excluded.net_volume_btc,
                  total_notional_usd = excluded.total_notional_usd,
                  dominance = excluded.dominance,
                  price_move_pct = excluded.price_move_pct,
                  main_exchange = excluded.main_exchange,
                  exchanges_json = excluded.exchanges_json,
                  dynamic_multiple = excluded.dynamic_multiple,
                  data_quality = excluded.data_quality,
                  discord_eligible = excluded.discord_eligible,
                  discord_sent = excluded.discord_sent,
                  discord_sent_at = excluded.discord_sent_at,
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
                    signal.total_volume_btc,
                    signal.net_volume_btc,
                    signal.total_notional_usd,
                    signal.dominance,
                    signal.price_move_pct,
                    signal.main_exchange,
                    exchanges_json,
                    signal.dynamic_multiple,
                    signal.data_quality as i64,
                    bool_to_int(signal.discord_eligible),
                    bool_to_int(signal.discord_sent),
                    signal.discord_sent_at,
                    payload_json,
                    crate::normalizers::trade::now_ms(),
                ],
            )
            .context("failed to upsert contract whale signal")?;
            Ok(())
        })
    }

    fn list_contract_whale_signals(
        &self,
        symbol: &str,
        severity: Option<ContractWhaleSeverity>,
        limit: usize,
    ) -> anyhow::Result<Vec<ContractWhaleSignal>> {
        self.query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some(symbol.to_string()),
            severity,
            limit,
            ..ContractWhaleSignalQuery::default()
        })
    }

    fn query_contract_whale_signals(
        &self,
        query: &ContractWhaleSignalQuery,
    ) -> anyhow::Result<Vec<ContractWhaleSignal>> {
        let severity = query.severity.map(enum_value).transpose()?;
        let signal_type = query.signal_type.map(enum_value).transpose()?;
        let direction = query.direction.map(enum_value).transpose()?;
        let discord_sent = query.discord_sent.map(bool_to_int);
        let window_sec = query.window_sec.map(|window_sec| window_sec as i64);
        let exchange_like = query
            .exchange
            .as_deref()
            .map(|exchange| format!("%\"exchange\":\"{}\"%", exchange.to_ascii_lowercase()));
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT payload_json, discord_eligible, discord_sent, discord_sent_at
                FROM contract_whale_signals
                WHERE (?1 IS NULL OR symbol = ?1)
                  AND (?2 IS NULL OR severity = ?2)
                  AND (?3 IS NULL OR signal_type = ?3)
                  AND (?4 IS NULL OR direction = ?4)
                  AND (?5 IS NULL OR ts >= ?5)
                  AND (?6 IS NULL OR ts <= ?6)
                  AND (?7 IS NULL OR discord_sent = ?7)
                  AND (?8 IS NULL OR window_sec = ?8)
                  AND (?9 IS NULL OR exchanges_json LIKE ?9)
                ORDER BY ts DESC
                LIMIT ?10 OFFSET ?11
                "#,
            )?;
            let rows = stmt.query_map(
                params![
                    query.symbol.as_deref(),
                    severity.as_deref(),
                    signal_type.as_deref(),
                    direction.as_deref(),
                    query.from_ts,
                    query.to_ts,
                    discord_sent,
                    window_sec,
                    exchange_like.as_deref(),
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

    fn update_contract_whale_discord_status(
        &self,
        signal_id: &str,
        sent: bool,
        sent_at_ms: Option<i64>,
    ) -> anyhow::Result<usize> {
        self.with_connection(|conn| {
            let payload = conn
                .query_row(
                    "SELECT payload_json FROM contract_whale_signals WHERE signal_id = ?1",
                    params![signal_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            let updated_payload = payload
                .map(|json| {
                    let mut signal: ContractWhaleSignal = serde_json::from_str(&json)?;
                    signal.discord_sent = sent;
                    signal.discord_sent_at = sent_at_ms;
                    serde_json::to_string(&signal)
                })
                .transpose()?;
            let changed = conn
                .execute(
                    r#"
                    UPDATE contract_whale_signals
                    SET discord_sent = ?2,
                        discord_sent_at = ?3,
                        payload_json = COALESCE(?4, payload_json)
                    WHERE signal_id = ?1
                    "#,
                    params![signal_id, bool_to_int(sent), sent_at_ms, updated_payload,],
                )
                .context("failed to update contract whale discord status")?;
            Ok(changed)
        })
    }

    fn upsert_contract_whale_percentiles(
        &self,
        thresholds: &[ContractWhalePercentileThreshold],
    ) -> anyhow::Result<usize> {
        if thresholds.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut written = 0;
            {
                let mut stmt = tx.prepare(
                    r#"
                    INSERT INTO contract_whale_percentile_thresholds (
                      computed_at, symbol, exchange, window_sec,
                      p99_0_btc, p99_5_btc, p99_9_btc, sample_count, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    ON CONFLICT(symbol, exchange, window_sec, computed_at) DO UPDATE SET
                      p99_0_btc = excluded.p99_0_btc,
                      p99_5_btc = excluded.p99_5_btc,
                      p99_9_btc = excluded.p99_9_btc,
                      sample_count = excluded.sample_count,
                      created_at = excluded.created_at
                    "#,
                )?;
                let now = crate::normalizers::trade::now_ms();
                for threshold in thresholds {
                    stmt.execute(params![
                        threshold.computed_at,
                        threshold.symbol,
                        threshold.exchange,
                        threshold.window_sec as i64,
                        threshold.p99_0_btc,
                        threshold.p99_5_btc,
                        threshold.p99_9_btc,
                        threshold.sample_count as i64,
                        now,
                    ])
                    .context("failed to upsert contract whale percentile threshold")?;
                    written += 1;
                }
            }
            tx.commit()?;
            Ok(written)
        })
    }

    fn latest_contract_whale_percentile(
        &self,
        symbol: &str,
        exchange: &str,
        window_sec: u64,
    ) -> anyhow::Result<Option<ContractWhalePercentileThreshold>> {
        self.with_connection(|conn| {
            conn.query_row(
                r#"
                SELECT computed_at, symbol, exchange, window_sec,
                       p99_0_btc, p99_5_btc, p99_9_btc, sample_count
                FROM contract_whale_percentile_thresholds
                WHERE symbol = ?1
                  AND exchange = ?2
                  AND window_sec = ?3
                ORDER BY computed_at DESC
                LIMIT 1
                "#,
                params![symbol, exchange, window_sec as i64],
                |row| {
                    Ok(ContractWhalePercentileThreshold {
                        computed_at: row.get(0)?,
                        symbol: row.get(1)?,
                        exchange: row.get(2)?,
                        window_sec: row.get::<_, i64>(3)?.max(0) as u64,
                        p99_0_btc: row.get(4)?,
                        p99_5_btc: row.get(5)?,
                        p99_9_btc: row.get(6)?,
                        sample_count: row.get::<_, i64>(7)?.max(0) as usize,
                    })
                },
            )
            .optional()
            .context("failed to load latest contract whale percentile threshold")
        })
    }

    fn prune_contract_whale_retention(
        &self,
        flow_cutoff_ts: i64,
        signal_cutoff_ts: i64,
    ) -> anyhow::Result<ContractWhaleRetentionPruneResult> {
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let flow_1s_deleted = tx
                .execute(
                    "DELETE FROM contract_flow_1s WHERE ts_bucket < ?1",
                    params![flow_cutoff_ts],
                )
                .context("failed to prune contract flow 1s buckets")?;
            let signal_deleted = tx
                .execute(
                    "DELETE FROM contract_whale_signals WHERE ts < ?1",
                    params![signal_cutoff_ts],
                )
                .context("failed to prune contract whale signals")?;
            tx.commit()?;
            Ok(ContractWhaleRetentionPruneResult {
                flow_1s_deleted,
                signal_deleted,
            })
        })
    }
}

fn decode_signal_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContractWhaleSignal> {
    let payload_json: String = row.get(0)?;
    let mut signal = serde_json::from_str::<ContractWhaleSignal>(&payload_json)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    signal.discord_eligible = row.get::<_, i64>(1)? != 0;
    signal.discord_sent = row.get::<_, i64>(2)? != 0;
    signal.discord_sent_at = row.get(3)?;
    Ok(signal)
}

fn enum_value<T: serde::Serialize>(value: T) -> anyhow::Result<String> {
    Ok(serde_json::to_value(value)?
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}

fn exchange_from_key(value: &str) -> ContractExchange {
    match value.to_ascii_lowercase().as_str() {
        "okx" => ContractExchange::Okx,
        "bitfinex" => ContractExchange::Bitfinex,
        _ => ContractExchange::Binance,
    }
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
