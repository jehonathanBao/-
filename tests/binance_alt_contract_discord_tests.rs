use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::{
        BinanceAltContractRuntimeConfig, BinanceAltDataQualityConfig, BinanceAltDiscordConfig,
    },
    detector::{
        detect_alt_contract_signal_with_context, window_confirmation_for, MarketImpulseContext,
    },
    discord::{
        build_alt_contract_discord_payload, evaluate_alt_contract_discord_gate_with_store,
        AltContractDiscordCooldownStore,
    },
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractImpactScore, AltContractSeverity, AltContractSignal, AltContractSignalType,
        AltContractSymbolTier, AltContractWindowStats,
    },
};

fn config() -> BinanceAltContractRuntimeConfig {
    BinanceAltContractRuntimeConfig {
        enabled: true,
        dry_run: true,
        discord: BinanceAltDiscordConfig {
            dry_run: true,
            ..BinanceAltDiscordConfig::default()
        },
        data_quality: BinanceAltDataQualityConfig {
            warmup_ms: 1,
            ..BinanceAltDataQualityConfig::default()
        },
        ..BinanceAltContractRuntimeConfig::default()
    }
}

#[test]
fn low_notional_is_not_display_or_discord_eligible() {
    let config = config();
    let mut signal = main_force_signal(AltContractSymbolTier::C);
    signal.total_notional_usd = 499_999.0;
    signal.alt_impact_score = impact_score(42.0);

    let gate = gate(&signal, &config, 1);

    assert!(!gate.would_send);
    assert_eq!(gate.reason, "impact_score_low");
    assert_eq!(gate.alert_kind, "none");
}

#[test]
fn tier_c_main_force_build_can_would_send_at_800k() {
    let config = config();
    let mut signal = main_force_signal(AltContractSymbolTier::C);
    signal.total_notional_usd = 800_000.0;

    let gate = gate(&signal, &config, 2);

    assert!(gate.eligible);
    assert!(gate.would_send);
    assert!(!gate.sent);
    assert_eq!(gate.reason, "dry_run_would_send");
    assert_eq!(gate.alert_kind, "main_force_build");
}

#[test]
fn tier_a_800k_low_relative_impact_is_below_discord_threshold() {
    let config = config();
    let mut signal = main_force_signal(AltContractSymbolTier::A);
    signal.total_notional_usd = 800_000.0;
    signal.alt_impact_score = impact_score(50.0);

    let gate = gate(&signal, &config, 3);

    assert!(!gate.would_send);
    assert_eq!(gate.reason, "impact_score_low");
    assert_eq!(gate.alert_kind, "none");
}

#[test]
fn relative_impact_a_can_push_without_main_force_confirmation() {
    let config = config();
    let mut signal = main_force_signal(AltContractSymbolTier::A);
    signal.total_notional_usd = 800_000.0;
    signal.build_score = 45;
    signal.main_force_confidence = 40.0;
    signal.evidence_count = 1;
    signal.alt_impact_score = impact_score(85.0);

    let gate = gate(&signal, &config, 33);

    assert!(gate.eligible);
    assert!(gate.would_send);
    assert_eq!(gate.reason, "dry_run_would_send");
    assert_eq!(gate.alert_kind, "relative_impact");
}

#[test]
fn tier_d_abnormal_without_build_confirmation_does_not_push() {
    let config = config();
    let mut signal = extreme_signal(AltContractSymbolTier::B);
    signal.tier = AltContractSymbolTier::D;
    signal.total_notional_usd = 600_000.0;
    signal.build_score = 45;
    signal.alt_impact_score = impact_score(55.0);

    let gate = gate(&signal, &config, 4);

    assert!(!gate.would_send);
    assert_eq!(gate.reason, "impact_score_low");
    assert_eq!(gate.alert_kind, "none");
}

#[test]
fn tier_e_is_display_only_by_default() {
    let config = config();
    let mut signal = main_force_signal(AltContractSymbolTier::E);
    signal.total_notional_usd = 100_000_000.0;

    let gate = gate(&signal, &config, 5);

    assert!(!gate.would_send);
    assert_eq!(gate.reason, "tier_guard");
}

#[test]
fn liquidation_driven_signal_uses_liquidation_alert_kind() {
    let config = config();
    let mut signal = extreme_signal(AltContractSymbolTier::C);
    signal.signal_type = AltContractSignalType::LiquidationCascade;
    signal.liquidation_suspected = true;
    signal.force_order_snapshot = true;
    signal.abnormal_score = 95;
    signal.total_notional_usd = 2_000_000.0;
    signal.oi_change_pct = Some(-1.8);
    signal.oi_change_1m_pct = Some(-1.8);
    signal.price_move_pct = Some(-1.1);

    let gate = gate(&signal, &config, 6);

    assert!(gate.would_send);
    assert_eq!(gate.alert_kind, "liquidation_shock");
}

