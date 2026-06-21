#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketImpactNormalization {
    pub raw_volume: f64,
    pub impact_score: f64,
    pub z_score: f64,
    pub percentile: f64,
    pub normalized_score: f64,
    pub normalized_strength: String,
    pub impact_level: String,
    pub signal_level: String,
    pub signal_label: String,
}

#[derive(Debug, Clone)]
pub struct MarketImpactBaseline {
    mean: f64,
    std: f64,
    sorted_values: Vec<f64>,
}

impl MarketImpactBaseline {
    pub fn from_volumes(volumes: impl IntoIterator<Item = f64>) -> Self {
        let mut values: Vec<f64> = volumes
            .into_iter()
            .filter(|value| value.is_finite() && *value > 0.0)
            .collect();
        values.sort_by(|left, right| left.total_cmp(right));
        let mean = if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<f64>() / values.len() as f64
        };
        let variance = if values.len() < 2 {
            0.0
        } else {
            values
                .iter()
                .map(|value| {
                    let delta = *value - mean;
                    delta * delta
                })
                .sum::<f64>()
                / values.len() as f64
        };
        Self {
            mean,
            std: variance.sqrt(),
            sorted_values: values,
        }
    }

    pub fn normalize(&self, raw_volume: f64) -> MarketImpactNormalization {
        if !raw_volume.is_finite() || raw_volume <= 0.0 || self.mean <= f64::EPSILON {
            return MarketImpactNormalization::empty(raw_volume.max(0.0));
        }

        let impact_score = raw_volume / self.mean;
        let z_score = if self.std > f64::EPSILON {
            (raw_volume - self.mean) / self.std
        } else {
            0.0
        };
        let percentile = self.percentile(raw_volume);
        let z_score_normalized = (z_score.max(0.0) / 3.0).clamp(0.0, 1.0);
        let percentile_normalized = (percentile / 100.0).clamp(0.0, 1.0);
        let normalized_score =
            (0.4 * impact_score + 0.3 * z_score_normalized + 0.3 * percentile_normalized)
                .clamp(0.0, 1.0);
        let impact_level = classify_impact_level(percentile, z_score, impact_score);
        let signal_level = map_impact_to_signal_level(impact_level);

        MarketImpactNormalization {
            raw_volume,
            impact_score,
            z_score,
            percentile,
            normalized_score,
            normalized_strength: classify_normalized_strength(normalized_score).to_string(),
            impact_level: impact_level.to_string(),
            signal_level: signal_level.to_string(),
            signal_label: impact_signal_label(impact_level).to_string(),
        }
    }

    fn percentile(&self, raw_volume: f64) -> f64 {
        if self.sorted_values.len() < 2 {
            return 50.0;
        }
        let rank = self
            .sorted_values
            .iter()
            .filter(|value| **value <= raw_volume)
            .count();
        ((rank as f64 / self.sorted_values.len() as f64) * 100.0).clamp(0.0, 100.0)
    }
}

impl MarketImpactNormalization {
    fn empty(raw_volume: f64) -> Self {
        Self {
            raw_volume,
            impact_score: 0.0,
            z_score: 0.0,
            percentile: 0.0,
            normalized_score: 0.0,
            normalized_strength: "LOW".to_string(),
            impact_level: "C".to_string(),
            signal_level: "L1".to_string(),
            signal_label: "LOW IMPACT EVENT".to_string(),
        }
    }
}

fn classify_impact_level(percentile: f64, z_score: f64, impact_score: f64) -> &'static str {
    if percentile > 97.0 && z_score > 3.5 && impact_score > 5.0 {
        "S"
    } else if percentile >= 90.0 && z_score >= 2.5 && impact_score >= 3.0 {
        "A"
    } else if percentile >= 80.0 && z_score >= 1.5 && impact_score >= 1.8 {
        "B"
    } else {
        "C"
    }
}

fn map_impact_to_signal_level(impact_level: &str) -> &'static str {
    match impact_level {
        "S" => "S",
        "A" => "L3",
        "B" => "L2",
        _ => "L1",
    }
}

fn impact_signal_label(impact_level: &str) -> &'static str {
    match impact_level {
        "S" => "SHOCK IMPACT EVENT",
        "A" => "HIGH IMPACT EVENT",
        "B" => "MEDIUM IMPACT EVENT",
        _ => "LOW IMPACT EVENT",
    }
}

fn classify_normalized_strength(score: f64) -> &'static str {
    if score > 0.85 {
        "EXTREME"
    } else if score > 0.65 {
        "HIGH"
    } else if score > 0.4 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

#[cfg(test)]
mod tests {
    use super::MarketImpactBaseline;

    #[test]
    fn market_impact_maps_impact_levels_to_signal_levels() {
        let baseline = MarketImpactBaseline::from_volumes([
            100.0, 100.0, 100.0, 100.0, 200.0, 200.0, 300.0, 300.0, 400.0, 1_000.0,
        ]);

        let low = baseline.normalize(150.0);
        assert_eq!(low.impact_level, "C");
        assert_eq!(low.signal_level, "L1");
        assert_eq!(low.signal_label, "LOW IMPACT EVENT");

        let medium = baseline.normalize(700.0);
        assert_eq!(medium.impact_level, "B");
        assert_eq!(medium.signal_level, "L2");
        assert_eq!(medium.signal_label, "MEDIUM IMPACT EVENT");

        let high = baseline.normalize(1_000.0);
        assert_eq!(high.impact_level, "A");
        assert_eq!(high.signal_level, "L3");
        assert_eq!(high.signal_label, "HIGH IMPACT EVENT");

        let mut shock_values = vec![100.0; 99];
        shock_values.push(2_000.0);
        let shock_baseline = MarketImpactBaseline::from_volumes(shock_values);
        let shock = shock_baseline.normalize(2_000.0);
        assert_eq!(shock.impact_level, "S");
        assert_eq!(shock.signal_level, "S");
        assert_eq!(shock.signal_label, "SHOCK IMPACT EVENT");
    }
}
