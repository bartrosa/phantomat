.DEFAULT_GOAL := help
SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c

REPO_ROOT := $(CURDIR)
PYTHON ?= python3
VENV := $(REPO_ROOT)/.venv
VENVP := $(VENV)/bin
PNPM ?= pnpm

##@ Setup

setup: ## Install dev dependencies (Rust targets, pnpm, Python venv — run once per clone)
	@command -v rustup >/dev/null || (echo "Install rustup: https://rustup.rs"; exit 1)
	rustup target add wasm32-unknown-unknown
	rustup component add rustfmt clippy 2>/dev/null || true
	@command -v $(PNPM) >/dev/null || npm install -g pnpm
	$(PNPM) install --frozen-lockfile
	@if [ ! -d "$(VENV)" ]; then $(PYTHON) -m venv "$(VENV)"; fi
	"$(VENVP)/pip" install --upgrade pip maturin
	"$(VENVP)/pip" install -e "./python[dev]"
	@echo "✓ Setup complete. Activate: source .venv/bin/activate"

setup-cargo-tools: ## Install wasm-pack, cargo-nextest (optional), cargo-insta (optional)
	cargo install --locked wasm-pack
	-command -v cargo-nextest >/dev/null || cargo install --locked cargo-nextest
	-command -v cargo-insta >/dev/null || cargo install --locked cargo-insta

