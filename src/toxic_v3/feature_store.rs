use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use super::types::MarketFlowTick;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FlowStats {
    pub sample_count: usize,
    pub mean_flow: f64,
    pub std_flow: f64,
    pub entropy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureVector {
    pub tick: MarketFlowTick,
    pub rolling_mean_flow: f64,
    pub rolling_std_flow: f64,
    pub rolling_entropy: f64,
    pub flow_zscore: f64,
}

impl FeatureVector {
    pub fn from_tick_and_stats(tick: MarketFlowTick, stats: FlowStats) -> Self {
        let flow_zscore = if stats.std_flow <= f64::EPSILON {
            0.0
        } else {
            (tick.net_flow - stats.mean_flow) / stats.std_flow
        };
        Self {
            tick,
            rolling_mean_flow: stats.mean_flow,
            rolling_std_flow: stats.std_flow,
            rolling_entropy: stats.entropy,
            flow_zscore,
        }
    }
}

pub trait FeatureStore {
    fn update(&mut self, tick: &MarketFlowTick);
    fn rolling_stats(&self, symbol: &str) -> FlowStats;

    fn feature_vector(&self, tick: MarketFlowTick) -> FeatureVector {
        let stats = self.rolling_stats(&tick.symbol);
        FeatureVector::from_tick_and_stats(tick, stats)
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryFeatureStore {
    max_samples_per_symbol: usize,
    samples: BTreeMap<String, VecDeque<MarketFlowTick>>,
}

impl InMemoryFeatureStore {
    pub fn new(max_samples_per_symbol: usize) -> Self {
        Self {
            max_samples_per_symbol: max_samples_per_symbol.max(1),
            samples: BTreeMap::new(),
        }
    }

    pub fn sample_count(&self, symbol: &str) -> usize {
        self.samples.get(symbol).map(VecDeque::len).unwrap_or(0)
    }
}

impl Default for InMemoryFeatureStore {
    fn default() -> Self {
        Self::new(600)
    }
}

impl FeatureStore for InMemoryFeatureStore {
    fn update(&mut self, tick: &MarketFlowTick) {
        let samples = self.samples.entry(tick.symbol.clone()).or_default();
        samples.push_back(tick.clone());
        while samples.len() > self.max_samples_per_symbol {
            samples.pop_front();
        }
    }

    fn rolling_stats(&self, symbol: &str) -> FlowStats {
        let Some(samples) = self.samples.get(symbol) else {
            return FlowStats::default();
        };
        if samples.is_empty() {
            return FlowStats::default();
        }

        let sample_count = samples.len();
        let mean_flow = samples.iter().map(|tick| tick.net_flow).sum::<f64>() / sample_count as f64;
        let variance = samples
            .iter()
            .map(|tick| {
                let delta = tick.net_flow - mean_flow;
                delta * delta
            })
            .sum::<f64>()
            / sample_count as f64;
        let std_flow = variance.sqrt();
        let entropy = direction_entropy(samples);

        FlowStats {
            sample_count,
            mean_flow,
            std_flow,
            entropy,
        }
    }
}

fn direction_entropy(samples: &VecDeque<MarketFlowTick>) -> f64 {
    let mut buy = 0.0;
    let mut sell = 0.0;
    let mut neutral = 0.0;
    for tick in samples {
        if tick.net_flow > 0.0 {
            buy += 1.0;
        } else if tick.net_flow < 0.0 {
            sell += 1.0;
        } else {
            neutral += 1.0;
        }
    }
    let total = buy + sell + neutral;
    if total <= f64::EPSILON {
        return 0.0;
    }
    [buy, sell, neutral]
        .iter()
        .filter(|count| **count > 0.0)
        .map(|count| {
            let p: f64 = *count / total;
            -p * p.log2()
        })
        .sum::<f64>()
        / 3.0_f64.log2()
}
