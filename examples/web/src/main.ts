import init, { Scene, ScatterLayer } from "@phantomat/core";

const phantomatPreInit = {
  hasNavigatorGpu: "gpu" in navigator,
  hasWebGL2: !!document.createElement("canvas").getContext("webgl2"),
  userAgent: navigator.userAgent,
  chromeFlags: "unknown",
};
console.log("[phantomat:diag-pre-init]", JSON.stringify(phantomatPreInit, null, 2));
(window as unknown as { phantomatStats?: Record<string, unknown> }).phantomatStats = {
  ...(window.phantomatStats as Record<string, unknown> | undefined),
  preInit: phantomatPreInit,
};

/** WASM [`Scene`] bindings not yet in `@phantomat/core` typings until rebuild. */
type SceneWasm = Scene & {
  getAdapterInfo(): Record<string, unknown>;
  renderWithStats(): Promise<{
    encodeMs: number;
    submitMs: number;
    presentMs: number;
  }>;
};

declare global {
  interface Window {
    phantomatScene?: Scene;
    phantomatStats?: {
      ttfr: number;
      frameTime: number;
      jsHeap?: number;
      backend: string;
      scenario: string;
      adapter?: PhantomatAdapterStats;
      frameBreakdown?: FrameBreakdownAvg;
      preInit?: Record<string, unknown>;
      richAdapter?: Record<string, unknown>;
    };
    /** `?diagnose=arrays` — WeakRefs for E2E / leak experiments */
    __phantomatArrayWeakRefs?: {
      positions?: WeakRef<Float32Array>;
      colors?: WeakRef<Float32Array>;
      sizes?: WeakRef<Float32Array>;
    };
  }
}

interface PhantomatAdapterStats {
  backend: string;
  /** Raw [`wgpu::Adapter::get_info`] JSON from WASM (often empty on browser WebGPU). */
  wgpu: Record<string, unknown>;
  vendor?: string;
  architecture?: string;
  /** Human-readable; WebGPU `requestAdapterInfo` or WebGL unmasked renderer. */
  description: string;
  isFallback: boolean;
  webglVendor?: string;
  webglRenderer?: string;
}

