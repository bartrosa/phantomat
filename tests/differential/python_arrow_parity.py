"""Write PNG from fixture for cross-runtime parity checks (compare with TS offline)."""

from __future__ import annotations

import os
import sys

import pyarrow.parquet as pq

from phantomat import from_arrow


def main() -> None:
    root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    fixture = os.path.join(root, "fixtures", "scatter_1k.parquet")
    if not os.path.isfile(fixture):
        print(f"missing {fixture} — run scripts/generate_fixtures.py", file=sys.stderr)
        raise SystemExit(2)

    t = pq.read_table(fixture)
    s = from_arrow(t, width=512, height=512)
    png_py = s.render_to_png()
    out = "/tmp/parity_py.png"
    with open(out, "wb") as f:
        f.write(png_py)
    print(f"wrote {out} ({len(png_py)} bytes)")


if __name__ == "__main__":
    main()
