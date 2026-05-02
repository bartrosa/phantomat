"""Fixture: Arrow scatter render must match numpy/vec scatter (same PNG bytes)."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import pytest

pytest.importorskip("pyarrow")
import pyarrow.parquet as pq  # noqa: E402

from phantomat import Scene

FIXTURE = Path(__file__).resolve().parents[2] / "fixtures" / "scatter_1k.parquet"


@pytest.mark.skipif(
    not FIXTURE.is_file(),
    reason="run `python scripts/generate_fixtures.py` to create scatter_1k.parquet",
)
def test_scatter_1k_arrow_png_matches_vec_png() -> None:
    t = pq.read_table(str(FIXTURE))
    batch = t.combine_chunks()

    s_arrow = Scene(512, 512)
    s_arrow.add_scatter_arrow(batch)
    png_arrow = s_arrow.render_to_png()

    x = batch.column("x").to_numpy(zero_copy_only=False)
    y = batch.column("y").to_numpy(zero_copy_only=False)
    r = batch.column("r").to_numpy(zero_copy_only=False)
    g = batch.column("g").to_numpy(zero_copy_only=False)
    b = batch.column("b").to_numpy(zero_copy_only=False)
    a = batch.column("a").to_numpy(zero_copy_only=False)
    size = batch.column("size").to_numpy(zero_copy_only=False)

    positions = np.column_stack((x, y)).astype(np.float32, copy=False)
    colors = np.column_stack((r, g, b, a)).astype(np.float32, copy=False)

    s_vec = Scene(512, 512)
    s_vec.add_scatter(positions, colors, size)
    png_vec = s_vec.render_to_png()

    assert png_arrow == png_vec, "Arrow vs vec PNG mismatch for scatter_1k fixture"
