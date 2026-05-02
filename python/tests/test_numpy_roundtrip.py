import numpy as np
import pytest

from phantomat import Scene


@pytest.mark.parametrize("N", [1, 10, 1000, 1_000_000])
def test_float32_roundtrip(N: int) -> None:
    np.random.seed(42)
    pos = np.random.rand(N, 2).astype(np.float32)
    col = np.random.rand(N, 4).astype(np.float32)
    siz = np.random.rand(N).astype(np.float32) * 10
    s = Scene(256, 256)
    s.add_scatter(pos, col, siz)
    png = s.render_to_png()
    assert len(png) > 100
    assert png[:4] == b"\x89PNG"
