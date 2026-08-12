use btc_toxic_flow_monitor_rs::api::discord_notification_routes::{
    discord_embed_color_from_direction, discord_payload_for_tests, evaluate_discord_alert_gate,
    normalize_signal_direction, reserve_discord_push_for_tests,
    reserve_discord_push_for_tests_with_cooldown, reset_discord_auto_push_for_tests,
    reset_discord_push_limits_for_tests, DiscordAlertMode, DiscordNotificationRequest,
    NormalizedDiscordDirection,
};
use btc_toxic_flow_monitor_rs::runtime::{
    perp_tof_metrics::PerpTofMetrics,
    tof_metrics::{
        build_tof_metrics_from_observed, enhance_signal_summary, ObservedTofSnapshot, TofDirection,
        TofSummaryInput,
    },
};
use std::{collections::BTreeMap, fs, path::Path};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn duplicate_discord_push_is_suppressed_for_same_signal_key() {
    let _guard = ENV_LOCK.lock().expect("env lock");
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
    let _guard = ENV_LOCK.lock().expect("env lock");
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
fn inferred_or_client_forged_evidence_cannot_pass_short_toxic_gate() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_alert_env();

    let mut inferred = high_request();
    inferred.server_evidence_verified = false;
    let decision = evaluate_discord_alert_gate(&inferred, DiscordAlertMode::Auto);
    assert!(!decision.allowed);
    assert_eq!(decision.reason, "authoritative_evidence_unavailable");

    let forged: DiscordNotificationRequest = serde_json::from_value(serde_json::json!({
        "alertFamily": "short_toxic_order",
        "signalId": "forged",
        "level": "critical",
        "score": 99,
        "confidence": 99,
        "dataQuality": 99,
        "serverEvidenceVerified": true
    }))
    .expect("request deserializes");
    assert!(!forged.server_evidence_verified);
    let forged_decision = evaluate_discord_alert_gate(&forged, DiscordAlertMode::Auto);
    assert_eq!(forged_decision.reason, "authoritative_evidence_unavailable");

    let forged_market: DiscordNotificationRequest = serde_json::from_value(serde_json::json!({
        "alertFamily": "market_structure",
        "signalId": "forged-market",
        "mainForceScore": 99,
        "marketStructureConfidence": 99,
        "marketStructureDataQuality": 99,
        "mainForceConfirmed": true,
        "serverEvidenceVerified": true
    }))
    .expect("market request deserializes");
    assert!(!forged_market.server_evidence_verified);
    let forged_market_decision =
        evaluate_discord_alert_gate(&forged_market, DiscordAlertMode::Auto);
    assert_eq!(
        forged_market_decision.reason,
        "authoritative_evidence_unavailable"
    );

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
    low_score.score = Some(84);
    assert_eq!(
        evaluate_discord_alert_gate(&low_score, DiscordAlertMode::Auto).reason,
        "score_below_threshold"
    );

    let mut low_confidence = high_request();
    low_confidence.confidence = Some(69.0);
    assert_eq!(
        evaluate_discord_alert_gate(&low_confidence, DiscordAlertMode::Auto).reason,
        "confidence_below_threshold"
    );

    let mut low_quality = high_request();
    low_quality.data_quality = Some(69.0);
    assert_eq!(
        evaluate_discord_alert_gate(&low_quality, DiscordAlertMode::Auto).reason,
        "data_quality_below_threshold"
    );

    std::env::remove_var("SHORT_TOXIC_DISCORD_WEBHOOK_URL");
    assert_eq!(
        evaluate_discord_alert_gate(&high_request(), DiscordAlertMode::Auto).reason,
        "webhook_missing"
    );

    clear_alert_env();
}

#[test]
fn auto_push_respects_duplicate_and_cooldown_limiters() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    reset_discord_push_limits_for_tests();

    assert_eq!(reserve_discord_push_for_tests("sig_auto_001"), None);
    assert_eq!(
        reserve_discord_push_for_tests("sig_auto_001"),
        Some("DUPLICATE_PUSH_SUPPRESSED")
    );

    reset_discord_push_limits_for_tests();
    assert_eq!(
        reserve_discord_push_for_tests_with_cooldown("sig_auto_001", "BTC:spoof:ask", 60),
        None
    );
    assert_eq!(
        reserve_discord_push_for_tests_with_cooldown("sig_auto_002", "BTC:spoof:ask", 60),
        Some("COOLDOWN_SUPPRESSED")
    );

    reset_discord_push_limits_for_tests();
}

