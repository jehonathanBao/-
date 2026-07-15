use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricProvenance {
    Observed,
    CalculatedFromObserved,
    Inferred,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricLineage {
    pub provenance: MetricProvenance,
    pub available: bool,
    pub fresh: bool,
    pub source: String,
    pub observed_at_ms: Option<i64>,
    pub unavailable_reason: Option<String>,
    pub alert_eligible: bool,
}

impl MetricLineage {
    pub fn observed(source: impl Into<String>, observed_at_ms: i64, fresh: bool) -> Self {
        Self::eligible_source(MetricProvenance::Observed, source, observed_at_ms, fresh)
    }

    pub fn calculated(source: impl Into<String>, observed_at_ms: i64, fresh: bool) -> Self {
        Self::eligible_source(
            MetricProvenance::CalculatedFromObserved,
            source,
            observed_at_ms,
            fresh,
        )
    }

    pub fn inferred(source: impl Into<String>, observed_at_ms: i64) -> Self {
        Self {
            provenance: MetricProvenance::Inferred,
            available: true,
            fresh: true,
            source: source.into(),
            observed_at_ms: Some(observed_at_ms),
            unavailable_reason: Some("inferred_not_alert_eligible".to_string()),
            alert_eligible: false,
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            unavailable_reason: Some(reason.into()),
            ..Self::default()
        }
    }

    fn eligible_source(
        provenance: MetricProvenance,
        source: impl Into<String>,
        observed_at_ms: i64,
        fresh: bool,
    ) -> Self {
        Self {
            provenance,
            available: true,
            fresh,
            source: source.into(),
            observed_at_ms: Some(observed_at_ms),
            unavailable_reason: (!fresh).then(|| "source_stale".to_string()),
            alert_eligible: fresh,
        }
    }
}

impl Default for MetricLineage {
    fn default() -> Self {
        Self {
            provenance: MetricProvenance::Unavailable,
            available: false,
            fresh: false,
            source: "unavailable".to_string(),
            observed_at_ms: None,
            unavailable_reason: Some("source_unavailable".to_string()),
            alert_eligible: false,
        }
    }
}
