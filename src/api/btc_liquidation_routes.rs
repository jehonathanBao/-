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
    let flow_state = state.flow_state_for_symbol("BTC");
    Json(build_btc_liquidation_dashboard(&flow_state, now))
}
