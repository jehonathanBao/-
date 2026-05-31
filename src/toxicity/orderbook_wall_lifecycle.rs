use std::collections::{BTreeMap, VecDeque};

use crate::types::{
    liquidation::{
        LiquidationToxicSignal, LiquidationToxicSignalType, LiquidationToxicityRecentResponse,
    },
    market::NormalizedBook,
    orderbook_wall::{
        OrderbookWallCandidateType, OrderbookWallEventType, OrderbookWallLifecycleEvent,
        OrderbookWallLifecycleReport, OrderbookWallLifecycleState, OrderbookWallSide,
        OrderbookWallToxicityCandidate, TrackedOrderbookWall,
    },
    toxic_flow::{
        ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
        ToxicConfidence,
    },
};

const MAX_LEVELS_PER_SIDE: usize = 8;
const MAX_TRACKED_WALLS: usize = 32;
const MAX_EVENTS: usize = 128;
const MIN_WALL_NOTIONAL_USD: f64 = 200_000.0;
const MIN_WALL_RATIO: f64 = 0.22;
const TOUCH_DISTANCE_BPS: f64 = 3.0;
const MOVE_DISTANCE_BPS: f64 = 18.0;
const PARTIAL_FILL_DROP_RATIO: f64 = 0.20;
const UPDATE_CHANGE_RATIO: f64 = 0.05;
const SHORT_LIFETIME_MS: u64 = 5_000;

#[derive(Clone, Default)]
pub struct OrderbookWallLifecycleEngine {
    symbol: String,
    next_wall_id: u64,
    next_event_id: u64,
    tracked: BTreeMap<String, TrackedWallInternal>,
    events: VecDeque<OrderbookWallLifecycleEvent>,
    last_book_ts: Option<u64>,
    warnings: Vec<String>,
}

#[derive(Clone)]
struct TrackedWallInternal {
    wall_id: String,
    symbol: String,
    side: OrderbookWallSide,
    price: f64,
    quantity: f64,
    notional: f64,
    distance_bps: f64,
    first_seen_ms: u64,
    last_seen_ms: u64,
    updates: usize,
    touches: usize,
    status: String,
}

#[derive(Clone, Copy)]
struct WallObservation {
    side: OrderbookWallSide,
    price: f64,
    quantity: f64,
    notional: f64,
    distance_bps: f64,
    observed_at_ms: u64,
}

