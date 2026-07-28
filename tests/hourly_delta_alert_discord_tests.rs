use std::time::{SystemTime, UNIX_EPOCH};

use btc_toxic_flow_monitor_rs::{
    contract_whale_monitor::hourly_delta_alert::{
        build_hourly_delta_discord_content, build_hourly_delta_discord_payload,
        notify_hourly_delta_discord,
        types::{
            HourlyDeltaDataStatus, HourlyDeltaDirection, HourlyDeltaDiscordStatus,
            HourlyDeltaResult,
        },
        HourlyDeltaDiscordSettings,
    },
    storage::{hourly_delta_repo::HourlyDeltaRepo, SqliteStore},
};

fn sample_result(delta: f64, above: bool) -> HourlyDeltaResult {
    let volume = 7_700.0;
    let buy = (volume + delta) / 2.0;
    HourlyDeltaResult {
        record_key: format!("binance:BTCUSDT:1h:{}", 1_700_000_000_000i64),
        exchange: "binance".into(),
        symbol: "BTCUSDT".into(),
        interval: "1h".into(),
        kline_open_time_ms: 1_700_000_000_000,
        kline_close_time_ms: 1_700_003_599_999,
        taker_buy_btc: buy,
        taker_sell_btc: volume - buy,
        delta_btc: delta,
        volume_btc: volume,
        direction: if delta > 0.0 {
            HourlyDeltaDirection::NetBuy
        } else if delta < 0.0 {
            HourlyDeltaDirection::NetSell
        } else {
            HourlyDeltaDirection::Flat
        },
        above_threshold: above,
        threshold_btc: 1000.0,
        data_status: HourlyDeltaDataStatus::Closed,
    }
}

fn temp_store(label: &str) -> SqliteStore {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("hourly-delta-{label}-{nanos}.sqlite"));
    let _ = std::fs::remove_file(&path);
    let store = SqliteStore::open(path.to_string_lossy().as_ref()).expect("temp sqlite");
    store.migrate().expect("migrate sqlite");
    store
}

#[test]
fn discord_copy_is_active_trade_net_delta_only() {
    let content = build_hourly_delta_discord_content(&sample_result(-2_800.0, true));
    assert!(content.contains("主动成交净卖出"));
    assert!(content.contains("🔴 偏空"));
    assert!(content.contains("净卖出：2,800 BTC"));
    assert!(content.contains("卖出占比：68.2%"));
    assert!(content.contains("买入占比：31.8%"));
    assert!(content.contains("方向强度"));
    assert!(content.contains("主动买入"));
    assert!(content.contains("主动卖出"));
    assert!(content.contains("净差"));
    assert!(content.contains("状态：1H 已收线"));
    assert!(!content.contains("资金净流入"));
    assert!(!content.contains("持仓增加"));
    assert!(!content.contains("主力买卖"));

    let buy = build_hourly_delta_discord_content(&sample_result(1_200.0, true));
    assert!(buy.contains("主动成交净买入"));
    assert!(buy.contains("🟢 偏多"));
    assert!(buy.contains("净买入：1,200 BTC"));
    assert!(buy.contains("🟢"));
}

#[test]
fn direction_first_payload_prioritizes_bias_and_delta() {
    let payload = build_hourly_delta_discord_payload(&sample_result(-2_800.0, true));
    let embed = &payload["embeds"][0];
    assert_eq!(embed["color"], 0xEF_44_44);
    assert!(embed["title"].as_str().unwrap().contains("偏空"));

    let fields = embed["fields"].as_array().unwrap();
    assert_eq!(fields[0]["name"], "方向");
    assert_eq!(fields[1]["name"], "净差 Delta");
    assert_eq!(fields[2]["name"], "净卖出");
    assert_eq!(fields[3]["name"], "卖出占比");
    assert_eq!(fields[4]["name"], "买入占比");
    assert_eq!(fields[5]["name"], "方向强度");
    assert_eq!(fields[6]["name"], "主动卖出");
    assert_eq!(fields[7]["name"], "主动买入");
    assert!(fields[8]["value"].as_str().unwrap().contains("7,700 BTC"));
    assert!(embed["footer"]["text"]
        .as_str()
        .unwrap()
        .contains("record="));
}

#[test]
fn dry_run_builds_payload_without_sending() {
    let settings = HourlyDeltaDiscordSettings {
        enabled: true,
        dry_run: true,
        webhook_url: None,
        timeout_ms: 1_000,
        max_attempts: 1,
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let outcome = runtime.block_on(notify_hourly_delta_discord(
        &settings,
        &sample_result(-2_800.0, true),
    ));
    assert!(outcome.eligible);
    assert!(outcome.dry_run);
    assert!(!outcome.sent);
    assert_eq!(outcome.reason, "dry_run");
    assert!(outcome.payload.is_some());

    let payload = build_hourly_delta_discord_payload(&sample_result(-2_800.0, true));
    assert!(payload.get("content").is_some());
    assert!(payload.get("embeds").is_some());
}

#[test]
fn outbox_is_idempotent_and_marks_sent() {
    let store = temp_store("outbox");
    let result = sample_result(-2_800.0, true);
    assert!(store
        .upsert_hourly_delta_closed_result(&result, result.kline_open_time_ms)
        .unwrap());
    assert!(!store
        .upsert_hourly_delta_closed_result(&result, result.kline_open_time_ms)
        .unwrap());

    assert!(store
        .enqueue_hourly_delta_discord_outbox(&result.record_key, result.kline_open_time_ms)
        .unwrap());
    assert!(!store
        .enqueue_hourly_delta_discord_outbox(&result.record_key, result.kline_open_time_ms)
        .unwrap());

    let claimed = store
        .claim_hourly_delta_discord_outbox(10, result.kline_open_time_ms)
        .unwrap();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].record_key, result.record_key);

    store
        .finish_hourly_delta_discord_outbox(
            &result.record_key,
            HourlyDeltaDiscordStatus::Sent,
            None,
            Some(result.kline_open_time_ms + 1),
            None,
        )
        .unwrap();

    assert!(store
        .claim_hourly_delta_discord_outbox(10, result.kline_open_time_ms + 10_000)
        .unwrap()
        .is_empty());
    assert!(!store
        .enqueue_hourly_delta_discord_outbox(&result.record_key, result.kline_open_time_ms + 20_000)
        .unwrap());

    let loaded = store
        .get_hourly_delta_record(&result.record_key)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.discord_status, HourlyDeltaDiscordStatus::Sent);
    assert!((loaded.delta_btc - (-2_800.0)).abs() < 1e-9);
}

#[test]
fn below_threshold_record_is_not_enqueued() {
    let store = temp_store("below");
    let result = sample_result(500.0, false);
    assert!(store
        .upsert_hourly_delta_closed_result(&result, result.kline_open_time_ms)
        .unwrap());
    assert!(!store
        .enqueue_hourly_delta_discord_outbox(&result.record_key, result.kline_open_time_ms)
        .unwrap());
}
