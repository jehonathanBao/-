use std::{fs, path::Path};

#[test]
fn optimization_skill_assets_exist_with_safety_anchors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let required = [
        ".agents/skills/rust-backend-security/SKILL.md",
        ".agents/skills/risk-control-business/SKILL.md",
        ".agents/skills/frontend-risk-console/SKILL.md",
        ".agents/skills/test-matrix/SKILL.md",
        ".agents/skills/langflow-risk-sidecar/SKILL.md",
        ".agents/templates/rust/order-risk/risk_decision.template.rs",
        ".agents/templates/rust/order-risk/order_repository_sqlx.template.rs",
        ".agents/templates/rust/order-risk/langflow_client.template.rs",
        ".agents/templates/frontend/react-risk-console/RiskOrdersTable.tsx",
        ".agents/templates/ai-sidecar/langflow/order_risk_flow.example.json",
        ".agents/templates/ai-sidecar/webhook/langflow_order_risk_webhook.py",
        "docs/langflow-order-risk-sidecar.md",
        "docs/langflow-risk-webhook-prototype.md",
        "docs/production-replay-runbook.md",
        "docs/react-risk-console-scaffold.md",
        "scripts/scaffold-react-risk-console.ps1",
        "scripts/run_production_replay.ps1",
        "scripts/run_production_replay.sh",
    ];

    for relative in required {
        assert!(root.join(relative).exists(), "missing asset {relative}");
    }

    let backend_skill =
        fs::read_to_string(root.join(".agents/skills/rust-backend-security/SKILL.md"))
            .expect("backend skill");
    assert!(backend_skill.contains("tenant_id"));
    assert!(backend_skill.contains("shop_id"));
    assert!(backend_skill.contains("user_id"));
    assert!(backend_skill.contains("Never build SQL with string interpolation"));
    assert!(backend_skill.contains("Never log phone numbers"));
    assert!(backend_skill.contains("SSRF"));

    let sidecar_skill =
        fs::read_to_string(root.join(".agents/skills/langflow-risk-sidecar/SKILL.md"))
            .expect("sidecar skill");
    assert!(sidecar_skill.contains("advisory only"));
    assert!(sidecar_skill.contains("Low-risk output must never trigger automatic blacklist"));
    assert!(sidecar_skill.contains("fail closed"));
    assert!(sidecar_skill.contains("Webhook Prototype Rules"));

    let replay_runbook =
        fs::read_to_string(root.join("docs/production-replay-runbook.md")).expect("runbook");
    assert!(replay_runbook.contains("config/replay.production.local.toml"));
    assert!(replay_runbook.contains("snapshot_reset"));
    assert!(replay_runbook.contains("Medium candidates"));
    assert!(replay_runbook.contains("[BLOCKED]"));

    let replay_script =
        fs::read_to_string(root.join("scripts/run_production_replay.ps1")).expect("replay script");
    assert!(replay_script.contains("config/replay.production.local.toml"));
    assert!(replay_script.contains("data/production_replay"));
    assert!(!replay_script.contains("--config config/replay.production.example.toml"));

    let scaffold =
        fs::read_to_string(root.join("scripts/scaffold-react-risk-console.ps1")).expect("scaffold");
    assert!(scaffold.contains("/notifications/discord/test"));
    assert!(scaffold.contains("browser code never owns a Discord webhook"));
    assert!(scaffold.contains("openGroups"));
    assert!(scaffold.contains("System time"));
    assert!(scaffold.contains("PAGE_SIZE"));
    assert!(scaffold.contains("setSelectedRisk"));
    assert!(scaffold.contains("requestDiscordAlert"));
    assert!(!scaffold.contains("VITE_DISCORD_WEBHOOK"));
    assert!(!scaffold.contains("discord.com/api/webhooks"));
}
