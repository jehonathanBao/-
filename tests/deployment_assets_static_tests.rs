use std::{fs, path::Path};

#[test]
fn docker_deployment_assets_keep_runtime_and_token_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let required = [
        "Dockerfile.backend",
        "docker-compose.yml",
        ".dockerignore",
        "toxic-order-monitor/Dockerfile.frontend",
        "toxic-order-monitor/vite.config.js",
        "toxic-order-monitor/src/hooks/useReconnectingWebSocket.js",
        "docs/server-deployment-runbook.md",
        "docs/reverse-proxy-production-example.md",
        "docs/production-smoke-checklist.md",
        "docs/windows-rust-build-troubleshooting.md",
        "scripts/smoke-compose.ps1",
        "scripts/smoke-compose.sh",
    ];

    for relative in required {
        assert!(root.join(relative).exists(), "missing asset {relative}");
    }

    let backend = fs::read_to_string(root.join("Dockerfile.backend")).expect("backend dockerfile");
    assert!(backend.contains("cargo build --release --bin btc-toxic-flow-monitor-rs"));
    assert!(backend.contains("READ_ONLY=true"));
    assert!(backend.contains("API_HOST=0.0.0.0"));
    assert!(backend.contains("CMD [\"./btc-toxic-flow-monitor-rs\"]"));

    let compose = fs::read_to_string(root.join("docker-compose.yml")).expect("compose");
    assert!(compose.contains("OPERATOR_TOKEN: ${OPERATOR_TOKEN:?"));
    assert!(compose.contains("INTERNAL_API_ORIGIN: ${INTERNAL_API_ORIGIN:-http://127.0.0.1:3000}"));
    assert!(compose.contains("WS_SIGNAL_INTERVAL_MS"));
    assert!(compose.contains("127.0.0.1:8000:3000"));
    assert!(compose.contains("${DASHBOARD_BIND_HOST:-127.0.0.1}:5174:5173"));
    assert!(compose.contains("./data"));
    assert!(compose.contains("/app/data"));
    assert!(compose.contains("./config"));
    assert!(compose.contains("/app/config"));
    assert!(compose.contains("8000:3000"));
    assert!(!compose.contains("VITE_OPERATOR_TOKEN"));
    assert!(!compose.contains("VITE_API_TOKEN"));

    for relative in [
        "deploy/nginx-site.toxic-order-monitor.conf",
        "toxic-order-monitor/nginx.conf.template",
    ] {
        let nginx = fs::read_to_string(root.join(relative)).expect("nginx config");
        assert!(
            nginx.contains(
                "location = /api/binance-alt-contract/runtime-debug {\n        return 404;\n    }"
            ) || nginx.contains(
                "location = /api/binance-alt-contract/runtime-debug {\n    return 404;\n  }"
            ),
            "operator-only BACM diagnostics must not be forwarded by {relative}"
        );
    }

    let vite = fs::read_to_string(root.join("toxic-order-monitor/vite.config.js")).expect("vite");
    assert!(vite.contains("VITE_PROXY_API_TARGET"));
    assert!(vite.contains("x-operator-api-token"));
    assert!(!vite.contains("VITE_OPERATOR_TOKEN"));
    assert!(!vite.contains("VITE_API_TOKEN"));

    let runbook =
        fs::read_to_string(root.join("docs/server-deployment-runbook.md")).expect("runbook");
    assert!(runbook.contains("browser bundle should not receive it"));
    assert!(runbook.contains("127.0.0.1:8000"));
    assert!(runbook.contains("http://<server-ip>:5173"));
    assert!(runbook.contains("`/ws/signals` streams redacted toxic signal snapshots"));
    assert!(runbook.contains("Browser refreshes and Vite HMR reloads"));

    let smoke_script = fs::read_to_string(root.join("scripts/smoke-compose.sh")).expect("smoke");
    assert!(smoke_script.contains("backend StartedAt before frontend refresh"));
    assert!(smoke_script.contains("docker compose restart frontend"));
    assert!(smoke_script.contains("OPERATOR_TOKEN leaked in frontend"));

    let reverse_proxy = fs::read_to_string(root.join("docs/reverse-proxy-production-example.md"))
        .expect("reverse proxy doc");
    assert!(reverse_proxy.contains("REPLACE_WITH_SERVER_SIDE_SECRET"));
    assert!(reverse_proxy.contains("proxy_set_header x-operator-api-token"));
    assert!(!reverse_proxy.contains("dummy-local-smoke-token"));
}
