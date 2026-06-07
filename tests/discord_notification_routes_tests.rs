use btc_toxic_flow_monitor_rs::api::discord_notification_routes::{
    discord_embed_color_from_direction, discord_payload_for_tests, evaluate_discord_alert_gate,
    normalize_signal_direction, reserve_discord_push_for_tests,
    reserve_discord_push_for_tests_with_cooldown, reset_discord_auto_push_for_tests,
    reset_discord_push_limits_for_tests, DiscordAlertMode, DiscordNotificationRequest,
    NormalizedDiscordDirection,
};
use btc_toxic_flow_monitor_rs::runtime::{
    advanced_tof_metrics::AdvancedTofMetrics, perp_tof_metrics::PerpTofMetrics,
    tof_metrics::TofDirection,
};
use std::{fs, path::Path};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn duplicate_discord_push_is_suppressed_for_same_signal_key() {
    reset_discord_push_limits_for_tests();

    assert_eq!(reserve_discord_push_for_tests("sig_001"), None);
    assert_eq!(
        reserve_discord_push_for_tests("sig_001"),
        Some("DUPLICATE_PUSH_SUPPRESSED")
    );

    reset_discord_push_limits_for_tests();
}

#[test]
fn burst_discord_pushes_are_rate_limited() {
    reset_discord_push_limits_for_tests();

    for index in 0..5 {
        assert_eq!(
            reserve_discord_push_for_tests(&format!("sig_{index}")),
            None
        );
    }
    assert_eq!(
        reserve_discord_push_for_tests("sig_over_limit"),
        Some("RATE_LIMITED")
    );

    reset_discord_push_limits_for_tests();
}

#[test]
fn auto_push_high_critical_candidate_passes_gate() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_alert_env();
    let decision = evaluate_discord_alert_gate(&high_request(), DiscordAlertMode::Auto);

    assert!(decision.allowed);
    assert_eq!(decision.reason, "passed");

    clear_alert_env();
}

#[test]
fn auto_push_rejects_medium_low_score_quality_and_missing_webhook() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_alert_env();

    let mut medium = high_request();
    medium.level = Some("medium".to_string());
    assert_eq!(
        evaluate_discord_alert_gate(&medium, DiscordAlertMode::Auto).reason,
        "non_high_risk"
    );

    let mut low = high_request();
    low.level = Some("low".to_string());
    assert_eq!(
        evaluate_discord_alert_gate(&low, DiscordAlertMode::Auto).reason,
        "non_high_risk"
    );

    let mut low_score = high_request();
    low_score.score = Some(79);
    assert_eq!(
        evaluate_discord_alert_gate(&low_score, DiscordAlertMode::Auto).reason,
        "score_below_threshold"
    );

    let mut low_quality = high_request();
    low_quality.data_quality = Some(69.0);
    assert_eq!(
        evaluate_discord_alert_gate(&low_quality, DiscordAlertMode::Auto).reason,
        "data_quality_below_threshold"
    );

    std::env::remove_var("DISCORD_WEBHOOK_URL");
    assert_eq!(
        evaluate_discord_alert_gate(&high_request(), DiscordAlertMode::Auto).reason,
        "webhook_missing"
    );

    clear_alert_env();
}

#[test]
fn auto_push_respects_duplicate_and_cooldown_limiters() {
    reset_discord_push_limits_for_tests();

    assert_eq!(reserve_discord_push_for_tests("sig_auto_001"), None);
    assert_eq!(
        reserve_discord_push_for_tests("sig_auto_001"),
        Some("DUPLICATE_PUSH_SUPPRESSED")
    );

    reset_discord_push_limits_for_tests();
    assert_eq!(
        reserve_discord_push_for_tests_with_cooldown("sig_auto_001", "BTC:spoof:ask"),
        None
    );
    assert_eq!(
        reserve_discord_push_for_tests_with_cooldown("sig_auto_002", "BTC:spoof:ask"),
        Some("COOLDOWN_SUPPRESSED")
    );

    reset_discord_push_limits_for_tests();
}

#[test]
fn manual_push_still_works_when_auto_push_is_disabled() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_alert_env();
    std::env::set_var("DISCORD_AUTO_PUSH_ENABLED", "false");

    assert!(!evaluate_discord_alert_gate(&high_request(), DiscordAlertMode::Auto).allowed);
    assert!(evaluate_discord_alert_gate(&high_request(), DiscordAlertMode::Manual).allowed);

    clear_alert_env();
}

#[test]
fn read_only_does_not_disable_notifications_but_dry_run_does() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_alert_env();
    std::env::set_var("READ_ONLY", "true");

    assert!(evaluate_discord_alert_gate(&high_request(), DiscordAlertMode::Auto).allowed);

    std::env::set_var("DRY_RUN", "true");
    let decision = evaluate_discord_alert_gate(&high_request(), DiscordAlertMode::Auto);
    assert!(!decision.allowed);
    assert_eq!(decision.reason, "dry_run");

    clear_alert_env();
}

