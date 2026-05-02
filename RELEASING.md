# Releasing Phantomat

Publishing is **fully automated from Git tags** (`.github/workflows/release.yml`). Nothing is uploaded to PyPI or npm from maintainers’ laptops by default.

## What runs automatically

When you push a SemVer tag matching `vX.Y.Z`:

1. **validate** — `node scripts/sync-versions.mjs --check` (Cargo / Python / npm versions must match) and a CHANGELOG sanity grep.
2. **build-python** — Builds manylinux wheels with **maturin** and publishes to **PyPI** via **Trusted Publishing (OIDC)** (recommended) or an API token stored as a secret if you must fall back.
3. **publish-npm** — Builds `@phantomat/core` after wasm-pack, then **`npm publish`** using **`NPM_TOKEN`** from GitHub Actions secrets.

## Manual checklist (maintainer)

Complete these **once per ecosystem** before the first release:

### PyPI

- [ ] Register the project on [pypi.org](https://pypi.org) (if new).
- [ ] Configure **Trusted Publisher** for this GitHub repo + workflow + environment (`pypi`), per [PyPI docs](https://docs.pypi.org/trusted-publishers/).
- [ ] Add GitHub **Environment** named `pypi` (Settings → Environments) if you use deployment protection rules.
- [ ] Confirm `PyO3/maturin-action` + `pypa/gh-action-pypi-publish` succeed on a **test tag** or dry-run wheel upload.

### npm

- [ ] Ensure `@phantomat/core` name and `access: public` match your org policy.
- [ ] Create an **automation** granular token (classic tokens are discouraged) or configure **OIDC** if/when npm supports your setup.
- [ ] Add repository secret **`NPM_TOKEN`** (Settings → Secrets → Actions).
- [ ] Add GitHub **Environment** named `npm` if you gate publishes.

### Repository

- [ ] Align versions: `node scripts/sync-versions.mjs --check`.
- [ ] Update **`CHANGELOG.md`** (`## [Unreleased]` → move sections under `## [X.Y.Z]` when tagging).
- [ ] Tag and push (you run locally):

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Post-release smoke test

After artifacts appear on PyPI:

```bash
make release-verify VERSION=0.1.0
```

(Optional npm checks — extend `scripts/verify-release.sh` as needed.)

## What we deliberately do **not** automate here

- **release-please** / changelog automation (optional follow-up).
- **Publishing from CI without tags** — workflow triggers **only** on version tags.
- **Staging PyPI** — add a separate workflow/environment if you need **test.pypi.org**.
