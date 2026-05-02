import { sceneBuilder } from "@phantomat/core";

async function main() {
  const canvas = document.getElementById("c");
  if (!(canvas instanceof HTMLCanvasElement)) {
    throw new Error("missing #c canvas");
  }

  const N = 10_000;
  const positions = new Float32Array(N * 2);
  const colors = new Float32Array(N * 4);
  const sizes = new Float32Array(N);

  for (let i = 0; i < N; i++) {
    const t = i / N;
    positions[i * 2] = (Math.random() * 2 - 1) * 0.95;
    positions[i * 2 + 1] = (Math.random() * 2 - 1) * 0.95;
    colors[i * 4] = 0.2 + 0.8 * t;
    colors[i * 4 + 1] = 0.3 + 0.5 * Math.sin(t * 6.28);
    colors[i * 4 + 2] = 0.6;
    colors[i * 4 + 3] = 1.0;
    sizes[i] = 3 + Math.random() * 12;
  }

  const scene = await sceneBuilder()
    .scatter({ positions, colors, sizes })
    .build(canvas);
  await scene.render();
}

main().catch((err) => {
  console.error(err);
  const p = document.createElement("pre");
  p.style.color = "#f66";
  p.textContent = String(err);
  document.body.appendChild(p);
});
