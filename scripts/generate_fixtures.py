#!/usr/bin/env python3
"""Generate `fixtures/scatter_1k.parquet` and `fixtures/scatter_1k.ipc` (do not commit binaries; use Git LFS locally)."""

from __future__ import annotations

import os
import sys

try:
    import numpy as np
    import pyarrow as pa
    import pyarrow.ipc as ipc
    import pyarrow.parquet as pq
except ImportError as e:
    print("Requires: numpy, pyarrow (pip install numpy pyarrow)", file=sys.stderr)
    raise SystemExit(1) from e


def main() -> None:
    root = os.path.join(os.path.dirname(__file__), "..")
    fixtures = os.path.join(root, "fixtures")
    os.makedirs(fixtures, exist_ok=True)

    rng = np.random.default_rng(42)
    n = 1000
    t = pa.table(
        {
            "x": rng.random(n).astype(np.float32),
            "y": rng.random(n).astype(np.float32),
            "r": rng.random(n).astype(np.float32),
            "g": rng.random(n).astype(np.float32),
            "b": rng.random(n).astype(np.float32),
            "a": np.ones(n, dtype=np.float32),
            "size": (rng.random(n) * 20 + 2).astype(np.float32),
        }
    )

    pq.write_table(t, os.path.join(fixtures, "scatter_1k.parquet"))
    with open(os.path.join(fixtures, "scatter_1k.ipc"), "wb") as f:
        with ipc.new_file(f, t.schema) as writer:
            writer.write_table(t)

    print("Wrote fixtures/scatter_1k.parquet and fixtures/scatter_1k.ipc")


if __name__ == "__main__":
    main()
