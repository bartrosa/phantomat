import io

import numpy as np
from PIL import Image

from phantomat import Scene


def test_renders_to_valid_png() -> None:
    s = Scene(128, 128)
    n = 10
    pos = np.zeros((n, 2), dtype=np.float32)
    col = np.ones((n, 4), dtype=np.float32)
    siz = np.full(n, 6.0, dtype=np.float32)
    for i in range(n):
        pos[i, 0] = (i / max(n - 1, 1)) * 1.8 - 0.9
        pos[i, 1] = 0.1 * np.sin(i)

    s.add_scatter(pos, col, siz)
    png_bytes = s.render_to_png()
    img = Image.open(io.BytesIO(png_bytes))
    assert img.size == (128, 128)
    assert img.mode == "RGBA"
