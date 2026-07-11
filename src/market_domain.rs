#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketDomain {
    BtcStructure,
}

impl MarketDomain {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::BtcStructure => "BTC_STRUCTURE",
        }
    }
}

pub fn classify_market_domain(symbol: &str) -> MarketDomain {
    let _ = symbol;
    MarketDomain::BtcStructure
}
