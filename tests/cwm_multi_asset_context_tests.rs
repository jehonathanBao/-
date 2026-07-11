use btc_toxic_flow_monitor_rs::contract_whale_monitor::{
    aggregator::{liquidation_context_for_window, market_context_from_snapshots},
    context::{contract_context_v2_from_snapshots, DataSource},
    normalizer::{
        normalize_binance_funding_rate_json_for_symbol,
        normalize_binance_open_interest_json_for_symbol, normalize_okx_funding_rate_json_for_inst,
        normalize_okx_open_interest_json_for_inst,
    },
    types::{ContractExchange, ContractLiquidationBucket, ContractOiSnapshot},
};

#[test]
fn eth_context_normalizers_reject_btc_payloads_instead_of_proxying() {
    let now = 1_700_000_300_000;

    assert!(normalize_binance_open_interest_json_for_symbol(
        "ETH",
        &serde_json::json!({
            "symbol": "BTCUSDT",
            "openInterest": "50000",
            "time": now
        }),
        Some(63_000.0),
        now,
    )
    .is_none());
    assert!(normalize_binance_funding_rate_json_for_symbol(
        "ETH",
        &serde_json::json!({
            "symbol": "BTCUSDT",
            "lastFundingRate": "0.00020",
            "time": now
        }),
        now,
    )
    .is_none());
    assert!(normalize_okx_open_interest_json_for_inst(
        "ETH-USDT-SWAP",
        &serde_json::json!({
            "data": [{
                "instId": "BTC-USDT-SWAP",
                "oi": "1000000",
                "ts": now.to_string()
            }]
        }),
        0.01,
    )
    .is_none());
    assert!(normalize_okx_funding_rate_json_for_inst(
        "ETH-USDT-SWAP",
        &serde_json::json!({
            "data": [{
                "instId": "BTC-USDT-SWAP",
                "fundingRate": "0.00010",
                "ts": now.to_string()
            }]
        }),
    )
    .is_none());
}

#[test]
fn eth_market_context_uses_eth_oi_and_funding_only() {
    let now = 1_700_000_300_000;
    let oi_snapshots = vec![
        normalize_binance_open_interest_json_for_symbol(
            "BTC",
            &serde_json::json!({
                "symbol": "BTCUSDT",
                "openInterest": "60000",
                "time": now
            }),
            Some(63_000.0),
            now,
        )
        .unwrap(),
        normalize_binance_open_interest_json_for_symbol(
            "ETH",
            &serde_json::json!({
                "symbol": "ETHUSDT",
                "openInterest": "1000",
                "time": now - 300_000
            }),
            Some(1_700.0),
            now - 300_000,
        )
        .unwrap(),
        normalize_binance_open_interest_json_for_symbol(
            "ETH",
            &serde_json::json!({
                "symbol": "ETHUSDT",
                "openInterest": "1100",
                "time": now
            }),
            Some(1_700.0),
            now,
        )
        .unwrap(),
    ];
    let funding_snapshots = vec![
        normalize_binance_funding_rate_json_for_symbol(
            "BTC",
            &serde_json::json!({
                "symbol": "BTCUSDT",
                "lastFundingRate": "-0.00100",
                "time": now
            }),
            now,
        )
        .unwrap(),
        normalize_binance_funding_rate_json_for_symbol(
            "ETH",
            &serde_json::json!({
                "symbol": "ETHUSDT",
                "lastFundingRate": "0.00030",
                "time": now
            }),
            now,
        )
        .unwrap(),
    ];

    let context = market_context_from_snapshots(&oi_snapshots, &funding_snapshots, "ETH", now);
    let context_v2 = contract_context_v2_from_snapshots(
        "ETH",
        now,
        360_000,
        &oi_snapshots,
        &funding_snapshots,
        &[],
    );

    assert!(context.oi_available);
    assert!(context.funding_available);
    assert_eq!(context.oi_change_5m_btc, Some(100.0));
    assert_eq!(context.funding_rate, Some(0.00030));
    assert_eq!(context.oi_bias.as_deref(), Some("rising"));
    assert_eq!(context_v2.symbol, "ETH");
    assert_eq!(context_v2.open_interest, Some(1_100.0));
    assert_eq!(context_v2.funding_rate, Some(0.00030));
    assert_eq!(context_v2.oi_source, Some(DataSource::Binance));
}