#[test]
fn severity_s_and_a_map_to_high_critical_gate() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_alert_env();

    let mut s_level = high_request();
    s_level.level = Some("S".to_string());
    assert!(evaluate_discord_alert_gate(&s_level, DiscordAlertMode::Auto).allowed);

    let mut a_level = high_request();
    a_level.level = Some("A".to_string());
    assert!(evaluate_discord_alert_gate(&a_level, DiscordAlertMode::Auto).allowed);

    clear_alert_env();
}

#[test]
fn discord_auto_push_sent_log_kind_is_defined() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/api/discord_notification_routes.rs"))
        .expect("discord route source");

    assert!(source.contains("alert_gate_evaluated"));
    assert!(source.contains("discord_auto_push_queued"));
    assert!(source.contains("discord_auto_push_sent"));
    assert!(source.contains("discord_auto_push_skipped"));
}

#[test]
fn bullish_discord_payload_uses_green_embed_and_title_emoji() {
    let mut request = high_request();
    request.side = Some("Bid/Buy".to_string());

    let payload = discord_payload_for_tests(&request);
    let embed = payload.embeds.first().expect("discord embed");
    let text = serde_json::to_string(&payload).expect("payload json");

    assert_eq!(
        normalize_signal_direction(Some("Bid/Buy")),
        NormalizedDiscordDirection::Bullish
    );
    assert_eq!(
        discord_embed_color_from_direction(NormalizedDiscordDirection::Bullish),
        5_763_719
    );
    assert_eq!(embed.color, 5_763_719);
    assert!(embed.title.starts_with("🟢"));
    assert!(text.contains("🟢 看涨 / Bid-Buy"));
}

#[test]
fn bearish_discord_payload_uses_red_embed_and_title_emoji() {
    let mut request = high_request();
    request.side = Some("Ask/Sell".to_string());

    let payload = discord_payload_for_tests(&request);
    let embed = payload.embeds.first().expect("discord embed");
    let text = serde_json::to_string(&payload).expect("payload json");

    assert_eq!(
        normalize_signal_direction(Some("Ask/Sell")),
        NormalizedDiscordDirection::Bearish
    );
    assert_eq!(
        discord_embed_color_from_direction(NormalizedDiscordDirection::Bearish),
        15_548_997
    );
    assert_eq!(embed.color, 15_548_997);
    assert!(embed.title.starts_with("🔴"));
    assert!(text.contains("🔴 看跌 / Ask-Sell"));
}

#[test]
fn neutral_discord_payload_uses_gray_embed_and_title_emoji() {
    let mut request = high_request();
    request.side = Some("unknown".to_string());

    let payload = discord_payload_for_tests(&request);
    let embed = payload.embeds.first().expect("discord embed");
    let text = serde_json::to_string(&payload).expect("payload json");

    assert_eq!(
        normalize_signal_direction(Some("unknown")),
        NormalizedDiscordDirection::Neutral
    );
    assert_eq!(
        discord_embed_color_from_direction(NormalizedDiscordDirection::Neutral),
        9_807_270
    );
    assert_eq!(embed.color, 9_807_270);
    assert!(embed.title.starts_with("🟡"));
    assert!(text.contains("🟡 中性 / 未知"));
}

#[test]
fn direction_colored_payload_still_does_not_leak_sensitive_or_technical_fields() {
    let mut request = high_request();
    request.side = Some("Ask/Sell".to_string());
    request.markout_1s_bps = Some(-2.0);
    request.markout_5s_bps = Some(-4.0);
    request.markout_30s_bps = Some(-8.0);
    request.impact = Some("rawPayload evidence markout webhook token Authorization".to_string());

    let payload = discord_payload_for_tests(&request);
    let text = serde_json::to_string(&payload).expect("payload json");

    assert!(text.contains("🔴 看跌 / Ask-Sell"));
    for forbidden in [
        "rawPayload",
        "evidence",
        "markout",
        "webhook",
        "token",
        "Authorization",
        "DISCORD_WEBHOOK_URL",
        "OPERATOR_TOKEN",
    ] {
        assert!(
            !text.contains(forbidden),
            "forbidden field leaked into discord payload: {forbidden}"
        );
    }
}

