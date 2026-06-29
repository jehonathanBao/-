#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticType {
    #[default]
    Analysis,
    DecisionSupport,
    RiskOverride,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRiskState {
    #[default]
    Low,
    Guarded,
    High,
}

impl SemanticRiskState {
    pub fn from_label(value: &str) -> Self {
        let normalized = value.trim().to_ascii_uppercase();
        if normalized.contains("HIGH") {
            Self::High
        } else if normalized.contains("MEDIUM") || normalized.contains("GUARDED") {
            Self::Guarded
        } else {
            Self::Low
        }
    }

    pub fn suppresses_decision_support(self) -> bool {
        matches!(self, Self::High)
    }
}
