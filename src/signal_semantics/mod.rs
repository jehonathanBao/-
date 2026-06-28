pub mod classifier;

pub use classifier::{classify_signal_semantic, SignalSemanticInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalSemanticTier {
    Observe,
    Alert,
    Execution,
}

impl SignalSemanticTier {
    pub fn allows_discord(self) -> bool {
        matches!(self, Self::Alert | Self::Execution)
    }

    pub fn discord_cooldown_seconds(self) -> Option<i64> {
        match self {
            Self::Observe => None,
            Self::Alert => Some(30),
            Self::Execution => Some(10),
        }
    }
}
