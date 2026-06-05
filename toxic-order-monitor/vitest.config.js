import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  cacheDir: "node_modules/.vitest",
  plugins: [react()],
  test: {
    environment: "jsdom",
  },
});