impl OrderbookWallLifecycleEngine {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            ..Self::default()
        }
    }

    pub fn on_book(&mut self, book: &NormalizedBook) {
        if !book.symbol.eq_ignore_ascii_case(&self.symbol) {
            return;
        }
        self.last_book_ts = Some(book.ts.max(0) as u64);
        let observations = significant_walls(book);
        let mut matched_walls = BTreeMap::new();
        let mut seen_wall_ids = Vec::new();

        for observation in &observations {
            if let Some(wall_id) = self.find_exact_match(*observation, &seen_wall_ids) {
                self.update_existing_wall(&wall_id, *observation, &mut matched_walls);
                seen_wall_ids.push(wall_id);
                continue;
            }
            if let Some(wall_id) = self.find_move_match(*observation, &seen_wall_ids) {
                self.move_wall(&wall_id, *observation, &mut matched_walls);
                seen_wall_ids.push(wall_id);
                continue;
            }
            let wall_id = self.create_wall(*observation, &mut matched_walls);
            seen_wall_ids.push(wall_id);
        }

        let missing_wall_ids = self
            .tracked
            .keys()
            .filter(|wall_id| !matched_walls.contains_key(*wall_id))
            .cloned()
            .collect::<Vec<_>>();
        for wall_id in missing_wall_ids {
            self.remove_or_consume_wall(&wall_id, book.mid, book.ts.max(0) as u64);
        }

        for (wall_id, observation) in matched_walls {
            if observation.distance_bps <= TOUCH_DISTANCE_BPS {
                self.touch_wall(&wall_id, observation);
            }
        }

        self.trim_tracked();
    }

    pub fn get_state(&self) -> OrderbookWallLifecycleState {
        let tracked_walls = self
            .tracked
            .values()
            .cloned()
            .map(to_public_wall)
            .collect::<Vec<_>>();
        let recent_events = self.events.iter().cloned().collect::<Vec<_>>();
        let status = if self.last_book_ts.is_none() {
            "insufficient_data"
        } else if tracked_walls.is_empty() && recent_events.is_empty() {
            "neutral"
        } else {
            "tracking_active"
        };

        OrderbookWallLifecycleState {
            read_only: true,
            runtime_modified: false,
            analysis_mode: "analysis_only".to_string(),
            symbol: self.symbol.clone(),
            generated_at_ms: self.last_book_ts.unwrap_or_default(),
            status: status.to_string(),
            tracked_walls,
            recent_events,
            warnings: self.warnings.clone(),
            no_trade_reasons: if self.last_book_ts.is_none() {
                vec!["orderbook wall lifecycle requires recent book snapshots".to_string()]
            } else {
                Vec::new()
            },
        }
    }

    fn find_exact_match(
        &self,
        observation: WallObservation,
        seen_wall_ids: &[String],
    ) -> Option<String> {
        self.tracked
            .values()
            .find(|wall| {
                wall.side == observation.side
                    && !seen_wall_ids.contains(&wall.wall_id)
                    && (wall.price - observation.price).abs() <= 0.5
            })
            .map(|wall| wall.wall_id.clone())
    }

    fn find_move_match(
        &self,
        observation: WallObservation,
        seen_wall_ids: &[String],
    ) -> Option<String> {
        self.tracked
            .values()
            .find(|wall| {
                wall.side == observation.side
                    && !seen_wall_ids.contains(&wall.wall_id)
                    && distance_bps(wall.price, observation.price) <= MOVE_DISTANCE_BPS
                    && wall.status == "active"
            })
            .map(|wall| wall.wall_id.clone())
    }

    fn create_wall(
        &mut self,
        observation: WallObservation,
        matched_walls: &mut BTreeMap<String, WallObservation>,
    ) -> String {
        self.next_wall_id += 1;
        let wall_id = format!("wall-{:06}", self.next_wall_id);
        let wall = TrackedWallInternal {
            wall_id: wall_id.clone(),
            symbol: self.symbol.clone(),
            side: observation.side,
            price: observation.price,
            quantity: observation.quantity,
            notional: observation.notional,
            distance_bps: observation.distance_bps,
            first_seen_ms: observation.observed_at_ms,
            last_seen_ms: observation.observed_at_ms,
            updates: 0,
            touches: 0,
            status: "active".to_string(),
        };
        self.tracked.insert(wall_id.clone(), wall);
        matched_walls.insert(wall_id.clone(), observation);
        self.push_event(
            &wall_id,
            if observation.side == OrderbookWallSide::Bid {
                OrderbookWallEventType::SupportWallAppeared
            } else {
                OrderbookWallEventType::ResistanceWallAppeared
            },
            observation,
            if observation.side == OrderbookWallSide::Bid {
                "support wall appeared from concentrated bid liquidity"
            } else {
                "resistance wall appeared from concentrated ask liquidity"
            },
        );
        wall_id
    }

    fn update_existing_wall(
        &mut self,
        wall_id: &str,
        observation: WallObservation,
        matched_walls: &mut BTreeMap<String, WallObservation>,
    ) {
        let mut event_to_emit = None;
        if let Some(wall) = self.tracked.get_mut(wall_id) {
            let previous_quantity = wall.quantity;
            let change_ratio = ratio_delta(previous_quantity, observation.quantity);
            wall.quantity = observation.quantity;
            wall.notional = observation.notional;
            wall.distance_bps = observation.distance_bps;
            wall.last_seen_ms = observation.observed_at_ms;
            wall.status = "active".to_string();
            if change_ratio >= PARTIAL_FILL_DROP_RATIO && observation.quantity < previous_quantity {
                wall.updates += 1;
                event_to_emit = Some((
                    OrderbookWallEventType::WallPartiallyFilled,
                    "wall size dropped materially while staying at the same price".to_string(),
                ));
            } else if change_ratio >= UPDATE_CHANGE_RATIO {
                wall.updates += 1;
                event_to_emit = Some((
                    OrderbookWallEventType::WallUpdated,
                    "wall size changed while holding its price level".to_string(),
                ));
            }
        }
        matched_walls.insert(wall_id.to_string(), observation);
        if let Some((event_type, reason)) = event_to_emit {
            self.push_event(wall_id, event_type, observation, &reason);
        }
    }

    fn move_wall(
        &mut self,
        wall_id: &str,
        observation: WallObservation,
        matched_walls: &mut BTreeMap<String, WallObservation>,
    ) {
        let mut event_type = OrderbookWallEventType::WallMovedUp;
        if let Some(wall) = self.tracked.get_mut(wall_id) {
            event_type = if observation.price >= wall.price {
                OrderbookWallEventType::WallMovedUp
            } else {
                OrderbookWallEventType::WallMovedDown
            };
            wall.price = observation.price;
            wall.quantity = observation.quantity;
            wall.notional = observation.notional;
            wall.distance_bps = observation.distance_bps;
            wall.last_seen_ms = observation.observed_at_ms;
            wall.updates += 1;
            wall.status = "active".to_string();
        }
        matched_walls.insert(wall_id.to_string(), observation);
        self.push_event(
            wall_id,
            event_type,
            observation,
            "wall relocated to a nearby price level while staying visible",
        );
    }

    fn touch_wall(&mut self, wall_id: &str, observation: WallObservation) {
        if let Some(wall) = self.tracked.get_mut(wall_id) {
            wall.touches += 1;
        }
        self.push_event(
            wall_id,
            OrderbookWallEventType::WallTouched,
            observation,
            "wall moved within touch distance of the current mid price",
        );
    }

    fn remove_or_consume_wall(&mut self, wall_id: &str, current_mid: f64, observed_at_ms: u64) {
        let Some(wall) = self.tracked.remove(wall_id) else {
            return;
        };
        let observation = WallObservation {
            side: wall.side,
            price: wall.price,
            quantity: wall.quantity,
            notional: wall.notional,
            distance_bps: distance_bps(current_mid, wall.price),
            observed_at_ms,
        };
        let lifetime_ms = observed_at_ms.saturating_sub(wall.first_seen_ms);
        let crossed = matches!(wall.side, OrderbookWallSide::Ask) && current_mid >= wall.price
            || matches!(wall.side, OrderbookWallSide::Bid) && current_mid <= wall.price;
        if crossed || wall.touches > 0 {
            self.push_event(
                &wall.wall_id,
                OrderbookWallEventType::WallConsumed,
                observation,
                "wall disappeared after price traded into or through the level",
            );
            self.push_event(
                &wall.wall_id,
                OrderbookWallEventType::AbsorptionCandidate,
                observation,
                "wall interaction suggests possible absorption or refill behavior",
            );
        } else if lifetime_ms <= SHORT_LIFETIME_MS && wall.touches == 0 {
            self.push_event(
                &wall.wall_id,
                OrderbookWallEventType::WallRemoved,
                observation,
                "wall disappeared shortly after appearing",
            );
            self.push_event(
                &wall.wall_id,
                OrderbookWallEventType::FakeWallCandidate,
                observation,
                "short-lived wall removal looks like a possible fake wall candidate",
            );
        } else {
            self.push_event(
                &wall.wall_id,
                OrderbookWallEventType::WallRemoved,
                observation,
                "wall was removed without a direct cross through the level",
            );
            self.push_event(
                &wall.wall_id,
                OrderbookWallEventType::LiquidityInducementCandidate,
                observation,
                "wall removal may have pulled visible liquidity away from the market",
            );
        }
    }

    fn push_event(
        &mut self,
        wall_id: &str,
        event_type: OrderbookWallEventType,
        observation: WallObservation,
        reason: &str,
    ) {
        self.next_event_id += 1;
        self.events.push_front(OrderbookWallLifecycleEvent {
            event_id: format!("wall-event-{:06}", self.next_event_id),
            wall_id: wall_id.to_string(),
            symbol: self.symbol.clone(),
            event_type,
            side: observation.side,
            price: round2(observation.price),
            notional: round2(observation.notional),
            distance_bps: round2(observation.distance_bps),
            observed_at_ms: observation.observed_at_ms,
            reason: reason.to_string(),
        });
        while self.events.len() > MAX_EVENTS {
            self.events.pop_back();
        }
    }

    fn trim_tracked(&mut self) {
        if self.tracked.len() <= MAX_TRACKED_WALLS {
            return;
        }
        let mut walls = self.tracked.values().cloned().collect::<Vec<_>>();
        walls.sort_by_key(|wall| wall.last_seen_ms);
        let remove_count = self.tracked.len() - MAX_TRACKED_WALLS;
        for wall in walls.into_iter().take(remove_count) {
            self.tracked.remove(&wall.wall_id);
        }
    }
}