#[test]
fn alt_missing_context_degrades_without_btc_fallback() {
    let now = 1_700_000_300_000;
    let oi_snapshots = vec![normalize_binance_open_interest_json_for_symbol(
        "BTC",
        &serde_json::json!({
            "symbol": "BTCUSDT",
            "openInterest": "60000",
            "time": now
        }),
        Some(63_000.0),
        now,
    )
    .unwrap()];
    let funding_snapshots = vec![normalize_binance_funding_rate_json_for_symbol(
        "BTC",
        &serde_json::json!({
            "symbol": "BTCUSDT",
            "lastFundingRate": "0.00020",
            "time": now
        }),
        now,
    )
    .unwrap()];

    let context = market_context_from_snapshots(&oi_snapshots, &funding_snapshots, "ARB", now);
    let context_v2 = contract_context_v2_from_snapshots(
        "ARB",
        now,
        360_000,
        &oi_snapshots,
        &funding_snapshots,
        &[],
    );

    assert!(!context.oi_available);
    assert!(!context.funding_available);
    assert_eq!(context.oi_bias.as_deref(), Some("unknown"));
    assert_eq!(context.funding_bias.as_deref(), Some("unknown"));
    assert_eq!(context_v2.symbol, "ARB");
    assert_eq!(context_v2.open_interest, None);
    assert_eq!(context_v2.funding_rate, None);
    assert!(context_v2.oi_stale);
    assert!(context_v2.funding_stale);
}

#[test]
fn market_context_marks_okx_ctval_fallback_as_degraded_evidence() {
    let now = 1_700_000_300_000;
    let snapshots = vec![ContractOiSnapshot {
        ts: now,
        exchange: ContractExchange::Okx,
        symbol: "ETH".to_string(),
        oi_btc: 100.0,
        oi_notional_usd: Some(300_000.0),
        ct_val_available: false,
        evidence_degraded_reason: Some("okx_metadata_fallback".to_string()),
    }];

    let context = market_context_from_snapshots(&snapshots, &[], "ETH", now);

    assert!(!context.ct_val_available);
    assert!(context.evidence_degraded);
    assert_eq!(
        context.evidence_reason.as_deref(),
        Some("okx_metadata_fallback")
    );
}

#[test]
fn liquidation_context_is_symbol_isolated() {
    let now = 1_700_000_300_000;
    let buckets = vec![
        ContractLiquidationBucket {
            ts_bucket: now - 1_000,
            exchange: "binance".to_string(),
            symbol: "BTC".to_string(),
            long_liq_btc: 10_000.0,
            short_liq_btc: 5_000.0,
            liq_notional_usd: 900_000_000.0,
            ..Default::default()
        },
        ContractLiquidationBucket {
            ts_bucket: now - 1_000,
            exchange: "binance".to_string(),
            symbol: "ETH".to_string(),
            long_liq_btc: 12.0,
            short_liq_btc: 8.0,
            liq_notional_usd: 34_000.0,
            ..Default::default()
        },
    ];

    let context = liquidation_context_for_window(&buckets, "ETH", 15, now, 200.0);
    let context_v2 = contract_context_v2_from_snapshots("ETH", now, 15_000, &[], &[], &buckets);

    assert_eq!(context.total_liq_btc, 20.0);
    assert_eq!(context.long_liq_btc, 12.0);
    assert_eq!(context.short_liq_btc, 8.0);
    assert_eq!(context.liq_to_volume_ratio, Some(0.10));
    assert_eq!(context_v2.liquidation_volume, Some(20.0));
    assert_eq!(context_v2.liquidation_source, Some(DataSource::Binance));
}

#[test]
fn alt_symbols_can_build_independent_context_snapshots() {
    let now = 1_700_000_300_000;
    let sol_oi = normalize_binance_open_interest_json_for_symbol(
        "SOL",
        &serde_json::json!({
            "symbol": "SOLUSDT",
            "openInterest": "850000",
            "time": now
        }),
        Some(155.0),
        now,
    )
    .unwrap();
    let arb_oi = normalize_okx_open_interest_json_for_inst(
        "ARB-USDT-SWAP",
        &serde_json::json!({
            "data": [{
                "instId": "ARB-USDT-SWAP",
                "oi": "2400000",
                "ts": now.to_string()
            }]
        }),
        1.0,
    )
    .unwrap();
    let pepe_funding = normalize_binance_funding_rate_json_for_symbol(
        "PEPE",
        &serde_json::json!({
            "symbol": "PEPEUSDT",
            "lastFundingRate": "-0.00040",
            "time": now
        }),
        now,
    )
    .unwrap();

    assert_eq!(sol_oi.symbol, "SOL");
    assert_eq!(arb_oi.symbol, "ARB");
    assert_eq!(pepe_funding.symbol, "PEPE");

    let context = contract_context_v2_from_snapshots(
        "ARB",
        now,
        360_000,
        &[sol_oi, arb_oi],
        &[pepe_funding],
        &[],
    );

    assert_eq!(context.symbol, "ARB");
    assert_eq!(context.open_interest, Some(2_400_000.0));
    assert_eq!(context.funding_rate, None);
    assert!(!context.oi_stale);
    assert!(context.funding_stale);
}
