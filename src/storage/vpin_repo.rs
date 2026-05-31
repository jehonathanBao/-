use anyhow::Context;
use rusqlite::params;

use crate::types::vpin::VpinBucket;

use super::sqlite::SqliteStore;

pub trait VpinRepo {
    fn insert_bucket(&self, bucket: &VpinBucket) -> anyhow::Result<()>;
    fn list_recent_buckets(&self, limit: usize) -> anyhow::Result<Vec<VpinBucket>>;
}

impl VpinRepo for SqliteStore {
    fn insert_bucket(&self, bucket: &VpinBucket) -> anyhow::Result<()> {
        let venue_breakdown_json = serde_json::to_string(&bucket.venue_breakdown)?;
        let payload_json = serde_json::to_string(bucket)?;
        self.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO vpin_buckets (
                  id, start_ts, end_ts, symbol, bucket_size_btc, total_btc,
                  buy_btc, sell_btc, net_btc, imbalance_btc, imbalance_ratio,
                  direction, venue_breakdown_json, payload_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                "#,
                params![
                    bucket.id as i64,
                    bucket.start_ts,
                    bucket.end_ts,
                    bucket.symbol,
                    bucket.bucket_size_btc,
                    bucket.total_btc,
                    bucket.buy_btc,
                    bucket.sell_btc,
                    bucket.net_btc,
                    bucket.imbalance_btc,
                    bucket.imbalance_ratio,
                    serde_json::to_string(&bucket.direction)?,
                    venue_breakdown_json,
                    payload_json,
                ],
            )
            .context("failed to insert vpin bucket")?;
            Ok(())
        })
    }

    fn list_recent_buckets(&self, limit: usize) -> anyhow::Result<Vec<VpinBucket>> {
        self.with_connection(|conn| {
            let mut stmt = conn
                .prepare("SELECT payload_json FROM vpin_buckets ORDER BY end_ts DESC LIMIT ?1")?;
            let rows = stmt.query_map([limit as i64], |row| row.get::<_, String>(0))?;
            let mut buckets = Vec::new();
            for row in rows {
                let payload = row?;
                buckets.push(serde_json::from_str::<VpinBucket>(&payload)?);
            }
            Ok(buckets)
        })
    }
}