pub fn build_orderbook_wall_lifecycle_report(
    state: &OrderbookWallLifecycleState,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
) -> OrderbookWallLifecycleReport {
    let mut toxicity_candidates = intrinsic_candidates(&state.recent_events);
    toxicity_candidates.extend(delta_confluence_candidates(
        &state.recent_events,
        &state.symbol,
        &active_trade_recent.signals,
    ));
    toxicity_candidates.extend(liquidation_confluence_candidates(
        &state.recent_events,
        &state.symbol,
        &liquidation_recent.signals,
    ));

    OrderbookWallLifecycleReport {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        symbol: state.symbol.clone(),
        generated_at_ms: state.generated_at_ms,
        status: state.status.clone(),
        tracked_walls: state.tracked_walls.clone(),
        recent_events: state.recent_events.clone(),
        toxicity_candidates,
        warnings: state.warnings.clone(),
        no_trade_reasons: state.no_trade_reasons.clone(),
    }
}

fn intrinsic_candidates(
    events: &[OrderbookWallLifecycleEvent],
) -> Vec<OrderbookWallToxicityCandidate> {
    events
        .iter()
        .filter_map(|event| match event.event_type {
            OrderbookWallEventType::FakeWallCandidate => Some(candidate_from_event(
                event,
                if event.side == OrderbookWallSide::Bid {
                    OrderbookWallCandidateType::FakeSupportWall
                } else {
                    OrderbookWallCandidateType::FakeResistanceWall
                },
                72.0,
                vec![event.reason.clone()],
                Vec::new(),
            )),
            OrderbookWallEventType::AbsorptionCandidate => Some(candidate_from_event(
                event,
                if event.side == OrderbookWallSide::Bid {
                    OrderbookWallCandidateType::SupportAbsorption
                } else {
                    OrderbookWallCandidateType::ResistanceAbsorption
                },
                68.0,
                vec![event.reason.clone()],
                Vec::new(),
            )),
            OrderbookWallEventType::LiquidityInducementCandidate => Some(candidate_from_event(
                event,
                OrderbookWallCandidateType::LiquidityInducementCandidate,
                64.0,
                vec![event.reason.clone()],
                Vec::new(),
            )),
            OrderbookWallEventType::WallRemoved => Some(candidate_from_event(
                event,
                OrderbookWallCandidateType::LiquidityPullCandidate,
                52.0,
                vec![event.reason.clone()],
                Vec::new(),
            )),
            _ => None,
        })
        .collect()
}

