use crate::contract_whale_monitor::trading::classifier::TradingDirection;

pub fn direction_bias_from_trading_direction(direction: TradingDirection) -> &'static str {
    match direction {
        TradingDirection::Long => "BULLISH_BIAS",
        TradingDirection::Short => "BEARISH_BIAS",
        TradingDirection::NoTrade => "NEUTRAL_BIAS",
    }
}

pub fn sanitize_decision_copy(value: &str) -> String {
    value
        .replace("LONG", "bullish bias")
        .replace("SHORT", "bearish bias")
        .replace("做多", "偏多结构")
        .replace("做空", "偏空结构")
        .replace("入场", "压力观察")
        .replace("止损", "风险边界")
        .replace("交易失效线", "结构风险边界")
        .replace("失效参考位", "风险边界")
}
