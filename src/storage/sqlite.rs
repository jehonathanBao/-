use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;
use rusqlite::Connection;

use super::migrations::MIGRATIONS;

#[derive(Debug, Clone)]
pub struct SqliteStore {
    path: PathBuf,
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
        let store = Self { path };
        store.health_check()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn migrate(&self) -> anyhow::Result<()> {
        let conn = self.open_connection()?;
        for migration in MIGRATIONS {
            conn.execute_batch(migration)
                .context("failed to run sqlite migration")?;
        }
        ensure_contract_whale_columns(&conn)?;
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

    fn open_connection(&self) -> anyhow::Result<Connection> {
        let conn = Connection::open(&self.path)
            .with_context(|| format!("failed to open sqlite {}", self.path.display()))?;
        conn.busy_timeout(Duration::from_secs(5))
            .context("failed to set sqlite busy_timeout")?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("failed to enable sqlite WAL journal mode")?;
        Ok(conn)
    }
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
    ensure_contract_flow_market_type_primary_key(conn)?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
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
