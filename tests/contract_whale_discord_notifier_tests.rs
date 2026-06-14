use std::time::{SystemTime, UNIX_EPOCH};

use btc_toxic_flow_monitor_rs::{
    contract_whale_monitor::{
        aggregator::{aggregate_1s_buckets, rolling_window_stats},
        detector::detect_contract_whale_signal,
        discord_notifier::{
            build_contract_whale_discord_log_preview, build_contract_whale_discord_payload,
            evaluate_contract_whale_discord_gate, notify_contract_whale_discord_with_cooldown,
            validate_discord_webhook_url, ContractWhaleDiscordCooldownStore,
            ContractWhaleDiscordSettings,
        },
        normalizer::{normalize_binance_agg_trade, normalize_bitfinex_trade},
        types::{ContractWhaleDirection, ContractWhaleSeverity, ContractWhaleSignalType},
    },
    storage::{contract_whale_repo::ContractWhaleRepo, SqliteStore},
};

#[tokio::test]
async fn cwm_discord_dry_run_generates_payload_without_real_send() {
    let store = temp_store("cwm-discord-dry-run");
    let signal = sample_s_signal();
    store.upsert_contract_whale_signal(&signal).unwrap();

    let cooldown = ContractWhaleDiscordCooldownStore::new();
    let outcome = notify_contract_whale_discord_with_cooldown(
        &ContractWhaleDiscordSettings::dry_run_for_tests(),
        &signal,
        Some(store.clone()),
        &cooldown,
    )
    .await;

    assert!(outcome.eligible);
    assert!(!outcome.sent);
    assert!(outcome.dry_run);
    assert_eq!(outcome.reason, "dry_run");

    let stored = store
        .list_contract_whale_signals("BTC", Some(ContractWhaleSeverity::S), 10)
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert!(stored[0].discord_eligible);
    assert!(!stored[0].discord_sent);
}

#[tokio::test]
async fn cwm_discord_rejects_high_single_exchange_by_default() {
    let signal = sample_single_exchange_high_signal();
    let cooldown = ContractWhaleDiscordCooldownStore::new();
    let outcome = notify_contract_whale_discord_with_cooldown(
        &ContractWhaleDiscordSettings::dry_run_for_tests(),
        &signal,
        None,
        &cooldown,
    )
    .await;

    assert!(!outcome.eligible);
    assert!(!outcome.sent);
    assert_eq!(outcome.reason, "data_quality_low");
}

#[test]
fn cwm_discord_cooldown_blocks_same_direction_without_upgrade() {
    let settings = live_settings_for_tests();
    let cooldown = ContractWhaleDiscordCooldownStore::new();
    let signal = sample_s_signal();
    let now = signal.ts;

    let first = evaluate_contract_whale_discord_gate(&settings, &signal, &cooldown, now);
    assert!(first.allowed);
    assert_eq!(first.reason, "eligible");

    cooldown.record_sent(&signal, now);
    let mut repeated = signal.clone();
    repeated.id = "contract-whale:BTC:15:1700000060000:buy-repeat".to_string();
    repeated.ts = now + 60_000;
    let second =
        evaluate_contract_whale_discord_gate(&settings, &repeated, &cooldown, now + 60_000);

    assert!(!second.allowed);
    assert_eq!(second.reason, "cooldown");
}

#[test]
fn cwm_discord_allows_severity_upgrade_and_direction_reversal() {
    let settings = live_settings_for_tests();
    let cooldown = ContractWhaleDiscordCooldownStore::new();
    let now = 1_700_000_015_000;
    let mut critical = sample_s_signal();
    critical.id = "contract-whale:BTC:15:1700000015000:buy-critical".to_string();
    critical.severity = ContractWhaleSeverity::Critical;
    critical.score = 88;
    cooldown.record_sent(&critical, now);

    let mut upgraded = sample_s_signal();
    upgraded.id = "contract-whale:BTC:15:1700000020000:buy-s".to_string();
    let upgrade =
        evaluate_contract_whale_discord_gate(&settings, &upgraded, &cooldown, now + 60_000);
    assert!(upgrade.allowed);
    assert_eq!(upgrade.reason, "eligible");

    let mut reversed = upgraded.clone();
    reversed.id = "contract-whale:BTC:15:1700000020000:sell-s".to_string();
    reversed.direction = ContractWhaleDirection::Sell;
    reversed.signal_type = ContractWhaleSignalType::AggressiveSell;
    let reversal =
        evaluate_contract_whale_discord_gate(&settings, &reversed, &cooldown, now + 90_000);
    assert!(reversal.allowed);
    assert_eq!(reversal.reason, "eligible");
}