clean: ## Remove build artifacts (Cargo, JS dist, wasm pkg, Python caches)
	cargo clean || true
	rm -rf target/wheels target/dist-preview
	rm -rf packages/*/dist packages/*/node_modules examples/web/dist
	rm -rf python/phantomat/static/widget.js python/phantomat.egg-info python/build python/*.egg-info 2>/dev/null || true
	rm -rf crates/phantomat-wasm/pkg
	find . -type d -name '__pycache__' -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name '.pytest_cache' -exec rm -rf {} + 2>/dev/null || true

##@ Lint & Format

fmt: ## Format Rust + TS/JS (Prettier) + Python (Ruff)
	cargo fmt --all
	$(PNPM) run format
	@if [ -x "$(VENVP)/ruff" ]; then "$(VENVP)/ruff" format python/; else echo "Skipping ruff (run $(MAKE) setup)"; fi

fmt-check: ## Check formatting (CI-style — requires venv + pnpm install)
	cargo fmt --all -- --check
	$(PNPM) run format:check
	@if [ -x "$(VENVP)/ruff" ]; then "$(VENVP)/ruff" format --check python/; else echo "⚠ No .venv/ruff — $(MAKE) setup"; exit 1; fi

fmt-check-rust: ## cargo fmt --check only (no Python / TS — usable without venv)
	cargo fmt --all -- --check

lint: lint-rust lint-py lint-ts ## Run linters (Rust, Python, TS typecheck)

lint-rust: ## cargo clippy
	cargo clippy --workspace --all-targets --all-features -- -D warnings

lint-py: ## ruff check on python/
	@if [ ! -x "$(VENVP)/ruff" ]; then echo "Run $(MAKE) setup first"; exit 1; fi
	"$(VENVP)/ruff" check python/

lint-ts: ## TypeScript typecheck (@phantomat/core)
	$(PNPM) --filter @phantomat/core run typecheck

##@ Build

build: build-rust build-wasm build-ts build-examples-web ## Build Rust + wasm + TS + examples web dist

build-rust: ## cargo build workspace (release); PyO3 crate excluded — use build-python / maturin for _native
	cargo build --workspace --release --exclude phantomat-python

build-wasm: ## wasm-pack release build for browsers
	cd crates/phantomat-wasm && wasm-pack build --target web --release
	@du -h crates/phantomat-wasm/pkg/phantomat_wasm_bg.wasm | awk '{print "✓ WASM " $$0}'

build-ts: build-wasm ## Build @phantomat/* packages (requires wasm pkg)
	$(PNPM) --filter '@phantomat/*' build

build-examples-web: build-ts ## Static Vite build for examples/web
	$(PNPM) --filter examples-web build

build-widget: ## Jupyter anywidget bundle → python/phantomat/static/
	bash scripts/build_widget.sh

build-python: build-widget ## Editable Python install (maturin develop)
	@if [ ! -x "$(VENVP)/maturin" ]; then echo "Run $(MAKE) setup"; exit 1; fi
	cd python && "$(VENVP)/maturin" develop --release

##@ Test (fast)

test: test-rust test-python test-ts-ci ## Fast tests (no GPU PNG goldens, no E2E, no wasm-pack browser tests)

test-rust: ## Rust unit tests (excludes GPU-heavy renderer/layers crates)
	@if command -v cargo-nextest >/dev/null 2>&1; then \
		cargo nextest run --workspace --exclude phantomat-renderer --exclude phantomat-layers --no-fail-fast; \
	else \
		cargo test --workspace --exclude phantomat-renderer --exclude phantomat-layers; \
	fi

test-rust-gpu: ## Renderer + layers golden tests (GPU / Lavapipe / Metal / DX12)
	cargo test -p phantomat-renderer --release
	cargo test -p phantomat-layers --release

test-wasm: ## wasm-pack tests (Chrome + Firefox headless)
	cd crates/phantomat-wasm && wasm-pack test --headless --chrome
	cd crates/phantomat-wasm && wasm-pack test --headless --firefox

test-python: ## pytest python/tests (needs maturin develop for native extension)
	@if [ ! -x "$(VENVP)/pytest" ]; then echo "Run $(MAKE) setup && $(MAKE) build-python"; exit 1; fi
	"$(VENVP)/pytest" python/tests/ -v

test-ts-ci: ## Same checks as CI npm-test (@phantomat/core)
	$(PNPM) --filter @phantomat/core run build
	$(PNPM) --filter @phantomat/core test
	$(PNPM) --filter @phantomat/core run test:arrow
	$(PNPM) --filter @phantomat/core run typecheck
	$(PNPM) --filter @phantomat/core run size

test-ts: test-ts-ci ## Alias for CI-parity TS checks

test-notebook: ## nbval on examples/python (requires kernel + editable install)
	@if [ ! -x "$(VENVP)/pytest" ]; then echo "Run $(MAKE) setup"; exit 1; fi
	"$(VENVP)/pytest" --nbval-lax examples/python/

##@ Test (slow / full)

test-all: test test-rust-gpu test-wasm test-notebook test-e2e test-differential ## Full suite (slow)

test-e2e: ## Playwright cross-browser (builds examples/web)
	$(PNPM) --filter examples-web build
	$(PNPM) --filter e2e test

test-differential: ## Python ↔ TS parity + Arrow differential tests
	@if [ ! -x "$(VENVP)/pytest" ]; then echo "Run $(MAKE) setup"; exit 1; fi
	"$(VENVP)/pytest" tests/differential/ -v
	$(PNPM) --filter @phantomat/core run test:differential

##@ Benchmark

bench: ## criterion benchmarks
	cargo bench --workspace

bench-quick: ## Compile benchmarks only
	cargo bench --workspace --no-run

bench-perf-budgets: ## Playwright perf_budgets.spec.ts only
	$(PNPM) --filter examples-web build
	$(PNPM) --filter e2e exec playwright test tests/perf_budgets.spec.ts

##@ Golden images & fixtures

goldens-update: ## Regenerate Rust PNG goldens (requires REASON=...)
	@if [ -z "$(REASON)" ]; then echo "Usage: $(MAKE) goldens-update REASON='why'"; exit 1; fi
	cargo run -p xtask -- update-goldens --reason "$(REASON)"

screenshots-update: ## Regenerate Playwright pixel baselines (packages/e2e)
	UPDATE_BASELINES=1 $(PNPM) --dir packages/e2e exec playwright test tests/scatter_renders.spec.ts

fixtures-generate: ## Regenerate IPC / parity fixtures (Python)
	@if [ ! -x "$(VENVP)/python" ]; then echo "Run $(MAKE) setup"; exit 1; fi
	"$(VENVP)/python" scripts/generate_fixtures.py

##@ Demo & examples

demo-web: build-examples-web ## Vite preview for examples/web on http://localhost:4173
	cd examples/web && npx vite preview --port 4173 --strictPort

demo-jupyter: ## Start JupyterLab on quickstart notebook (manual)
	@echo "Run: source .venv/bin/activate && jupyter lab examples/python/quickstart.ipynb"

##@ Release (local validation only — publishing happens from GitHub Actions on tag)

release-check: ## Pre-tag checks (sync-versions, CHANGELOG, fast test suite). Use RELEASE_FULL=1 for test-all.
	@echo "==> sync-versions --check"
	node "$(REPO_ROOT)/scripts/sync-versions.mjs" --check
	@echo "==> CHANGELOG [Unreleased] sections"
	@grep -qE '^### (Added|Changed|Fixed|Removed)' CHANGELOG.md || (echo "✗ Add ### headings under ## [Unreleased] in CHANGELOG.md"; exit 1)
	@if [ -n "$(RELEASE_FULL)" ]; then echo "==> Full tests (slow)"; $(MAKE) test-all; else echo "==> Fast tests (make test)"; $(MAKE) test; fi
	@echo "✓ release-check OK. Tag manually when ready: git tag vX.Y.Z && git push origin vX.Y.Z"

release-dry-run: ## Build wheels + npm tarballs locally (nothing published)
	$(MAKE) build
	mkdir -p target/dist-preview/wheels
	@if [ ! -x "$(VENVP)/maturin" ]; then echo "Run $(MAKE) setup"; exit 1; fi
	cd python && "$(VENVP)/maturin" build --release --strip -o ../target/dist-preview/wheels
	mkdir -p target/dist-preview/npm
	cd packages/core && $(PNPM) pack --pack-destination "$(REPO_ROOT)/target/dist-preview/npm"
	cd packages/jupyter && $(PNPM) pack --pack-destination "$(REPO_ROOT)/target/dist-preview/npm"
	@echo "==> Artifacts:"
	@ls -lh target/dist-preview/wheels 2>/dev/null || true
	@ls -lh target/dist-preview/npm 2>/dev/null || true

release-verify: ## Verify release exists on PyPI (optional CHECK_NPM=1 NPM_PACKAGE=@scope/pkg)
	@if [ -z "$(VERSION)" ]; then echo "Usage: $(MAKE) release-verify VERSION=0.1.0"; exit 1; fi
	bash "$(REPO_ROOT)/scripts/verify-release.sh" "$(VERSION)"

##@ Help

help: ## Display Makefile targets
	@awk 'BEGIN {FS = ":.*##"; printf "\nPhantomat — development commands\n\nUsage: make \033[36m<target>\033[0m\n"} \
		/^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } \
		/^[a-zA-Z0-9_.-]+:.*?## / { printf "  \033[36m%-26s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

.PHONY: help setup setup-cargo-tools clean \
	fmt fmt-check fmt-check-rust lint lint-rust lint-py lint-ts \
	build build-rust build-wasm build-ts build-examples-web build-widget build-python \
	test test-rust test-rust-gpu test-wasm test-python test-ts test-ts-ci test-notebook \
	test-all test-e2e test-differential \
	bench bench-quick bench-perf-budgets \
	goldens-update screenshots-update fixtures-generate \
	demo-web demo-jupyter \
	release-check release-dry-run release-verify