#[test]
fn abnormal_impulse_payload_does_not_claim_main_force_build() {
    let config = config();
    let mut signal = extreme_signal(AltContractSymbolTier::B);
    signal.total_notional_usd = 5_000_000.0;

    let gate = gate(&signal, &config, 7);
    signal.discord_alert_kind = gate.alert_kind;
    signal.discord_reason = gate.reason;
    let payload = build_alt_contract_discord_payload(&signal).to_string();

    assert!(gate.would_send);
    assert_eq!(signal.discord_alert_kind, "extreme_impulse");
    assert!(payload.contains("极端异常冲击"));
    assert!(!payload.contains("疑似主力建多"));
    assert!(!payload.contains("疑似主力建空"));
}

#[test]
fn main_force_payload_uses_semantic_copy_instead_of_trade_hint() {
    let config = config();
    let signal = main_force_signal(AltContractSymbolTier::C);

    let gate = gate(&signal, &config, 70);
    let mut exposed = signal.clone();
    exposed.discord_alert_kind = gate.alert_kind;
    exposed.discord_reason = gate.reason;
    let payload = build_alt_contract_discord_payload(&exposed).to_string();

    assert!(gate.would_send);
    assert!(payload.contains("累积压力观察"));
    assert!(!payload.contains("疑似主力建多"));
    assert!(!payload.contains("自动交易"));
}

#[test]
fn cooldown_blocks_duplicate_signal() {
    let config = config();
    let signal = main_force_signal(AltContractSymbolTier::C);
    let store = AltContractDiscordCooldownStore::new();

    let first =
        evaluate_alt_contract_discord_gate_with_store(&signal, &config.discord, false, &store, 10);
    let second =
        evaluate_alt_contract_discord_gate_with_store(&signal, &config.discord, false, &store, 20);

    assert!(first.would_send);
    assert!(!second.would_send);
    assert_eq!(second.reason, "duplicate");
}

#[test]
fn severity_upgrade_breaks_cooldown() {
    let config = config();
    let mut critical = main_force_signal(AltContractSymbolTier::C);
    critical.severity = AltContractSeverity::Critical;
    critical.id = "critical-signal".to_string();
    let mut upgraded = critical.clone();
    upgraded.id = "upgraded-signal".to_string();
    upgraded.severity = AltContractSeverity::S;
    let store = AltContractDiscordCooldownStore::new();

    let first = evaluate_alt_contract_discord_gate_with_store(
        &critical,
        &config.discord,
        false,
        &store,
        30,
    );
    let second = evaluate_alt_contract_discord_gate_with_store(
        &upgraded,
        &config.discord,
        false,
        &store,
        40,
    );

    assert!(first.would_send);
    assert!(second.would_send);
}

#[test]
fn dry_run_never_marks_sent() {
    let config = config();
    let signal = main_force_signal(AltContractSymbolTier::C);

    let gate = gate(&signal, &config, 8);

    assert!(gate.dry_run);
    assert!(gate.would_send);
    assert!(!gate.sent);
}

fn gate(
    signal: &AltContractSignal,
    config: &BinanceAltContractRuntimeConfig,
    seed: i64,
) -> btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::discord::AltContractDiscordGate {
    evaluate_alt_contract_discord_gate_with_store(
        signal,
        &config.discord,
        false,
        &AltContractDiscordCooldownStore::new(),
        1_700_000_000_000 + seed,
    )
}

fn main_force_signal(tier: AltContractSymbolTier) -> AltContractSignal {
    let config = config();
    let stats = stats(
        "SOL",
        AltContractDirection::Buy,
        70_000_000.0,
        0.82,
        1.2,
        7.0,
        tier,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(120_000.0),
        oi_change_pct: Some(1.7),
        oi_updated_at: Some(stats.ts - 10_000),
        funding_rate: Some(0.0),
        persistence_windows: 3,
        ticker_quote_volume_24h_usd: Some(1_500_000_000.0),
        ticker_updated_at: Some(stats.ts - 10_000),
        ..AltContractContext::default()
    };
    let mut signal = detect_alt_contract_signal_with_context(
        &stats,
        &context,
        &config,
        vec![window_confirmation_for(&stats, &config)],
        MarketImpulseContext::default(),
    )
    .expect("main force signal");
    signal.signal_type = AltContractSignalType::MainForceLongBuild;
    signal.direction = AltContractDirection::Buy;
    signal.severity = AltContractSeverity::Critical;
    signal.abnormal_score = 88;
    signal.build_score = 85;
    signal.main_force_confidence = 82.0;
    signal.evidence_count = 4;
    signal.dominance = 0.72;
    signal.oi_quality = "fresh".to_string();
    signal.oi_change_pct = Some(1.7);
    signal.oi_change_1m_pct = Some(1.7);
    signal.oi_change_1m_base = Some(120_000.0);
    signal.funding_crowding = "neutral".to_string();
    signal.liquidation_suspected = false;
    signal.force_order_snapshot = false;
    signal
}

