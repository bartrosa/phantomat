"""Parity: canonical scene JSON from Python build_scene.py vs Node build_scene.mjs."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent
FIXTURES = [
    "scenario_scatter_basic.json",
    "scenario_heatmap_clusters.json",
    "scenario_line_timeseries.json",
    "scenario_bar_categorical.json",
    "scenario_combined_3layers.json",
]


@pytest.mark.parametrize("name", FIXTURES)
def test_python_ts_scene_json_matches(name: str) -> None:
    py_script = ROOT / "python_ts_parity" / "build_scene.py"
    js_script = ROOT / "python_ts_parity" / "build_scene.mjs"
    py_out = subprocess.run(
        [sys.executable, str(py_script), name],
        cwd=str(ROOT),
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    js_out = subprocess.run(
        ["node", str(js_script), name],
        cwd=str(ROOT),
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    assert py_out == js_out