#[test]
fn cwm_discord_gate_reports_duplicate_low_score_and_data_quality_reasons() {
    let settings = live_settings_for_tests();
    let cooldown = ContractWhaleDiscordCooldownStore::new();
    let signal = sample_s_signal();
    cooldown.record_sent(&signal, signal.ts);

    let duplicate =
        evaluate_contract_whale_discord_gate(&settings, &signal, &cooldown, signal.ts + 10_000);
    assert!(!duplicate.allowed);
    assert_eq!(duplicate.reason, "duplicate");

    let mut low_score = sample_s_signal();
    low_score.id = "contract-whale-low-score".to_string();
    low_score.score = 69;
    let low_score_decision =
        evaluate_contract_whale_discord_gate(&settings, &low_score, &cooldown, low_score.ts);
    assert!(!low_score_decision.allowed);
    assert_eq!(low_score_decision.reason, "low_score");

    let mut low_quality = sample_s_signal();
    low_quality.id = "contract-whale-low-quality".to_string();
    low_quality.data_quality = 69;
    let low_quality_decision =
        evaluate_contract_whale_discord_gate(&settings, &low_quality, &cooldown, low_quality.ts);
    assert!(!low_quality_decision.allowed);
    assert_eq!(low_quality_decision.reason, "data_quality_low");
}

#[test]
fn cwm_discord_gate_allows_primary_source_high_override() {
    let settings = live_settings_for_tests();
    let cooldown = ContractWhaleDiscordCooldownStore::new();
    let mut signal = sample_single_exchange_high_signal();
    signal.id = "contract-whale-primary-high".to_string();
    signal.severity = ContractWhaleSeverity::High;
    signal.score = 54;
    signal.data_quality = 70;
    signal.discord_eligible = true;
    signal.discord_reason = "high_primary_source_extreme".to_string();

    let decision = evaluate_contract_whale_discord_gate(&settings, &signal, &cooldown, signal.ts);
    assert!(decision.allowed);
    assert_eq!(decision.reason, "eligible");
}

#[test]
fn cwm_discord_gate_allows_btc_high_without_multi_exchange_score_gate() {
    let settings = live_settings_for_tests();
    let cooldown = ContractWhaleDiscordCooldownStore::new();
    let mut signal = sample_single_exchange_high_signal();
    signal.id = "contract-whale-btc-high".to_string();
    signal.severity = ContractWhaleSeverity::High;
    signal.score = 54;
    signal.data_quality = 70;
    signal.discord_eligible = true;
    signal.discord_reason = "btc_high_gate".to_string();

    let decision = evaluate_contract_whale_discord_gate(&settings, &signal, &cooldown, signal.ts);
    assert!(decision.allowed);
    assert_eq!(decision.reason, "eligible");
}

#[test]
fn cwm_discord_payload_uses_safe_final_fields_only() {
    let mut signal = sample_s_signal();
    signal.oi_change_5m_btc = Some(900.0);
    signal.oi_change_pct = Some(1.2);
    signal.oi_bias = Some("rising".to_string());
    signal.funding_rate = Some(0.00018);
    signal.funding_bias = Some("long".to_string());
    let payload = build_contract_whale_discord_payload(&signal);
    let text = payload.to_string();

    assert!(text.contains("contract_whale_flow"));
    assert!(text.contains("Risk Score"));
    assert!(text.contains("Data Quality"));
    assert!(text.contains("OI Change"));
    assert!(text.contains("Funding"));
    assert!(text.contains("OI 上升"));
    assert!(text.contains("偏多"));
    assert!(!text.contains("rawPayload"));
    assert!(!text.contains("webhook"));
    assert!(!text.contains("token"));
    assert!(!text.contains("evidence"));
    assert!(!text.contains("markout"));
    assert!(!text.contains("discordEligible"));
    assert!(!text.contains("discordSent"));

    let mut eth_signal = signal.clone();
    eth_signal.symbol = "ETH".to_string();
    eth_signal.id = "contract-whale:ETH:15:1700000015000:buy".to_string();
    let eth_payload = build_contract_whale_discord_payload(&eth_signal).to_string();
    let eth_preview = build_contract_whale_discord_log_preview(&eth_signal);
    assert!(eth_payload.contains("ETH 主力合约异动"));
    assert!(!eth_payload.contains("BTC 主力合约异动"));
    assert!(eth_preview.contains("ETH CWM S级"));
}

#[test]
fn cwm_discord_payload_snapshots_cover_core_signal_language() {
    let cases = [
        (
            sample_s_signal(),
            ["S级", "主力拉盘 / Aggressive Buy", "主动买入", "15s"],
        ),
        (
            sample_signal_variant(
                ContractWhaleSeverity::Critical,
                ContractWhaleSignalType::AggressiveSell,
                ContractWhaleDirection::Sell,
                "多平台主动卖出爆发，疑似主力合约砸盘",
                -0.24,
            ),
            ["Critical", "主力砸盘 / Aggressive Sell", "主动卖出", "15s"],
        ),
        (
            sample_signal_variant(
                ContractWhaleSeverity::Critical,
                ContractWhaleSignalType::DownsideAbsorption,
                ContractWhaleDirection::Absorption,
                "主动卖出放大但价格未明显下跌，疑似下方承接吸收",
                -0.02,
            ),
            ["Critical", "空头打不动 / 下方吸收", "卖出被吸收", "15s"],
        ),
        (
            sample_signal_variant(
                ContractWhaleSeverity::High,
                ContractWhaleSignalType::UpsideSuppression,
                ContractWhaleDirection::Suppression,
                "主动买入放大但价格未明显上涨，疑似上方卖盘压制",
                0.02,
            ),
            ["High", "多头打不动 / 上方压制", "买入被压制", "15s"],
        ),
    ];

    for (signal, expected_text) in cases {
        let payload = build_contract_whale_discord_payload(&signal);
        let text = payload.to_string();

        assert!(text.contains("BTC"));
        assert!(text.contains("Symbol"));
        assert!(text.contains("Direction"));
        assert!(text.contains("Window"));
        assert!(text.contains("Total Volume"));
        assert!(text.contains("Notional"));
        assert!(text.contains("Dominance"));
        assert!(text.contains("Exchanges"));
        assert!(text.contains("Price Move"));
        assert!(text.contains("binance"));
        assert!(text.contains("bitfinex"));
        for expected in expected_text {
            assert!(text.contains(expected), "missing `{expected}` in {text}");
        }
        assert!(!text.contains("discordEligible"));
        assert!(!text.contains("discordSent"));
    }
}

