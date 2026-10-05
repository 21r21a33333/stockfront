import { defineConfig } from "@playwright/test";

const baseURL = process.env.STOCKFRONT_FE_URL ?? "http://127.0.0.1:5174";

export default defineConfig({
  testDir: "./e2e",
  workers: 1,
  webServer: process.env.STOCKFRONT_FE_URL
    ? undefined
    : {
        command:
          "VITE_API_BASE_URL=/test-api pnpm dev --host 127.0.0.1 --port 5174 --strictPort",
        url: baseURL,
        reuseExistingServer: false,
      },
  use: {
    baseURL,
    channel: "chrome",
    viewport: { width: 1440, height: 920 },
    trace: "retain-on-failure",
  },
});
