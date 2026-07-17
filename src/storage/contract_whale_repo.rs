use std::collections::BTreeMap;

use anyhow::Context;
use rusqlite::{params, OptionalExtension};

use crate::contract_whale_monitor::outcome_calibration::ContractWhaleSignalOutcome;
use crate::contract_whale_monitor::types::{
    ContractExchange, ContractFlowBucket, ContractFundingSnapshot, ContractLiquidationBucket,
    ContractOiSnapshot, ContractWhaleActiveSources, ContractWhaleDirection,
    ContractWhaleEmissionFingerprint, ContractWhaleMarketType, ContractWhaleOiExchangeDelta,
    ContractWhaleOiWindowContext, ContractWhalePercentileThreshold, ContractWhaleSeverity,
    ContractWhaleSignal, ContractWhaleSignalType, ContractWhaleSourceRole,
};

use super::{
    sqlite::{column_exists, table_exists, SqliteStore},
    storage_health::{
        classify_retention_error, RetentionTableResult, RetentionTableStatus, WalCheckpointResult,
    },
};

pub const CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC: f64 = 500.0;

#[derive(Debug, Clone, Default)]
pub struct ContractWhaleSignalQuery {
    pub symbol: Option<String>,
    pub severity: Option<ContractWhaleSeverity>,
    pub signal_type: Option<ContractWhaleSignalType>,
    pub direction: Option<ContractWhaleDirection>,
    pub discord_sent: Option<bool>,
    pub window_sec: Option<u64>,
    pub exchange: Option<String>,
    pub min_abs_net_volume_btc: Option<f64>,
    pub impact_level: Option<String>,
    pub min_notional_usd: Option<f64>,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub cursor_ts: Option<i64>,
    pub cursor_signal_id: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContractWhaleSignalQueryPath {
    LatestBySymbol,
    EventFeed,
    General,
}

fn contract_whale_signal_query_path(
    query: &ContractWhaleSignalQuery,
) -> ContractWhaleSignalQueryPath {
    let min_notional_usd = query
        .min_notional_usd
        .filter(|value| value.is_finite() && *value > 0.0);
    let has_optional_filters = query.severity.is_some()
        || query.signal_type.is_some()
        || query.direction.is_some()
        || query.discord_sent.is_some()
        || query.window_sec.is_some()
        || query.exchange.is_some()
        || query.min_abs_net_volume_btc.is_some()
        || query.impact_level.is_some();
    let has_positioned_cursor = query.cursor_ts.is_some() || query.cursor_signal_id.is_some();

    if query.symbol.is_some()
        && !has_optional_filters
        && min_notional_usd.is_none()
        && query.from_ts.is_none()
        && query.to_ts.is_none()
        && !has_positioned_cursor
    {
        ContractWhaleSignalQueryPath::LatestBySymbol
    } else if query.symbol.is_some()
        && query.from_ts.is_some()
        && min_notional_usd.is_some()
        && !has_optional_filters
        && !has_positioned_cursor
    {
        ContractWhaleSignalQueryPath::EventFeed
    } else {
        ContractWhaleSignalQueryPath::General
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractWhaleDiscordOutboxStatus {
    Pending,
    Sending,
    Sent,
    DryRun,
    Retry,
    Dead,
}

impl ContractWhaleDiscordOutboxStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sending => "sending",
            Self::Sent => "sent",
            Self::DryRun => "dry_run",
            Self::Retry => "retry",
            Self::Dead => "dead",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleDiscordOutboxItem {
    pub signal_id: String,
    pub signal: ContractWhaleSignal,
    pub attempts: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractWhaleDiscordOutboxStats {
    pub pending: usize,
    pub retrying: usize,
    pub failed: usize,
    pub oldest_pending_age_sec: i64,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleOutcomeSummaryRow {
    pub symbol: String,
    pub signal_type: String,
    pub classification_v2: String,
    pub severity: String,
    pub impact_level: Option<String>,
    pub window_sec: u64,
    pub oi_context: String,
    pub regime: String,
    pub hour_utc: String,
    pub sample_count: usize,
    pub avg_absolute_return_30s_bps: Option<f64>,
    pub avg_absolute_return_2m_bps: Option<f64>,
    pub avg_absolute_return_5m_bps: Option<f64>,
    pub avg_realized_volatility_5m_bps: Option<f64>,
    pub avg_max_absolute_excursion_5m_bps: Option<f64>,
    pub avg_price_sample_count_5m: Option<f64>,
    pub avg_markout_30s_bps: Option<f64>,
    pub avg_markout_2m_bps: Option<f64>,
    pub avg_markout_5m_bps: Option<f64>,
    pub follow_through_30s_rate: Option<f64>,
    pub follow_through_2m_rate: Option<f64>,
    pub follow_through_5m_rate: Option<f64>,
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
    fn load_oi_snapshots_for_range(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractOiSnapshot>>;
    fn find_oi_context_for_event(
        &self,
        symbol: &str,
        event_ts: i64,
        window_sec: i64,
        max_gap_sec: i64,
    ) -> anyhow::Result<ContractWhaleOiWindowContext>;
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
    fn upsert_contract_whale_signals(
        &self,
        signals: &[ContractWhaleSignal],
    ) -> anyhow::Result<usize>;
    fn upsert_contract_whale_signals_with_outbox(
        &self,
        signals: &[ContractWhaleSignal],
        outbox_signals: &[ContractWhaleSignal],
        now_ms: i64,
    ) -> anyhow::Result<(usize, usize)>;
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
    fn enqueue_contract_whale_discord_outbox(
        &self,
        signals: &[ContractWhaleSignal],
        now_ms: i64,
    ) -> anyhow::Result<usize>;
    fn claim_contract_whale_discord_outbox(
        &self,
        limit: usize,
        now_ms: i64,
    ) -> anyhow::Result<Vec<ContractWhaleDiscordOutboxItem>>;
    fn finish_contract_whale_discord_outbox(
        &self,
        signal_id: &str,
        status: ContractWhaleDiscordOutboxStatus,
        next_attempt_at: Option<i64>,
        sent_at: Option<i64>,
        last_error: Option<&str>,
    ) -> anyhow::Result<usize>;
    fn contract_whale_discord_outbox_stats(
        &self,
        now_ms: i64,
    ) -> anyhow::Result<ContractWhaleDiscordOutboxStats>;
    fn load_contract_whale_emission_watermarks(
        &self,
    ) -> anyhow::Result<BTreeMap<String, ContractWhaleEmissionFingerprint>>;
    fn upsert_contract_whale_emission_watermarks(
        &self,
        watermarks: &BTreeMap<String, ContractWhaleEmissionFingerprint>,
    ) -> anyhow::Result<usize>;
    fn upsert_contract_whale_signal_outcomes(
        &self,
        outcomes: &[ContractWhaleSignalOutcome],
    ) -> anyhow::Result<usize>;
    fn contract_whale_outcome_summary(
        &self,
        outcome_version: &str,
    ) -> anyhow::Result<Vec<ContractWhaleOutcomeSummaryRow>>;
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
        impact_b_cutoff_ts: i64,
    ) -> anyhow::Result<ContractWhaleRetentionPruneResult>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractWhaleRetentionPruneResult {
    pub flow_1s_deleted: usize,
    pub liquidation_deleted: usize,
    pub oi_deleted: usize,
    pub funding_deleted: usize,
    pub percentile_deleted: usize,
    pub signal_deleted: usize,
    pub flow_cutoff_ts: i64,
    pub signal_cutoff_ts: i64,
    pub impact_b_cutoff_ts: i64,
    pub protected_s_count: usize,
    pub protected_net_volume_count: usize,
    pub table_results: Vec<RetentionTableResult>,
    pub wal_checkpoint: Option<WalCheckpointResult>,
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
                      market_type, source_role, product_id,
                      buy_notional_usd, sell_notional_usd, trade_count,
                      max_single_trade_btc, vwap, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                    ON CONFLICT(ts_bucket, exchange, symbol, market_type) DO UPDATE SET
                      buy_volume_btc = excluded.buy_volume_btc,
                      sell_volume_btc = excluded.sell_volume_btc,
                      source_role = excluded.source_role,
                      product_id = excluded.product_id,
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
                    let market_type = enum_value(bucket.market_type)?;
                    let source_role = enum_value(bucket.source_role)?;
                    stmt.execute(params![
                        bucket.ts_bucket,
                        bucket.exchange,
                        bucket.symbol,
                        bucket.buy_volume_btc,
                        bucket.sell_volume_btc,
                        market_type,
                        source_role,
                        bucket.product_id,
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
                       market_type, source_role, product_id,
                       buy_notional_usd, sell_notional_usd, trade_count,
                       max_single_trade_btc, vwap
                FROM contract_flow_1s
                WHERE symbol = ?1
                  AND market_type = 'perp'
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
                    market_type: market_type_from_key(row.get::<_, String>(5)?.as_str()),
                    source_role: source_role_from_key(row.get::<_, String>(6)?.as_str()),
                    product_id: row.get(7)?,
                    buy_notional_usd: row.get(8)?,
                    sell_notional_usd: row.get(9)?,
                    trade_count: row.get::<_, i64>(10)?.max(0) as u64,
                    max_single_trade_btc: row.get::<_, Option<f64>>(11)?.unwrap_or(0.0),
                    vwap: row.get(12)?,
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
                       market_type, source_role, product_id,
                       buy_notional_usd, sell_notional_usd, trade_count,
                       max_single_trade_btc, vwap
                FROM contract_flow_1s
                WHERE symbol = ?1
                  AND market_type = 'perp'
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
                    market_type: market_type_from_key(row.get::<_, String>(5)?.as_str()),
                    source_role: source_role_from_key(row.get::<_, String>(6)?.as_str()),
                    product_id: row.get(7)?,
                    buy_notional_usd: row.get(8)?,
                    sell_notional_usd: row.get(9)?,
                    trade_count: row.get::<_, i64>(10)?.max(0) as u64,
                    max_single_trade_btc: row.get::<_, Option<f64>>(11)?.unwrap_or(0.0),
                    vwap: row.get(12)?,
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
                      ts, exchange, symbol, oi_btc, oi_notional_usd,
                      ct_val_available, evidence_degraded_reason, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ON CONFLICT(ts, exchange, symbol) DO UPDATE SET
                      oi_btc = excluded.oi_btc,
                      oi_notional_usd = excluded.oi_notional_usd,
                      ct_val_available = excluded.ct_val_available,
                      evidence_degraded_reason = excluded.evidence_degraded_reason,
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
                        snapshot.ct_val_available,
                        snapshot.evidence_degraded_reason,
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
                SELECT ts, exchange, symbol, oi_btc, oi_notional_usd,
                       ct_val_available, evidence_degraded_reason
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
                    ct_val_available: row.get::<_, i64>(5)? != 0,
                    evidence_degraded_reason: row.get(6)?,
                })
            })?;
            let mut snapshots = Vec::new();
            for row in rows {
                snapshots.push(row?);
            }
            Ok(snapshots)
        })
    }

    fn load_oi_snapshots_for_range(
        &self,
        symbol: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<ContractOiSnapshot>> {
        self.list_contract_oi_snapshots_between(symbol, from_ts, to_ts)
    }

    fn find_oi_context_for_event(
        &self,
        symbol: &str,
        event_ts: i64,
        window_sec: i64,
        max_gap_sec: i64,
    ) -> anyhow::Result<ContractWhaleOiWindowContext> {
        let window_ms = window_sec.max(0).saturating_mul(1000);
        let max_gap_ms = max_gap_sec.max(0).saturating_mul(1000);
        let start_ts = event_ts.saturating_sub(window_ms);
        let snapshots = self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT ts, exchange, symbol, oi_btc, oi_notional_usd,
                       ct_val_available, evidence_degraded_reason
                FROM contract_oi_snapshots
                WHERE symbol = ?1
                  AND ts >= ?2
                  AND ts <= ?3
                ORDER BY exchange ASC, ts ASC
                "#,
            )?;
            let rows = stmt.query_map(
                params![symbol, start_ts.saturating_sub(max_gap_ms), event_ts],
                |row| {
                    Ok(ContractOiSnapshot {
                        ts: row.get(0)?,
                        exchange: exchange_from_key(row.get::<_, String>(1)?.as_str()),
                        symbol: row.get(2)?,
                        oi_btc: row.get(3)?,
                        oi_notional_usd: row.get(4)?,
                        ct_val_available: row.get::<_, i64>(5)? != 0,
                        evidence_degraded_reason: row.get(6)?,
                    })
                },
            )?;
            let mut snapshots = Vec::new();
            for row in rows {
                snapshots.push(row?);
            }
            Ok(snapshots)
        })?;
        Ok(resolve_oi_window_from_snapshots(
            &snapshots,
            start_ts,
            event_ts,
            max_gap_sec,
        ))
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
        self.upsert_contract_whale_signals(std::slice::from_ref(signal))
            .map(|_| ())
    }

    fn upsert_contract_whale_signals(
        &self,
        signals: &[ContractWhaleSignal],
    ) -> anyhow::Result<usize> {
        if signals.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let written = upsert_contract_whale_signals_in_transaction(
                &tx,
                signals,
                crate::normalizers::trade::now_ms(),
            )?;
            tx.commit()?;
            Ok(written)
        })
    }

    fn upsert_contract_whale_signals_with_outbox(
        &self,
        signals: &[ContractWhaleSignal],
        outbox_signals: &[ContractWhaleSignal],
        now_ms: i64,
    ) -> anyhow::Result<(usize, usize)> {
        if signals.is_empty() && outbox_signals.is_empty() {
            return Ok((0, 0));
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let written = upsert_contract_whale_signals_in_transaction(&tx, signals, now_ms)?;
            let mut outbox_stmt = tx.prepare(
                r#"
                INSERT INTO contract_whale_discord_outbox (
                  signal_id, symbol, payload_json, status, attempts, next_attempt_at, created_at
                ) VALUES (?1, ?2, ?3, 'pending', 0, ?4, ?5)
                ON CONFLICT(signal_id) DO NOTHING
                "#,
            )?;
            let mut queued = 0;
            for signal in outbox_signals {
                queued += outbox_stmt.execute(params![
                    signal.id,
                    signal.symbol,
                    serde_json::to_string(signal)?,
                    now_ms,
                    now_ms,
                ])?;
            }
            drop(outbox_stmt);
            tx.commit()?;
            Ok((written, queued))
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
        let query_path = contract_whale_signal_query_path(query);
        if query_path == ContractWhaleSignalQueryPath::LatestBySymbol {
            return self.with_connection(|conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT payload_json, discord_eligible, discord_sent, discord_sent_at,
                           active_sources_json, threshold_profile
                    FROM contract_whale_signals
                    WHERE market_type = 'perp'
                      AND symbol = ?1
                    ORDER BY ts DESC, signal_id DESC
                    LIMIT ?2 OFFSET ?3
                    "#,
                )?;
                let rows = stmt.query_map(
                    params![
                        query.symbol.as_deref(),
                        query.limit as i64,
                        query.offset as i64
                    ],
                    decode_signal_row,
                )?;
                let mut signals = Vec::new();
                for row in rows {
                    signals.push(row?);
                }
                Ok(signals)
            });
        }

        let min_notional_usd = query
            .min_notional_usd
            .filter(|value| value.is_finite() && *value > 0.0);
        if query_path == ContractWhaleSignalQueryPath::EventFeed {
            let symbol = query
                .symbol
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("event-feed query requires symbol"))?;
            let from_ts = query
                .from_ts
                .ok_or_else(|| anyhow::anyhow!("event-feed query requires from_ts"))?;
            let min_notional_usd = min_notional_usd
                .ok_or_else(|| anyhow::anyhow!("event-feed query requires min_notional_usd"))?;
            return self.with_connection(|conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT payload_json, discord_eligible, discord_sent, discord_sent_at,
                           active_sources_json, threshold_profile
                    FROM contract_whale_signals
                    WHERE market_type = 'perp'
                      AND symbol = ?1
                      AND ts >= ?2
                      AND (?3 IS NULL OR ts <= ?3)
                      AND total_notional_usd >= ?4
                    ORDER BY ts DESC, signal_id DESC
                    LIMIT ?5 OFFSET ?6
                    "#,
                )?;
                let rows = stmt.query_map(
                    params![
                        symbol,
                        from_ts,
                        query.to_ts,
                        min_notional_usd,
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
            });
        }

        let severity = query.severity.map(enum_value).transpose()?;
        let signal_type = query.signal_type.map(enum_value).transpose()?;
        let direction = query.direction.map(enum_value).transpose()?;
        let discord_sent = query.discord_sent.map(bool_to_int);
        let window_sec = query.window_sec.map(|window_sec| window_sec as i64);
        let min_abs_net_volume_btc = query
            .min_abs_net_volume_btc
            .filter(|value| value.is_finite() && *value > 0.0);
        let impact_level = query
            .impact_level
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_uppercase());
        let cursor_signal_id = query.cursor_signal_id.as_deref().map(str::to_string);
        let exchange_like = query
            .exchange
            .as_deref()
            .map(|exchange| format!("%\"exchange\":\"{}\"%", exchange.to_ascii_lowercase()));
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT payload_json, discord_eligible, discord_sent, discord_sent_at,
                       active_sources_json, threshold_profile
                FROM contract_whale_signals
                WHERE market_type = 'perp'
                  AND (?1 IS NULL OR symbol = ?1)
                  AND (?2 IS NULL OR severity = ?2)
                  AND (?3 IS NULL OR signal_type = ?3)
                  AND (?4 IS NULL OR direction = ?4)
                  AND (?5 IS NULL OR ts >= ?5)
                  AND (?6 IS NULL OR ts <= ?6)
                  AND (?7 IS NULL OR discord_sent = ?7)
                  AND (?8 IS NULL OR window_sec = ?8)
                  AND (?9 IS NULL OR exchanges_json LIKE ?9)
                  AND (?10 IS NULL OR ABS(net_volume_btc) >= ?10)
                  AND (?11 IS NULL OR total_notional_usd >= ?11)
                  AND (
                        ?12 IS NULL
                        OR UPPER(COALESCE(
                              json_extract(payload_json, '$.impactLevel'),
                              json_extract(payload_json, '$.impact_level'),
                              ''
                            )) = ?12
                  )
                  AND (
                        ?13 IS NULL
                        OR ts < ?13
                        OR (ts = ?13 AND signal_id < ?14)
                  )
                ORDER BY ts DESC, signal_id DESC
                LIMIT ?15 OFFSET ?16
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
                    min_abs_net_volume_btc,
                    min_notional_usd,
                    impact_level.as_deref(),
                    query.cursor_ts,
                    cursor_signal_id.as_deref(),
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

    fn enqueue_contract_whale_discord_outbox(
        &self,
        signals: &[ContractWhaleSignal],
        now_ms: i64,
    ) -> anyhow::Result<usize> {
        if signals.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO contract_whale_discord_outbox (
                  signal_id, symbol, payload_json, status, attempts, next_attempt_at, created_at
                ) VALUES (?1, ?2, ?3, 'pending', 0, ?4, ?5)
                ON CONFLICT(signal_id) DO NOTHING
                "#,
            )?;
            let mut inserted = 0;
            for signal in signals {
                inserted += stmt.execute(params![
                    signal.id,
                    signal.symbol,
                    serde_json::to_string(signal)?,
                    now_ms,
                    now_ms,
                ])?;
            }
            drop(stmt);
            tx.commit()?;
            Ok(inserted)
        })
    }

    fn claim_contract_whale_discord_outbox(
        &self,
        limit: usize,
        now_ms: i64,
    ) -> anyhow::Result<Vec<ContractWhaleDiscordOutboxItem>> {
        let limit = limit.clamp(1, 100) as i64;
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut stmt = tx.prepare(
                r#"
                SELECT signal_id, payload_json, attempts
                FROM contract_whale_discord_outbox
                WHERE status IN ('pending', 'retry')
                  AND (next_attempt_at IS NULL OR next_attempt_at <= ?1)
                ORDER BY created_at ASC, id ASC
                LIMIT ?2
                "#,
            )?;
            let rows = stmt.query_map(params![now_ms, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            let mut claimed = Vec::new();
            for row in rows {
                let (signal_id, payload_json, attempts) = row?;
                let changed = tx.execute(
                    r#"
                    UPDATE contract_whale_discord_outbox
                    SET status = 'sending', attempts = attempts + 1, next_attempt_at = NULL
                    WHERE signal_id = ?1 AND status IN ('pending', 'retry')
                    "#,
                    params![signal_id],
                )?;
                if changed == 1 {
                    claimed.push(ContractWhaleDiscordOutboxItem {
                        signal_id,
                        signal: serde_json::from_str(&payload_json)
                            .context("invalid contract whale discord outbox payload")?,
                        attempts: attempts.max(0) as usize + 1,
                    });
                }
            }
            drop(stmt);
            tx.commit()?;
            Ok(claimed)
        })
    }

    fn finish_contract_whale_discord_outbox(
        &self,
        signal_id: &str,
        status: ContractWhaleDiscordOutboxStatus,
        next_attempt_at: Option<i64>,
        sent_at: Option<i64>,
        last_error: Option<&str>,
    ) -> anyhow::Result<usize> {
        self.with_connection(|conn| {
            conn.execute(
                r#"
                UPDATE contract_whale_discord_outbox
                SET status = ?2,
                    next_attempt_at = ?3,
                    sent_at = ?4,
                    last_error = ?5
                WHERE signal_id = ?1
                "#,
                params![
                    signal_id,
                    status.as_str(),
                    next_attempt_at,
                    sent_at,
                    last_error
                ],
            )
            .context("failed to update contract whale discord outbox")
        })
    }

    fn contract_whale_discord_outbox_stats(
        &self,
        now_ms: i64,
    ) -> anyhow::Result<ContractWhaleDiscordOutboxStats> {
        self.with_connection(|conn| {
            conn.query_row(
                r#"
                SELECT
                  SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END),
                  SUM(CASE WHEN status = 'retry' THEN 1 ELSE 0 END),
                  SUM(CASE WHEN status = 'dead' THEN 1 ELSE 0 END),
                  MIN(CASE WHEN status IN ('pending', 'retry') THEN created_at END)
                FROM contract_whale_discord_outbox
                "#,
                [],
                |row| {
                    let oldest_pending_at = row.get::<_, Option<i64>>(3)?;
                    Ok(ContractWhaleDiscordOutboxStats {
                        pending: row.get::<_, Option<i64>>(0)?.unwrap_or_default().max(0) as usize,
                        retrying: row.get::<_, Option<i64>>(1)?.unwrap_or_default().max(0) as usize,
                        failed: row.get::<_, Option<i64>>(2)?.unwrap_or_default().max(0) as usize,
                        oldest_pending_age_sec: oldest_pending_at
                            .map(|created_at| now_ms.saturating_sub(created_at) / 1000)
                            .unwrap_or_default(),
                    })
                },
            )
            .context("failed to query contract whale discord outbox stats")
        })
    }

    fn load_contract_whale_emission_watermarks(
        &self,
    ) -> anyhow::Result<BTreeMap<String, ContractWhaleEmissionFingerprint>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT emission_key, payload_json FROM contract_whale_emission_watermarks",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut watermarks = BTreeMap::new();
            for row in rows {
                let (key, payload_json) = row?;
                watermarks.insert(
                    key,
                    serde_json::from_str(&payload_json)
                        .context("invalid contract whale emission watermark payload")?,
                );
            }
            Ok(watermarks)
        })
    }

    fn upsert_contract_whale_emission_watermarks(
        &self,
        watermarks: &BTreeMap<String, ContractWhaleEmissionFingerprint>,
    ) -> anyhow::Result<usize> {
        if watermarks.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO contract_whale_emission_watermarks (emission_key, payload_json, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(emission_key) DO UPDATE SET
                  payload_json = excluded.payload_json,
                  updated_at = excluded.updated_at
                "#,
            )?;
            let mut written = 0;
            for (key, fingerprint) in watermarks {
                written += stmt.execute(params![
                    key,
                    serde_json::to_string(fingerprint)?,
                    fingerprint.last_emitted_at,
                ])?;
            }
            drop(stmt);
            tx.commit()?;
            Ok(written)
        })
    }

    fn upsert_contract_whale_signal_outcomes(
        &self,
        outcomes: &[ContractWhaleSignalOutcome],
    ) -> anyhow::Result<usize> {
        if outcomes.is_empty() {
            return Ok(0);
        }
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO contract_whale_signal_outcomes (
                  signal_id, symbol, signal_ts, signal_type, classification_v2, severity,
                  impact_level, window_sec, oi_context, regime, entry_price,
                  markout_30s_bps, markout_2m_bps, markout_5m_bps, mfe_5m_bps, mae_5m_bps,
                  absolute_return_30s_bps, absolute_return_2m_bps, absolute_return_5m_bps,
                  realized_volatility_5m_bps, max_absolute_excursion_5m_bps,
                  price_sample_count_5m, liquidity_recovered_5m, liquidity_recovery_ms,
                  liquidity_recovery_reason, setup_outcome,
                  follow_through_30s, follow_through_2m, follow_through_5m, evaluated_at,
                  outcome_version
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                  ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21,
                  ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31
                )
                ON CONFLICT(signal_id) DO UPDATE SET
                  classification_v2 = excluded.classification_v2,
                  severity = excluded.severity,
                  impact_level = excluded.impact_level,
                  oi_context = excluded.oi_context,
                  regime = excluded.regime,
                  entry_price = excluded.entry_price,
                  markout_30s_bps = excluded.markout_30s_bps,
                  markout_2m_bps = excluded.markout_2m_bps,
                  markout_5m_bps = excluded.markout_5m_bps,
                  mfe_5m_bps = excluded.mfe_5m_bps,
                  mae_5m_bps = excluded.mae_5m_bps,
                  absolute_return_30s_bps = excluded.absolute_return_30s_bps,
                  absolute_return_2m_bps = excluded.absolute_return_2m_bps,
                  absolute_return_5m_bps = excluded.absolute_return_5m_bps,
                  realized_volatility_5m_bps = excluded.realized_volatility_5m_bps,
                  max_absolute_excursion_5m_bps = excluded.max_absolute_excursion_5m_bps,
                  price_sample_count_5m = excluded.price_sample_count_5m,
                  liquidity_recovered_5m = excluded.liquidity_recovered_5m,
                  liquidity_recovery_ms = excluded.liquidity_recovery_ms,
                  liquidity_recovery_reason = excluded.liquidity_recovery_reason,
                  setup_outcome = excluded.setup_outcome,
                  follow_through_30s = excluded.follow_through_30s,
                  follow_through_2m = excluded.follow_through_2m,
                  follow_through_5m = excluded.follow_through_5m,
                  evaluated_at = excluded.evaluated_at,
                  outcome_version = excluded.outcome_version
                "#,
            )?;
            let mut written = 0;
            for outcome in outcomes {
                stmt.execute(params![
                    outcome.signal_id,
                    outcome.symbol,
                    outcome.signal_ts,
                    outcome.signal_type,
                    outcome.classification_v2,
                    outcome.severity,
                    outcome.impact_level,
                    outcome.window_sec as i64,
                    outcome.oi_context,
                    outcome.regime,
                    outcome.entry_price,
                    outcome.markout_30s_bps,
                    outcome.markout_2m_bps,
                    outcome.markout_5m_bps,
                    outcome.mfe_5m_bps,
                    outcome.mae_5m_bps,
                    outcome.absolute_return_30s_bps,
                    outcome.absolute_return_2m_bps,
                    outcome.absolute_return_5m_bps,
                    outcome.realized_volatility_5m_bps,
                    outcome.max_absolute_excursion_5m_bps,
                    outcome
                        .price_sample_count_5m
                        .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
                    outcome.liquidity_recovered_5m.map(bool_to_int),
                    outcome.liquidity_recovery_ms,
                    outcome.liquidity_recovery_reason,
                    outcome.setup_outcome,
                    outcome.follow_through_30s.map(bool_to_int),
                    outcome.follow_through_2m.map(bool_to_int),
                    outcome.follow_through_5m.map(bool_to_int),
                    outcome.evaluated_at,
                    outcome.outcome_version,
                ])?;
                written += 1;
            }
            drop(stmt);
            tx.commit()?;
            Ok(written)
        })
    }

    fn contract_whale_outcome_summary(
        &self,
        outcome_version: &str,
    ) -> anyhow::Result<Vec<ContractWhaleOutcomeSummaryRow>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT symbol, signal_type, COALESCE(classification_v2, ''), severity,
                       impact_level, window_sec, COALESCE(oi_context, ''), COALESCE(regime, ''),
                       strftime('%H', signal_ts / 1000, 'unixepoch') AS hour_utc,
                       COUNT(*) AS sample_count,
                       AVG(absolute_return_30s_bps), AVG(absolute_return_2m_bps),
                       AVG(absolute_return_5m_bps), AVG(realized_volatility_5m_bps),
                       AVG(max_absolute_excursion_5m_bps), AVG(price_sample_count_5m),
                       AVG(markout_30s_bps), AVG(markout_2m_bps), AVG(markout_5m_bps),
                       AVG(follow_through_30s), AVG(follow_through_2m), AVG(follow_through_5m)
                FROM contract_whale_signal_outcomes
                WHERE outcome_version = ?1
                GROUP BY symbol, signal_type, classification_v2, severity, impact_level,
                         window_sec, oi_context, regime, hour_utc
                ORDER BY sample_count DESC, symbol ASC
                LIMIT 500
                "#,
            )?;
            let rows = stmt.query_map([outcome_version], |row| {
                Ok(ContractWhaleOutcomeSummaryRow {
                    symbol: row.get(0)?,
                    signal_type: row.get(1)?,
                    classification_v2: row.get(2)?,
                    severity: row.get(3)?,
                    impact_level: row.get(4)?,
                    window_sec: row.get::<_, i64>(5)?.max(0) as u64,
                    oi_context: row.get(6)?,
                    regime: row.get(7)?,
                    hour_utc: row.get(8)?,
                    sample_count: row.get::<_, i64>(9)?.max(0) as usize,
                    avg_absolute_return_30s_bps: row.get(10)?,
                    avg_absolute_return_2m_bps: row.get(11)?,
                    avg_absolute_return_5m_bps: row.get(12)?,
                    avg_realized_volatility_5m_bps: row.get(13)?,
                    avg_max_absolute_excursion_5m_bps: row.get(14)?,
                    avg_price_sample_count_5m: row.get(15)?,
                    avg_markout_30s_bps: row.get(16)?,
                    avg_markout_2m_bps: row.get(17)?,
                    avg_markout_5m_bps: row.get(18)?,
                    follow_through_30s_rate: row.get(19)?,
                    follow_through_2m_rate: row.get(20)?,
                    follow_through_5m_rate: row.get(21)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
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
        impact_b_cutoff_ts: i64,
    ) -> anyhow::Result<ContractWhaleRetentionPruneResult> {
        let s_severity = enum_value(ContractWhaleSeverity::S)?;
        self.with_connection(|conn| {
            let mut result = ContractWhaleRetentionPruneResult {
                flow_cutoff_ts,
                signal_cutoff_ts,
                impact_b_cutoff_ts,
                ..ContractWhaleRetentionPruneResult::default()
            };
            if table_exists(conn, "contract_whale_signals")?
                && column_exists(conn, "contract_whale_signals", "ts")?
                && column_exists(conn, "contract_whale_signals", "severity")?
            {
                result.protected_s_count = conn.query_row(
                    "SELECT COUNT(*) FROM contract_whale_signals WHERE ts < ?1 AND severity = ?2",
                    params![signal_cutoff_ts, s_severity.clone()],
                    |row| row.get::<_, i64>(0),
                )? as usize;
            }
            if table_exists(conn, "contract_whale_signals")?
                && column_exists(conn, "contract_whale_signals", "ts")?
                && column_exists(conn, "contract_whale_signals", "severity")?
                && column_exists(conn, "contract_whale_signals", "net_volume_btc")?
            {
                result.protected_net_volume_count = conn.query_row(
                    "SELECT COUNT(*) FROM contract_whale_signals WHERE ts < ?1 AND severity != ?2 AND ABS(COALESCE(net_volume_btc, 0.0)) >= ?3",
                    params![
                        signal_cutoff_ts,
                        s_severity.clone(),
                        CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC,
                    ],
                    |row| row.get::<_, i64>(0),
                )? as usize;
            }

            result.flow_1s_deleted = prune_contract_table(
                conn,
                "contract_flow_1s",
                "ts_bucket",
                "DELETE FROM contract_flow_1s WHERE ts_bucket < ?1",
                params![flow_cutoff_ts],
                &mut result.table_results,
            )?;
            result.liquidation_deleted = prune_contract_table(
                conn,
                "contract_liquidation_1s",
                "ts_bucket",
                "DELETE FROM contract_liquidation_1s WHERE ts_bucket < ?1",
                params![flow_cutoff_ts],
                &mut result.table_results,
            )?;
            result.oi_deleted = prune_contract_table(
                conn,
                "contract_oi_snapshots",
                "ts",
                "DELETE FROM contract_oi_snapshots WHERE ts < ?1",
                params![flow_cutoff_ts],
                &mut result.table_results,
            )?;
            result.funding_deleted = prune_contract_table(
                conn,
                "contract_funding_snapshots",
                "ts",
                "DELETE FROM contract_funding_snapshots WHERE ts < ?1",
                params![flow_cutoff_ts],
                &mut result.table_results,
            )?;
            result.percentile_deleted = prune_contract_table(
                conn,
                "contract_whale_percentile_thresholds",
                "computed_at",
                "DELETE FROM contract_whale_percentile_thresholds WHERE computed_at < ?1",
                params![flow_cutoff_ts],
                &mut result.table_results,
            )?;
            // Retention tiers:
            // - impact A/S, severity S, |net|>=500: permanent
            // - impact B: keep until impact_b_cutoff_ts (default 90d)
            // - everything else: keep until signal_cutoff_ts (default 7d)
            result.signal_deleted = prune_contract_table(
                conn,
                "contract_whale_signals",
                "ts",
                r#"
                DELETE FROM contract_whale_signals
                WHERE severity != ?1
                  AND ABS(COALESCE(net_volume_btc, 0.0)) < ?2
                  AND UPPER(COALESCE(
                        json_extract(payload_json, '$.impactLevel'),
                        json_extract(payload_json, '$.impact_level'),
                        ''
                      )) NOT IN ('A', 'S')
                  AND (
                        (
                          UPPER(COALESCE(
                            json_extract(payload_json, '$.impactLevel'),
                            json_extract(payload_json, '$.impact_level'),
                            ''
                          )) = 'B'
                          AND ts < ?3
                        )
                        OR
                        (
                          UPPER(COALESCE(
                            json_extract(payload_json, '$.impactLevel'),
                            json_extract(payload_json, '$.impact_level'),
                            ''
                          )) != 'B'
                          AND ts < ?4
                        )
                  )
                "#,
                params![
                    s_severity,
                    CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC,
                    impact_b_cutoff_ts,
                    signal_cutoff_ts,
                ],
                &mut result.table_results,
            )?;
            if let Some(last_entry) = result.table_results.last_mut() {
                if last_entry.status == RetentionTableStatus::Ok {
                    last_entry.reason = Some(format!(
                        "impact_as_permanent_b_days_default_days_net_lt_{}",
                        CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC
                    ));
                }
            }

            let deleted_any = result.flow_1s_deleted > 0
                || result.liquidation_deleted > 0
                || result.oi_deleted > 0
                || result.funding_deleted > 0
                || result.percentile_deleted > 0
                || result.signal_deleted > 0;
            if deleted_any {
                result.wal_checkpoint = Some(run_contract_wal_checkpoint(conn));
            }
            Ok(result)
        })
    }
}

fn prune_contract_table(
    conn: &rusqlite::Connection,
    table: &str,
    time_column: &str,
    sql: &str,
    sql_params: impl rusqlite::Params,
    table_results: &mut Vec<RetentionTableResult>,
) -> anyhow::Result<usize> {
    let started_at = std::time::Instant::now();
    if !table_exists(conn, table)? {
        table_results.push(RetentionTableResult {
            table: table.to_string(),
            time_column: time_column.to_string(),
            status: RetentionTableStatus::Skipped,
            deleted_rows: 0,
            duration_ms: started_at.elapsed().as_millis() as u64,
            reason: Some("table_missing".to_string()),
            error: None,
            error_kind: None,
        });
        return Ok(0);
    }
    if !column_exists(conn, table, time_column)? {
        table_results.push(RetentionTableResult {
            table: table.to_string(),
            time_column: time_column.to_string(),
            status: RetentionTableStatus::Skipped,
            deleted_rows: 0,
            duration_ms: started_at.elapsed().as_millis() as u64,
            reason: Some("time_column_missing".to_string()),
            error: None,
            error_kind: None,
        });
        return Ok(0);
    }

    match conn.execute(sql, sql_params) {
        Ok(deleted_rows) => {
            table_results.push(RetentionTableResult {
                table: table.to_string(),
                time_column: time_column.to_string(),
                status: RetentionTableStatus::Ok,
                deleted_rows,
                duration_ms: started_at.elapsed().as_millis() as u64,
                reason: None,
                error: None,
                error_kind: None,
            });
            Ok(deleted_rows)
        }
        Err(error) => {
            let message = format!("{error:#}");
            table_results.push(RetentionTableResult {
                table: table.to_string(),
                time_column: time_column.to_string(),
                status: RetentionTableStatus::Error,
                deleted_rows: 0,
                duration_ms: started_at.elapsed().as_millis() as u64,
                reason: None,
                error_kind: Some(classify_retention_error(&message)),
                error: Some(message),
            });
            Ok(0)
        }
    }
}

fn run_contract_wal_checkpoint(conn: &rusqlite::Connection) -> WalCheckpointResult {
    let started_at = std::time::Instant::now();
    match conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
        Ok(()) => WalCheckpointResult {
            attempted: true,
            ok: true,
            duration_ms: started_at.elapsed().as_millis() as u64,
            error: None,
        },
        Err(error) => WalCheckpointResult {
            attempted: true,
            ok: false,
            duration_ms: started_at.elapsed().as_millis() as u64,
            error: Some(format!("{error:#}")),
        },
    }
}

fn decode_signal_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContractWhaleSignal> {
    let payload_json: String = row.get(0)?;
    let mut signal = serde_json::from_str::<ContractWhaleSignal>(&payload_json)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    signal.discord_eligible = row.get::<_, i64>(1)? != 0;
    signal.discord_sent = row.get::<_, i64>(2)? != 0;
    signal.discord_sent_at = row.get(3)?;
    let active_sources_json: Option<String> = row.get(4)?;
    let threshold_profile: Option<String> = row.get(5)?;
    repair_signal_profile_snapshot(&mut signal, active_sources_json, threshold_profile)?;
    Ok(signal)
}

fn repair_signal_profile_snapshot(
    signal: &mut ContractWhaleSignal,
    active_sources_json: Option<String>,
    threshold_profile: Option<String>,
) -> rusqlite::Result<()> {
    let column_sources = active_sources_json
        .as_deref()
        .filter(|json| !json.trim().is_empty())
        .map(|json| {
            serde_json::from_str::<ContractWhaleActiveSources>(json)
                .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
        })
        .transpose()?;
    let column_profile = threshold_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let signal_has_snapshot = !signal.active_sources.contract.is_empty()
        || !signal.active_sources.spot.is_empty()
        || !signal.active_sources.configured_contract_sources.is_empty()
        || !signal.active_sources.eligible_contract_sources.is_empty()
        || !signal.active_sources.active_contract_sources.is_empty();
    let column_has_snapshot = column_sources.as_ref().is_some_and(|sources| {
        !sources.contract.is_empty()
            || !sources.spot.is_empty()
            || !sources.configured_contract_sources.is_empty()
            || !sources.eligible_contract_sources.is_empty()
            || !sources.active_contract_sources.is_empty()
    });

    if !signal_has_snapshot {
        if let Some(sources) = column_sources {
            signal.active_sources = sources;
        }
        signal.threshold_profile = column_profile
            .or_else(|| {
                (!signal.active_sources.threshold_profile.trim().is_empty())
                    .then_some(signal.active_sources.threshold_profile.as_str())
            })
            .unwrap_or("unknown")
            .to_string();
    }

    let has_recovered_snapshot = signal_has_snapshot || column_has_snapshot;
    if !has_recovered_snapshot {
        signal.threshold_profile = "unknown".to_string();
        signal.threshold_profile_reason = "legacy_signal".to_string();
        signal.configured_contract_sources.clear();
        signal.eligible_contract_sources.clear();
        signal.active_contract_sources.clear();
        signal.active_sources.threshold_profile = "unknown".to_string();
        signal.active_sources.threshold_profile_reason = "legacy_signal".to_string();
        signal.active_sources.configured_contract_sources.clear();
        signal.active_sources.eligible_contract_sources.clear();
        signal.active_sources.active_contract_sources.clear();
        return Ok(());
    }

    if signal.threshold_profile.trim().is_empty() {
        signal.threshold_profile = column_profile.unwrap_or("unknown").to_string();
    }
    if signal.threshold_profile_reason.trim().is_empty() {
        signal.threshold_profile_reason = signal.active_sources.threshold_profile_reason.clone();
    }
    if signal.configured_contract_sources.is_empty() {
        signal.configured_contract_sources =
            signal.active_sources.configured_contract_sources.clone();
    }
    if signal.eligible_contract_sources.is_empty() {
        signal.eligible_contract_sources = signal.active_sources.eligible_contract_sources.clone();
    }
    if signal.active_contract_sources.is_empty() {
        signal.active_contract_sources = signal.active_sources.active_contract_sources.clone();
    }
    if signal.active_sources.threshold_profile.trim().is_empty()
        || signal.active_sources.threshold_profile == "three_exchange"
            && signal.threshold_profile != "three_exchange"
    {
        signal.active_sources.threshold_profile = signal.threshold_profile.clone();
    }
    if signal
        .active_sources
        .threshold_profile_reason
        .trim()
        .is_empty()
    {
        signal.active_sources.threshold_profile_reason = signal.threshold_profile_reason.clone();
    }
    if signal.active_sources.configured_contract_sources.is_empty() {
        signal.active_sources.configured_contract_sources =
            signal.configured_contract_sources.clone();
    }
    if signal.active_sources.eligible_contract_sources.is_empty() {
        signal.active_sources.eligible_contract_sources = signal.eligible_contract_sources.clone();
    }
    if signal.active_sources.active_contract_sources.is_empty() {
        signal.active_sources.active_contract_sources = signal.active_contract_sources.clone();
    }
    Ok(())
}

pub fn resolve_oi_window_from_snapshots(
    snapshots: &[ContractOiSnapshot],
    start_ts: i64,
    end_ts: i64,
    max_gap_sec: i64,
) -> ContractWhaleOiWindowContext {
    let max_gap_ms = max_gap_sec.max(0).saturating_mul(1000);
    let mut snapshots_by_exchange = BTreeMap::<ContractExchange, Vec<&ContractOiSnapshot>>::new();
    for snapshot in snapshots {
        if snapshot.ts >= start_ts.saturating_sub(max_gap_ms) && snapshot.ts <= end_ts {
            snapshots_by_exchange
                .entry(snapshot.exchange)
                .or_default()
                .push(snapshot);
        }
    }

    let mut exchanges = Vec::new();
    let mut consistent_sources = Vec::new();
    let mut excluded_sources = Vec::new();
    let mut before_sources = Vec::new();
    let mut after_sources = Vec::new();
    let mut oi_before = 0.0;
    let mut oi_after = 0.0;
    let mut metadata_degraded = false;
    let mut metadata_degraded_reason = None;

    for (exchange, snapshots) in snapshots_by_exchange.iter_mut() {
        snapshots.sort_by_key(|snapshot| snapshot.ts);
        let before = snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.ts <= start_ts)
            .copied();
        let after = snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.ts <= end_ts)
            .copied();
        let exchange_key = exchange.as_key().to_string();
        if before.is_some() {
            before_sources.push(exchange_key.clone());
        }
        if after.is_some() {
            after_sources.push(exchange_key.clone());
        }
        let (Some(before), Some(after)) = (before, after) else {
            excluded_sources.push(format!("{exchange_key}:missing_before_or_after"));
            continue;
        };
        if start_ts.saturating_sub(before.ts).abs() > max_gap_ms
            || end_ts.saturating_sub(after.ts).abs() > max_gap_ms
        {
            excluded_sources.push(format!("{exchange_key}:snapshot_gap_too_large"));
            continue;
        }
        if !before.oi_btc.is_finite() || before.oi_btc <= 0.0 || !after.oi_btc.is_finite() {
            excluded_sources.push(format!("{exchange_key}:invalid_oi_value"));
            continue;
        }
        if !before.ct_val_available || !after.ct_val_available {
            metadata_degraded = true;
            metadata_degraded_reason = before
                .evidence_degraded_reason
                .clone()
                .or_else(|| after.evidence_degraded_reason.clone())
                .or_else(|| Some("ct_val_unavailable".to_string()));
        }

        let oi_delta_btc = after.oi_btc - before.oi_btc;
        let oi_delta_pct = (oi_delta_btc / before.oi_btc) * 100.0;
        exchanges.push(ContractWhaleOiExchangeDelta {
            exchange: *exchange,
            before_ts: before.ts,
            after_ts: after.ts,
            oi_before_btc: before.oi_btc,
            oi_after_btc: after.oi_btc,
            oi_delta_btc,
            oi_delta_pct,
        });
        consistent_sources.push(exchange_key);
        oi_before += before.oi_btc;
        oi_after += after.oi_btc;
    }

    let source_coverage_changed = before_sources != after_sources;
    if exchanges.is_empty() {
        let has_gap_rejection = excluded_sources
            .iter()
            .any(|source| source.ends_with(":snapshot_gap_too_large"));
        return ContractWhaleOiWindowContext {
            excluded_sources,
            source_coverage_changed,
            available: false,
            reason: Some(
                if has_gap_rejection {
                    "oi_snapshot_gap_too_large"
                } else {
                    "no_consistent_oi_sources"
                }
                .to_string(),
            ),
            ..ContractWhaleOiWindowContext::default()
        };
    }

    let directional = exchanges
        .iter()
        .filter(|entry| entry.oi_delta_btc.abs() > f64::EPSILON)
        .collect::<Vec<_>>();
    let cross_exchange_consensus = if directional.len() < 2 {
        None
    } else {
        let positive = directional
            .iter()
            .filter(|entry| entry.oi_delta_btc.is_sign_positive())
            .count();
        let negative = directional.len().saturating_sub(positive);
        Some(positive * 3 >= directional.len() * 2 || negative * 3 >= directional.len() * 2)
    };
    let evidence_degraded = exchanges.len() == 1 || source_coverage_changed || metadata_degraded;
    let evidence_reason = if metadata_degraded {
        metadata_degraded_reason
    } else if exchanges.len() == 1 {
        Some("single_consistent_oi_source".to_string())
    } else if source_coverage_changed {
        Some("oi_source_coverage_changed".to_string())
    } else if matches!(cross_exchange_consensus, Some(false)) {
        Some("oi_cross_exchange_conflict".to_string())
    } else {
        None
    };
    let oi_delta = oi_after - oi_before;
    ContractWhaleOiWindowContext {
        before_ts: exchanges.iter().map(|entry| entry.before_ts).max(),
        after_ts: exchanges.iter().map(|entry| entry.after_ts).max(),
        oi_before: Some(oi_before),
        oi_after: Some(oi_after),
        oi_delta: Some(oi_delta),
        oi_delta_pct: (oi_delta / oi_before)
            .is_finite()
            .then_some((oi_delta / oi_before) * 100.0),
        exchanges,
        consistent_sources,
        excluded_sources,
        source_coverage_changed,
        cross_exchange_consensus,
        evidence_degraded,
        evidence_reason,
        available: true,
        reason: None,
    }
}