#[tokio::test]
async fn cwm_discord_dry_run_and_cooldown_outcomes_have_clear_operator_copy() {
    let dry_run_signal = sample_s_signal();
    let dry_run_preview = build_contract_whale_discord_log_preview(&dry_run_signal);

    assert!(dry_run_preview.contains("BTC CWM S级"));
    assert!(dry_run_preview.contains("score="));
    assert!(dry_run_preview.contains("dataQuality="));
    assert!(!dry_run_preview.contains("webhook"));
    assert!(!dry_run_preview.contains("token"));

    let cooldown = ContractWhaleDiscordCooldownStore::new();
    let settings = live_settings_for_tests();
    cooldown.record_sent(&dry_run_signal, dry_run_signal.ts);
    let mut repeated = dry_run_signal.clone();
    repeated.id = "contract-whale:BTC:15:1700000020000:buy-repeat".to_string();
    repeated.ts = dry_run_signal.ts + 30_000;

    let decision =
        evaluate_contract_whale_discord_gate(&settings, &repeated, &cooldown, repeated.ts);
    assert!(!decision.allowed);
    assert_eq!(decision.reason, "cooldown");
}

#[test]
fn cwm_discord_webhook_validation_allows_only_discord_https_webhooks() {
    assert!(
        validate_discord_webhook_url("https://discord.com/api/webhooks/1234567890/abcdef").is_ok()
    );
    assert!(
        validate_discord_webhook_url("https://discordapp.com/api/webhooks/1234567890/abcdef")
            .is_ok()
    );
    assert!(
        validate_discord_webhook_url("http://discord.com/api/webhooks/1234567890/abcdef").is_err()
    );
    assert!(
        validate_discord_webhook_url("https://127.0.0.1/api/webhooks/1234567890/abcdef").is_err()
    );
    assert!(validate_discord_webhook_url("https://discord.com/channels/123").is_err());
}

fn sample_s_signal() -> btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleSignal
{
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_200.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 430.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 500.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(10.4), 94)
        .expect("window stats");
    detect_contract_whale_signal(&stats).expect("signal")
}

fn sample_single_exchange_high_signal(
) -> btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleSignal {
    let now = 1_700_000_015_000;
    let trades = vec![normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_600.0, false).unwrap()];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.12), Some(4.5), 80)
        .expect("window stats");
    detect_contract_whale_signal(&stats).expect("signal")
}

fn sample_signal_variant(
    severity: ContractWhaleSeverity,
    signal_type: ContractWhaleSignalType,
    direction: ContractWhaleDirection,
    final_result: &str,
    price_move_pct: f64,
) -> btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleSignal {
    let mut signal = sample_s_signal();
    signal.id = format!("contract-whale:BTC:15:{}:{:?}", signal.ts, signal_type);
    signal.severity = severity;
    signal.signal_type = signal_type;
    signal.direction = direction;
    signal.final_result = final_result.to_string();
    signal.price_move_pct = Some(price_move_pct);
    signal.score = match severity {
        ContractWhaleSeverity::High => 86,
        ContractWhaleSeverity::Critical => 88,
        ContractWhaleSeverity::S => 94,
        ContractWhaleSeverity::Medium | ContractWhaleSeverity::Calm => 70,
    };
    if matches!(
        direction,
        ContractWhaleDirection::Sell | ContractWhaleDirection::Absorption
    ) {
        signal.net_volume_btc = -signal.net_volume_btc.abs();
        signal.net_volume = signal.net_volume_btc;
    }
    signal
}

fn live_settings_for_tests() -> ContractWhaleDiscordSettings {
    let mut settings = ContractWhaleDiscordSettings::dry_run_for_tests();
    settings.dry_run = false;
    settings.webhook_url = Some("https://discord.com/api/webhooks/1234567890/abcdef".to_string());
    settings
}

fn temp_store(name: &str) -> SqliteStore {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "btc-toxic-flow-{name}-{unique}-{}.sqlite",
        std::process::id()
    ));
    let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
    store.migrate().unwrap();
    store
}
