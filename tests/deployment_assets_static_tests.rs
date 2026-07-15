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
    assert!(backend.contains("ARG CARGO_BUILD_JOBS=2"));
    assert!(backend.contains("ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"));
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
    assert!(compose.contains("mem_limit: ${BACKEND_MEMORY_LIMIT:-3g}"));
    assert!(compose.contains("mem_reservation: ${BACKEND_MEMORY_RESERVATION:-512m}"));
    assert!(compose.contains("memswap_limit: ${BACKEND_MEMORY_SWAP_LIMIT:-4g}"));
    assert!(compose.contains("pids_limit: ${BACKEND_PIDS_LIMIT:-256}"));
    assert!(compose.contains("mem_limit: ${FRONTEND_MEMORY_LIMIT:-256m}"));
    assert!(compose.contains("mem_reservation: ${FRONTEND_MEMORY_RESERVATION:-32m}"));
    assert!(compose.contains("memswap_limit: ${FRONTEND_MEMORY_SWAP_LIMIT:-384m}"));
    assert!(compose.contains("pids_limit: ${FRONTEND_PIDS_LIMIT:-64}"));
    assert!(!compose.contains("VITE_OPERATOR_TOKEN"));
    assert!(!compose.contains("VITE_API_TOKEN"));

    for relative in [
        "deploy/nginx-site.toxic-order-monitor.conf",
        "toxic-order-monitor/nginx.conf.template",
    ] {
        let nginx = fs::read_to_string(root.join(relative))
            .expect("nginx config")
            .replace("\r\n", "\n");
        assert!(
            nginx.contains("gzip on;"),
            "gzip must be enabled by {relative}"
        );
        assert!(
            nginx.contains("application/json")
                && nginx.contains("application/javascript")
                && nginx.contains("text/css"),
            "compressible dashboard MIME types must be configured by {relative}"
        );
        assert!(
            nginx.contains("gzip_vary on;"),
            "proxy caches must vary compressed responses by Accept-Encoding in {relative}"
        );
        assert!(
            nginx.contains("max-age=31536000, immutable"),
            "hashed frontend assets must be cached immutably by {relative}"
        );
        for operator_only_path in [
            "/api/binance-alt-contract/runtime-debug",
            "/api/new-token-watch/runtime-debug",
        ] {
            assert!(
                nginx.contains(&format!(
                    "location = {operator_only_path} {{\n        return 404;\n    }}"
                )) || nginx.contains(&format!(
                    "location = {operator_only_path} {{\n    return 404;\n  }}"
                )),
                "operator-only route {operator_only_path} must not be forwarded by {relative}"
            );
        }
        assert!(
            nginx.contains(
                "location /api/ {\n        if ($request_method = POST) {\n            return 403;\n        }"
            ) || nginx.contains(
                "location /api/ {\n    if ($request_method = POST) {\n      return 403;\n    }"
            ),
            "public API proxy must reject mutation requests in {relative}"
        );
    }

    let host_nginx = fs::read_to_string(root.join("deploy/nginx-site.toxic-order-monitor.conf"))
        .expect("host nginx config");
    assert!(
        host_nginx.contains("proxy_hide_header Cache-Control;"),
        "host nginx must replace the upstream asset cache header instead of duplicating it"
    );

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
    assert!(runbook.contains("Do not use `drop_caches` as a recurring memory fix"));
    assert!(runbook.contains("docker builder prune -af --keep-storage 2GB"));
    assert!(runbook.contains("MemoryMax=3G"));

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
