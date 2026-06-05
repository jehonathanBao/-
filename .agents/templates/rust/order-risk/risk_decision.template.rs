use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
    DataInsufficient,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SuggestedAction {
    Allow,
    Review,
    Hold,
    Block,
}

#[derive(Debug, Clone)]
pub struct TenantScope {
    pub tenant_id: String,
    pub shop_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone)]
pub struct OrderRiskInput {
    pub order_id: String,
    pub buyer_id: Option<String>,
    pub amount_cents: i64,
    pub risk_version: String,
    pub phone_hash: Option<String>,
    pub address_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDecision {
    pub order_id: String,
    pub risk_score: f64,
    pub risk_level: RiskLevel,
    pub rule_hits: Vec<String>,
    pub model_reason: Option<String>,
    pub suggested_action: SuggestedAction,
    pub requires_manual_review: bool,
    pub dedupe_key: String,
}

pub fn evaluate_order(scope: &TenantScope, input: &OrderRiskInput) -> RiskDecision {
    let mut score = 0.0;
    let mut hits = Vec::new();

    if input.buyer_id.as_deref().unwrap_or_default().trim().is_empty() {
        score += 30.0;
        hits.push("missing_buyer_id".to_string());
    }
    if input.amount_cents <= 0 {
        score += 25.0;
        hits.push("non_positive_amount".to_string());
    }

    let risk_level = match score {
        value if value >= 80.0 => RiskLevel::Critical,
        value if value >= 60.0 => RiskLevel::High,
        value if value >= 30.0 => RiskLevel::Medium,
        _ => RiskLevel::Low,
    };
    let requires_manual_review = matches!(risk_level, RiskLevel::High | RiskLevel::Critical);

    RiskDecision {
        order_id: input.order_id.clone(),
        risk_score: score,
        risk_level,
        rule_hits: hits,
        model_reason: None,
        suggested_action: if requires_manual_review {
            SuggestedAction::Review
        } else {
            SuggestedAction::Allow
        },
        requires_manual_review,
        dedupe_key: format!(
            "{}:{}:{}:{}",
            scope.tenant_id, scope.shop_id, input.order_id, input.risk_version
        ),
    }
}