#[test]
fn manual_push_still_works_when_auto_push_is_disabled() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_alert_env();
    std::env::set_var("SHORT_TOXIC_DISCORD_AUTO_PUSH_ENABLED", "false");

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
    assert!(text.contains("短线有毒订单"));
    assert!(text.contains("虚假挂单 / 撤单诱导"));
    assert!(text.contains("偏多"));
    assert!(text.contains("不代表中长线趋势"));
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
    assert!(text.contains("短线有毒订单"));
    assert!(text.contains("虚假挂单 / 撤单诱导"));
    assert!(text.contains("偏空"));
    assert!(text.contains("不代表中长线趋势"));
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
    assert!(text.contains("短线有毒订单"));
    assert!(text.contains("虚假挂单 / 撤单诱导"));
    assert!(text.contains("中性 / 不明确"));
    assert!(text.contains("不代表中长线趋势"));
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

    assert!(text.contains("短线有毒订单"));
    assert!(text.contains("偏空"));
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
fn short_toxic_discord_payload_excludes_perp_and_advanced_context() {
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
        observed_liquidation_notional: None,
        lineage: Default::default(),
        liquidation_lineage: Default::default(),
    });

    let payload = discord_payload_for_tests(&request);
    let text = serde_json::to_string(&payload).expect("payload json");

    assert!(text.contains("短线有毒订单"));
    assert!(text.contains("不代表中长线趋势"));
    for forbidden in [
        "合约 TOF 指标",
        "高级指标",
        "最终候选",
        "OpenInterestCandidate",
        "MarketPressureHeatmapCandidate",
        "rawPayload",
        "evidence",
        "markout",
        "webhook",
        "token",
    ] {
        assert!(!text.contains(forbidden));
    }
}

#[test]
fn unavailable_tof_payload_does_not_claim_sweep_spike_book_or_l2_evidence() {
    let mut request = high_request();
    request.signal_type = Some("unclassified_toxic_flow".to_string());
    request.candidate_type = None;
    request.tof_metrics = Some(
        enhance_signal_summary(&TofSummaryInput {
            signal_kind: "unclassified_toxic_flow",
            direction_bias: "bearish",
            severity: "critical",
            confidence: 0.9,
            quality_bucket: "unavailable",
            summary: "detector-only",
            existing_risk_score: 92,
            existing_data_quality: 88.0,
        })
        .tof_metrics,
    );

    let payload = discord_payload_for_tests(&request);
    let text = serde_json::to_string(&payload).expect("payload json");

    assert!(text.contains("短线毒性风险升高"));
    for unsupported in ["扫盘", "插针", "扫穿", "L2"] {
        assert!(
            !text.contains(unsupported),
            "unavailable TOF must not claim {unsupported} evidence"
        );
    }
}

#[test]
fn discord_tof_field_respects_per_metric_lineage() {
    let mut request = high_request();
    let metrics = build_tof_metrics_from_observed(
        &ObservedTofSnapshot {
            symbol: "BTC-PERP".to_string(),
            observed_at_ms: 10_000,
            buy_volume: 300.0,
            sell_volume: 100.0,
            trade_count: 40,
            window_ms: 5_000,
            vpin: Some(0.82),
            vpin_zscore: Some(2.4),
            vpin_percentile: Some(0.96),
            vpin_bucket_count: 20,
            vpin_window_volume: 2_000.0,
            per_venue_vpin: BTreeMap::from([("binance".to_string(), 0.82)]),
            bid_depth_withdrawal: Some(12.0),
            ask_depth_withdrawal: Some(60.0),
            spread_bps: None,
            book_update_rate: None,
            sweep_score: None,
        },
        "BTC-PERP",
        9_500,
        10_000,
        92,
    );
    assert!(metrics.lineage.alert_eligible);
    assert!(!metrics.metric_lineage["spread"].available);
    request.tof_score = Some(metrics.tof_score);
    request.tof_metrics = Some(metrics);

    let payload = discord_payload_for_tests(&request);
    let text = serde_json::to_string(&payload).expect("payload json");

    assert!(text.contains("Spread N/A"));
    assert!(!text.contains("Spread 0.0bps"));
    assert!(text.contains("Depth 60"));
}

#[test]
fn market_structure_main_force_gate_passes_with_separate_family() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_market_structure_env();

    let mut request = high_request();
    request.alert_family = Some("market_structure".to_string());
    request.level = Some("Major".to_string());
    request.main_force_score = Some(84);
    request.market_structure_confidence = Some(76.0);
    request.market_structure_data_quality = Some(74.0);
    request.extreme_impact_score = Some(58);
    request.structure_bias = Some(62);
    request.main_force_confirmed = Some(true);

    let decision = evaluate_discord_alert_gate(&request, DiscordAlertMode::Auto);
    assert!(decision.allowed);
    assert_eq!(decision.reason, "passed");

    clear_alert_env();
}

