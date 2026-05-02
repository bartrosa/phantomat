# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo workspace (`resolver = "2"`) with `crates/*` and `xtask` members; shared `workspace.package` (version **0.0.1**, edition **2021**, **Apache-2.0**, `rust-version` **1.82**).
- Placeholder crate **`phantomat-core`** (sanity unit test).
- **`xtask`** binary crate for future automation (`publish = false`).
- **`rust-toolchain.toml`**: stable **1.82**, `rustfmt` + `clippy`, **`wasm32-unknown-unknown`** target.
- CI: separate **fmt**, **clippy** (with `--all-features`, `-D warnings`), and **test** jobs on **ubuntu-latest**, **macos-latest**, and **windows-latest** with **Swatinem/rust-cache@v2**; triggers on all **push** and **pull_request** branches.

### Removed

- Template-only additions outside the workspace scaffold: `docs/`, GitHub issue templates and PR template, Dependabot config, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `.githooks/`.
