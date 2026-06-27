use btc_toxic_flow_monitor_rs::storage::SqliteStore;

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
}
