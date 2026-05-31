use std::path::Path;

use serde_json::Value;

fn fixture_root() -> Value {
    serde_json::from_str(include_str!("fixtures/toxic_signal_samples.json"))
        .expect("parse toxic signal fixtures")
}

fn fixture_samples() -> Vec<Value> {
    fixture_root()["samples"]
        .as_array()
        .cloned()
        .expect("samples array")
}

fn sample_by_name<'a>(samples: &'a [Value], name: &str) -> &'a Value {
    samples
        .iter()
        .find(|sample| sample["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing fixture {name}"))
}

fn assert_signal_only_safety(sample: &Value) {
    assert_eq!(sample["readOnly"], true);
    assert_eq!(sample["runtimeModified"], false);
    assert_eq!(sample["analysisOnly"], true);
    assert_eq!(sample["executionEnabled"], false);
    assert_eq!(sample["notificationSent"], false);
    assert_eq!(sample["executionTriggered"], false);
}

fn assert_history_not_durable(sample: &Value) {
    assert_eq!(sample["retentionMode"], "in_memory_bounded");
    assert_eq!(sample["durableStorageEnabled"], false);
    assert_eq!(sample["databaseWriteEnabled"], false);
}

fn assert_alert_not_sent(sample: &Value) {
    assert_eq!(sample["notificationSent"], false);
    assert_eq!(sample["executionTriggered"], false);
}

fn assert_no_execution_words(sample: &Value) {
    let forbidden = [
        "execute",
        "trade",
        "buy",
        "sell",
        "place_order",
        "cancel_order",
        "amend_order",
        "wallet",
        "sign",
        "telegram_send",
        "webhook_send",
        "apply_weight",
        "reload_strategy",
    ];
    for field in [
        "operatorAction",
        "buttonLabel",
        "recommendedAction",
        "nextAction",
    ] {
        if let Some(value) = sample.get(field).and_then(Value::as_str) {
            let lower = value.to_ascii_lowercase();
            let tokens: Vec<&str> = lower
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|token| !token.is_empty())
                .collect();
            for forbidden_word in forbidden {
                assert!(
                    lower != forbidden_word && !tokens.contains(&forbidden_word),
                    "fixture {} field {} contains forbidden word {}",
                    sample["name"].as_str().unwrap_or("unknown"),
                    field,
                    forbidden_word
                );
            }
        }
    }
}

#[test]
fn toxic_signal_fixture_file_exists_and_parses() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/toxic_signal_samples.json");
    assert!(path.exists(), "fixture file should exist");

    let samples = fixture_samples();
    assert_eq!(samples.len(), 12);
}

#[test]
fn all_fixtures_lock_signal_only_safety_fields() {
    for sample in fixture_samples() {
        assert_signal_only_safety(&sample);
    }
}

#[test]
fn alert_preview_fixtures_keep_notification_disabled() {
    let samples = fixture_samples();

    let notify_candidate = sample_by_name(&samples, "high_severity_notify_candidate");
    assert_eq!(notify_candidate["alertDecision"], "notify_candidate");
    assert_alert_not_sent(notify_candidate);
    assert_no_execution_words(notify_candidate);

    let review_candidate = sample_by_name(&samples, "review_candidate");
    assert_eq!(review_candidate["alertDecision"], "review_candidate");
    assert_alert_not_sent(review_candidate);
    assert_no_execution_words(review_candidate);

    let suppressed = sample_by_name(&samples, "suppressed_no_trade_only");
    assert_eq!(suppressed["alertDecision"], "suppressed_no_trade_only");
    assert_eq!(suppressed["noTradeOnly"], true);
    assert_alert_not_sent(suppressed);
    assert_no_execution_words(suppressed);

    let explain = sample_by_name(&samples, "alert_explanation_found_false");
    assert_eq!(explain["found"], false);
    assert_alert_not_sent(explain);
    assert_no_execution_words(explain);
}

#[test]
fn markout_and_history_fixtures_remain_honest() {
    let samples = fixture_samples();

    let not_enough_data = sample_by_name(&samples, "not_enough_data");
    assert_eq!(not_enough_data["markoutStatus"], "not_enough_data");
    assert_eq!(not_enough_data["aligned"], false);
    assert_no_execution_words(not_enough_data);

    let missing_markout = sample_by_name(&samples, "missing_markout");
    assert_eq!(missing_markout["markoutStatus"], "not_enough_data");
    assert_eq!(missing_markout["markoutAvailable"], false);
    assert_no_execution_words(missing_markout);

    let empty_history = sample_by_name(&samples, "empty_history");
    assert_history_not_durable(empty_history);
    assert_eq!(empty_history["items"], Value::Array(vec![]));
    assert_no_execution_words(empty_history);
}

#[test]
fn report_and_governance_fixtures_preserve_boundaries() {
    let samples = fixture_samples();

    let downgrade = sample_by_name(&samples, "downgrade_candidate");
    assert_eq!(downgrade["recommendationAction"], "downgrade_candidate");
    assert_history_not_durable(downgrade);
    assert_no_execution_words(downgrade);

    let rolling = sample_by_name(&samples, "rolling_digest_mixed_markout");
    assert_history_not_durable(rolling);
    assert_eq!(rolling["reportType"], "rolling");
    let summary = rolling["summary"].as_object().expect("rolling summary");
    let total = summary
        .get("totalSignals")
        .and_then(Value::as_u64)
        .expect("total signals");
    let aligned = summary
        .get("aligned")
        .and_then(Value::as_u64)
        .expect("aligned");
    let adverse = summary
        .get("adverse")
        .and_then(Value::as_u64)
        .expect("adverse");
    let neutral = summary
        .get("neutral")
        .and_then(Value::as_u64)
        .expect("neutral");
    let not_enough_data = summary
        .get("notEnoughData")
        .and_then(Value::as_u64)
        .expect("not enough data");
    assert_eq!(total, aligned + adverse + neutral + not_enough_data);
    assert_no_execution_words(rolling);

    let governance = sample_by_name(&samples, "no_governance_ledger");
    assert_eq!(governance["manualReviewRequired"], true);
    assert_eq!(governance["runtimeWeightModified"], false);
    assert_eq!(governance["configModified"], false);
    assert_eq!(governance["ledgerAvailable"], false);
    assert_no_execution_words(governance);
}

#[test]
fn grouping_and_filter_fixtures_keep_view_only_boundaries() {
    let samples = fixture_samples();

    let group = sample_by_name(&samples, "repeated_burst_group");
    assert_eq!(group["originalSignalsPreserved"], true);
    assert!(
        group["memberSignalIds"]
            .as_array()
            .expect("member ids")
            .len()
            >= 3
    );
    assert_no_execution_words(group);

    let filtered = sample_by_name(&samples, "symbol_filtered_view");
    let filter = filtered["filter"].as_object().expect("filter object");
    assert_eq!(filter["viewOnly"], true);
    assert_eq!(filter["persistentWatchlistEnabled"], false);
    assert_eq!(filter["runtimeMonitorModified"], false);
    assert_no_execution_words(filtered);
}
