/**
 * anywidget ESM entry: WebGPU scatter from Arrow IPC bytes synced from Python.
 */
import { tableFromIPC } from "apache-arrow";
import { init, Scene, scatterFromArrow } from "@phantomat/core";

function asUint8Array(v: unknown): Uint8Array {
  if (v instanceof Uint8Array) return v;
  if (v instanceof ArrayBuffer) return new Uint8Array(v);
  if (ArrayBuffer.isView(v)) {
    return new Uint8Array(v.buffer, v.byteOffset, v.byteLength);
  }
  return new Uint8Array();
}

export default {
  async render({
    model,
    el,
  }: {
    model: {
      get: (name: string) => unknown;
      on: (ev: string, cb: () => void) => void;
    };
    el: HTMLElement;
  }) {
    await init({
      /* @vite-ignore — resolved next to this bundle when served from `phantomat/static` */
      module_or_path: new URL("phantomat_wasm_bg.wasm", import.meta.url),
    });

    const canvas = document.createElement("canvas");
    canvas.width = 800;
    canvas.height = 600;
    canvas.style.maxWidth = "100%";
    el.appendChild(canvas);

    const scene = await Scene.new(canvas);

    const updateLayers = async () => {
      const raw = model.get("_layers_arrow_ipc");
      const ipc = asUint8Array(raw);
      if (ipc.byteLength === 0) return;

      const table = tableFromIPC(ipc);
      scene.clear();
      const layer = await scatterFromArrow(table);
      scene.add_layer(layer);
      await scene.render();
    };

    await updateLayers();
    model.on("change:_layers_arrow_ipc", updateLayers);
  },
};
