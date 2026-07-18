use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Context;
use rusqlite::{params, Connection, Transaction};

use super::migrations::MIGRATIONS;
use crate::storage::spot_whale_repo::{
    SPOT_WHALE_BTC_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
    SPOT_WHALE_ETH_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
};

#[derive(Debug, Clone)]
pub struct SqliteStore {
    path: PathBuf,
    journal_mode_initializations: Arc<AtomicUsize>,
}

impl SqliteStore {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create sqlite directory {}", parent.display())
                })?;
            }
        }
        let store = Self {
            path,
            journal_mode_initializations: Arc::new(AtomicUsize::new(0)),
        };
        store.initialize_database()?;
        store.health_check()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    #[doc(hidden)]
    pub fn journal_mode_initializations(&self) -> usize {
        self.journal_mode_initializations.load(Ordering::SeqCst)
    }

    pub fn migrate(&self) -> anyhow::Result<()> {
        let conn = self.open_connection()?;
        for migration in MIGRATIONS {
            conn.execute_batch(migration)
                .context("failed to run sqlite migration")?;
        }
        ensure_contract_whale_columns(&conn)?;
        ensure_spot_whale_columns(&conn)?;
        Ok(())
    }

    pub fn health_check(&self) -> anyhow::Result<()> {
        let conn = self.open_connection()?;
        conn.query_row("SELECT 1", [], |_row| Ok(()))
            .context("sqlite health check failed")?;
        Ok(())
    }

    pub fn with_connection<T, F>(&self, op: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Connection) -> anyhow::Result<T>,
    {
        let conn = self.open_connection()?;
        op(&conn)
    }

    pub fn with_transaction<T, F>(&self, op: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Transaction<'_>) -> anyhow::Result<T>,
    {
        let mut conn = self.open_connection()?;
        let transaction = conn
            .transaction()
            .context("failed to begin sqlite transaction")?;
        let result = op(&transaction)?;
        transaction
            .commit()
            .context("failed to commit sqlite transaction")?;
        Ok(result)
    }

    fn open_connection(&self) -> anyhow::Result<Connection> {
        let conn = Connection::open(&self.path)
            .with_context(|| format!("failed to open sqlite {}", self.path.display()))?;
        conn.busy_timeout(Duration::from_secs(5))
            .context("failed to set sqlite busy_timeout")?;
        Ok(conn)
    }

    fn initialize_database(&self) -> anyhow::Result<()> {
        let conn = self.open_connection()?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("failed to enable sqlite WAL journal mode")?;
        self.journal_mode_initializations
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

pub fn table_exists(conn: &Connection, table: &str) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .with_context(|| format!("failed to inspect sqlite table {table}"))
}

pub fn column_exists(conn: &Connection, table: &str, column: &str) -> anyhow::Result<bool> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn
        .prepare(&pragma)
        .with_context(|| format!("failed to inspect sqlite schema for {table}"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .with_context(|| format!("failed to query sqlite schema for {table}"))?;
    let has_column = columns
        .flatten()
        .any(|existing| existing.eq_ignore_ascii_case(column));
    Ok(has_column)
}

fn ensure_contract_whale_columns(conn: &Connection) -> anyhow::Result<()> {
    ensure_column(
        conn,
        "contract_flow_1s",
        "market_type",
        "TEXT NOT NULL DEFAULT 'perp'",
    )?;
    ensure_column(
        conn,
        "contract_flow_1s",
        "source_role",
        "TEXT NOT NULL DEFAULT 'primary'",
    )?;
    ensure_column(conn, "contract_flow_1s", "product_id", "TEXT")?;

    ensure_column(
        conn,
        "contract_whale_signals",
        "market_type",
        "TEXT NOT NULL DEFAULT 'perp'",
    )?;
    ensure_column(
        conn,
        "contract_whale_signals",
        "source_role",
        "TEXT NOT NULL DEFAULT 'primary'",
    )?;
    ensure_column(
        conn,
        "contract_whale_signals",
        "active_sources_json",
        "TEXT NOT NULL DEFAULT '{\"contract\":[],\"spot\":[]}'",
    )?;
    ensure_column(
        conn,
        "contract_whale_signals",
        "threshold_profile",
        "TEXT NOT NULL DEFAULT 'three_exchange'",
    )?;
    ensure_column(
        conn,
        "contract_oi_snapshots",
        "ct_val_available",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column(
        conn,
        "contract_oi_snapshots",
        "evidence_degraded_reason",
        "TEXT",
    )?;
    for (column, definition) in [
        ("absolute_return_30s_bps", "REAL"),
        ("absolute_return_2m_bps", "REAL"),
        ("absolute_return_5m_bps", "REAL"),
        ("realized_volatility_5m_bps", "REAL"),
        ("max_absolute_excursion_5m_bps", "REAL"),
        ("price_sample_count_5m", "INTEGER"),
        ("liquidity_recovered_5m", "INTEGER"),
        ("liquidity_recovery_ms", "INTEGER"),
        ("liquidity_recovery_reason", "TEXT"),
        ("setup_outcome", "TEXT"),
    ] {
        ensure_column(conn, "contract_whale_signal_outcomes", column, definition)?;
    }
    ensure_contract_flow_market_type_primary_key(conn)?;
    Ok(())
}

fn ensure_spot_whale_columns(conn: &Connection) -> anyhow::Result<()> {
    ensure_column(
        conn,
        "spot_whale_signals",
        "is_permanent",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    conn.execute(
        r#"
        UPDATE spot_whale_signals
        SET is_permanent = CASE
          WHEN UPPER(TRIM(symbol)) = 'ETH'
            AND ABS(net_volume_base) >= ?1 THEN 1
          WHEN UPPER(TRIM(symbol)) != 'ETH'
            AND ABS(net_volume_base) >= ?2 THEN 1
          ELSE 0
        END
        WHERE is_permanent != CASE
          WHEN UPPER(TRIM(symbol)) = 'ETH'
            AND ABS(net_volume_base) >= ?1 THEN 1
          WHEN UPPER(TRIM(symbol)) != 'ETH'
            AND ABS(net_volume_base) >= ?2 THEN 1
          ELSE 0
        END
        "#,
        params![
            SPOT_WHALE_ETH_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
            SPOT_WHALE_BTC_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
        ],
    )
    .context("failed to backfill spot_whale_signals.is_permanent")?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let has_column = column_exists(conn, table, column)?;
    if has_column {
        return Ok(());
    }

    let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    conn.execute_batch(&sql)
        .with_context(|| format!("failed to add {table}.{column}"))?;
    Ok(())
}

fn ensure_contract_flow_market_type_primary_key(conn: &Connection) -> anyhow::Result<()> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(contract_flow_1s)")
        .context("failed to inspect contract_flow_1s primary key")?;
    let columns = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
    })?;
    let mut market_type_in_primary_key = false;
    for column in columns {
        let (name, pk) = column?;
        if name.eq_ignore_ascii_case("market_type") && pk > 0 {
            market_type_in_primary_key = true;
            break;
        }
    }
    if market_type_in_primary_key {
        return Ok(());
    }

    conn.execute_batch(
        r#"
        DROP TABLE IF EXISTS contract_flow_1s_next;
        CREATE TABLE IF NOT EXISTS contract_flow_1s_next (
          ts_bucket INTEGER NOT NULL,
          exchange TEXT NOT NULL,
          symbol TEXT NOT NULL,
          market_type TEXT NOT NULL DEFAULT 'perp',
          source_role TEXT NOT NULL DEFAULT 'primary',
          product_id TEXT,
          buy_volume_btc REAL NOT NULL,
          sell_volume_btc REAL NOT NULL,
          buy_notional_usd REAL NOT NULL,
          sell_notional_usd REAL NOT NULL,
          trade_count INTEGER NOT NULL,
          max_single_trade_btc REAL,
          vwap REAL,
          created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
          PRIMARY KEY (ts_bucket, exchange, symbol, market_type)
        );
        INSERT OR REPLACE INTO contract_flow_1s_next (
          ts_bucket, exchange, symbol, market_type, source_role, product_id,
          buy_volume_btc, sell_volume_btc, buy_notional_usd, sell_notional_usd,
          trade_count, max_single_trade_btc, vwap, created_at
        )
        SELECT
          ts_bucket, exchange, symbol,
          COALESCE(NULLIF(market_type, ''), 'perp'),
          COALESCE(NULLIF(source_role, ''), 'primary'),
          product_id,
          buy_volume_btc, sell_volume_btc, buy_notional_usd, sell_notional_usd,
          trade_count, max_single_trade_btc, vwap, created_at
        FROM contract_flow_1s;
        DROP TABLE contract_flow_1s;
        ALTER TABLE contract_flow_1s_next RENAME TO contract_flow_1s;
        CREATE INDEX IF NOT EXISTS idx_contract_flow_1s_symbol_ts
          ON contract_flow_1s(symbol, ts_bucket DESC);
        "#,
    )
    .context("failed to rebuild contract_flow_1s primary key")?;
    Ok(())
}
