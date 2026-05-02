import numpy as np
from hypothesis import given, strategies as st

from phantomat import Scene


@given(n=st.integers(1, 1000), seed=st.integers(0, 2**32 - 1))
def test_random_inputs_dont_panic(n: int, seed: int) -> None:
    rng = np.random.default_rng(seed)
    pos = rng.random((n, 2), dtype=np.float32)
    col = rng.random((n, 4), dtype=np.float32)
    siz = rng.random(n, dtype=np.float32) * 10
    s = Scene(64, 64)
    s.add_scatter(pos, col, siz)
    out = s.render_to_png()
    assert len(out) > 100
    assert out[:4] == b"\x89PNG"
