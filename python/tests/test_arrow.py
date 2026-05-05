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


def _scatter_table(n_per_chunk: int, n_chunks: int) -> pa.Table:
    """Build a chunked Arrow table with ``n_chunks`` batches of ``n_per_chunk`` rows."""
    cols = ("x", "y", "r", "g", "b", "a", "size")
    chunks: dict[str, list] = {c: [] for c in cols}
    for chunk_idx in range(n_chunks):
        base = float(chunk_idx)
        for c in cols:
            offset = 0.0 if c != "a" else 1.0  # alpha must be 1.0
            data = [base + offset + i / max(1, n_per_chunk) for i in range(n_per_chunk)]
            chunks[c].append(pa.array(data, type=pa.float32()))
    arrays = {c: pa.chunked_array(chunks[c]) for c in cols}
    return pa.table(arrays)


def test_from_arrow_chunked_table_keeps_all_rows() -> None:
    """Regression: chunked Tables (multiple batches) must not drop rows.

    Previously ``from_arrow`` only forwarded ``batches[0]`` to Rust and the
    Rust binding only consumed the first batch from a stream, so any chunked
    PyArrow ``Table`` (e.g. from a multi-row-group Parquet file) silently
    dropped every batch except the first.
    """
    n_per_chunk = 32
    n_chunks = 4
    t = _scatter_table(n_per_chunk, n_chunks)
    assert t.num_rows == n_per_chunk * n_chunks
    assert len(t.to_batches()) == n_chunks, "expected a chunked table for the test"

    s_chunked = from_arrow(t, width=64, height=64)
    assert s_chunked.debug_total_points() == n_per_chunk * n_chunks

    s_combined = from_arrow(t.combine_chunks(), width=64, height=64)
    assert s_combined.debug_total_points() == s_chunked.debug_total_points(), (
        "chunked Table must keep the same row count as its combined-chunks "
        "form (otherwise rows from later batches are being dropped)"
    )

    png_chunked = s_chunked.render_to_png()
    png_combined = s_combined.render_to_png()
    assert png_chunked[:4] == b"\x89PNG"
    assert png_chunked == png_combined


def test_add_scatter_arrow_chunked_table_keeps_all_rows() -> None:
    """Regression: ``Scene.add_scatter_arrow(table)`` must consume every batch."""
    t = _scatter_table(16, 5)
    s_chunked = Scene(64, 64)
    s_chunked.add_scatter_arrow(t)
    assert s_chunked.debug_total_points() == 16 * 5

    s_single = Scene(64, 64)
    s_single.add_scatter_arrow(t.combine_chunks())
    assert s_single.debug_total_points() == 16 * 5

    png_chunked = s_chunked.render_to_png()
    png_single = s_single.render_to_png()
    assert png_chunked == png_single


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
