use super::SignalSemanticTier;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalSemanticInput {
    pub severity_rank: u8,
    pub score: u8,
    pub confidence: Option<f64>,
    pub data_quality: u8,
    pub consistency_confirmed: bool,
    pub strong_price_response: bool,
    pub multi_window_aligned: bool,
    pub multi_exchange_confirmed: bool,
    pub has_price_response: bool,
}

pub fn classify_signal_semantic(input: SignalSemanticInput) -> SignalSemanticTier {
    if input.severity_rank <= 1
        || input.data_quality < 70
        || input.confidence.is_some_and(|confidence| confidence < 70.0)
        || !input.has_price_response
    {
        return SignalSemanticTier::Observe;
    }

    if input.score >= 90
        && input.strong_price_response
        && input.multi_window_aligned
        && input.multi_exchange_confirmed
    {
        return SignalSemanticTier::Execution;
    }

    if input.score >= 85
        || (input.confidence.unwrap_or_default() >= 70.0 && input.consistency_confirmed)
    {
        return SignalSemanticTier::Alert;
    }

    SignalSemanticTier::Observe
}
