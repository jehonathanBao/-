use std::fs;

fn read_workspace_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn assert_tokens(text: &str, tokens: &[String], source: &str) {
    for token in tokens {
        assert!(text.contains(token), "{source} is missing token: {token}");
    }
}

#[test]
fn dashboard_signal_ux_template_locks_read_only_polish() {
    let spec_text = include_str!("fixtures/dashboard_signal_ux_spec.json");
    let spec: serde_json::Value = serde_json::from_str(spec_text).expect("dashboard ux spec");

    let index_html = read_workspace_file("web/index.html");
    let script_text = read_workspace_file("web/app.js");
    let styles_text = read_workspace_file("web/styles.css");
    let regression_fixture = read_workspace_file("tests/fixtures/toxic_signal_samples.json");

    assert_tokens(
        &index_html,
        &spec["indexHtmlTokens"]
            .as_array()
            .expect("indexHtmlTokens")
            .iter()
            .map(|value| value.as_str().expect("string").to_owned())
            .collect::<Vec<_>>(),
        "web/index.html",
    );
    assert_tokens(
        &script_text,
        &spec["scriptTokens"]
            .as_array()
            .expect("scriptTokens")
            .iter()
            .map(|value| value.as_str().expect("string").to_owned())
            .collect::<Vec<_>>(),
        "web/app.js",
    );
    assert_tokens(
        &styles_text,
        &spec["styleTokens"]
            .as_array()
            .expect("styleTokens")
            .iter()
            .map(|value| value.as_str().expect("string").to_owned())
            .collect::<Vec<_>>(),
        "web/styles.css",
    );
    assert_tokens(
        &regression_fixture,
        &spec["s13RegressionTokens"]
            .as_array()
            .expect("s13RegressionTokens")
            .iter()
            .map(|value| value.as_str().expect("string").to_owned())
            .collect::<Vec<_>>(),
        "tests/fixtures/toxic_signal_samples.json",
    );

    assert!(script_text.contains("Read-only. Analysis only. Monitoring only."));
    assert!(script_text.contains("No order placement"));
    assert!(script_text.contains("No live trading"));
    assert!(script_text.contains("notificationSent=false"));
    assert!(script_text.contains("executionTriggered=false"));
    assert!(script_text.contains("retentionMode=in_memory_bounded"));
    assert!(script_text.contains("durableStorageEnabled=false"));
    assert!(script_text.contains("databaseWriteEnabled=false"));
}
