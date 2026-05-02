# `phantomat-renderer`

Headless [**wgpu**](https://wgpu.rs/) rendering for Phantomat: draw minimal scenes offscreen, read back RGBA, and export **PNG** bytes.

## Layout

- `HeadlessRenderer::new(w, h)` — picks an adapter (**high performance**, then **low power**).
- `Scene` — `Clear` (solid fill) or `Triangle` (NDC triangle + uniform color).
- Golden tests live in `tests/golden.rs`; baseline PNGs are per **GPU stack** under `tests/golden/<backend>/`.

## Adding a golden test

1. Add a `#[test]` in `tests/golden.rs` that builds a `Scene`, calls `render_png`, and ends with `assert_image_matches(&png, &golden_path("my_case"), Tolerance::Dssim(…))`.
2. Prefer **`Tolerance::Dssim(max)`** (or **`Ssim`**) — do **not** expect pixel-identical output across backends or drivers.
3. On **your** OS, regenerate the PNG once (see below), then commit the file under the correct `tests/golden/<backend>/` folder.

### Backend folder names

| Subdirectory       | Typical platform / stack                          |
|--------------------|---------------------------------------------------|
| `linux-lavapipe`   | Linux + Mesa Lavapipe (CI uses `VK_ICD_FILENAMES`) |
| `macos-metal`      | macOS + Metal                                      |
| `windows-warp`     | Windows + WARP / DX12 (`--features dx12` on CI)   |

Override with **`PHANTOMAT_TEST_BACKEND`** (`linux-lavapipe`, `macos-metal`, `windows-warp`) if needed.

## Regenerating goldens (`xtask`)

From the workspace root:

```bash
cargo run -p xtask -- update-goldens --reason "why these PNGs changed"
```

This runs `cargo test -p phantomat-renderer --release --test golden` with **`PHANTOMAT_UPDATE_GOLDENS=1`**, overwriting PNGs for the **current** machine’s backend only, and appends an entry to `tests/golden/UPDATE_LOG.md`.

**Only regenerate on the platform that owns that subdirectory** — other backends get their baselines when CI (or a matching machine) runs the same command.

## MSRV / dependencies

- **`wgpu = "23"`** — pinned major; see code comments if upgrading.
- **`image`** is pinned to **`=0.25.5`** so the workspace MSRV (**1.85**) stays compatible (newer `image` releases raised their MSRV).

## Optional feature

- **`dx12`** — forwards `wgpu`’s DX12 backend (used in Windows CI).