#[test]
fn discord_payload_includes_safe_perp_tof_metrics() {
    let mut request = high_request();
    request.final_candidate_type = Some("High Risk Bullish Candidate".to_string());
    request.perp_candidate_type = Some("OpenInterestCandidate".to_string());
    request.metrics_direction = Some("bullish".to_string());
    request.perp_score = Some(87);
    request.perp_tof_metrics = Some(PerpTofMetrics {
        oi_change: 150_000.0,
        oi_direction: "long_increase".to_string(),
        funding_rate: -0.071,
        funding_side: "short".to_string(),
        liquidation_pressure: 82.0,
        squeeze_side: "short".to_string(),
        agg_buy_volume: 1_500_000.0,
        agg_sell_volume: 420_000.0,
        direction_bias: TofDirection::Bullish,
        metrics_direction: TofDirection::Bullish,
        risk_score: 87,
        data_quality: 88.0,
        candidate_type: "OpenInterestCandidate".to_string(),
        explain_tags: vec!["OI long increase".to_string()],
        confidence: 87.0,
    });

    let payload = discord_payload_for_tests(&request);
    let text = serde_json::to_string(&payload).expect("payload json");

    assert!(text.contains("合约 TOF 指标"));
    assert!(text.contains("OpenInterestCandidate"));
    assert!(text.contains("long_increase"));
    assert!(text.contains("Funding"));
    assert!(text.contains("Liq"));
    for forbidden in ["rawPayload", "evidence", "markout", "webhook", "token"] {
        assert!(!text.contains(forbidden));
    }
}

#[test]
fn discord_payload_includes_safe_advanced_metrics() {
    let mut request = high_request();
    request.advanced_candidate_type = Some("MarketPressureHeatmapCandidate".to_string());
    request.advanced_score = Some(89);
    request.advanced_tof_metrics = Some(AdvancedTofMetrics {
        vpin_enhanced: 88.0,
        large_order_flow_cluster: 76.0,
        historical_funding_oi_trend: 84.0,
        market_pressure_heatmap: 91.0,
        spot_risk_score: 86,
        spot_tof_score: 88.0,
        perp_score: 87,
        final_risk_score: 89,
        data_quality: 86.0,
        metrics_completeness: 95.0,
        fresh_data_coverage: 92.0,
        candidate_type: "MarketPressureHeatmapCandidate".to_string(),
        final_candidate_type: "High Risk Bullish Advanced Candidate".to_string(),
        metrics_direction: TofDirection::Bullish,
        confidence: 90.0,
        explain_tags: vec!["Market pressure heatmap".to_string()],
    });

    let payload = discord_payload_for_tests(&request);
    let text = serde_json::to_string(&payload).expect("payload json");

    assert!(text.contains("高级指标"));
    assert!(text.contains("MarketPressureHeatmapCandidate"));
    assert!(text.contains("VPIN+"));
    assert!(text.contains("FlowCluster"));
    assert!(text.contains("FundingOI"));
    assert!(text.contains("Heatmap"));
    for forbidden in ["rawPayload", "evidence", "markout", "webhook", "token"] {
        assert!(!text.contains(forbidden));
    }
}

fn high_request() -> DiscordNotificationRequest {
    DiscordNotificationRequest {
        signal_id: Some("manual-high-direction-color".to_string()),
        id: None,
        dedupe_key: Some("manual-high-direction-color".to_string()),
        exchange: Some("manual-smoke".to_string()),
        symbol: Some("BTC-PERP".to_string()),
        signal_type: Some("spoofing_candidate".to_string()),
        level: Some("critical".to_string()),
        side: Some("Ask/Sell".to_string()),
        score: Some(92),
        data_quality: Some(88.0),
        reason: Some("Manual smoke test High/Critical candidate Discord path".to_string()),
        impact: None,
        time: Some("2026-06-05T21:51:00.000Z".to_string()),
        price_range: None,
        add_qty: Some(1000.0),
        cancel_qty: Some(900.0),
        fill_qty: Some(10.0),
        cancel_to_trade_ratio: Some(90.0),
        depth_before: Some(100.0),
        depth_after: Some(20.0),
        depth_impact: Some(0.8),
        price_impact_bps: Some(-4.0),
        markout_1s_bps: None,
        markout_5s_bps: None,
        markout_30s_bps: None,
        tof_metrics: None,
        tof_score: None,
        candidate_type: None,
        explain_tags: None,
        direction_confidence: None,
        perp_tof_metrics: None,
        perp_score: None,
        perp_candidate_type: None,
        final_candidate_type: None,
        metrics_direction: None,
        advanced_tof_metrics: None,
        advanced_score: None,
        advanced_candidate_type: None,
        test: None,
    }
}

fn set_alert_env() {
    reset_discord_push_limits_for_tests();
    reset_discord_auto_push_for_tests();
    std::env::set_var("ALERT_MIN_SCORE", "80");
    std::env::set_var("ALERT_MIN_DATA_QUALITY", "70");
    std::env::set_var("DISCORD_AUTO_PUSH_ENABLED", "true");
    std::env::set_var("DRY_RUN", "false");
    std::env::set_var(
        "DISCORD_WEBHOOK_URL",
        "https://discord.com/api/webhooks/test-id/test-token",
    );
}

fn clear_alert_env() {
    reset_discord_push_limits_for_tests();
    reset_discord_auto_push_for_tests();
    for key in [
        "ALERT_MIN_SCORE",
        "ALERT_MIN_DATA_QUALITY",
        "DISCORD_AUTO_PUSH_ENABLED",
        "DRY_RUN",
        "READ_ONLY",
        "DISCORD_WEBHOOK_URL",
    ] {
        std::env::remove_var(key);
    }
}
