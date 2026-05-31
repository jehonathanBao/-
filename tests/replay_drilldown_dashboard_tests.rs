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
fn replay_drilldown_dashboard_template_locks_read_only_overlay() {
    let spec_text = include_str!("fixtures/replay_drilldown_ui_spec.json");
    let spec: serde_json::Value = serde_json::from_str(spec_text).expect("replay drilldown spec");

    let index_html = read_workspace_file("web/index.html");
    let script_text = read_workspace_file("web/app.js");
    let styles_text = read_workspace_file("web/styles.css");

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

    assert!(script_text.contains("readOnly=true"));
    assert!(script_text.contains("analysisOnly=true"));
    assert!(script_text.contains("executionEnabled=false"));
    assert!(script_text.contains("persistentWatchlistEnabled=false"));
    assert!(script_text.contains("runtimeMonitorModified=false"));
    assert!(script_text.contains("/api/toxicity/markout/:symbol"));
    assert!(script_text.contains("/api/toxicity/quality-scorecard/:symbol"));
    assert!(script_text.contains("/api/toxicity/weight-recommendation/:symbol"));
    assert!(script_text.contains("/api/toxicity/weight-review/:symbol"));
    assert!(script_text.contains("/api/toxicity/governance-ledger/:symbol"));
    assert!(script_text.contains("/api/toxicity/signal-alert-preview/explain/:signal_id"));
}
