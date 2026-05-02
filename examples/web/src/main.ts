import init, { Scene, ScatterLayer } from "@phantomat/core";

declare global {
  interface Window {
    phantomatStats?: {
      ttfr: number;
      frameTime: number;
      jsHeap?: number;
      backend: string;
      scenario: string;
    };
  }
}

function mulberry32(seed: number): () => number {
  return () => {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function parseParams(): {
  scenario: string;
  seed: number;
  backend: "webgpu" | "webgl" | "auto";
  measureFrames: number;
} {
  const p = new URLSearchParams(window.location.search);
  const scenario = p.get("scenario") ?? "scatter_10k";
  const seed = Number.parseInt(p.get("seed") ?? "1", 10) || 1;
  const b = (p.get("backend") ?? "auto").toLowerCase();
  const backend =
    b === "webgpu" || b === "webgl" ? b : ("auto" as const);
  const measureFrames = Math.min(
    120,
    Math.max(0, Number.parseInt(p.get("frames") ?? "60", 10) || 60),
  );
  return { scenario, seed, backend, measureFrames };
}

function scenarioPointCount(name: string): number {
  switch (name) {
    case "scatter_100":
      return 100;
    case "scatter_1m":
      return 1_000_000;
    case "scatter_10k":
    default:
      return 10_000;
  }
}

function fillScatter(
  n: number,
  rand: () => number,
): {
  positions: Float32Array;
  colors: Float32Array;
  sizes: Float32Array;
} {
  const positions = new Float32Array(n * 2);
  const colors = new Float32Array(n * 4);
  const sizes = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const t = i / Math.max(1, n - 1);
    positions[i * 2] = (rand() * 2 - 1) * 0.95;
    positions[i * 2 + 1] = (rand() * 2 - 1) * 0.95;
    colors[i * 4] = 0.2 + 0.8 * t;
    colors[i * 4 + 1] = 0.3 + 0.5 * Math.sin(t * 6.28);
    colors[i * 4 + 2] = 0.6;
    colors[i * 4 + 3] = 1.0;
    sizes[i] = 3 + rand() * 12;
  }
  return { positions, colors, sizes };
}

async function main(): Promise<void> {
  const canvas = document.getElementById("c");
  if (!(canvas instanceof HTMLCanvasElement)) {
    throw new Error("missing #c canvas");
  }
  const params = parseParams();
  try {
    await runDemo(canvas, params);
  } catch (err) {
    console.error(err);
    window.phantomatStats = {
      ttfr: -1,
      frameTime: -1,
      backend: "error",
      scenario: params.scenario,
    };
    throw err;
  }
}

async function runDemo(
  canvas: HTMLCanvasElement,
  params: ReturnType<typeof parseParams>,
): Promise<void> {
  const { scenario, seed, backend, measureFrames } = params;
  const n = scenarioPointCount(scenario);
  const rand = mulberry32(seed >>> 0);

  await init();

  const pref =
    backend === "auto" ? "auto" : backend === "webgpu" ? "webgpu" : "webgl";

  const { positions, colors, sizes } = fillScatter(n, rand);

  // TTFR: GPU adapter/device + layer upload + first present (excludes JS data generation).
  const tStart = performance.now();
  const scene = await Scene.newWithBackend(canvas, pref);

  const layer = new ScatterLayer(positions, colors, sizes);
  scene.add_layer(layer);

  await scene.render();
  const ttfr = performance.now() - tStart;

  let frameTime = ttfr;
  if (scenario === "scatter_1m" && measureFrames > 0) {
    let sum = 0;
    for (let i = 0; i < measureFrames; i++) {
      const t0 = performance.now();
      await scene.render();
      sum += performance.now() - t0;
    }
    frameTime = sum / measureFrames;
  }

  const mem = (performance as unknown as { memory?: { usedJSHeapSize: number } })
    .memory;
  const jsHeap = mem ? mem.usedJSHeapSize / (1024 * 1024) : undefined;

  window.phantomatStats = {
    ttfr,
    frameTime,
    jsHeap,
    backend: scene.backend,
    scenario,
  };
}

main().catch((err) => {
  const p = document.createElement("pre");
  p.style.color = "#f66";
  p.textContent = String(err);
  document.body.appendChild(p);
});
