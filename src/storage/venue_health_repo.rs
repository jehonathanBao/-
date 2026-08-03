use anyhow::Context;
use rusqlite::params;

use crate::types::status::VenueHealthMap;

use super::sqlite::SqliteStore;

pub trait VenueHealthRepo {
    fn insert_venue_health_snapshot(&self, ts: i64, health: &VenueHealthMap) -> anyhow::Result<()>;
}

impl VenueHealthRepo for SqliteStore {
    fn insert_venue_health_snapshot(&self, ts: i64, health: &VenueHealthMap) -> anyhow::Result<()> {
        self.with_write_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            for venue_health in health.values() {
                tx.execute(
                    r#"
                    INSERT INTO venue_health_snapshots (
                      ts, venue, enabled, status, last_trade_ts, last_book_ts, last_message_ts,
                      reconnect_count, last_error
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                    params![
                        ts,
                        venue_health.venue.as_key(),
                        bool_to_int(venue_health.enabled),
                        serde_json::to_string(&venue_health.status)?,
                        venue_health.last_trade_ts,
                        venue_health.last_book_ts,
                        venue_health.last_message_ts,
                        venue_health.reconnect_count as i64,
                        venue_health.last_error,
                    ],
                )
                .context("failed to insert venue health snapshot")?;
            }
            tx.commit()?;
            Ok(())
        })
    }
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
