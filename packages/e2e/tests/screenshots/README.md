# Visual baselines (`scatter_100.png`)

Pixel baselines are **per Playwright project** (browser profile):

```
tests/screenshots/<project-name>/scatter_100.png
```

Examples:

- `chromium-webgpu/scatter_100.png`
- `webkit/scatter_100.png`

Create or refresh them locally (review before committing):

```bash
UPDATE_BASELINES=1 pnpm --dir packages/e2e exec playwright test tests/scatter_renders.spec.ts --project chromium-webgpu
```

Repeat with `--project <name>` for each matrix entry you care about. Missing baselines cause the pixel test to fail until you add files here.
