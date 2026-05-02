#!/usr/bin/env bash
# Smoke-check that a version exists on PyPI (and optionally npm). Does not install into your env long-term.
set -euo pipefail

VERSION="${1:-}"
if [[ -z "${VERSION}" ]]; then
  echo "Usage: bash scripts/verify-release.sh <semver>"
  echo "Example: bash scripts/verify-release.sh 0.1.0"
  exit 1
fi

echo "==> PyPI: phantomat==${VERSION}"
code="$(curl -s -o /tmp/pypi-phantomat.json -w "%{http_code}" "https://pypi.org/pypi/phantomat/${VERSION}/json")"
if [[ "${code}" != "200" ]]; then
  echo "✗ PyPI returned HTTP ${code} for phantomat ${VERSION}"
  exit 1
fi
python3 - <<'PY'
import json, sys
with open("/tmp/pypi-phantomat.json") as f:
    j = json.load(f)
print("✓ PyPI:", j["info"]["summary"][:80], "...")
PY

if [[ "${CHECK_NPM:-}" == "1" ]] && [[ -n "${NPM_PACKAGE:-}" ]]; then
  echo "==> npm: ${NPM_PACKAGE}@${VERSION}"
  npm view "${NPM_PACKAGE}@${VERSION}" version
fi

echo "✓ verify-release basic checks passed."
