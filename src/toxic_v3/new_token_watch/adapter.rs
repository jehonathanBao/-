use super::types::{TokenFlowRegime, TokenFlowSignal};
use crate::toxic_v3::{
    DecisionEngine, Direction, MarketFlowExchange, MarketFlowTick, SignalAggregator, SignalEvent,
    SignalSource,
};

pub struct NewTokenSignalAdapter;

impl NewTokenSignalAdapter {
    pub fn to_signal_event(signal: &TokenFlowSignal) -> SignalEvent {
        let direction = match signal.regime {
            TokenFlowRegime::Accumulation | TokenFlowRegime::Building => Direction::Buy,
            TokenFlowRegime::Distribution => Direction::Sell,
            TokenFlowRegime::Neutral => Direction::Neutral,
        };
        let signed_flow = match direction {
            Direction::Buy => signal.strength.max(0.01),
            Direction::Sell => -signal.strength.max(0.01),
            _ => 0.0,
        };
        let tick = MarketFlowTick {
            ts: signal.updated_at_ms,
            exchange: MarketFlowExchange::Binance,
            symbol: signal.symbol.clone(),
            buy_volume: if signed_flow > 0.0 { signed_flow } else { 0.0 },
            sell_volume: if signed_flow < 0.0 {
                signed_flow.abs()
            } else {
                0.0
            },
            net_flow: signed_flow,
            flow_acceleration: signal.strength,
            trade_count: 12,
            avg_trade_size: signal.strength.max(0.01),
            large_trade_ratio: signal.strength,
            realized_vol: (1.0 - signal.confidence).max(0.0) * 0.05,
            open_interest_delta: signal.strength * 0.2,
            funding_rate: 0.0,
            liquidation_pressure: 0.0,
            price_move_pct: match direction {
                Direction::Buy => signal.strength * 0.01,
                Direction::Sell => -signal.strength * 0.01,
                _ => 0.0,
            },
            dynamic_multiple: 1.0 + signal.strength * 4.0,
            anomaly_persistence_sec: 30.0,
            cross_exchange_dispersion: 0.0,
        };
        let mut decision = DecisionEngine::default();
        decision.external_dispatch_enabled = false;
        SignalAggregator::evaluate_tick(
            &tick,
            SignalSource::FlowInference,
            signal.confidence * 100.0,
            &decision,
        )
    }
}