fn delta_confluence_candidates(
    events: &[OrderbookWallLifecycleEvent],
    symbol: &str,
    active_signals: &[ActiveTradeToxicSignal],
) -> Vec<OrderbookWallToxicityCandidate> {
    let bullish = active_signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
                | ActiveTradeToxicSignalType::LargeAggressiveBuy
                | ActiveTradeToxicSignalType::BuySweep
        )
    });
    let bearish = active_signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            ActiveTradeToxicSignalType::OneHourDeltaSellDominant
                | ActiveTradeToxicSignalType::LargeAggressiveSell
                | ActiveTradeToxicSignalType::SellSweep
        )
    });

    events
        .iter()
        .filter_map(|event| {
            let bullish_match = bullish
                && event.side == OrderbookWallSide::Ask
                && matches!(
                    event.event_type,
                    OrderbookWallEventType::WallRemoved
                        | OrderbookWallEventType::WallConsumed
                        | OrderbookWallEventType::AbsorptionCandidate
                );
            let bearish_match = bearish
                && event.side == OrderbookWallSide::Bid
                && matches!(
                    event.event_type,
                    OrderbookWallEventType::WallRemoved
                        | OrderbookWallEventType::WallConsumed
                        | OrderbookWallEventType::AbsorptionCandidate
                );
            if !(bullish_match || bearish_match) {
                return None;
            }
            let confluence = if bullish_match {
                vec![
                    "active_trade_buy_toxicity".to_string(),
                    "1h_delta_buy_dominant".to_string(),
                ]
            } else {
                vec![
                    "active_trade_sell_toxicity".to_string(),
                    "1h_delta_sell_dominant".to_string(),
                ]
            };
            Some(OrderbookWallToxicityCandidate {
                candidate_id: format!("wall-delta-confluence-{}", event.event_id),
                symbol: symbol.to_string(),
                candidate_type: OrderbookWallCandidateType::WallDeltaConfluence,
                side: event.side,
                price: event.price,
                score: 78.0,
                confidence: ToxicConfidence::Medium,
                reasons: vec![
                    "wall lifecycle event aligned with active-trade toxicity pressure".to_string(),
                ],
                confluence,
            })
        })
        .collect()
}

