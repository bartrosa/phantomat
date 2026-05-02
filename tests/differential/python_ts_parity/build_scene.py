#!/usr/bin/env python3
"""Emit canonical JSON scene description for parity diff (sorted keys)."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path


def scene_from_fixture(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("version") != 1:
        raise ValueError("unsupported fixture version")
    # Canonical numeric payload for scatter / heatmap (deterministic PRNG placeholder).
    layers_out = []
    for layer in data["layers"]:
        t = layer["type"]
        if t == "scatter":
            seed = int(layer["seed"])
            n = int(layer["points"])
            h = hashlib.sha256(f"scatter-{seed}-{n}".encode()).hexdigest()[:16]
            layers_out.append({"type": "scatter", "digest": h, "n": n, "seed": seed})
        elif t == "heatmap":
            seed = int(layer["seed"])
            n = int(layer["points"])
            h = hashlib.sha256(f"heatmap-{seed}-{n}".encode()).hexdigest()[:16]
            layers_out.append(
                {
                    "type": "heatmap",
                    "digest": h,
                    "bins": list(layer["bins"]),
                    "n": n,
                    "seed": seed,
                }
            )
        elif t == "line_stub":
            layers_out.append({"type": "line_stub", "n": int(layer["n"])})
        elif t == "bar_stub":
            layers_out.append(
                {"type": "bar_stub", "categories": list(layer["categories"])}
            )
        else:
            raise ValueError(t)
    return {"layers": layers_out, "version": 1}


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("name", help="fixture file name under fixtures/, e.g. scenario_scatter_basic.json")
    args = p.parse_args()
    path = Path(__file__).resolve().parent / "fixtures" / args.name
    scene = scene_from_fixture(path)
    sys.stdout.write(json.dumps(scene, sort_keys=True, separators=(",", ":")))
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
