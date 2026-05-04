# E2E / WebGPU–WebGL diagnostics — known issues and future work

This note captures problems we hit while hardening cross-browser Playwright E2E, the **wgpu** WASM path, and the `examples/web` demo. It is a **backlog for future PRs**, not a status of shipped code.

---

## 1. Chrome flags are an unreliable way to force WebGL-only

**Problem:** Using `--disable-features=WebGPU` on the `chromium-webgl` project did not guarantee that the app used a WebGL backend. `navigator.gpu` could still be present, and E2E could report `backend: "webgpu"` for a project that was meant to test **WebGL2 / wgpu GL** only. That made WebGL baselines and perf budgets misleading (testing the same path as `chromium-webgpu`).

**Direction taken:** Force the render path in app code via the URL: `?backend=webgl` or `?backend=webgpu`, and add a Playwright helper (`urlForProject`) so every test passes the right query for its project. Chrome launch flags are no longer the primary lever for backend selection.

### TODO (future)

- [ ] **CI verification:** In GitHub Actions (macOS), assert from `test-results/diagnostics/chromium-webgl.json` that `phantomatStats.backend` is `webgl` and `adapter.wgpu` / wgpu info reflects a GL backend (e.g. `webgl2`), not `browser-webgpu`. Automate a one-line check in the workflow or a small follow-up test.
- [ ] **Document** in `packages/e2e/README.md` that `chromium-webgl` depends on `backend=webgl` in the URL, not on `--disable-features=WebGPU`.

---

## 2. Probing WebGL on the same canvas wgpu owns can hang or deadlock

**Problem:** Calling `canvas.getContext("webgl2")` on **`#c`** after **wgpu** has created its surface / GL context on that canvas caused **long hangs** (minute-scale) on some runs—especially when combining extra `navigator.gpu.requestAdapter()` probes with a forced GL path.

**Direction taken:**

- Use a **small offscreen canvas** (e.g. 1×1) only for `WEBGL_debug_renderer_info` strings when we need UNMASKED vendor/renderer text.
- When `?backend=webgl`, **skip** the extra `navigator.gpu.requestAdapter()` “rich adapter” probe before WASM init to avoid driver-specific stalls.

### TODO (future)

- [ ] Optional: centralize “safe GL probe” helpers in one module and unit-test the **never touch `#c` twice for probing`** rule.
- [ ] If we ever need GPU-accurate strings tied to the **same** GL context wgpu uses, investigate backend-specific hooks (may not exist on the web stack).

---

## 3. `wgpu::AdapterInfo` is often empty on WASM; JS `requestAdapterInfo` may still be empty in headless Chrome

**Problem:** WASM `getAdapterInfo()` mirrors **wgpu’s** Web stub, which frequently returns empty name/driver fields. A separate JS probe (`richAdapter`) fills gaps using `navigator.gpu.requestAdapter()` + `requestAdapterInfo()`, but **vendor/architecture/device/description** can still be empty strings in **headless** Chrome while **limits** (e.g. max buffer sizes) are populated.

### TODO (future)

- [ ] Consider gated **`GPUAdapter.requestAdapterInfo()` unmask prompt** / documented Chrome flags only where CI allows (often impractical on hosted runners).
- [ ] Track **wgpu / WebGPU** upstream improvements for richer adapter info on the web backend.

---

## 4. Performance budgets: measurement backlog

**Problem:** We want budgets grounded in **median of repeated runs** (and heap stability over many frames), not “safe” large numbers. `_budgets_basis` / `_heap_basis` in `packages/e2e/budgets.json` were left as **PENDING** until:

- three perf runs per chromium GPU project capture `[METRIC]` lines, and  
- `diagnostics_heap.spec.ts` completes for **heap1** and growth % after 1000 frames.

### TODO (future)

- [ ] Run **3×** `pnpm --filter e2e test tests/perf_budgets.spec.ts` per project (`chromium-webgpu`, `chromium-webgl`), compute median × **1.2** margin for TTFR and steady-state frame time, and record in `_budgets_basis`.
- [ ] Run `diagnostics_heap.spec.ts`, confirm growth \< 10%, then set **heap** budgets from **heap1 × 1.1** (not from inflated caps).
- [ ] If frame median variability \> ~15% across runs, revisit warmup frame count (e.g. 10 → 20) or sample size (60 → 120).

---

## 5. Firefox vs Chromium perf comparison

**Problem:** Firefox WebGPU/WebGL on typical CI runners is often **not comparable** to Chromium (software vs hardware paths, different scheduling).

**Direction taken:** Firefox Playwright projects use `metadata.perfBudgetsApply: false`; TTFR / frame / heap **budget tests skip** on Firefox while visual and fallback tests still run.

### TODO (future)

- [ ] Optional: document **when** (if ever) to re-enable Firefox perf budgets (dedicated GPU runners, pinned Firefox + Mesa versions, etc.).

---

## References

- `packages/e2e/tests/_url.ts` — URL helper for `backend=`  
- `examples/web/src/main.ts` — `preInit`, `richAdapter`, offscreen GL probe, optional leak WeakRefs  
- `packages/e2e/README.md` — Firefox perf skip and budget philosophy  
