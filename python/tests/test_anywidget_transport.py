"""anywidget: Arrow IPC bytes roundtrip on the Python side (no browser)."""

from __future__ import annotations

from io import BytesIO

import numpy as np
import pyarrow as pa
import pyarrow.ipc as ipc

from phantomat.widget import Scene

REQUIRED = ("x", "y", "r", "g", "b", "a", "size")


def test_scatter_arrow_synced() -> None:
    s = Scene()
    n = 100
    rng = np.random.default_rng(0)
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
    s.add_scatter(t)
    assert len(s._layers_arrow_ipc) > 0  # noqa: SLF001

    reader = ipc.open_stream(BytesIO(s._layers_arrow_ipc))  # noqa: SLF001
    batches = list(reader)
    assert len(batches) == 1
    assert batches[0].num_rows == n
    for col in REQUIRED:
        np.testing.assert_array_equal(
            batches[0].column(col).to_numpy(),
            t.column(col).to_numpy(),
        )
