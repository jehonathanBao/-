use crate::contract_whale_monitor::types::{
    ContractWhalePriceResponseType, ContractWhaleSignal, ContractWhaleSignalType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradingDirection {
    Long,
    Short,
    NoTrade,
}

impl TradingDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Long => "LONG",
            Self::Short => "SHORT",
            Self::NoTrade => "NO_TRADE",
        }
    }
}

pub fn classify_direction(signal: &ContractWhaleSignal, score: u8) -> TradingDirection {
    if score < 70 {
        return TradingDirection::NoTrade;
    }
    match signal.signal_type {
        ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::DownsideAbsorption => {
            if price_supports_direction(signal.price_response_type, true) {
                TradingDirection::Long
            } else {
                TradingDirection::NoTrade
            }
        }
        ContractWhaleSignalType::AggressiveSell | ContractWhaleSignalType::UpsideSuppression => {
            if price_supports_direction(signal.price_response_type, false) {
                TradingDirection::Short
            } else {
                TradingDirection::NoTrade
            }
        }
    }
}

pub fn setup_type_label(signal_type: ContractWhaleSignalType) -> &'static str {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy => "主力拉盘",
        ContractWhaleSignalType::AggressiveSell => "主力砸盘",
        ContractWhaleSignalType::DownsideAbsorption => "下方吸收",
        ContractWhaleSignalType::UpsideSuppression => "上方压制",
    }
}

fn price_supports_direction(response_type: ContractWhalePriceResponseType, is_long: bool) -> bool {
    match response_type {
        ContractWhalePriceResponseType::TrendFollowUp => is_long,
        ContractWhalePriceResponseType::TrendFollowDown => !is_long,
        ContractWhalePriceResponseType::DownsideAbsorption => is_long,
        ContractWhalePriceResponseType::UpsideResistance => !is_long,
        ContractWhalePriceResponseType::NoClearResponse => false,
    }
}
