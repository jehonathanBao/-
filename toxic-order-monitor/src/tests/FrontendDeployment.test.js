import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

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
    expect(nginxConfig).toContain("location /ws/");
    expect(nginxConfig).toContain("try_files $uri $uri/ /index.html");
    expect(nginxConfig).toContain("X-Operator-Api-Token ${OPERATOR_TOKEN}");
    expect(nginxConfig).toContain("Origin ${INTERNAL_API_ORIGIN}");
  });

  it("keeps the frontend container on loopback-only upstream ports with health supervision", () => {
    const compose = readFile("docker-compose.yml");
    expect(compose).toContain('- "${DASHBOARD_BIND_HOST:-127.0.0.1}:5174:5173"');
    expect(compose).not.toContain(':5173:5173"');
    expect(compose).toContain("restart: unless-stopped");
    expect(compose).toContain("healthcheck:");
    expect(compose).toContain("http://127.0.0.1:5173/");
  });

  it("ships a host nginx site template that serves SPA assets directly and only proxies API/ws", () => {
    const ingressTemplatePath = path.join(repoRoot, "deploy", "nginx-site.toxic-order-monitor.conf");
    expect(fs.existsSync(ingressTemplatePath)).toBe(true);
    const ingressTemplate = fs.readFileSync(ingressTemplatePath, "utf8");
    expect(ingressTemplate).toContain("root /opt/toxic-order-monitor-rs/toxic-order-monitor/dist;");
    expect(ingressTemplate).toContain("location /assets/");
    expect(ingressTemplate).toContain("try_files $uri $uri/ /index.html;");
    expect(ingressTemplate).toContain("listen 80;");
    expect(ingressTemplate).toContain("listen 5173;");
    expect(ingressTemplate).toContain("proxy_pass http://127.0.0.1:8000");
    expect(ingressTemplate).toContain("location /api/");
    expect(ingressTemplate).toContain("location /ws/");
    expect(ingressTemplate).not.toContain("proxy_pass http://127.0.0.1:5174");
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
