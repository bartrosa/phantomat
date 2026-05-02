"""Jupyter / anywidget: WebGPU canvas driven by Arrow IPC stream bytes (`_layers_arrow_ipc`)."""

from __future__ import annotations

from io import BytesIO
from pathlib import Path
from typing import Any

import anywidget
import pyarrow as pa
import traitlets

_STATIC = Path(__file__).parent / "static"


def _as_table(obj: Any) -> pa.Table:
    """Accept PyArrow Table/RecordBatch, Polars, pandas, or anything with ``to_arrow``."""
    if isinstance(obj, pa.Table):
        return obj
    if isinstance(obj, pa.RecordBatch):
        return pa.Table.from_batches([obj])
    if hasattr(obj, "to_arrow"):
        return _as_table(obj.to_arrow())
    try:
        import pandas as pd
    except ImportError:
        pd = None  # type: ignore[assignment]
    if pd is not None and isinstance(obj, pd.DataFrame):
        return pa.Table.from_pandas(obj, preserve_index=False)
    raise TypeError(f"Unsupported table type: {type(obj)}")


class Scene(anywidget.AnyWidget):
    """ipywidgets / anywidget `Scene` (WebGPU in the browser; not the headless `phantomat.Scene`)."""

    _esm = _STATIC.joinpath("widget.js").read_text(encoding="utf-8")
    _layers_arrow_ipc = traitlets.Bytes(b"").tag(sync=True)

    def __init__(self, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        self._layers: list[pa.RecordBatch] = []

    def add_scatter(self, table: Any) -> None:
        """Append scatter geometry from a table-like object (see [`_as_table`])."""
        t = _as_table(table).combine_chunks()
        for b in t.to_batches():
            self._layers.append(b)
        self._sync_layers()

    def _sync_layers(self) -> None:
        if not self._layers:
            self._layers_arrow_ipc = b""
            return
        sink = BytesIO()
        writer = pa.ipc.new_stream(sink, self._layers[0].schema)
        try:
            for batch in self._layers:
                writer.write_batch(batch)
        finally:
            writer.close()
        self._layers_arrow_ipc = sink.getvalue()