fn enum_value<T: serde::Serialize>(value: T) -> anyhow::Result<String> {
    Ok(serde_json::to_value(value)?
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}

fn upsert_contract_whale_signals_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    signals: &[ContractWhaleSignal],
    now: i64,
) -> anyhow::Result<usize> {
    if signals.is_empty() {
        return Ok(0);
    }
    let mut stmt = tx.prepare(
        r#"
        INSERT INTO contract_whale_signals (
          signal_id, ts, symbol, window_sec, signal_type, direction, severity, score,
          total_volume_btc, net_volume_btc, total_notional_usd, dominance,
          price_start, price_end, price_move_pct, main_exchange, market_type,
          source_role, exchanges_json, active_sources_json, threshold_profile,
          dynamic_multiple, data_quality, discord_eligible, discord_sent,
          discord_sent_at, payload_json, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                  NULL, NULL, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)
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
          market_type = excluded.market_type,
          source_role = excluded.source_role,
          exchanges_json = excluded.exchanges_json,
          active_sources_json = excluded.active_sources_json,
          threshold_profile = excluded.threshold_profile,
          dynamic_multiple = excluded.dynamic_multiple,
          data_quality = excluded.data_quality,
          discord_eligible = excluded.discord_eligible,
          discord_sent = excluded.discord_sent,
          discord_sent_at = excluded.discord_sent_at,
          payload_json = excluded.payload_json,
          created_at = excluded.created_at
        "#,
    )?;
    let mut written = 0;
    for signal in signals {
        let signal_type = enum_value(signal.signal_type)?;
        let direction = enum_value(signal.direction)?;
        let severity = enum_value(signal.severity)?;
        let market_type = enum_value(signal.market_type)?;
        let source_role = enum_value(signal.source_role)?;
        let exchanges_json = serde_json::to_string(&signal.exchanges)?;
        let active_sources_json = serde_json::to_string(&signal.active_sources)?;
        let payload_json = serde_json::to_string(signal)?;
        stmt.execute(params![
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
            market_type,
            source_role,
            exchanges_json,
            active_sources_json,
            signal.threshold_profile,
            signal.dynamic_multiple,
            signal.data_quality as i64,
            bool_to_int(signal.discord_eligible),
            bool_to_int(signal.discord_sent),
            signal.discord_sent_at,
            payload_json,
            now,
        ])
        .context("failed to upsert contract whale signal")?;
        written += 1;
    }
    Ok(written)
}

