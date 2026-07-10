use std::sync::{Mutex, MutexGuard, OnceLock};

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::{
        reset_binance_alt_contract_runtime_config, set_binance_alt_contract_runtime_config,
        BinanceAltContractRuntimeConfig,
    },
    context::context_for_window,
    service::BinanceAltContractService,
    types::{AltContractContext, OiPeriodDelta},
};

fn guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn oi_periods_use_independent_reference_snapshots_and_denominators() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    set_binance_alt_contract_runtime_config(BinanceAltContractRuntimeConfig {
        enabled: true,
        ..BinanceAltContractRuntimeConfig::default()
    });
    let service = BinanceAltContractService::new(true, true, 0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_millis() as i64;

    service.update_open_interest("SOL", now - 300_000, 100.0);
    service.update_open_interest("SOL", now - 240_000, 110.0);
    service.update_open_interest("SOL", now - 60_000, 120.0);
    service.update_open_interest("SOL", now, 130.0);

    let context = service.context_snapshot("SOL");
    assert!(context.oi_change_1m.available);
    assert!(context.oi_change_5m.available);
    assert_eq!(context.oi_change_1m.before_oi, Some(120.0));
    assert_eq!(context.oi_change_5m.before_oi, Some(100.0));
    assert_eq!(context.oi_change_1m.after_oi, Some(130.0));
    assert_eq!(context.oi_change_5m.after_oi, Some(130.0));
    assert_eq!(context.oi_change_1m.delta_pct, Some(8.333333333333332));
    assert_eq!(context.oi_change_5m.delta_pct, Some(30.0));
    assert_ne!(
        context.oi_change_1m.delta_pct,
        context.oi_change_5m.delta_pct
    );

    reset_binance_alt_contract_runtime_config();
}

#[test]
fn stale_oi_periods_are_explicitly_unavailable() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    set_binance_alt_contract_runtime_config(BinanceAltContractRuntimeConfig {
        enabled: true,
        ..BinanceAltContractRuntimeConfig::default()
    });
    let service = BinanceAltContractService::new(true, true, 0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_millis() as i64;

    service.update_open_interest("SOL", now - 500_000, 100.0);
    service.update_open_interest("SOL", now - 400_000, 110.0);

    let context = service.context_snapshot("SOL");
    assert!(!context.oi_change_1m.available);
    assert!(!context.oi_change_5m.available);
    assert!(context.oi_change_1m.stale);
    assert!(context.oi_change_5m.stale);
    assert_eq!(
        context.oi_change_1m.reason.as_deref(),
        Some("latest_snapshot_stale")
    );

    reset_binance_alt_contract_runtime_config();
}

#[test]
fn window_context_uses_only_the_matching_oi_period() {
    let context = AltContractContext {
        oi_change_1m: OiPeriodDelta {
            period_sec: 60,
            delta: Some(10.0),
            delta_pct: Some(10.0),
            available: true,
            ..OiPeriodDelta::default()
        },
        oi_change_5m: OiPeriodDelta {
            period_sec: 300,
            delta: Some(30.0),
            delta_pct: Some(30.0),
            available: true,
            ..OiPeriodDelta::default()
        },
        ..AltContractContext::default()
    };

    let short = context_for_window(&context, 15);
    assert!(short.oi_change_1m_base.is_none());
    assert!(short.oi_change_5m_base.is_none());
    assert_eq!(short.oi_change_pct, None);

    let one_minute = context_for_window(&context, 60);
    assert_eq!(one_minute.oi_change_1m_base, Some(10.0));
    assert!(one_minute.oi_change_5m_base.is_none());
    assert_eq!(one_minute.oi_change_pct, Some(10.0));

    let five_minutes = context_for_window(&context, 300);
    assert!(five_minutes.oi_change_1m_base.is_none());
    assert_eq!(five_minutes.oi_change_5m_base, Some(30.0));
    assert_eq!(five_minutes.oi_change_pct, Some(30.0));
}
