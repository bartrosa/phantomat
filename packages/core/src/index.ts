import init, { ScatterLayer, Scene } from "phantomat-wasm";

export { init, Scene, ScatterLayer };

/** Typed scatter layer inputs (mirrors raw WASM constructor before GPU upload). */
export interface ScatterOpts {
  positions: Float32Array;
  colors: Float32Array;
  sizes: Float32Array;
}

export interface ScatterLayerSpec {
  readonly kind: "scatter";
  readonly opts: ScatterOpts;
}

export type Layer = ScatterLayerSpec;

function validateScatterOpts(opts: ScatterOpts): void {
  const pointCount = opts.positions.length / 2;
  if (!Number.isInteger(pointCount) || pointCount < 0) {
    throw new Error("positions length must be a non-negative multiple of 2");
  }
  if (opts.colors.length !== pointCount * 4) {
    throw new Error(
      "colors length must be positions.length / 2 * 4 (RGBA per point)",
    );
  }
  if (opts.sizes.length !== pointCount) {
    throw new Error("sizes length must match number of points");
  }
}

/** Fluent builder: queues layers and runs WASM `init` inside `build()`. */
export class SceneBuilder {
  private readonly layers: Layer[] = [];

  /** Number of layers queued (useful for tests / diagnostics). */
  get layerCount(): number {
    return this.layers.length;
  }

  scatter(opts: ScatterOpts): this {
    validateScatterOpts(opts);
    this.layers.push({ kind: "scatter", opts });
    return this;
  }

  async build(canvas: HTMLCanvasElement): Promise<Scene> {
    await init();
    const scene = await Scene.new(canvas);
    for (const layer of this.layers) {
      if (layer.kind === "scatter") {
        const { positions, colors, sizes } = layer.opts;
        scene.add_layer(new ScatterLayer(positions, colors, sizes));
      }
    }
    return scene;
  }
}

export const sceneBuilder = (): SceneBuilder => new SceneBuilder();
