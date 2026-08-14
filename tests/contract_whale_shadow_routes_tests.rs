use btc_toxic_flow_monitor_rs::{
    api::contract_whale_shadow_routes::build_shadow_candidates,
    contract_whale_monitor::{
        shadow::ShadowState,
        types::{
            ContractExchange, ContractFlowBucket, ContractOiSnapshot, ContractWhaleMarketType,
            ContractWhaleSourceRole,
        },
    },
};

fn bucket(ts: i64, exchange: &str) -> ContractFlowBucket {
    ContractFlowBucket {
        ts_bucket: ts,
        exchange: exchange.to_string(),
        symbol: "BTC".to_string(),
        market_type: ContractWhaleMarketType::Perp,
        source_role: ContractWhaleSourceRole::Primary,
        product_id: Some("BTCUSDT".to_string()),
        buy_volume_btc: 250.0,
        sell_volume_btc: 0.0,
        buy_notional_usd: 17_500_000.0,
        sell_notional_usd: 0.0,
        trade_count: 20,
        buy_trade_count: 20,
        sell_trade_count: 0,
        max_single_trade_btc: 30.0,
        max_single_trade_share: 0.12,
        vwap: Some(70_000.0),
    }
}

#[test]
fn shadow_route_builder_exposes_only_persistent_sub_high_candidates() {
    let base = 1_700_000_000_000;
    let mut flow = Vec::new();
    for offset in [0, 60_000, 120_000] {
        flow.push(bucket(base + offset, "binance"));
        flow.push(bucket(base + offset, "okx"));
    }
    let oi = vec![
        ContractOiSnapshot {
            ts: base,
            exchange: ContractExchange::Binance,
            symbol: "BTC".to_string(),
            oi_btc: 100_000.0,
            oi_notional_usd: None,
            ct_val_available: true,
            evidence_degraded_reason: None,
        },
        ContractOiSnapshot {
            ts: base + 120_000,
            exchange: ContractExchange::Binance,
            symbol: "BTC".to_string(),
            oi_btc: 100_200.0,
            oi_notional_usd: None,
            ct_val_available: true,
            evidence_degraded_reason: None,
        },
    ];
    let candidates = build_shadow_candidates("BTC", flow, Vec::new(), oi, 10);
    assert_eq!(candidates.len(), 3);
    assert!(candidates
        .iter()
        .any(|candidate| candidate.state == ShadowState::Corroborated));
    assert!(candidates
        .iter()
        .all(|item| item.read_only && item.analysis_only));
    assert!(candidates.iter().all(|item| !item.execution_enabled));
}
