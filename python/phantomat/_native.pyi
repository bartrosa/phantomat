"""Type stubs for the PyO3 extension `phantomat._native`."""

import numpy as np
from numpy.typing import NDArray

class Scene:
    """Headless canvas + scatter layers; renders to PNG bytes via wgpu."""

    def __init__(self, width: int, height: int) -> None: ...
    def add_scatter(
        self,
        positions: NDArray[np.float32],
        colors: NDArray[np.float32],
        sizes: NDArray[np.float32],
    ) -> None:
        """Scatter positions shape ``(N, 2)``, colors ``(N, 4)``, sizes ``(N,)``."""
        ...
    def render_to_png(self) -> bytes: ...
