from phantomat._native import Scene

__all__ = ["Scene", "from_arrow"]
__version__ = "0.0.1"


def from_arrow(obj, width: int = 512, height: int = 512):
    """Return a [`Scene`] with one scatter layer from Arrow data (PyCapsule / stream).

    Accepts PyArrow ``RecordBatch`` or ``Table``, Polars, DuckDB-registered objects, etc.,
    without listing ``pyarrow`` as a required runtime dependency of the wheel (optional for apps).

    Multi-chunk Tables are handed off as a stream to the native side, which concatenates all
    batches into a single ``RecordBatch`` before rendering (previously only the first batch was
    rendered, silently dropping the rest of the data).
    """
    s = Scene(width, height)
    s.add_scatter_arrow(obj)
    return s
