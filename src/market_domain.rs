#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketDomain {
    BtcStructure,
    AltcoinManipulation,
}

impl MarketDomain {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::BtcStructure => "BTC_STRUCTURE",
            Self::AltcoinManipulation => "ALTCOIN_MANIPULATION",
        }
    }
}

pub fn classify_market_domain(symbol: &str) -> MarketDomain {
    let normalized = crate::market_regime_engine::normalize_market_symbol(symbol);
    if normalized == "BTC" {
        MarketDomain::BtcStructure
    } else {
        MarketDomain::AltcoinManipulation
    }
}
