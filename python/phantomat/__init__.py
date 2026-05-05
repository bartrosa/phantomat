from phantomat._native import Scene

__all__ = ["Scene", "from_arrow"]
__version__ = "0.0.1"


def from_arrow(obj, width: int = 512, height: int = 512):
    """Return a [`Scene`] with one scatter layer from Arrow data (PyCapsule / stream).

    Accepts PyArrow ``RecordBatch`` or ``Table``, Polars, DuckDB-registered objects, etc.,
    without listing ``pyarrow`` as a required runtime dependency of the wheel (optional for apps).

    Chunked ``Table``s with multiple record batches (Parquet row groups, polars
    conversions, ``concat_tables``, …) are passed through in full: the Rust
    binding concatenates every batch from the underlying stream so no rows are
    silently dropped (previously only the first batch was rendered).
    """
    s = Scene(width, height)
    s.add_scatter_arrow(obj)
    return s
