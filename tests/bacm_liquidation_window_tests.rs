use std::sync::{Mutex, MutexGuard, OnceLock};

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::{
        reset_binance_alt_contract_runtime_config, set_binance_alt_contract_runtime_config,
        BinanceAltContractRuntimeConfig,
    },
    service::BinanceAltContractService,
    types::{AltLiquidationEvent, LiquidationSide},
};

fn guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn liquidation_windows_expire_deduplicate_and_keep_side_totals() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    set_binance_alt_contract_runtime_config(BinanceAltContractRuntimeConfig {
        enabled: true,
        ..BinanceAltContractRuntimeConfig::default()
    });
    let service = BinanceAltContractService::new(true, true, 0);

    service.update_liquidation_event(AltLiquidationEvent {
        product_id: "SOLUSDT".to_string(),
        ts: 1_000,
        side: LiquidationSide::LongLiquidation,
        notional_usd: 2_000_000.0,
        price: Some(100.0),
        quantity: Some(20_000.0),
        source_event_id: Some("force-1".to_string()),
    });
    service.update_liquidation_event(AltLiquidationEvent {
        product_id: "SOLUSDT".to_string(),
        ts: 1_000,
        side: LiquidationSide::LongLiquidation,
        notional_usd: 2_000_000.0,
        price: Some(100.0),
        quantity: Some(20_000.0),
        source_event_id: Some("force-1".to_string()),
    });
    service.update_liquidation_event(AltLiquidationEvent {
        product_id: "SOLUSDT".to_string(),
        ts: 50_000,
        side: LiquidationSide::ShortLiquidation,
        notional_usd: 3_000_000.0,
        price: Some(100.0),
        quantity: Some(30_000.0),
        source_event_id: Some("force-2".to_string()),
    });

    let current = service.liquidation_window_snapshot("SOL", 60, 60_000);
    assert_eq!(current.liquidation_count, 2);
    assert_eq!(current.liquidation_total_usd, 5_000_000.0);
    assert_eq!(current.long_liquidation_usd, 2_000_000.0);
    assert_eq!(current.short_liquidation_usd, 3_000_000.0);
    assert_eq!(
        current.dominant_liquidation_side,
        LiquidationSide::ShortLiquidation
    );

    let expired = service.liquidation_window_snapshot("SOL", 60, 400_000);
    assert_eq!(expired.liquidation_count, 0);
    assert_eq!(expired.liquidation_total_usd, 0.0);
    reset_binance_alt_contract_runtime_config();
}
