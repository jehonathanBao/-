use anyhow::Context;
use rusqlite::params;

use super::sqlite::SqliteStore;

/// Persisted monitor-flow values for a completed Binance candle.  Binance OHLCV
/// remains the source of truth for the candle itself; this table only freezes
/// the expensive buy/sell aggregation once the candle has closed.
#[derive(Debug, Clone)]
pub struct BinanceOrderflowDeltaCacheRow {
    pub symbol: String,
    pub interval: String,
    pub candle_time: i64,
    pub close_time: i64,
    pub volume_base: f64,
    pub volume_quote: f64,
    pub buy_base: f64,
    pub sell_base: f64,
    pub buy_quote: f64,
    pub sell_quote: f64,
    pub delta_base: f64,
    pub delta_quote: f64,
    pub delta_pct: f64,
    pub trade_count: u64,
    pub computed_at_ms: i64,
}

impl SqliteStore {
    pub fn list_binance_orderflow_delta_cache(
        &self,
        symbol: &str,
        interval: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> anyhow::Result<Vec<BinanceOrderflowDeltaCacheRow>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT symbol, interval, candle_time, close_time,
                       volume_base, volume_quote, buy_base, sell_base,
                       buy_quote, sell_quote, delta_base, delta_quote,
                       delta_pct, trade_count, computed_at_ms
                  FROM binance_orderflow_delta_cache
                 WHERE symbol = ?1 AND interval = ?2
                   AND candle_time >= ?3 AND candle_time <= ?4
                 ORDER BY candle_time ASC
                "#,
            )?;
            let rows = stmt.query_map(params![symbol, interval, from_ts, to_ts], |row| {
                Ok(BinanceOrderflowDeltaCacheRow {
                    symbol: row.get(0)?,
                    interval: row.get(1)?,
                    candle_time: row.get(2)?,
                    close_time: row.get(3)?,
                    volume_base: row.get(4)?,
                    volume_quote: row.get(5)?,
                    buy_base: row.get(6)?,
                    sell_base: row.get(7)?,
                    buy_quote: row.get(8)?,
                    sell_quote: row.get(9)?,
                    delta_base: row.get(10)?,
                    delta_quote: row.get(11)?,
                    delta_pct: row.get(12)?,
                    trade_count: row.get::<_, i64>(13)?.max(0) as u64,
                    computed_at_ms: row.get(14)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
        })
    }

    pub fn upsert_binance_orderflow_delta_cache(
        &self,
        rows: &[BinanceOrderflowDeltaCacheRow],
    ) -> anyhow::Result<usize> {
        if rows.is_empty() {
            return Ok(0);
        }
        self.with_write_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO binance_orderflow_delta_cache (
                  symbol, interval, candle_time, close_time,
                  volume_base, volume_quote, buy_base, sell_base,
                  buy_quote, sell_quote, delta_base, delta_quote,
                  delta_pct, trade_count, computed_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                          ?11, ?12, ?13, ?14, ?15)
                ON CONFLICT(symbol, interval, candle_time) DO UPDATE SET
                  close_time = excluded.close_time,
                  volume_base = excluded.volume_base,
                  volume_quote = excluded.volume_quote,
                  buy_base = excluded.buy_base,
                  sell_base = excluded.sell_base,
                  buy_quote = excluded.buy_quote,
                  sell_quote = excluded.sell_quote,
                  delta_base = excluded.delta_base,
                  delta_quote = excluded.delta_quote,
                  delta_pct = excluded.delta_pct,
                  trade_count = excluded.trade_count,
                  computed_at_ms = excluded.computed_at_ms
                "#,
            )?;
            for row in rows {
                stmt.execute(params![
                    row.symbol,
                    row.interval,
                    row.candle_time,
                    row.close_time,
                    row.volume_base,
                    row.volume_quote,
                    row.buy_base,
                    row.sell_base,
                    row.buy_quote,
                    row.sell_quote,
                    row.delta_base,
                    row.delta_quote,
                    row.delta_pct,
                    row.trade_count as i64,
                    row.computed_at_ms,
                ])
                .context("failed to upsert Binance orderflow delta cache row")?;
            }
            drop(stmt);
            tx.commit()?;
            Ok(rows.len())
        })
    }

    pub fn purge_binance_orderflow_delta_cache_before(&self, cutoff_ms: i64) -> anyhow::Result<usize> {
        self.with_write_connection(|conn| {
            let deleted = conn.execute(
                "DELETE FROM binance_orderflow_delta_cache WHERE candle_time < ?1",
                params![cutoff_ms],
            )?;
            Ok(deleted)
        })
    }
}
