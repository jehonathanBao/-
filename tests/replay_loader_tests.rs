use std::fs;

use btc_toxic_flow_monitor_rs::replay::{replay_loader::load_jsonl, replay_types::ReplayEvent};

#[test]
fn loader_parses_supported_event_types() {
    let path = temp_file(
        "loader_parses_supported_event_types",
        r#"
{"type":"trade","venue":"binance","ts":1,"price":100000,"sizeBtc":300,"aggressorSide":"buy","tradeId":"t1"}
{"type":"book","venue":"binance","ts":2,"bestBid":99990,"bestAsk":100010,"bids":[[99990,10]],"asks":[[100010,10]]}
{"type":"expect_toxic","ts":3,"direction":"buy","minToxicVolumeBtc":1000,"windowMs":5000}
"#,
    );

    let events = load_jsonl(&path).expect("load");
    assert_eq!(events.len(), 3);
    assert!(matches!(events[0], ReplayEvent::Trade(_)));
    assert!(matches!(events[1], ReplayEvent::Book(_)));
    assert!(matches!(events[2], ReplayEvent::ExpectToxic(_)));
}

#[test]
fn loader_reports_bad_json_with_line_numbers() {
    let path = temp_file("loader_reports_bad_json", "not-json\n");
    let err = load_jsonl(&path).expect_err("expected error");
    assert!(err.to_string().contains("line 1"));
}

#[test]
fn loader_reports_unknown_types_with_line_numbers() {
    let path = temp_file("loader_reports_unknown", r#"{"type":"mystery","ts":1}"#);
    let err = load_jsonl(&path).expect_err("expected error");
    assert!(err.to_string().contains("unknown type"));
}

fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "btc_toxic_flow_monitor_rs_{}_{}_{}.jsonl",
        name,
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().expect("nanos")
    ));
    fs::write(&path, contents.trim_start()).expect("write temp file");
    path
}