#[test]
fn market_structure_extreme_gate_passes_without_main_force_confirmation() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_market_structure_env();

    let mut request = high_request();
    request.alert_family = Some("market_structure".to_string());
    request.level = Some("Extreme".to_string());
    request.main_force_score = Some(54);
    request.market_structure_confidence = Some(52.0);
    request.market_structure_data_quality = Some(76.0);
    request.extreme_impact_score = Some(91);
    request.structure_bias = Some(-68);
    request.main_force_confirmed = Some(false);

    let decision = evaluate_discord_alert_gate(&request, DiscordAlertMode::Auto);
    assert!(decision.allowed);
    assert_eq!(decision.reason, "passed");

    clear_alert_env();
}

#[test]
fn market_structure_impact_level_a_enters_discord_gate_without_extreme_score() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_market_structure_env();

    let mut request = high_request();
    request.alert_family = Some("market_structure".to_string());
    request.level = Some("A".to_string());
    request.main_force_score = Some(54);
    request.market_structure_confidence = Some(52.0);
    request.market_structure_data_quality = Some(76.0);
    request.extreme_impact_score = Some(64);
    request.structure_bias = Some(20);
    request.main_force_confirmed = Some(false);
    request.impact_level = Some("A".to_string());

    let decision = evaluate_discord_alert_gate(&request, DiscordAlertMode::Auto);
    assert!(decision.allowed);
    assert_eq!(decision.reason, "passed");
    assert_eq!(decision.score, 85);

    clear_alert_env();
}

#[test]
fn market_structure_impact_level_b_enters_discord_gate_without_extreme_score() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    set_market_structure_env();

    let mut request = high_request();
    request.alert_family = Some("market_structure".to_string());
    request.level = Some("B".to_string());
    request.main_force_score = Some(54);
    request.market_structure_confidence = Some(52.0);
    request.market_structure_data_quality = Some(76.0);
    request.extreme_impact_score = Some(64);
    request.structure_bias = Some(-20);
    request.main_force_confirmed = Some(false);
    request.impact_level = Some("B".to_string());

    let decision = evaluate_discord_alert_gate(&request, DiscordAlertMode::Auto);
    assert!(decision.allowed);
    assert_eq!(decision.reason, "passed");
    assert_eq!(decision.score, 80);

    clear_alert_env();
}

#[test]
fn market_structure_discord_payload_uses_main_force_wording() {
    let mut request = high_request();
    request.alert_family = Some("market_structure".to_string());
    request.level = Some("Major".to_string());
    request.main_force_score = Some(84);
    request.market_structure_confidence = Some(76.0);
    request.market_structure_data_quality = Some(74.0);
    request.extreme_impact_score = Some(58);
    request.structure_bias = Some(62);
    request.market_structure_severity = Some("Major".to_string());
    request.regime_type = Some("main_force_long_build".to_string());
    request.spot_score = Some(71);
    request.contract_score = Some(86);
    request.cross_confirm_score = Some(74);
    request.main_force_confirmed = Some(true);
    request.oi_score = Some(82);
    request.liquidation_score = Some(44);

    let payload = discord_payload_for_tests(&request);
    let text = serde_json::to_string(&payload).expect("payload json");

    assert!(text.contains("主力结构异动"));
    assert!(text.contains("主力建多"));
    assert!(text.contains("主力评分"));
    assert!(text.contains("结构方向"));
    assert!(text.contains("现货评分"));
    assert!(text.contains("合约评分"));
    assert!(text.contains("现货合约确认"));
    assert!(text.contains("高概率主力建多，不是单纯清算推动。"));
    assert!(!text.contains("现货主动买入跟随"));
}

#[test]
fn market_structure_discord_payload_uses_extreme_impact_wording() {
    let mut request = high_request();
    request.alert_family = Some("market_structure".to_string());
    request.level = Some("Extreme".to_string());
    request.main_force_score = Some(54);
    request.market_structure_confidence = Some(52.0);
    request.market_structure_data_quality = Some(76.0);
    request.extreme_impact_score = Some(91);
    request.structure_bias = Some(-68);
    request.market_structure_severity = Some("Extreme".to_string());
    request.regime_type = Some("long_liquidation_cascade".to_string());
    request.spot_score = Some(42);
    request.contract_score = Some(89);
    request.cross_confirm_score = Some(48);
    request.main_force_confirmed = Some(false);
    request.oi_score = Some(52);
    request.liquidation_score = Some(91);

    let payload = discord_payload_for_tests(&request);
    let text = serde_json::to_string(&payload).expect("payload json");

    assert!(text.contains("极端行情冲击"));
    assert!(text.contains("多头清算瀑布"));
    assert!(text.contains("极端冲击"));
    assert!(text.contains("暂不确认是主力建空"));
    assert!(!text.contains("多头清算显著增加"));
}

