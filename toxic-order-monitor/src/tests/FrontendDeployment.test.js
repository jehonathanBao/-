import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { JSDOM } from "jsdom";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const frontendRoot = path.resolve(testDir, "..", "..");
const repoRoot = path.resolve(frontendRoot, "..");

function readFile(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

describe("frontend production deployment", () => {
  it("does not expose the Vite dev server as the public container command", () => {
    const dockerfile = readFile("toxic-order-monitor/Dockerfile.frontend");
    expect(dockerfile).not.toContain('CMD ["npm", "run", "dev"');
  });

  it("does not mount frontend source files into the public compose service", () => {
    const compose = readFile("docker-compose.yml");
    expect(compose).not.toContain("./toxic-order-monitor/src:/app/src");
    expect(compose).not.toContain("./toxic-order-monitor/index.html:/app/index.html");
    expect(compose).not.toContain("./toxic-order-monitor/vite.config.js:/app/vite.config.js");
  });

  it("declares a frontend healthcheck so nginx availability can be supervised", () => {
    const dockerfile = readFile("toxic-order-monitor/Dockerfile.frontend");
    expect(dockerfile).toContain("HEALTHCHECK");
    expect(dockerfile).toContain("http://127.0.0.1:5173/");
  });

  it("ships an explicit production web server config for SPA assets and backend proxying", () => {
    const nginxConfigPath = path.join(repoRoot, "toxic-order-monitor", "nginx.conf.template");
    expect(fs.existsSync(nginxConfigPath)).toBe(true);
    const nginxConfig = fs.readFileSync(nginxConfigPath, "utf8");
    expect(nginxConfig).toContain("location /api/");
    expect(nginxConfig).toContain("location ^~ /api/system/");
    expect(nginxConfig).toContain("location = /api/contract-whale/pipeline-debug");
    expect(nginxConfig).toContain("location = /api/contract-whale/raw-flow-debug");
    expect(nginxConfig).toContain("location = /api/contract-whale/latency-debug");
    expect(nginxConfig).toContain("location = /api/binance-alt-contract/runtime-debug");
    expect(nginxConfig).toContain("location = /api/new-token-watch/runtime-debug");
    expect(nginxConfig).toContain("if ($request_method = POST)");
    expect(nginxConfig).toContain("return 403;");
    expect(nginxConfig).toContain("location /ws/");
    expect(nginxConfig).toContain("try_files $uri $uri/ /index.html");
    expect(nginxConfig).toContain("X-Operator-Api-Token ${OPERATOR_TOKEN}");
    expect(nginxConfig).toContain("Origin ${INTERNAL_API_ORIGIN}");
  });

  it("does not cache the SPA entry document across hashed deployments", () => {
    const nginxConfig = readFile("toxic-order-monitor/nginx.conf.template");

    expect(nginxConfig).toContain("location = /index.html");
    expect(nginxConfig).toContain('Cache-Control "no-cache, no-store, must-revalidate"');
  });

  it("keeps a visible boot shell and arms one guarded retry for failed build assets", () => {
    const html = readFile("toxic-order-monitor/index.html");
    const dom = new JSDOM(html, {
      runScripts: "outside-only",
      url: "http://127.0.0.1:5173/contract-whale/btc",
    });
    const recoveryScript = dom.window.document.querySelector("script[data-bootstrap-recovery]");
    const shell = dom.window.document.querySelector("[data-bootstrap-shell]");

    expect(shell).not.toBeNull();
    expect(recoveryScript).not.toBeNull();
    if (!shell || !recoveryScript) {
      dom.window.close();
      return;
    }

    dom.window.eval(recoveryScript.textContent);
    const failedScript = dom.window.document.createElement("script");
    failedScript.src = "/assets/app-stale.js";
    dom.window.document.head.appendChild(failedScript);
    failedScript.dispatchEvent(new dom.window.Event("error"));

    expect(shell.getAttribute("data-bootstrap-failed")).toBe("true");
    expect(shell.textContent).toContain("正在自动重试");
    expect(dom.window.sessionStorage.getItem("toxic-order-monitor.boot-retry.v1")).toBe("1");
    expect(dom.window.__toxicOrderMonitorMarkBooted).toEqual(expect.any(Function));

    dom.window.__toxicOrderMonitorMarkBooted();
    expect(dom.window.sessionStorage.getItem("toxic-order-monitor.boot-retry.v1")).toBeNull();

    failedScript.dispatchEvent(new dom.window.Event("error"));
    expect(dom.window.sessionStorage.getItem("toxic-order-monitor.boot-retry.v1")).toBeNull();
    dom.window.close();
  });

  it("keeps the frontend container on loopback-only upstream ports with health supervision", () => {
    const compose = readFile("docker-compose.yml");
    expect(compose).toContain('- "${DASHBOARD_BIND_HOST:-127.0.0.1}:5174:5173"');
    expect(compose).not.toContain(':5173:5173"');
    expect(compose).toContain("restart: unless-stopped");
    expect(compose).toContain("healthcheck:");
    expect(compose).toContain("http://127.0.0.1:5173/");
  });

  it("ships a host nginx site template that reverse proxies the SPA to the frontend upstream", () => {
    const ingressTemplatePath = path.join(repoRoot, "deploy", "nginx-site.toxic-order-monitor.conf");
    expect(fs.existsSync(ingressTemplatePath)).toBe(true);
    const ingressTemplate = fs.readFileSync(ingressTemplatePath, "utf8");
    expect(ingressTemplate).toContain("listen 80;");
    expect(ingressTemplate).toContain("listen 5173;");
    expect(ingressTemplate).toContain("proxy_pass http://127.0.0.1:8000");
    expect(ingressTemplate).toContain("location /api/");
    expect(ingressTemplate).toContain("location ^~ /api/system/");
    expect(ingressTemplate).toContain("location = /api/contract-whale/pipeline-debug");
    expect(ingressTemplate).toContain("location = /api/contract-whale/raw-flow-debug");
    expect(ingressTemplate).toContain("location = /api/contract-whale/latency-debug");
    expect(ingressTemplate).toContain("location = /api/binance-alt-contract/runtime-debug");
    expect(ingressTemplate).toContain("location = /api/new-token-watch/runtime-debug");
    expect(ingressTemplate).toContain("if ($request_method = POST)");
    expect(ingressTemplate).toContain("return 403;");
    expect(ingressTemplate).toContain("location /ws/");
    expect(ingressTemplate).toContain("proxy_pass http://127.0.0.1:5174");
    expect(ingressTemplate).toContain("proxy_buffering off;");
    expect(ingressTemplate).toContain("proxy_request_buffering off;");
    expect(ingressTemplate).toContain("proxy_max_temp_file_size 0;");
    expect(ingressTemplate).not.toContain("root /opt/toxic-order-monitor-rs/toxic-order-monitor/dist;");
    expect(ingressTemplate).not.toContain("location /assets/");
    expect(ingressTemplate).not.toContain("try_files $uri $uri/ /index.html;");
    expect(ingressTemplate).not.toContain("location = /dashboard");
  });

  it("ships a production front-end verification script for /contract-whale stability", () => {
    const scriptPath = path.join(repoRoot, "scripts", "check_frontend_prod.sh");
    expect(fs.existsSync(scriptPath)).toBe(true);
    const script = fs.readFileSync(scriptPath, "utf8");
    expect(script).toContain("/contract-whale");
    expect(script).toContain("/dashboard");
    expect(script).toContain("/api/contract-events?symbol=BTC");
  });
});
