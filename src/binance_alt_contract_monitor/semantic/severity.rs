use crate::binance_alt_contract_monitor::types::AltContractSeverity;

pub fn descriptive_intensity_label(severity: AltContractSeverity) -> &'static str {
    match severity {
        AltContractSeverity::Calm => "low_intensity_observation",
        AltContractSeverity::Medium => "watch_intensity_observation",
        AltContractSeverity::High => "elevated_intensity_observation",
        AltContractSeverity::Critical | AltContractSeverity::S => "high_intensity_observation",
    }
}