fn extreme_signal(tier: AltContractSymbolTier) -> AltContractSignal {
    let config = config();
    let stats = stats(
        "ALT",
        AltContractDirection::Buy,
        12_000_000.0,
        0.82,
        1.4,
        9.0,
        tier,
    );
    let context = AltContractContext {
        oi_updated_at: None,
        funding_rate: Some(0.0),
        persistence_windows: 1,
        ticker_quote_volume_24h_usd: Some(250_000_000.0),
        ticker_updated_at: Some(stats.ts - 10_000),
        ..AltContractContext::default()
    };
    let mut signal = detect_alt_contract_signal_with_context(
        &stats,
        &context,
        &config,
        vec![window_confirmation_for(&stats, &config)],
        MarketImpulseContext::default(),
    )
    .expect("extreme signal");
    signal.signal_type = AltContractSignalType::AbnormalPump;
    signal.abnormal_score = 95;
    signal.build_score = 45;
    signal.evidence_count = 2;
    signal.main_force_confidence = 40.0;
    signal.dynamic_multiple = Some(9.0);
    signal.dominance = 0.82;
    signal.price_move_pct = Some(1.4);
    signal.liquidation_suspected = false;
    signal
}

fn impact_score(final_score: f64) -> AltContractImpactScore {
    AltContractImpactScore {
        market_impact_ratio: if final_score >= 70.0 { 0.03 } else { 0.0008 },
        market_impact_score: if final_score >= 70.0 { 40.0 } else { 4.0 },
        liquidity_impact: if final_score >= 70.0 { 24.0 } else { 6.0 },
        cap_impact: 0.0,
        directional_strength: 0.74,
        directional_score: if final_score >= 70.0 { 20.0 } else { 10.0 },
        oi_confirmation: if final_score >= 70.0 { 10.0 } else { 0.0 },
        final_score,
        display_threshold: 70.0,
        discord_threshold: 85.0,
        s_threshold: 90.0,
        reference_volume_24h_usd: Some(1_500_000_000.0),
        reference_age_sec: Some(0),
        evidence_degraded: false,
        reference_source: "ticker_quote_volume_24h".to_string(),
        interpretation: if final_score >= 70.0 {
            "有效相对冲击".to_string()
        } else {
            "相对市场冲击偏弱".to_string()
        },
    }
}

fn stats(
    symbol: &str,
    direction: AltContractDirection,
    notional: f64,
    dominance: f64,
    price_move_pct: f64,
    dynamic_multiple: f64,
    tier: AltContractSymbolTier,
) -> AltContractWindowStats {
    let signed_net = if direction == AltContractDirection::Buy {
        10_000.0 * dominance
    } else {
        -10_000.0 * dominance
    };
    AltContractWindowStats {
        symbol: symbol.to_string(),
        product_id: format!("{symbol}USDT"),
        tier,
        window_sec: 60,
        ts: 1_700_000_000_000,
        buy_volume_base: if signed_net > 0.0 { 9_000.0 } else { 1_000.0 },
        sell_volume_base: if signed_net < 0.0 { 9_000.0 } else { 1_000.0 },
        total_volume_base: 10_000.0,
        net_volume_base: signed_net,
        total_notional_usd: notional,
        dominance,
        direction,
        trigger_price_usd: Some(notional / 10_000.0),
        price_move_pct: Some(price_move_pct),
        price_threshold_pct: None,
        exchange_count: 1,
        main_exchange: Some("binance".to_string()),
        exchanges: vec![AltContractExchangeContribution {
            exchange: "binance".to_string(),
            total_volume_base: 10_000.0,
            total_notional_usd: notional,
            net_volume_base: signed_net,
            dominance,
            trade_count: 100,
            ..AltContractExchangeContribution::default()
        }],
        dynamic_multiple: Some(dynamic_multiple),
        data_quality: 92,
        startup_age_ms: Some(120_000),
    }
}
