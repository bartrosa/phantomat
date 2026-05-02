# Phantomat

[![CI](https://github.com/bartrosa/phantomat/actions/workflows/ci.yml/badge.svg)](https://github.com/bartrosa/phantomat/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

GPU-oriented visualization experiments for linked exploration in 2D/3D—**Rust** numerics and rendering ([wgpu](https://wgpu.rs/)), **Python** bindings and Jupyter widgets ([PyO3](https://pyo3.rs/) / [maturin](https://www.maturin.rs/)), and **TypeScript** packages that bundle to WASM for the browser.

## Repository layout

| Path | Role |
|------|------|
| `crates/phantomat-core` | Core types, color scales, reference math |
| `crates/phantomat-renderer` / `crates/phantomat-layers` | GPU path; heavy tests are opt-in via `make test-rust-gpu` |
| `crates/phantomat-wasm` | WASM build (`wasm-pack`); consumed by JS packages |
| `crates/phantomat-python` | PyO3 extension (`_native`); built with **maturin**, not plain `cargo build --release` for the full workspace |
| `xtask` | Maintainer utilities (e.g. golden image updates) |
| `python/` | `phantomat` PyPI package, static assets for the widget, Python tests |
| `packages/core`, `packages/jupyter` | npm packages `@phantomat/core`, `@phantomat/jupyter` (pnpm workspace) |
| `packages/e2e` | Playwright tests and perf budgets |
| `examples/web` | Vite demo app used by e2e and `make demo-web` |

## Prerequisites

- **Rust 1.88** — pinned in [`rust-toolchain.toml`](rust-toolchain.toml); install via [rustup](https://rustup.rs/). The workspace expects the **`wasm32-unknown-unknown`** target (`rustup target add wasm32-unknown-unknown`).
- **Node.js** and **pnpm 9** — version pinned in [`package.json`](package.json) (`packageManager`: `pnpm@9.x`).
- **Python 3.10+** — for the editable Python package, pytest, and maturin (`python/`).

Optional but useful: **wasm-pack** (see `make setup-cargo-tools`), **cargo-nextest** (faster `make test-rust`).

## Getting started (full stack)

After cloning, use the root **`Makefile`** as the single entry point for setup and checks:

```bash
git clone https://github.com/bartrosa/phantomat.git
cd phantomat

make setup          # rust toolchain bits, pnpm install, .venv, editable python[dev]
make build-python   # widget bundle + maturin develop (needed before pytest)
make test           # fast: Rust (no renderer/layers GPU crates) + pytest + @phantomat/core CI parity
make demo-web       # http://localhost:4173 — Vite preview of examples/web (after build)
```

Discover everything else with:

```bash
make help
```

## Development

The Makefile is POSIX-oriented (`bash`, no recursive `make -j` inside recipes); use `make -j4 …` yourself if you want parallelism.

Common targets:

- **Format / lint:** `make fmt`, `make fmt-check`, `make lint` (Rust + Prettier + Ruff + TS typecheck).
- **Build:** `make build` — Rust release (PyO3 crate **excluded**; use `make build-python` for `_native`), wasm-pack, `@phantomat/*`, and `examples/web` dist.
- **Tests:** `make test` (fast); `make test-rust-gpu` for renderer/layers goldens; `make test-e2e` for Playwright (install browsers per [`packages/e2e/README.md`](packages/e2e/README.md)).
- **Release prep (local only — nothing is published):** `make release-check`, `make release-dry-run`. Tag-triggered publishing is described in **[RELEASING.md](RELEASING.md)**.

### Rust-only smoke (no Python)

You can run subset builds/tests without the Python env, for example:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p phantomat-core
```

Workspace **`cargo build --release`** for every crate is split: the **`phantomat-python`** crate is excluded from `make build-rust` because it must be built like a Python extension (see **`make build-python`** / maturin).

### Windows

GNU **`make`** is not installed by default. Use **WSL**, **Git Bash**, install **`make`** (e.g. `choco install make`), or mirror the recipes from the **`Makefile`**. GitHub Actions installs **`make`** on `windows-latest` where jobs call it.

## Status

Early development; see **[CHANGELOG.md](CHANGELOG.md)** for notable changes.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
