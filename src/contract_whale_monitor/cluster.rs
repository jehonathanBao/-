use std::collections::BTreeMap;

use super::types::{
    ContractWhaleDirection, ContractWhalePersistenceState, ContractWhaleSignal,
    ContractWhaleSignalCluster, ContractWhaleSignalType,
};

const CLUSTER_WINDOW_MS: i64 = 120_000;
const REDUNDANT_WINDOW_MS: i64 = 60_000;
const MAX_PRICE_RANGE_PCT: f64 = 0.30;
const SIGNAL_HALF_LIFE_MS: u64 = 60_000;

pub fn apply_contract_whale_signal_clusters(signals: &mut [ContractWhaleSignal]) {
    if signals.is_empty() {
        return;
    }

    let mut sorted_indices = (0..signals.len()).collect::<Vec<_>>();
    sorted_indices.sort_by_key(|index| signals[*index].ts);

    let mut clusters: Vec<ClusterAccumulator> = Vec::new();
    for index in sorted_indices {
        let signal = &signals[index];
        let price = signal_cluster_price(signal);
        if let Some(cluster) = clusters
            .iter_mut()
            .find(|cluster| cluster.accepts(signal, price))
        {
            cluster.push(index, signal, price);
        } else {
            clusters.push(ClusterAccumulator::new(index, signal, price));
        }
    }

    for cluster in clusters {
        let snapshot = cluster.snapshot();
        let persistence = cluster.persistence_state();
        let mut members = cluster.indices.clone();
        members.sort_by_key(|index| signals[*index].ts);
        let mut previous: Option<usize> = None;
        for index in members {
            let mut member_persistence = persistence.clone();
            if let Some(previous_index) = previous {
                if is_redundant(&signals[index], &signals[previous_index]) {
                    member_persistence.redundant_with_previous = true;
                    member_persistence.redundant_reason = "same_intent_within_60s".to_string();
                }
            }
            signals[index].cluster = snapshot.clone();
            signals[index].persistence = member_persistence;
            previous = Some(index);
        }
    }
}

pub fn is_redundant(new: &ContractWhaleSignal, last: &ContractWhaleSignal) -> bool {
    new.signal_type == last.signal_type
        && new.direction == last.direction
        && new.ts.saturating_sub(last.ts).abs() < REDUNDANT_WINDOW_MS
        && (new.score as f64 - last.score as f64).abs() < 5.0
}

#[derive(Debug, Clone)]
struct ClusterAccumulator {
    symbol: String,
    direction: ContractWhaleDirection,
    indices: Vec<usize>,
    started_at: i64,
    updated_at: i64,
    min_price: Option<f64>,
    max_price: Option<f64>,
    max_score: u8,
    signal_type_counts: BTreeMap<ContractWhaleSignalType, usize>,
}

impl ClusterAccumulator {
    fn new(index: usize, signal: &ContractWhaleSignal, price: Option<f64>) -> Self {
        let mut signal_type_counts = BTreeMap::new();
        signal_type_counts.insert(signal.signal_type, 1);
        Self {
            symbol: normalized_symbol(&signal.symbol),
            direction: signal.direction,
            indices: vec![index],
            started_at: signal.ts,
            updated_at: signal.ts,
            min_price: price,
            max_price: price,
            max_score: signal.score,
            signal_type_counts,
        }
    }

    fn accepts(&self, signal: &ContractWhaleSignal, price: Option<f64>) -> bool {
        self.symbol == normalized_symbol(&signal.symbol)
            && self.direction == signal.direction
            && signal.ts.saturating_sub(self.updated_at).abs() <= CLUSTER_WINDOW_MS
            && self
                .price_range_with(price)
                .is_none_or(|range| range <= MAX_PRICE_RANGE_PCT)
    }

    fn push(&mut self, index: usize, signal: &ContractWhaleSignal, price: Option<f64>) {
        self.indices.push(index);
        self.started_at = self.started_at.min(signal.ts);
        self.updated_at = self.updated_at.max(signal.ts);
        self.max_score = self.max_score.max(signal.score);
        *self
            .signal_type_counts
            .entry(signal.signal_type)
            .or_insert(0) += 1;
        if let Some(price) = price.filter(|value| value.is_finite() && *value > 0.0) {
            self.min_price = Some(self.min_price.map_or(price, |current| current.min(price)));
            self.max_price = Some(self.max_price.map_or(price, |current| current.max(price)));
        }
    }