fn liquidation_confluence_candidates(
    events: &[OrderbookWallLifecycleEvent],
    symbol: &str,
    liquidation_signals: &[LiquidationToxicSignal],
) -> Vec<OrderbookWallToxicityCandidate> {
    let upside = liquidation_signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            LiquidationToxicSignalType::UpsideLiquidationMagnet
                | LiquidationToxicSignalType::ShortSqueezeRisk
                | LiquidationToxicSignalType::LiquidationCascadeCandidate
                | LiquidationToxicSignalType::LiquidationDeltaConfluence
        ) && signal.direction == crate::types::liquidation::LiquidationToxicDirection::Upside
    });
    let downside = liquidation_signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            LiquidationToxicSignalType::DownsideLiquidationMagnet
                | LiquidationToxicSignalType::LongSqueezeRisk
                | LiquidationToxicSignalType::LiquidationCascadeCandidate
                | LiquidationToxicSignalType::LiquidationDeltaConfluence
        ) && signal.direction == crate::types::liquidation::LiquidationToxicDirection::Downside
    });

    events
        .iter()
        .filter_map(|event| {
            let upside_match = upside
                && event.side == OrderbookWallSide::Ask
                && matches!(
                    event.event_type,
                    OrderbookWallEventType::WallRemoved
                        | OrderbookWallEventType::WallConsumed
                        | OrderbookWallEventType::LiquidityInducementCandidate
                );
            let downside_match = downside
                && event.side == OrderbookWallSide::Bid
                && matches!(
                    event.event_type,
                    OrderbookWallEventType::WallRemoved
                        | OrderbookWallEventType::WallConsumed
                        | OrderbookWallEventType::LiquidityInducementCandidate
                );
            if !(upside_match || downside_match) {
                return None;
            }
            Some(OrderbookWallToxicityCandidate {
                candidate_id: format!("wall-liq-confluence-{}", event.event_id),
                symbol: symbol.to_string(),
                candidate_type: OrderbookWallCandidateType::WallLiquidationConfluence,
                side: event.side,
                price: event.price,
                score: 76.0,
                confidence: ToxicConfidence::Medium,
                reasons: vec![
                    "wall lifecycle event aligned with liquidation toxicity pressure".to_string(),
                ],
                confluence: if upside_match {
                    vec!["upside_liquidation_pressure".to_string()]
                } else {
                    vec!["downside_liquidation_pressure".to_string()]
                },
            })
        })
        .collect()
}

