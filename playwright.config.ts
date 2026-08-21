import { defineConfig, devices } from "@playwright/test";

const PORT = Number(process.env.PLAYWRIGHT_PORT ?? "3010");
const BASE_URL = `http://localhost:${PORT}`;

export default defineConfig({
  testDir: "./src/tests/e2e",
  timeout: 60_000,
  expect: { timeout: 10_000 },
  retries: process.env.CI ? 2 : 0,
  use: {
    baseURL: BASE_URL,
    extraHTTPHeaders: {
      "x-toolrakyat-e2e": "playwright",
    },
    trace: "retain-on-failure",
  },
  webServer: {
    command: `npm start -- --port ${PORT}`,
    url: BASE_URL,
    // Avoid accidentally reusing an unrelated app already running on the same port.
    reuseExistingServer: process.env.PLAYWRIGHT_REUSE_SERVER === "1",
    timeout: 120_000,
  },
  projects: [
    {
      name: "chromium",
      // Use the system-installed Google Chrome to avoid needing Playwright-managed
      // browser downloads in low-disk environments.
      use: { ...devices["Desktop Chrome"], channel: "chrome" },
    },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"] },
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
    },
  ],
});
