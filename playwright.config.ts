import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30000,
  retries: 1,
  use: {
    baseURL: "http://localhost:1421",
    viewport: { width: 1400, height: 900 },
    actionTimeout: 10000,
    screenshot: "only-on-failure",
  },
  webServer: {
    command: "npx vite --config vite.config.e2e.ts",
    port: 1421,
    reuseExistingServer: false,
    timeout: 30000,
  },
});
