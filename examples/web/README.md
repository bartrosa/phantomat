# Phantomat — examples web (Vite)

Interactive scatter demo used by **Playwright e2e** (`packages/e2e`) with `vite preview` on port **4173**.

## Build & preview

From the **repository root** (after `pnpm install` and a wasm build):

```bash
wasm-pack build --target web --release crates/phantomat-wasm
pnpm --filter @phantomat/core build
pnpm --filter examples-web build
pnpm --filter examples-web preview
```

Open `http://127.0.0.1:4173` (strict port).

## URL parameters

| Query | Meaning |
| --- | --- |
| `scenario` | `scatter_100`, `scatter_10k` (default), `scatter_1m` |
| `seed` | PRNG seed (unsigned int) |
| `backend` | `auto` (default), `webgpu`, or `webgl` — passed to `Scene.newWithBackend` |
| `frames` | For `scatter_1m`, number of extra `render()` samples for frame-time mean (default 60) |

After the first frame (and optional multi-frame timing for 1M points), the page sets:

`window.phantomatStats = { ttfr, frameTime, jsHeap?, backend, scenario }`.

## Dev server

```bash
pnpm --filter examples-web dev
```

Uses port **5173** by default (`vite.config.ts`).

## Legacy note

The old `examples/web/pkg/` path is gitignored; WASM now comes from `crates/phantomat-wasm/pkg/` via `@phantomat/core` → `phantomat-wasm`.
