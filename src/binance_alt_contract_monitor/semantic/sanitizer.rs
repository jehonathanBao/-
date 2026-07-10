use crate::binance_alt_contract_monitor::types::{
    AltContractSignal, AltContractSignalType, AltContractSmartMoneyPrediction,
};

pub fn semantic_label(signal_type: AltContractSignalType) -> &'static str {
    match signal_type {
        AltContractSignalType::MainForceLongBuild => "accumulation_pressure",
        AltContractSignalType::MainForceShortBuild => "distribution_pressure",
        AltContractSignalType::AbnormalPump => "upward_imbalance",
        AltContractSignalType::AbnormalDump => "downward_imbalance",
        AltContractSignalType::DownsideAbsorption => "downside_absorption",
        AltContractSignalType::UpsideResistance => "upside_resistance",
        AltContractSignalType::LiquidationCascade => "liquidation_event",
        AltContractSignalType::UnclearContractAnomaly => "contract_anomaly",
    }
}

pub fn semantic_title(signal_type: AltContractSignalType) -> &'static str {
    match signal_type {
        AltContractSignalType::MainForceLongBuild => "累积压力观察",
        AltContractSignalType::MainForceShortBuild => "分发压力观察",
        AltContractSignalType::AbnormalPump => "上行失衡观察",
        AltContractSignalType::AbnormalDump => "下行失衡观察",
        AltContractSignalType::DownsideAbsorption => "下方吸收观察",
        AltContractSignalType::UpsideResistance => "上方压制观察",
        AltContractSignalType::LiquidationCascade => "清算事件观察",
        AltContractSignalType::UnclearContractAnomaly => "合约异动观察",
    }
}

pub fn semantic_summary(signal: &AltContractSignal) -> String {
    if signal.liquidation_suspected {
        return "当前异动伴随清算上下文，优先按清算驱动的行情冲击解释，不直接确认主力建仓。"
            .to_string();
    }
    let prediction_hint = prediction_hint(&signal.smart_money_prediction);
    match signal.signal_type {
        AltContractSignalType::MainForceLongBuild => format!(
            "主动买入、OI 与价格响应呈现同向强化，当前更适合作为累积压力解释；{}",
            prediction_hint
        ),
        AltContractSignalType::MainForceShortBuild => format!(
            "主动卖出、OI 与价格响应呈现同向强化，当前更适合作为分发压力解释；{}",
            prediction_hint
        ),
        AltContractSignalType::AbnormalPump => format!(
            "上行成交冲击显著，但证据更适合解释为上行失衡观察，而不是执行指令；{}",
            prediction_hint
        ),
        AltContractSignalType::AbnormalDump => format!(
            "下行成交冲击显著，但证据更适合解释为下行失衡观察，而不是执行指令；{}",
            prediction_hint
        ),
        AltContractSignalType::DownsideAbsorption => {
            "主动卖出放大但价格跌不动，更适合作为下方吸收解释，不构成执行指令。".to_string()
        }
        AltContractSignalType::UpsideResistance => {
            "主动买入放大但价格涨不动，更适合作为上方压制解释，不构成执行指令。".to_string()
        }
        AltContractSignalType::LiquidationCascade => {
            "成交与强平上下文同时放大，应优先解释为清算事件，不构成执行指令。".to_string()
        }
        AltContractSignalType::UnclearContractAnomaly => {
            "当前只满足合约异动观察条件，证据不足以升级为外部行为提示。".to_string()
        }
    }
}

fn prediction_hint(prediction: &AltContractSmartMoneyPrediction) -> String {
    if prediction.next_state.is_empty() {
        "只读解释，不构成执行指令。".to_string()
    } else {
        format!(
            "下一阶段暂观察 {}（{:.0}% 置信）且保持只读解释，不构成执行指令。",
            prediction.next_state, prediction.probability
        )
    }
}
