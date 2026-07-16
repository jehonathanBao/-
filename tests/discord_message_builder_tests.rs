use btc_toxic_flow_monitor_rs::{
    alerts::discord_message_builder::{
        build_discord_candidate_message, embed_color, DiscordEmbedField,
    },
    types::{
        toxic_flow::ToxicConfidence,
        toxic_signal::{
            SignalEvidence, ToxicChaseRisk, ToxicSignal, ToxicSignalDirection, ToxicSignalType,
        },
    },
};

#[test]
fn spoofing_candidate_ask_side_generates_sell_wall_copy() {
    let payload = build_discord_candidate_message(&signal(
        ToxicSignalType::SpoofingCandidate,
        ToxicSignalDirection::ShortBias,
    ));
    let core = field(&payload.embeds[0].fields, "核心原因");

    assert!(core.contains("疑似大额卖墙压盘"));
    assert!(core.contains("Dashboard"));
}

#[test]
fn spoofing_candidate_bid_side_generates_buy_wall_copy() {
    let payload = build_discord_candidate_message(&signal(
        ToxicSignalType::SpoofingCandidate,
        ToxicSignalDirection::LongBias,
    ));

    assert!(field(&payload.embeds[0].fields, "核心原因").contains("疑似大额买墙托盘"));
}

#[test]
fn layering_candidate_generates_layering_copy() {
    let payload = build_discord_candidate_message(&signal(
        ToxicSignalType::LayeringCandidate,
        ToxicSignalDirection::ShortBias,
    ));

    assert!(field(&payload.embeds[0].fields, "核心原因").contains("疑似分层挂单"));
}

#[test]
fn iceberg_candidate_generates_iceberg_copy() {
    let payload = build_discord_candidate_message(&signal(
        ToxicSignalType::IcebergCandidate,
        ToxicSignalDirection::LongBias,
    ));

    assert!(field(&payload.embeds[0].fields, "核心原因").contains("疑似冰山单"));
}

#[test]
fn missing_evidence_keeps_safe_fields_without_panicking() {
    let mut signal = signal(
        ToxicSignalType::SpoofingCandidate,
        ToxicSignalDirection::Neutral,
    );
    signal.evidence = None;
    signal.data_quality = None;

    let payload = build_discord_candidate_message(&signal);
    let names: Vec<&str> = payload.embeds[0]
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();

    assert!(!names.contains(&"盘口证据"));
    assert!(!names.contains(&"撤后 Markout"));
    assert!(field(&payload.embeds[0].fields, "数据质量").contains("N/A"));
}

#[test]
fn message_omits_markout_raw_evidence_and_qty_dumps() {
    let payload = build_discord_candidate_message(&signal(
        ToxicSignalType::SpoofingCandidate,
        ToxicSignalDirection::ShortBias,
    ));
    let text = serde_json::to_string(&payload).expect("payload json");
    let lower = text.to_ascii_lowercase();

    assert!(!text.contains("盘口证据"));
    assert!(!text.contains("撤后 Markout"));
    assert!(!lower.contains("markout"));
    assert!(!text.contains("1,000 BTC"));
    assert!(!text.contains("980 BTC"));
    assert!(!text.contains("raw_evidence"));
    assert!(!lower.contains("confirmed manipulation"));
    assert!(text.contains("Candidate"));
}

#[test]
fn message_keeps_candidate_semantics_and_avoids_confirmed_wording() {
    let payload = build_discord_candidate_message(&signal(
        ToxicSignalType::SpoofingCandidate,
        ToxicSignalDirection::ShortBias,
    ));
    let text = serde_json::to_string(&payload).expect("payload json");
    let lower = text.to_ascii_lowercase();

    assert!(!lower.contains("confirmed manipulation"));
    assert!(!lower.contains("confirmed spoofing"));
    assert!(!text.contains("确认操纵"));
    assert!(!text.contains("已确认"));
    assert!(text.contains("Candidate"));
}

#[test]
fn high_score_uses_high_risk_color() {
    assert_eq!(embed_color(Some(91.0)), 0x8B0000);
    assert_eq!(embed_color(Some(85.0)), 0xFF0000);
}

#[test]
fn long_reason_is_truncated() {
    let mut signal = signal(ToxicSignalType::TrapRisk, ToxicSignalDirection::Neutral);
    signal.primary_reason = "x".repeat(1_200);

    let payload = build_discord_candidate_message(&signal);
    let core = field(&payload.embeds[0].fields, "核心原因");

    assert!(core.chars().count() <= 900);
    assert!(core.ends_with('…'));
}

fn field<'a>(fields: &'a [DiscordEmbedField], name: &str) -> &'a str {
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
        .expect("field")
}

fn signal(signal_type: ToxicSignalType, direction: ToxicSignalDirection) -> ToxicSignal {
    ToxicSignal {
        signal_id: "sig_001".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 1_700_000_000_000,
        signal_type,
        direction,
        toxicity_score: 91,
        confidence: ToxicConfidence::High,
        primary_reason: "candidate evidence".to_string(),
        reason: vec!["possible short-term liquidity distortion".to_string()],
        supporting_evidence: Vec::new(),
        invalidation_price: None,
        suggested_stop_distance_usd: None,
        chase_risk: ToxicChaseRisk::Medium,
        no_trade_reasons: vec!["candidate_only_not_confirmed".to_string()],
        linked_active_trade_signal_ids: Vec::new(),
        linked_liquidation_signal_ids: Vec::new(),
        linked_wall_lifecycle_signal_ids: Vec::new(),
        linked_wall_interpretation_signal_ids: Vec::new(),
        linked_structural_signal_ids: Vec::new(),
        read_only: true,
        detector_version: Some("test-detector".to_string()),
        score_breakdown: None,
        evidence: Some(SignalEvidence {
            venue: "binance".to_string(),
            symbol: "BTC-PERP".to_string(),
            window_ms: 5_000,
            observed_start_ms: 1_000,
            observed_end_ms: 3_000,
            add_qty: 1_000.0,
            cancel_qty: 980.0,
            fill_qty: 20.0,
            cancel_to_trade_ratio: Some(49.0),
            depth_before: Some(1_000.0),
            depth_after: Some(20.0),
            depth_impact: Some(0.98),
            price_impact_bps: Some(12.34),
            markout_1s_bps: Some(2.5),
            markout_5s_bps: Some(8.75),
            markout_30s_bps: None,
            raw_evidence_links: vec!["l2:binance:BTC-PERP".to_string()],
        }),
        data_quality: Some(92.0),
        dedupe_key: Some("spoofing:BTC-PERP:ask".to_string()),
        resolution_status: Some("candidate".to_string()),
    }
}
