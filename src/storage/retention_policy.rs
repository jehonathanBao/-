//! Shared retention classification for persisted market-monitor evidence.
//!
//! The policy is deliberately conservative: a high score alone never upgrades a
//! record to the one-year tier. Critical retention requires extreme, replayable
//! evidence so ordinary S-shaped candidates cannot permanently grow the database.

const BTC_SPOT_IMPORTANT_NET: f64 = 100.0;
const BTC_SPOT_CRITICAL_NET: f64 = 500.0;
const ETH_SPOT_IMPORTANT_NET: f64 = 1_000.0;
const ETH_SPOT_CRITICAL_NET: f64 = 5_000.0;
const CONTRACT_CRITICAL_VOLUME_BTC: f64 = 20_000.0;
const CONTRACT_CRITICAL_LIQUIDATION_BTC: f64 = 1_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum RetentionClass {
    #[default]
    Ordinary,
    Important,
    Critical,
}

impl RetentionClass {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::Important => "important",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    pub ordinary_days: i64,
    pub important_days: i64,
    pub critical_days: i64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            ordinary_days: 7,
            important_days: 30,
            critical_days: 365,
        }
    }
}

impl RetentionPolicy {
    pub fn days(self, class: RetentionClass) -> i64 {
        match class {
            RetentionClass::Ordinary => self.ordinary_days.max(1),
            RetentionClass::Important => self.important_days.max(self.ordinary_days.max(1)),
            RetentionClass::Critical => self
                .critical_days
                .max(self.important_days.max(self.ordinary_days.max(1))),
        }
    }

    pub fn retain_until(self, now_ms: i64, class: RetentionClass) -> i64 {
        now_ms.saturating_add(self.days(class).saturating_mul(86_400_000))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContractRetentionFacts {
    pub severity: String,
    pub impact_level: Option<String>,
    pub total_volume_btc: f64,
    pub window_sec: u64,
    pub net_volume_btc: f64,
    pub liquidation_btc: f64,
    pub multi_exchange_confirmed: bool,
    pub behavior_confirmed: bool,
    pub discord_sent: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpotRetentionFacts {
    pub symbol: String,
    pub net_volume_base: f64,
    pub multi_exchange_confirmed: bool,
    pub discord_sent: bool,
    pub behavior_confirmed: bool,
}

pub fn classify_contract(facts: &ContractRetentionFacts) -> RetentionClass {
    let impact = facts
        .impact_level
        .as_deref()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let extreme_turnover = impact == "S"
        && facts.total_volume_btc >= CONTRACT_CRITICAL_VOLUME_BTC
        && facts.window_sec >= 60
        && facts.multi_exchange_confirmed;
    let extreme_liquidation = facts.liquidation_btc >= CONTRACT_CRITICAL_LIQUIDATION_BTC;
    if extreme_turnover || extreme_liquidation {
        return RetentionClass::Critical;
    }

    if facts.discord_sent
        || facts.behavior_confirmed
        || matches!(impact.as_str(), "A" | "B" | "S")
        || facts.net_volume_btc.abs() >= 500.0
    {
        RetentionClass::Important
    } else {
        RetentionClass::Ordinary
    }
}

pub fn classify_spot(facts: &SpotRetentionFacts) -> RetentionClass {
    let symbol = facts.symbol.trim().to_ascii_uppercase();
    let abs_net = facts.net_volume_base.abs();
    let (important_net, critical_net) = if symbol == "ETH" {
        (ETH_SPOT_IMPORTANT_NET, ETH_SPOT_CRITICAL_NET)
    } else {
        (BTC_SPOT_IMPORTANT_NET, BTC_SPOT_CRITICAL_NET)
    };
    if abs_net >= critical_net {
        RetentionClass::Critical
    } else if facts.discord_sent
        || facts.behavior_confirmed
        || facts.multi_exchange_confirmed
        || abs_net >= important_net
    {
        RetentionClass::Important
    } else {
        RetentionClass::Ordinary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_days_are_monotonic() {
        let policy = RetentionPolicy::default();
        assert!(policy.days(RetentionClass::Ordinary) < policy.days(RetentionClass::Important));
        assert!(policy.days(RetentionClass::Important) < policy.days(RetentionClass::Critical));
    }
}
