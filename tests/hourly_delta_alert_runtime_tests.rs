use btc_toxic_flow_monitor_rs::contract_whale_monitor::hourly_delta_alert::{
    types::{
        HourlyDeltaDataStatus, HourlyDeltaDirection, HourlyDeltaDiscordStatus, HourlyDeltaResult,
    },
    HourlyDeltaAlertConfig, HourlyDeltaAlertRuntime,
};
use btc_toxic_flow_monitor_rs::storage::hourly_delta_repo::HourlyDeltaRepo;
use btc_toxic_flow_monitor_rs::storage::SqliteStore;

fn temp_store(label: &str) -> SqliteStore {
    let path = std::env::temp_dir().join(format!(
        "hourly-delta-runtime-{label}-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let store = SqliteStore::open(path.to_string_lossy().as_ref()).expect("sqlite");
    store.migrate().expect("migrate");
    store
}

fn pending_result() -> HourlyDeltaResult {
    HourlyDeltaResult {
        record_key: "binance:BTCUSDT:1h:1700000000000".to_string(),
        exchange: "binance".to_string(),
        symbol: "BTCUSDT".to_string(),
        interval: "1h".to_string(),
        kline_open_time_ms: 1_700_000_000_000,
        kline_close_time_ms: 1_700_003_599_999,
        taker_buy_btc: 2_200.0,
        taker_sell_btc: 5_000.0,
        delta_btc: -2_800.0,
        volume_btc: 7_200.0,
        direction: HourlyDeltaDirection::NetSell,
        above_threshold: true,
        threshold_btc: 1_000.0,
        data_status: HourlyDeltaDataStatus::Closed,
    }
}

#[test]
fn default_config_has_periodic_rest_reconciliation_interval() {
    let config = HourlyDeltaAlertConfig::default();
    assert!(
        format!("{config:?}").contains("rest_reconcile_interval_ms"),
        "the runtime must expose a periodic REST reconciliation interval"
    );
}

#[test]
fn default_config_keeps_a_six_hour_rest_reconciliation_window() {
    let config = HourlyDeltaAlertConfig::default();
    let debug = format!("{config:?}");
    assert!(
        debug.contains("rest_reconcile_lookback_hours: 6"),
        "periodic reconciliation must recover a short WebSocket outage beyond startup backfill"
    );
}

#[test]
fn enabled_runtime_starts_rest_reconciliation_task() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let alert_runtime = HourlyDeltaAlertRuntime::new(
            HourlyDeltaAlertConfig {
                enabled: true,
                ..HourlyDeltaAlertConfig::default()
            },
            false,
            None,
        );
        let handles = alert_runtime.clone().spawn();

        assert_eq!(
            handles.len(),
            4,
            "collector, processor, outbox, and REST reconciliation must all run"
        );

        alert_runtime.stop();
        for handle in handles {
            handle.abort();
        }
    });
}

#[test]
fn enabled_runtime_drains_pending_outbox_without_manual_intervention() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let store = temp_store("drain");
        let result = pending_result();
        store
            .upsert_hourly_delta_closed_result(&result, result.kline_open_time_ms)
            .expect("persist result");
        store
            .enqueue_hourly_delta_discord_outbox(&result.record_key, result.kline_open_time_ms)
            .expect("enqueue result");

        let alert_runtime = HourlyDeltaAlertRuntime::new(
            HourlyDeltaAlertConfig {
                enabled: true,
                dry_run: true,
                outbox_poll_interval_ms: 25,
                startup_backfill_hours: 1,
                rest_reconcile_interval_ms: 60_000,
                ..HourlyDeltaAlertConfig::default()
            },
            false,
            Some(store.clone()),
        );
        let handles = alert_runtime.clone().spawn();
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        alert_runtime.stop();
        for handle in handles {
            handle.abort();
        }

        let record = store
            .get_hourly_delta_record(&result.record_key)
            .expect("load result")
            .expect("result exists");
        assert_eq!(record.discord_status, HourlyDeltaDiscordStatus::DryRun);
        assert_eq!(record.attempts, 1);
        let diagnostics = alert_runtime.diagnostics();
        assert!(diagnostics.outbox_polls > 0);
        assert_eq!(diagnostics.outbox_claimed, 1);
    });
}