    fn snapshot(&self) -> ContractWhaleSignalCluster {
        let dominant_intent = self
            .signal_type_counts
            .iter()
            .max_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(right.0)))
            .map(|(signal_type, _)| signal_intent(*signal_type))
            .unwrap_or("single_signal")
            .to_string();
        ContractWhaleSignalCluster {
            cluster_id: format!(
                "cwm-cluster:{}:{}:{}",
                self.symbol,
                direction_key(self.direction),
                self.started_at.div_euclid(CLUSTER_WINDOW_MS)
            ),
            signal_count: self.indices.len(),
            dominant_intent,
            started_at: self.started_at,
            updated_at: self.updated_at,
            duration_ms: self.updated_at.saturating_sub(self.started_at).max(0) as u64,
            intensity: round((self.max_score as f64 / 100.0).clamp(0.0, 1.0), 4),
            price_range_pct: self.price_range_with(None).map(|value| round(value, 4)),
        }
    }

    fn persistence_state(&self) -> ContractWhalePersistenceState {
        let duration_component = (self.updated_at.saturating_sub(self.started_at).max(0) as f64
            / CLUSTER_WINDOW_MS as f64)
            .clamp(0.0, 1.0);
        let repetition_component =
            ((self.indices.len().saturating_sub(1)) as f64 / 3.0).clamp(0.0, 1.0);
        let dominant_count = self.signal_type_counts.values().copied().max().unwrap_or(1);
        let regime_stability = dominant_count as f64 / self.indices.len().max(1) as f64;
        ContractWhalePersistenceState {
            persistence_score: round(
                (duration_component * 0.45 + repetition_component * 0.35 + regime_stability * 0.20)
                    .clamp(0.0, 1.0),
                4,
            ),
            signal_half_life_ms: SIGNAL_HALF_LIFE_MS,
            regime_stability: round(regime_stability.clamp(0.0, 1.0), 4),
            redundant_with_previous: false,
            redundant_reason: String::new(),
        }
    }

    fn price_range_with(&self, price: Option<f64>) -> Option<f64> {
        let mut min_price = self.min_price;
        let mut max_price = self.max_price;
        if let Some(price) = price.filter(|value| value.is_finite() && *value > 0.0) {
            min_price = Some(min_price.map_or(price, |current| current.min(price)));
            max_price = Some(max_price.map_or(price, |current| current.max(price)));
        }
        let (min_price, max_price) = (min_price?, max_price?);
        let midpoint = (min_price + max_price) / 2.0;
        (midpoint > 0.0).then(|| (max_price - min_price).abs() / midpoint * 100.0)
    }
}

fn signal_cluster_price(signal: &ContractWhaleSignal) -> Option<f64> {
    signal
        .order_price_usd
        .or(signal.current_market_price_usd)
        .or_else(|| {
            (signal.total_volume_btc > f64::EPSILON && signal.total_notional_usd > 0.0)
                .then(|| signal.total_notional_usd / signal.total_volume_btc)
        })
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn normalized_symbol(symbol: &str) -> String {
    symbol
        .trim()
        .to_ascii_uppercase()
        .replace("-PERP", "")
        .replace("PERP", "")
        .replace("USDT", "")
        .replace("USD", "")
}

fn signal_intent(signal_type: ContractWhaleSignalType) -> &'static str {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy => "liquidity_probe_buy",
        ContractWhaleSignalType::AggressiveSell => "liquidity_probe_sell",
        ContractWhaleSignalType::DownsideAbsorption => "downside_absorption",
        ContractWhaleSignalType::UpsideSuppression => "upside_suppression",
    }
}

fn direction_key(direction: ContractWhaleDirection) -> &'static str {
    match direction {
        ContractWhaleDirection::Buy => "buy",
        ContractWhaleDirection::Sell => "sell",
        ContractWhaleDirection::Absorption => "absorption",
        ContractWhaleDirection::Suppression => "suppression",
    }
}

fn round(value: f64, decimals: i32) -> f64 {
    let factor = 10_f64.powi(decimals);
    (value * factor).round() / factor
}