fn exchange_from_key(value: &str) -> ContractExchange {
    match value.to_ascii_lowercase().as_str() {
        "okx" => ContractExchange::Okx,
        "bitfinex" => ContractExchange::Bitfinex,
        "coinbase" => ContractExchange::Coinbase,
        _ => ContractExchange::Binance,
    }
}

fn market_type_from_key(value: &str) -> ContractWhaleMarketType {
    match value.to_ascii_lowercase().as_str() {
        "spot" => ContractWhaleMarketType::Spot,
        "level2" => ContractWhaleMarketType::Level2,
        "funding" => ContractWhaleMarketType::Funding,
        "oi" => ContractWhaleMarketType::Oi,
        "liquidation" => ContractWhaleMarketType::Liquidation,
        _ => ContractWhaleMarketType::Perp,
    }
}

fn source_role_from_key(value: &str) -> ContractWhaleSourceRole {
    match value.to_ascii_lowercase().as_str() {
        "primary" => ContractWhaleSourceRole::Primary,
        "confirmation" => ContractWhaleSourceRole::Confirmation,
        "disabled" => ContractWhaleSourceRole::Disabled,
        _ => ContractWhaleSourceRole::Optional,
    }
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod query_path_tests {
    use super::*;

    #[test]
    fn symbol_range_notional_query_selects_event_feed_path() {
        let query = ContractWhaleSignalQuery {
            symbol: Some("ETH".to_string()),
            from_ts: Some(1_700_000_000_000),
            min_notional_usd: Some(10_000_000.0),
            limit: 20,
            ..ContractWhaleSignalQuery::default()
        };

        assert_eq!(
            contract_whale_signal_query_path(&query),
            ContractWhaleSignalQueryPath::EventFeed
        );
    }

    #[test]
    fn positioned_cursor_keeps_general_query_path() {
        let query = ContractWhaleSignalQuery {
            symbol: Some("ETH".to_string()),
            from_ts: Some(1_700_000_000_000),
            min_notional_usd: Some(10_000_000.0),
            cursor_ts: Some(1_700_000_010_000),
            cursor_signal_id: Some("contract-whale:ETH:cursor".to_string()),
            limit: 20,
            ..ContractWhaleSignalQuery::default()
        };

        assert_eq!(
            contract_whale_signal_query_path(&query),
            ContractWhaleSignalQueryPath::General
        );
    }
}
