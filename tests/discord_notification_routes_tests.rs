use btc_toxic_flow_monitor_rs::api::discord_notification_routes::{
    discord_embed_color_from_direction, discord_payload_for_tests, normalize_signal_direction,
    reserve_discord_push_for_tests, reset_discord_push_limits_for_tests,
    DiscordNotificationRequest, NormalizedDiscordDirection,
};

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
        test: None,
    }
}
