#!/usr/bin/env bash
# Build @phantomat/jupyter ESM and wasm into python/phantomat/static/ (required before maturin).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [[ ! -f "crates/phantomat-wasm/pkg/phantomat_wasm_bg.wasm" ]]; then
  (cd crates/phantomat-wasm && wasm-pack build --target web --release)
fi
npx --yes pnpm@9.15.4 install
npx --yes pnpm@9.15.4 --filter @phantomat/core run build
npx --yes pnpm@9.15.4 --filter @phantomat/jupyter run build
echo "OK: python/phantomat/static/widget.js and phantomat_wasm_bg.wasm"
