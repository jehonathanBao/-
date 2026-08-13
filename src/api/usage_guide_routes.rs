use axum::Json;

const USAGE_GUIDE_MARKDOWN: &str = include_str!("../../docs/usage-guide.md");

pub async fn usage_guide_route() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "title": "有毒订单监控用户使用指南",
        "sourcePath": "docs/usage-guide.md",
        "readOnly": true,
        "markdown": USAGE_GUIDE_MARKDOWN
    }))
}

#[cfg(test)]
mod tests {
    use super::USAGE_GUIDE_MARKDOWN;

    #[test]
    fn usage_guide_markdown_contains_signal_interpretation_sections() {
        assert!(USAGE_GUIDE_MARKDOWN.contains("主力行为状态怎么读"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("behaviorState = confirmed"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("invalidated / 已失效"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("BTC 1H 主动成交差值提醒"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("1H 已收线"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("ordinary / 普通：7 天"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("important / 重要：30 天"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("critical / 关键：365 天"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("S 级市场冲击 ≠ 主力确认"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("OKX disabled"));
        assert!(!USAGE_GUIDE_MARKDOWN.contains("discord.com/api/webhooks/"));
    }
}