fn candidate_from_event(
    event: &OrderbookWallLifecycleEvent,
    candidate_type: OrderbookWallCandidateType,
    score: f64,
    reasons: Vec<String>,
    confluence: Vec<String>,
) -> OrderbookWallToxicityCandidate {
    OrderbookWallToxicityCandidate {
        candidate_id: format!("wall-candidate-{}", event.event_id),
        symbol: event.symbol.clone(),
        candidate_type,
        side: event.side,
        price: event.price,
        score,
        confidence: confidence_for_score(score),
        reasons,
        confluence,
    }
}

fn significant_walls(book: &NormalizedBook) -> Vec<WallObservation> {
    let mut observations = Vec::new();
    observations.extend(side_observations(
        &book.symbol,
        book.ts.max(0) as u64,
        book.mid,
        OrderbookWallSide::Bid,
        &book.bids,
    ));
    observations.extend(side_observations(
        &book.symbol,
        book.ts.max(0) as u64,
        book.mid,
        OrderbookWallSide::Ask,
        &book.asks,
    ));
    observations
}

fn side_observations(
    _symbol: &str,
    observed_at_ms: u64,
    mid: f64,
    side: OrderbookWallSide,
    levels: &[(f64, f64)],
) -> Vec<WallObservation> {
    let capped = levels
        .iter()
        .take(MAX_LEVELS_PER_SIDE)
        .copied()
        .collect::<Vec<_>>();
    let total_notional = capped
        .iter()
        .map(|(price, qty)| price * qty)
        .sum::<f64>()
        .max(1.0);
    capped
        .into_iter()
        .filter_map(|(price, quantity)| {
            let notional = price * quantity;
            let wall_ratio = notional / total_notional;
            if notional < MIN_WALL_NOTIONAL_USD && wall_ratio < MIN_WALL_RATIO {
                return None;
            }
            Some(WallObservation {
                side,
                price,
                quantity,
                notional,
                distance_bps: distance_bps(mid, price),
                observed_at_ms,
            })
        })
        .collect()
}

fn to_public_wall(wall: TrackedWallInternal) -> TrackedOrderbookWall {
    TrackedOrderbookWall {
        wall_id: wall.wall_id,
        symbol: wall.symbol,
        side: wall.side,
        price: round2(wall.price),
        notional: round2(wall.notional),
        quantity: round4(wall.quantity),
        distance_bps: round2(wall.distance_bps),
        first_seen_ms: wall.first_seen_ms,
        last_seen_ms: wall.last_seen_ms,
        updates: wall.updates,
        touches: wall.touches,
        status: wall.status,
    }
}

fn distance_bps(base: f64, level: f64) -> f64 {
    if base.abs() <= f64::EPSILON {
        0.0
    } else {
        ((level - base).abs() / base) * 10_000.0
    }
}

fn ratio_delta(old: f64, new: f64) -> f64 {
    if old.abs() <= f64::EPSILON {
        1.0
    } else {
        ((old - new).abs() / old).abs()
    }
}

fn confidence_for_score(score: f64) -> ToxicConfidence {
    if score >= 80.0 {
        ToxicConfidence::High
    } else if score >= 55.0 {
        ToxicConfidence::Medium
    } else {
        ToxicConfidence::Low
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
