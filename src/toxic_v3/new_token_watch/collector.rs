use super::types::{ContractTick, ContractTickSide};

pub struct ContractFlowCollector;

impl ContractFlowCollector {
    pub fn deterministic_probe_ticks(symbol: &str, now: u64) -> Vec<ContractTick> {
        let hash = symbol_hash(symbol);
        let regime = ((hash + now / 15_000) % 4) as u8;
        let base_price = 0.5 + (hash % 20_000) as f64 / 100.0;
        (0..12)
            .map(|idx| {
                let step = idx as f64;
                let (side, drift, aggression, imbalance) = match regime {
                    0 => (ContractTickSide::Buy, step * 0.0008, 0.74, 0.22),
                    1 => (ContractTickSide::Buy, step * 0.004, 0.82, 0.16),
                    2 => (ContractTickSide::Sell, -step * 0.0015, 0.80, -0.24),
                    _ => {
                        let side = if idx % 2 == 0 {
                            ContractTickSide::Buy
                        } else {
                            ContractTickSide::Sell
                        };
                        (side, (idx % 3) as f64 * 0.0004, 0.48, 0.02)
                    }
                };
                ContractTick {
                    symbol: symbol.to_string(),
                    price: base_price * (1.0 + drift),
                    size: 1.0 + ((hash + idx as u64) % 40) as f64 / 10.0,
                    side,
                    aggression,
                    orderbook_imbalance: imbalance,
                    timestamp: now + idx as u64,
                }
            })
            .collect()
    }
}

fn symbol_hash(symbol: &str) -> u64 {
    symbol
        .bytes()
        .fold(14_695_981_039_346_656_037_u64, |acc, byte| {
            (acc ^ byte as u64).wrapping_mul(1_099_511_628_211)
        })
}
