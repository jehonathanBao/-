use axum::{extract::State, Json};

use crate::{
    app::AppState,
    normalizers::trade::now_ms,
    toxic_v3::{build_btc_liquidation_dashboard, BTCLiquidationDashboard},
};

pub async fn btc_liquidation_dashboard_route(
    State(state): State<AppState>,
) -> Json<BTCLiquidationDashboard> {
    let now = now_ms();
    let candidates = ["BTC-PERP", "BTCUSDT", "BTCUSD", "XBTUSD", "BTC"];
    let mut best = None;

    for symbol in candidates {
        let flow_state = state.flow_state_for_symbol(symbol);
        let dashboard = build_btc_liquidation_dashboard(&flow_state, now);
        if dashboard.live {
            return Json(dashboard);
        }
        if best
            .as_ref()
            .is_none_or(|current: &BTCLiquidationDashboard| {
                dashboard.current_price_usd.is_some() && current.current_price_usd.is_none()
            })
        {
            best = Some(dashboard);
        }
    }

    Json(best.unwrap_or_else(|| BTCLiquidationDashboard {
        ts: now,
        data_status: "waiting_for_btc_flow".to_string(),
        ..BTCLiquidationDashboard::default()
    }))
}
