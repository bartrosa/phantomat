import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, devices } from "@playwright/test";

const isCI = Boolean(process.env.CI);
/** Monorepo root (contains `pnpm-lock.yaml`). */
const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);

export default defineConfig({
  testDir: "tests",
  fullyParallel: true,
  forbidOnly: isCI,
  retries: isCI ? 1 : 0,
  workers: isCI ? 1 : undefined,
  reporter: [["list"], ["html", { open: "never" }]],
  timeout: 300_000,
  expect: { timeout: 60_000 },
  use: {
    trace: "on-first-retry",
    baseURL: "http://localhost:4173",
  },
  projects: [
    {
      name: "chromium-webgpu",
      use: {
        ...devices["Desktop Chrome"],
        launchOptions: {
          args: ["--enable-unsafe-webgpu"],
        },
      },
    },
    {
      name: "chromium-webgl",
      use: {
        ...devices["Desktop Chrome"],
        launchOptions: {
          args: ["--disable-features=WebGPU"],
        },
      },
    },
    {
      name: "firefox-webgpu",
      use: { ...devices["Desktop Firefox"] },
    },
    {
      name: "firefox-webgl",
      use: {
        ...devices["Desktop Firefox"],
        // WebGL-only (no WebGPU) for fallback coverage
        firefoxUserPrefs: {
          "dom.webgpu.enabled": false,
        },
      },
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
    },
  ],
  webServer: {
    // Use the example app’s local Vite CLI (no global `pnpm` on PATH required).
    command: "npx vite preview --port 4173 --strictPort",
    cwd: path.join(repoRoot, "examples", "web"),
    url: "http://localhost:4173",
    reuseExistingServer: !isCI,
    timeout: 300_000,
  },
});
