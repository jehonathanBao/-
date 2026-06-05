import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL(".", import.meta.url));
const apiProxyTarget = process.env.VITE_PROXY_API_TARGET || "http://127.0.0.1:3000";
const wsProxyTarget = process.env.VITE_PROXY_WS_TARGET || "ws://127.0.0.1:3000";
const operatorToken = process.env.OPERATOR_TOKEN || process.env.OPERATOR_API_TOKEN || "";

export default defineConfig({
  root,
  cacheDir: "node_modules/.vite",
  plugins: [react()],
  build: {
    rollupOptions: {
      input: {
        app: "index.html",
      },
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return undefined;
          }
          if (id.includes("echarts")) {
            return "echarts";
          }
          if (id.includes("zrender")) {
            return "zrender";
          }
          if (id.includes("react") || id.includes("react-dom") || id.includes("react-router")) {
            return "react-vendor";
          }
          return "vendor";
        },
      },
    },
  },
  test: {
    environment: "jsdom",
  },
  server: {
    host: "0.0.0.0",
    watch: {
      usePolling: process.env.CHOKIDAR_USEPOLLING === "true",
    },
    proxy: {
      "/api": {
        target: apiProxyTarget,
        changeOrigin: true,
        configure: (proxy) => {
          proxy.on("proxyReq", (proxyReq) => {
            proxyReq.removeHeader("origin");
            if (operatorToken) {
              proxyReq.setHeader("x-operator-api-token", operatorToken);
            }
          });
        },
      },
      "/ws": {
        target: wsProxyTarget,
        ws: true,
        changeOrigin: true,
        configure: (proxy) => {
          proxy.on("proxyReqWs", (proxyReq) => {
            if (operatorToken) {
              proxyReq.setHeader("x-operator-api-token", operatorToken);
            }
          });
        },
      },
    },
  },
});
