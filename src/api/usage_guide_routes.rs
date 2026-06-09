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
        assert!(USAGE_GUIDE_MARKDOWN.contains("当前有毒订单判断逻辑"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("mainForceConfirmed = true"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("OKX 关闭时"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("信号卡片字段怎么看"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("合约监控信号怎么解读"));
        assert!(USAGE_GUIDE_MARKDOWN.contains("Discord 状态怎么理解"));
        assert!(!USAGE_GUIDE_MARKDOWN.contains("discord.com/api/webhooks/"));
    }
}
