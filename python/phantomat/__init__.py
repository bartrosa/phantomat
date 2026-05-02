from phantomat._native import Scene

__all__ = ["Scene", "from_arrow"]
__version__ = "0.0.1"


def from_arrow(obj, width: int = 512, height: int = 512):
    """Return a [`Scene`] with one scatter layer from Arrow data (PyCapsule / stream).

    Accepts PyArrow ``RecordBatch`` or ``Table``, Polars, DuckDB-registered objects, etc.,
    without listing ``pyarrow`` as a required runtime dependency of the wheel (optional for apps).
    """
    s = Scene(width, height)
    if hasattr(obj, "to_batches"):
        batches = obj.to_batches()
        if not batches:
            raise ValueError("empty Arrow table")
        obj = batches[0]
    s.add_scatter_arrow(obj)
    return s
