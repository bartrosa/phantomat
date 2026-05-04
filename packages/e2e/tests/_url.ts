import type { TestInfo } from "@playwright/test";

/**
 * Maps Playwright project name → `backend` query param so wgpu uses the intended path
 * (GL-only vs WebGPU-only vs auto). See `examples/web` URL params.
 */
export function urlForProject(
  testInfo: Pick<TestInfo, "project">,
  query: Record<string, string | number | undefined>,
): string {
  const name = testInfo.project.name;
  const backend = name.endsWith("-webgl")
    ? "webgl"
    : name.endsWith("-webgpu")
      ? "webgpu"
      : null;

  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(query)) {
    if (v !== undefined) p.set(k, String(v));
  }
  if (backend) p.set("backend", backend);
  return `/?${p.toString()}`;
}
