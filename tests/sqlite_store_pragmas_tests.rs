use std::time::{Duration, Instant};

use btc_toxic_flow_monitor_rs::storage::SqliteStore;
use rusqlite::{Connection, TransactionBehavior};

fn unique_path(suffix: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "btc_toxic_flow_monitor_rs_{}_{}",
        std::process::id(),
        unique_nanos()
    ));
    let path = base.join(suffix);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create temp dir");
    }
    path
}

fn unique_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos()
}

#[test]
fn sqlite_connections_use_busy_timeout_and_wal() {
    let path = unique_path("sqlite/pragmas.sqlite");
    let store = SqliteStore::open(path.to_str().expect("utf8 path")).expect("open sqlite");

    let (busy_timeout_ms, journal_mode) = store
        .with_connection(|conn| {
            let busy_timeout_ms: i64 =
                conn.query_row("PRAGMA busy_timeout", [], |row| row.get(0))?;
            let journal_mode: String =
                conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
            Ok((busy_timeout_ms, journal_mode))
        })
        .expect("query pragmas");

    assert!(
        busy_timeout_ms >= 5_000,
        "expected busy_timeout to be at least 5000ms, got {busy_timeout_ms}"
    );
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    assert_eq!(
        store.journal_mode_initializations(),
        1,
        "WAL mode must be initialized once when the store opens"
    );

    for _ in 0..5 {
        store
            .with_connection(|conn| {
                conn.query_row("SELECT 1", [], |_row| Ok(()))?;
                Ok(())
            })
            .expect("open operation connection");
    }

    assert_eq!(
        store.journal_mode_initializations(),
        1,
        "ordinary operations must not rewrite journal_mode"
    );
}

#[test]
fn sqlite_reads_do_not_wait_for_an_active_wal_writer() {
    let path = unique_path("sqlite/read_during_write.sqlite");
    let store = SqliteStore::open(path.to_str().expect("utf8 path")).expect("open sqlite");
    store
        .with_connection(|conn| {
            conn.execute_batch("CREATE TABLE samples (id INTEGER PRIMARY KEY, value TEXT);")?;
            Ok(())
        })
        .expect("create sample table");

    let mut writer = Connection::open(&path).expect("open writer");
    writer
        .busy_timeout(Duration::from_secs(1))
        .expect("set writer timeout");
    let transaction = writer
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("begin writer transaction");
    transaction
        .execute("INSERT INTO samples (value) VALUES ('pending')", [])
        .expect("insert pending row");

    let started = Instant::now();
    let visible_rows: i64 = store
        .with_connection(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM samples", [], |row| row.get(0))?)
        })
        .expect("read while writer transaction is active");

    assert_eq!(
        visible_rows, 0,
        "uncommitted writer row must stay invisible"
    );
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "WAL readers must not wait on a writer; elapsed={:?}",
        started.elapsed()
    );
}

#[test]
fn sqlite_read_does_not_wait_for_an_internal_write_operation() {
    let path = unique_path("sqlite/operation_gate.sqlite");
    let store = SqliteStore::open(path.to_str().expect("utf8 path")).expect("open sqlite");
    let first = store.clone();
    let second = store.clone();

    let writer = std::thread::spawn(move || {
        first
            .with_write_connection(|_conn| {
                std::thread::sleep(Duration::from_millis(150));
                Ok(())
            })
            .expect("first operation");
    });
    std::thread::sleep(Duration::from_millis(25));
    let started = Instant::now();
    second
        .with_connection(|_conn| Ok(()))
        .expect("second operation");
    let waited = started.elapsed();
    writer.join().expect("join first operation");

    assert!(
        waited < Duration::from_millis(100),
        "WAL reads must not wait for an internal write operation; waited={waited:?}"
    );
}

#[test]
fn sqlite_write_operations_are_serialized_across_clones() {
    let path = unique_path("sqlite/write_gate.sqlite");
    let store = SqliteStore::open(path.to_str().expect("utf8 path")).expect("open sqlite");
    let first = store.clone();
    let second = store.clone();

    let writer = std::thread::spawn(move || {
        first
            .with_write_connection(|_conn| {
                std::thread::sleep(Duration::from_millis(150));
                Ok(())
            })
            .expect("first write");
    });
    std::thread::sleep(Duration::from_millis(25));
    let started = Instant::now();
    second
        .with_write_connection(|_conn| Ok(()))
        .expect("second write");
    let waited = started.elapsed();
    writer.join().expect("join first write");

    assert!(
        waited >= Duration::from_millis(100),
        "cloned stores must serialize writes; waited={waited:?}"
    );
}

#[test]
fn high_churn_snapshot_tables_have_retention_indexes() {
    let path = unique_path("sqlite/retention_indexes.sqlite");
    let store = SqliteStore::open(path.to_str().expect("utf8 path")).expect("open sqlite");
    store.migrate().expect("migrate");

    for (table, expected_index) in [
        ("flow_snapshots", "idx_flow_snapshots_ts"),
        ("venue_health_snapshots", "idx_venue_health_snapshots_ts"),
    ] {
        let indexes = store
            .with_connection(|conn| {
                let mut statement = conn.prepare(&format!("PRAGMA index_list({table})"))?;
                let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
                Ok(rows.filter_map(Result::ok).collect::<Vec<_>>())
            })
            .expect("list indexes");
        assert!(
            indexes.iter().any(|index| index == expected_index),
            "missing {expected_index} on {table}; indexes={indexes:?}"
        );
    }
}
