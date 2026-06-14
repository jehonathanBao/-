use super::{
    flow_reality::directional_strength,
    types::{clamp01, HazardState, MarketFlowTick, TrajectoryState, TrajectoryStateKind},
};

pub fn compute_trajectory_state(flow: &MarketFlowTick, hazard: &HazardState) -> TrajectoryState {
    let persistence = clamp01(flow.anomaly_persistence_sec / 600.0);
    let dynamic = clamp01(flow.dynamic_multiple / 10.0);
    let acceleration = if (flow.buy_volume + flow.sell_volume).abs() <= f64::EPSILON {
        0.0
    } else {
        clamp01(flow.flow_acceleration.abs() / (flow.buy_volume + flow.sell_volume).abs())
    };
    let direction = directional_strength(flow);
    let score = ((hazard.lambda_t * 0.35 + persistence * 0.30 + dynamic * 0.20 + direction * 0.15)
        * 100.0)
        .clamp(0.0, 100.0);

    let state = if flow.price_move_pct.signum() != flow.net_flow.signum() && direction >= 0.55 {
        TrajectoryStateKind::Reversal
    } else if persistence >= 0.50 && score >= 60.0 {
        TrajectoryStateKind::Persistent
    } else if acceleration >= 0.40 || dynamic >= 0.40 {
        TrajectoryStateKind::Building
    } else if score < 35.0 {
        TrajectoryStateKind::SinglePoint
    } else {
        TrajectoryStateKind::Decaying
    };

    TrajectoryState {
        score,
        state,
        persistence_sec: flow.anomaly_persistence_sec.max(0.0),
        acceleration: acceleration * 100.0,
        decay_rate: if matches!(state, TrajectoryStateKind::Decaying) {
            0.45
        } else {
            0.0
        },
    }
}
