# Phantomat browser E2E (Playwright)

Cross-browser checks against the static **`examples-web`** preview (`vite preview` on **:4173**): scatter PNG baselines, WebGPU/WebGL detection, and performance budgets.

## Setup (repo root)

1. Build WASM and the demo bundle:

   ```bash
   wasm-pack build --target web --release crates/phantomat-wasm
   pnpm install
   pnpm --filter @phantomat/core build
   pnpm --filter examples-web build
   ```

2. Install browsers **without sudo** (user cache, default Playwright behavior):

   ```bash
   pnpm --filter e2e exec playwright install chromium
   ```

   For the full matrix (matches CI):

   ```bash
   pnpm --filter e2e exec playwright install chromium firefox webkit
   ```

## Run tests

From the repo root:

```bash
pnpm --filter e2e test
```

Single browser project:

```bash
pnpm --filter e2e exec playwright test --project chromium-webgpu
```

### PNG baselines (`scatter_renders`)

Baselines live under `tests/screenshots/<project-name>/scatter_100.png`.  
**Do not** rely on Playwright’s `--update-snapshots` for these pixel tests — this package uses **pixelmatch** and explicit files.

**Generate or refresh baselines locally** (review diffs before committing):

```bash
UPDATE_BASELINES=1 pnpm --filter e2e exec playwright test tests/scatter_renders.spec.ts
```

Or one project at a time:

```bash
UPDATE_BASELINES=1 pnpm --filter e2e exec playwright test tests/scatter_renders.spec.ts --project chromium-webgpu
```

On platforms/browsers you cannot run (e.g. WebKit on Linux), CI on **macOS** can produce first-time baselines; download artifacts or re-run there and copy PNGs into `tests/screenshots/<project>/`.

### Performance budgets

Targets are in `budgets.json` (`ttfr_1m_pts`, `frame_time_1m_pts`, `js_heap_mb_1m_pts`, wasm gzip).

**GPU budgets (TTFR, steady-state median frame time, JS heap) run only on `chromium-webgpu` and `chromium-webgl`.**  
Firefox Playwright projects set `metadata.perfBudgetsApply: false` because WebGPU/WebGL on GitHub-hosted runners is often software-throttled; comparing Firefox frame times to Chromium would mix unlike-for-unlike. Firefox still runs **visual** (`scatter_renders`) and **fallback** (`webgl_fallback`) tests.

Steady-state frame time is measured in Playwright after **10 warmup frames** and uses the **median** of **60** frame samples (no CI shortcut — the demo loads with `frames=0` so the page does not pre-average frames).

Adapter strings for debugging come from `window.phantomatStats.adapter` (WASM `getAdapterInfo()` plus `navigator.gpu.requestAdapterInfo()` when available). `tests/diagnostics.spec.ts` writes `test-results/diagnostics/<project>.json` when you run the full suite.

Tune budgets using local runs (`pnpm --filter e2e test`) and the diagnostics artifacts; avoid raising caps only to silence failures.

### Chromium / WebGPU (headless)

If adapter selection hangs or WebGPU is unavailable, try **`--project chromium-webgl`** or **`firefox-webgl`** first. The `chromium-webgpu` project adds `--enable-unsafe-webgpu`; full GPU validation may still require a headed Chrome / real GPU.

### Faster smoke

```bash
pnpm --filter e2e run test:smoke
```

Runs only screenshot + WebGL fallback specs (no perf suite).

## Reports

CI uploads `playwright-report/` as an artifact on failure. That directory is gitignored.
