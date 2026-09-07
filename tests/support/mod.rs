use std::sync::atomic::{AtomicU64, Ordering};

use btc_toxic_flow_monitor_rs::storage::SqliteStore;

static TEMP_DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn test_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("test http client")
}

/// Creates an isolated SQLite store for integration tests.
///
/// Keeping this in the shared support module avoids every integration-test
/// binary carrying its own timestamp/pid-based path implementation. The
/// monotonic sequence also prevents collisions when tests run in parallel.
#[allow(dead_code)]
pub fn temp_store(name: &str) -> SqliteStore {
    let sequence = TEMP_DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "btc-toxic-flow-{name}-{}-{sequence}.sqlite",
        std::process::id()
    ));
    let store = SqliteStore::open(path.to_str().expect("utf8 sqlite path"))
        .expect("open sqlite test store");
    store.migrate().expect("migrate sqlite test store");
    store
}

#[allow(dead_code)]
pub async fn test_http_get<U: reqwest::IntoUrl>(url: U) -> reqwest::Result<reqwest::Response> {
    test_http_client().get(url).send().await
}
