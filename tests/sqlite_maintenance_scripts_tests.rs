use rusqlite::{params, Connection};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

fn find_bash() -> Option<PathBuf> {
    let candidates = [
        "bash",
        r"C:\Program Files\Git\bin\bash.exe",
        "/bin/bash",
        "/usr/bin/bash",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|candidate| {
            if candidate.as_os_str() == "bash" {
                Command::new("bash")
                    .arg("--version")
                    .output()
                    .map(|output| output.status.success())
                    .unwrap_or(false)
            } else {
                candidate.exists()
            }
        })
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn temp_path(name: &str) -> PathBuf {
    env::temp_dir().join(format!("{}-{}", Uuid::new_v4(), name))
}

fn seed_flow_snapshots(db_path: &Path) {
    let conn = Connection::open(db_path).expect("open sqlite temp db");
    conn.execute_batch(
        "
        CREATE TABLE flow_snapshots (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          ts INTEGER NOT NULL,
          symbol TEXT NOT NULL,
          window_ms INTEGER NOT NULL,
          aggressive_buy_btc REAL NOT NULL,
          aggressive_sell_btc REAL NOT NULL,
          net_aggressive_btc REAL NOT NULL,
          abs_aggressive_btc REAL NOT NULL,
          price_move_bps REAL,
          payload_json TEXT NOT NULL
        );
        ",
    )
    .expect("create flow_snapshots");

    let now_ms = 1_800_000_000_000_i64;
    let stale_ts = now_ms - (48 * 60 * 60 * 1000);
    let fresh_ts = now_ms - (2 * 60 * 60 * 1000);
    for ts in [stale_ts, stale_ts + 1, fresh_ts] {
        conn.execute(
            "INSERT INTO flow_snapshots (
                ts, symbol, window_ms, aggressive_buy_btc, aggressive_sell_btc,
                net_aggressive_btc, abs_aggressive_btc, price_move_bps, payload_json
            ) VALUES (?1, 'BTC', 1000, 1.0, 0.5, 0.5, 1.5, 10.0, '{}')",
            params![ts],
        )
        .expect("insert flow snapshot");
    }
}

fn maybe_posix_path_for_bash(value: &str) -> String {
    value.to_string()
}

fn run_script(script_name: &str, envs: &[(&str, &str)]) -> std::process::Output {
    let bash = find_bash().expect("bash available");
    let script_path = repo_root().join("scripts").join(script_name);
    let mut cmd = if bash.as_os_str() == "bash" {
        let mut command = Command::new("bash");
        command.arg(script_path);
        command
    } else {
        let mut command = Command::new(bash);
        command.arg(script_path);
        command
    };
    for (key, value) in envs {
        let normalized = match *key {
            "DB_PATH" | "STOP_FILE" => maybe_posix_path_for_bash(value),
            _ => value.to_string(),
        };
        cmd.env(key, normalized);
    }
    cmd.output().expect("run script")
}

#[test]
fn sqlite_safe_batch_delete_refuses_non_p0_tables() {
    let db_path = temp_path("sqlite-safe-delete-refuse.sqlite");
    seed_flow_snapshots(&db_path);

    let db_string = db_path.to_string_lossy().into_owned();
    let output = run_script(
        "sqlite_safe_batch_delete.sh",
        &[
            ("DB_PATH", &db_string),
            ("TABLE", "contract_whale_signals"),
            ("TIME_COLUMN", "ts"),
        ],
    );

    assert!(!output.status.success(), "script unexpectedly succeeded");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Refuse to clean non-P0 table"), "stderr={stderr}");

    let _ = fs::remove_file(db_path);
}

#[test]
fn sqlite_safe_batch_delete_dry_run_reports_rows_without_mutating() {
    let db_path = temp_path("sqlite-safe-delete-dry.sqlite");
    seed_flow_snapshots(&db_path);
    let db_string = db_path.to_string_lossy().into_owned();

    let output = run_script(
        "sqlite_safe_batch_delete.sh",
        &[
            ("DB_PATH", &db_string),
            ("TABLE", "flow_snapshots"),
            ("TIME_COLUMN", "ts"),
            ("RETENTION_HOURS", "24"),
            ("NOW_MS", "1800000000000"),
            ("DRY_RUN", "1"),
        ],
    );

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("initial_deletable_rows=2"), "stdout={stdout}");
    assert!(stdout.contains("DRY_RUN enabled. No deletion executed."), "stdout={stdout}");

    let conn = Connection::open(&db_path).expect("open sqlite temp db");
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM flow_snapshots", [], |row| row.get(0))
        .expect("count rows after dry run");
    assert_eq!(remaining, 3);

    let _ = fs::remove_file(db_path);
}

#[test]
fn sqlite_safe_batch_delete_executes_batched_deletes_for_epoch_ms_tables() {
    let db_path = temp_path("sqlite-safe-delete-live.sqlite");
    seed_flow_snapshots(&db_path);
    let stop_file = temp_path("sqlite-safe-delete.stop");
    let db_string = db_path.to_string_lossy().into_owned();
    let stop_file_string = stop_file.to_string_lossy().into_owned();

    let output = run_script(
        "sqlite_safe_batch_delete.sh",
        &[
            ("DB_PATH", &db_string),
            ("TABLE", "flow_snapshots"),
            ("TIME_COLUMN", "ts"),
            ("RETENTION_HOURS", "24"),
            ("NOW_MS", "1800000000000"),
            ("DRY_RUN", "0"),
            ("BATCH_SIZE", "1"),
            ("MAX_BATCHES", "1"),
            ("SLEEP_SECONDS", "0"),
            ("STOP_FILE", &stop_file_string),
        ],
    );

    assert!(output.status.success(), "stderr={}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("batch=1 deleted=1 remaining=1 total_deleted=1"), "stdout={stdout}");
    assert!(stdout.contains("completed batches=1 total_deleted=1"), "stdout={stdout}");

    let conn = Connection::open(&db_path).expect("open sqlite temp db");
    let stale_remaining: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM flow_snapshots WHERE ts < ?1",
            params![1_800_000_000_000_i64 - (24 * 60 * 60 * 1000)],
            |row| row.get(0),
        )
        .expect("count stale rows");
    assert_eq!(stale_remaining, 1);

    let total_remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM flow_snapshots", [], |row| row.get(0))
        .expect("count total rows");
    assert_eq!(total_remaining, 2);

    let _ = fs::remove_file(db_path);
    let _ = fs::remove_file(stop_file);
}
