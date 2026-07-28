use btc_toxic_flow_monitor_rs::contract_whale_monitor::hourly_delta_alert::{
    HourlyDeltaAlertConfig, HourlyDeltaAlertRuntime,
};

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