fn high_request() -> DiscordNotificationRequest {
    DiscordNotificationRequest {
        server_evidence_verified: true,
        alert_family: Some("short_toxic_order".to_string()),
        signal_id: Some("manual-high-direction-color".to_string()),
        id: None,
        dedupe_key: Some("manual-high-direction-color".to_string()),
        exchange: Some("manual-smoke".to_string()),
        symbol: Some("BTC-PERP".to_string()),
        signal_type: Some("spoofing_candidate".to_string()),
        level: Some("critical".to_string()),
        side: Some("Ask/Sell".to_string()),
        score: Some(92),
        confidence: Some(88.0),
        data_quality: Some(88.0),
        reason: Some("Manual smoke test High/Critical candidate Discord path".to_string()),
        impact: None,
        impact_level: None,
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
        main_force_score: None,
        extreme_impact_score: None,
        structure_bias: None,
        market_structure_confidence: None,
        market_structure_data_quality: None,
        market_structure_severity: None,
        behavior_type: None,
        behavior_state: None,
        behavior_confidence: None,
        behavior_main_force_confirmed: None,
        regime_type: None,
        spot_score: None,
        contract_score: None,
        cross_confirm_score: None,
        main_force_confirmed: None,
        signal_agreement: None,
        source_coverage: None,
        oi_score: None,
        liquidation_score: None,
        test: None,
    }
}

fn set_alert_env() {
    reset_discord_push_limits_for_tests();
    reset_discord_auto_push_for_tests();
    std::env::set_var("SHORT_TOXIC_ALERT_MIN_SCORE", "85");
    std::env::set_var("SHORT_TOXIC_ALERT_MIN_CONFIDENCE", "70");
    std::env::set_var("SHORT_TOXIC_ALERT_MIN_DATA_QUALITY", "70");
    std::env::set_var("SHORT_TOXIC_DISCORD_AUTO_PUSH_ENABLED", "true");
    std::env::set_var("SHORT_TOXIC_DISCORD_COOLDOWN_SECONDS", "60");
    std::env::set_var("DRY_RUN", "false");
    std::env::set_var(
        "SHORT_TOXIC_DISCORD_WEBHOOK_URL",
        "https://discord.com/api/webhooks/test-id/test-token",
    );
}

fn set_market_structure_env() {
    reset_discord_push_limits_for_tests();
    reset_discord_auto_push_for_tests();
    std::env::set_var("MARKET_STRUCTURE_ALERT_MIN_SCORE", "80");
    std::env::set_var("MARKET_STRUCTURE_EXTREME_MIN_SCORE", "85");
    std::env::set_var("MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE", "70");
    std::env::set_var("MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY", "70");
    std::env::set_var("MARKET_STRUCTURE_DISCORD_AUTO_PUSH_ENABLED", "true");
    std::env::set_var("MARKET_STRUCTURE_DISCORD_COOLDOWN_SECONDS", "900");
    std::env::set_var("DRY_RUN", "false");
    std::env::set_var(
        "MARKET_STRUCTURE_DISCORD_WEBHOOK_URL",
        "https://discord.com/api/webhooks/test-id/test-token",
    );
}

fn clear_alert_env() {
    reset_discord_push_limits_for_tests();
    reset_discord_auto_push_for_tests();
    for key in [
        "SHORT_TOXIC_ALERT_MIN_SCORE",
        "SHORT_TOXIC_ALERT_MIN_CONFIDENCE",
        "SHORT_TOXIC_ALERT_MIN_DATA_QUALITY",
        "SHORT_TOXIC_DISCORD_AUTO_PUSH_ENABLED",
        "SHORT_TOXIC_DISCORD_COOLDOWN_SECONDS",
        "SHORT_TOXIC_DISCORD_WEBHOOK_URL",
        "MARKET_STRUCTURE_ALERT_MIN_SCORE",
        "MARKET_STRUCTURE_EXTREME_MIN_SCORE",
        "MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE",
        "MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY",
        "MARKET_STRUCTURE_DISCORD_AUTO_PUSH_ENABLED",
        "MARKET_STRUCTURE_DISCORD_COOLDOWN_SECONDS",
        "MARKET_STRUCTURE_DISCORD_WEBHOOK_URL",
        "ALERT_MIN_SCORE",
        "ALERT_MIN_CONFIDENCE",
        "ALERT_MIN_DATA_QUALITY",
        "DISCORD_AUTO_PUSH_ENABLED",
        "DRY_RUN",
        "READ_ONLY",
        "DISCORD_WEBHOOK_URL",
    ] {
        std::env::remove_var(key);
    }
}