interface FrameBreakdownAvg {
  encodeMs: number;
  submitMs: number;
  presentMs: number;
  samples: number;
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
  diagnoseArrays: boolean;
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
  const diagnoseArrays = p.get("diagnose") === "arrays";
  return { scenario, seed, backend, measureFrames, diagnoseArrays };
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

function probeWebGlRenderer(canvas: HTMLCanvasElement): {
  webglVendor?: string;
  webglRenderer?: string;
} {
  const gl =
    canvas.getContext("webgl2") ?? (canvas.getContext("webgl") as WebGLRenderingContext | null);
  if (!gl) return {};
  const ext = gl.getExtension("WEBGL_debug_renderer_info");
  if (!ext) return {};
  const UNMASKED_VENDOR_WEBGL = 0x9245;
  const UNMASKED_RENDERER_WEBGL = 0x9246;
  return {
    webglVendor: gl.getParameter(UNMASKED_VENDOR_WEBGL),
    webglRenderer: gl.getParameter(UNMASKED_RENDERER_WEBGL),
  };
}

/** Native WebGPU adapter probe **before** WASM init (separate `requestAdapter` from wgpu). */
async function getRichAdapterInfo(): Promise<Record<string, unknown>> {
  if (!("gpu" in navigator)) {
    return { backend: "no-webgpu" };
  }
  type AdapterLike = {
    requestAdapterInfo?: () => Promise<Record<string, unknown>>;
    limits?: { maxStorageBufferBindingSize?: number; maxBufferSize?: number };
    isFallbackAdapter?: boolean;
  };
  type GpuNav = {
    requestAdapter: (o: object) => Promise<AdapterLike | null>;
  };
  const gpu = (navigator as unknown as { gpu: GpuNav }).gpu;
  try {
    const adapter = await gpu.requestAdapter({
      powerPreference: "high-performance",
    });
    if (!adapter) return { backend: "no-adapter" };

    const a = adapter as AdapterLike;

    let info: Record<string, unknown> = {};
    try {
      if (typeof adapter.requestAdapterInfo === "function") {
        info = (await adapter.requestAdapterInfo()) as Record<string, unknown>;
      }
    } catch {
      info = {};
    }

    const vendor = typeof info.vendor === "string" ? info.vendor : "";
    const architecture = typeof info.architecture === "string" ? info.architecture : "";
    const device = typeof info.device === "string" ? info.device : "";
    const description = typeof info.description === "string" ? info.description : "";

    const lim = a.limits;
    return {
      backend: "webgpu",
      source: "navigator.gpu.requestAdapter + requestAdapterInfo",
      vendor: vendor || "(empty)",
      architecture: architecture || "(empty)",
      device: device || "(empty)",
      description: description || "(empty)",
      maxStorageBufferBindingSize: lim?.maxStorageBufferBindingSize,
      maxBufferSize: lim?.maxBufferSize,
      isFallbackAdapter: Boolean(a.isFallbackAdapter),
    };
  } catch (e) {
    return { backend: "error", message: String(e) };
  }
}

async function probeNavigatorGpuAdapter(): Promise<{
  vendor?: string;
  architecture?: string;
  description?: string;
  isFallback?: boolean;
} | null> {
  type NavGpu = {
    requestAdapter: (o: object) => Promise<null | { requestAdapterInfo?: () => Promise<{ vendor?: string; architecture?: string }> }>;
  };
  const gpu = (navigator as unknown as { gpu?: NavGpu }).gpu;
  if (!gpu) return null;
  try {
    const adapter = await gpu.requestAdapter({
      powerPreference: "high-performance",
    });
    if (!adapter) {
      return { isFallback: true, description: "navigator.gpu: no adapter" };
    }
    const raw = adapter;
    const info =
      typeof raw.requestAdapterInfo === "function"
        ? await raw.requestAdapterInfo()
        : undefined;
    const vendor = info?.vendor ?? "";
    const architecture = info?.architecture ?? "";
    const blob = `${vendor} ${architecture}`.toLowerCase();
    const isFallback = /llvmpipe|swiftshader|software|swrast|lavapipe|microsoft basic render/i.test(
      blob,
    );
    return {
      vendor: vendor || undefined,
      architecture: architecture || undefined,
      description:
        [vendor, architecture].filter(Boolean).join(" ").trim() || "webgpu adapter",
      isFallback,
    };
  } catch {
    return null;
  }
}

async function buildAdapterStats(
  scene: SceneWasm,
  canvas: HTMLCanvasElement,
): Promise<PhantomatAdapterStats> {
  const wgpuRaw = scene.getAdapterInfo();
  const webgpuProbe =
    scene.backend === "webgpu" ? await probeNavigatorGpuAdapter() : null;
  // Never call getContext on `#c` after wgpu owns it — use a 1×1 offscreen canvas for UNMASKED_* strings.
  const glProbe =
    scene.backend === "webgl" || !(navigator as unknown as { gpu?: unknown }).gpu
      ? (() => {
          const c = document.createElement("canvas");
          c.width = 1;
          c.height = 1;
          return probeWebGlRenderer(c);
        })()
      : {};

  const wgpuFallback =
    typeof wgpuRaw.isFallback === "boolean" ? wgpuRaw.isFallback : false;
  const navFallback = webgpuProbe?.isFallback ?? false;
  const descriptionParts: string[] = [];
  if (webgpuProbe?.description) descriptionParts.push(webgpuProbe.description);
  if (glProbe.webglRenderer) descriptionParts.push(glProbe.webglRenderer);
  if (typeof wgpuRaw.name === "string" && (wgpuRaw.name as string).length > 0) {
    descriptionParts.push(wgpuRaw.name as string);
  }
  const description =
    descriptionParts.filter(Boolean).join(" · ") || (scene.backend as string);

  return {
    backend: scene.backend,
    wgpu: wgpuRaw,
    vendor: webgpuProbe?.vendor,
    architecture: webgpuProbe?.architecture,
    description,
    isFallback: wgpuFallback || navFallback,
    webglVendor: glProbe.webglVendor,
    webglRenderer: glProbe.webglRenderer,
  };
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
      ...window.phantomatStats,
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
  const { scenario, seed, backend, measureFrames, diagnoseArrays } = params;
  const n = scenarioPointCount(scenario);
  const rand = mulberry32(seed >>> 0);

  const richAdapter =
    backend === "webgl"
      ? {
          backend: "not-probed",
          reason:
            "URL backend=webgl forces GL-only init; skipping navigator.gpu.requestAdapter to avoid driver hangs when benchmarking WebGL",
        }
      : await getRichAdapterInfo();

  await init();

  const pref =
    backend === "auto" ? "auto" : backend === "webgpu" ? "webgpu" : "webgl";

  const filled = fillScatter(n, rand);
  let { positions, colors, sizes } = filled;

  const tStart = performance.now();
  const scene = (await Scene.newWithBackend(
    canvas,
    pref,
  )) as SceneWasm;
  window.phantomatScene = scene;

  const layer = new ScatterLayer(positions, colors, sizes);

  if (diagnoseArrays) {
    window.__phantomatArrayWeakRefs = {
      positions: new WeakRef(positions),
      colors: new WeakRef(colors),
      sizes: new WeakRef(sizes),
    };
  }

  positions = null as unknown as Float32Array;
  colors = null as unknown as Float32Array;
  sizes = null as unknown as Float32Array;

  scene.add_layer(layer);

  await scene.render();
  if (diagnoseArrays && window.__phantomatArrayWeakRefs?.positions) {
    if (window.__phantomatArrayWeakRefs.positions.deref() !== undefined) {
      console.error(
        "[LEAK] positions Float32Array still alive after null-out + first render",
      );
    }
  }
  const ttfr = performance.now() - tStart;

  let frameTime = ttfr;
  let frameBreakdown: FrameBreakdownAvg | undefined;

  if (scenario === "scatter_1m" && measureFrames > 0) {
    let sum = 0;
    let enc = 0;
    let sub = 0;
    let pres = 0;
    const breakdownSamples = Math.min(30, measureFrames);
    let br = 0;
    for (let i = 0; i < measureFrames; i++) {
      const t0 = performance.now();
      if (i >= measureFrames - breakdownSamples) {
        const s = (await scene.renderWithStats()) as unknown as {
          encodeMs: number;
          submitMs: number;
          presentMs: number;
        };
        sum += performance.now() - t0;
        enc += s.encodeMs;
        sub += s.submitMs;
        pres += s.presentMs;
        br += 1;
      } else {
        await scene.render();
        sum += performance.now() - t0;
      }
    }
    frameTime = sum / measureFrames;
    if (br > 0) {
      frameBreakdown = {
        encodeMs: enc / br,
        submitMs: sub / br,
        presentMs: pres / br,
        samples: br,
      };
    }
  }

  const mem = (performance as unknown as { memory?: { usedJSHeapSize: number } })
    .memory;
  const jsHeap = mem ? mem.usedJSHeapSize / (1024 * 1024) : undefined;

  const adapter = await buildAdapterStats(scene, canvas);

  window.phantomatStats = {
    ...window.phantomatStats,
    ttfr,
    frameTime,
    jsHeap,
    backend: scene.backend,
    scenario,
    adapter,
    frameBreakdown,
    richAdapter,
  };

  console.log("[phantomat] richAdapter:", richAdapter);
  console.log("[phantomat] adapter:", adapter);
  if (frameBreakdown) {
    console.log("[phantomat] frame breakdown (avg):", frameBreakdown);
  }
}

main().catch((err) => {
  const p = document.createElement("pre");
  p.style.color = "#f66";
  p.textContent = String(err);
  document.body.appendChild(p);
});
