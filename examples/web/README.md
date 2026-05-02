# Phantomat WebAssembly demo

Build the wasm package into this directory so `main.js` can import `./pkg/phantomat_wasm.js`:

```bash
cd crates/phantomat-wasm
wasm-pack build --target web --release --out-dir ../../examples/web/pkg
```

Serve locally (from the **repository root** or from this folder; adjust paths if needed):

```bash
cd examples/web
python -m http.server 8000
```

Open `http://localhost:8000` — you should see ~10k scatter points on a black canvas (requires **WebGPU** or **WebGL2** in the browser).

Release artifact sizes are roughly **~3 MiB** uncompressed `.wasm` (wgpu + layers) and **~900 KiB** gzip-compressed; CI checks gzip stays under **2 MiB**.

If the browser blocks ES modules from `file://`, always use the HTTP server above.
