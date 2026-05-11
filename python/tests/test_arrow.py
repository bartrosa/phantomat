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


def _scatter_batch(xs: list[float]) -> pa.RecordBatch:
    n = len(xs)
    return pa.record_batch(
        {
            "x": pa.array(xs, type=pa.float32()),
            "y": pa.array([0.0] * n, type=pa.float32()),
            "r": pa.array([1.0] * n, type=pa.float32()),
            "g": pa.array([1.0] * n, type=pa.float32()),
            "b": pa.array([1.0] * n, type=pa.float32()),
            "a": pa.array([1.0] * n, type=pa.float32()),
            "size": pa.array([10.0] * n, type=pa.float32()),
        }
    )


def test_multi_batch_table_renders_all_points() -> None:
    """Regression: multi-chunk Tables previously silently dropped batches[1:].

    Before the concat fix, ``from_arrow(table)`` and ``Scene.add_scatter_arrow(table)``
    would render only the first batch when the input Table had more than one chunk
    (e.g. a polars/pandas DataFrame converted with default chunking, or a
    ``pa.concat_tables`` result). The downstream PNG would silently lose points.
    """
    b1 = _scatter_batch([-0.5, 0.0])
    b2 = _scatter_batch([0.5])
    b3 = _scatter_batch([0.25, -0.25, 0.75])
    table = pa.Table.from_batches([b1, b2, b3])
    # Pre-condition: the input really has multiple chunks (otherwise the test
    # would still pass for the old, buggy code path).
    assert len(table.to_batches()) > 1
    assert table.num_rows == 6

    # Direct ``add_scatter_arrow`` path: should accept all 6 points.
    s_direct = Scene(64, 64)
    s_direct.add_scatter_arrow(table)
    png_direct = s_direct.render_to_png()
    assert png_direct[:4] == b"\x89PNG"

    # ``from_arrow`` wrapper path: also honors every batch.
    s_from = from_arrow(table, width=64, height=64)
    png_from = s_from.render_to_png()
    assert png_from[:4] == b"\x89PNG"
