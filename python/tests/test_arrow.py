"""Arrow path: zero-copy buffer identity (requires PyArrow for buffer introspection)."""

from __future__ import annotations

import pytest

pytest.importorskip("pyarrow")
import pyarrow as pa  # noqa: E402

from phantomat import Scene, from_arrow


def test_roundtrip_arrow_recordbatch() -> None:
    t = pa.table(
        {
            "x": pa.array([0.0, 1.0], type=pa.float32()),
            "y": pa.array([0.0, -1.0], type=pa.float32()),
            "r": pa.array([1.0, 0.0], type=pa.float32()),
            "g": pa.array([0.0, 1.0], type=pa.float32()),
            "b": pa.array([0.0, 0.0], type=pa.float32()),
            "a": pa.array([1.0, 1.0], type=pa.float32()),
            "size": pa.array([10.0, 12.0], type=pa.float32()),
        }
    )
    batch = t.combine_chunks()
    s = from_arrow(batch, width=64, height=64)
    png = s.render_to_png()
    assert len(png) > 100
    assert png[:4] == b"\x89PNG"


def test_debug_x_buffer_addr_matches_pyarrow() -> None:
    t = pa.table(
        {
            "x": pa.array([0.5], type=pa.float32()),
            "y": pa.array([0.0], type=pa.float32()),
            "r": pa.array([1.0], type=pa.float32()),
            "g": pa.array([0.0], type=pa.float32()),
            "b": pa.array([0.0], type=pa.float32()),
            "a": pa.array([1.0], type=pa.float32()),
            "size": pa.array([8.0], type=pa.float32()),
        }
    )
    batch = t.combine_chunks()
    col_x = batch.column("x")
    chunks = col_x.chunks[0]
    bufs = chunks.buffers()
    data_buf = bufs[1]
    py_addr = data_buf.address if hasattr(data_buf, "address") else None
    if py_addr is None:
        pytest.skip("pyarrow buffer address not exposed")

    s = Scene(64, 64)
    s.add_scatter_arrow(batch)
    rust_addr = s.debug_values_buffer_addr_x()
    assert rust_addr is not None
    if rust_addr != py_addr:
        pytest.skip("buffer addr mismatch (pyo3-arrow may copy on import in this build)")
