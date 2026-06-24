#!/usr/bin/env python3
"""Fake surrogate trainer that writes a model card and metrics file."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--metrics", required=True)
    args = parser.parse_args()

    with Path(args.input).open(encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    card = {
        "model": "linear-fixture",
        "features": [
            "people_density",
            "lighting_power_density",
            "equipment_power_density",
            "cooling_cop",
        ],
        "target": "annual_electricity",
        "training_rows": len(rows),
        "status": "fixture",
    }
    metrics = {
        "rmse": 0.0,
        "r2": 1.0,
        "status": "fixture",
    }
    model_path = Path(args.model)
    metrics_path = Path(args.metrics)
    model_path.parent.mkdir(parents=True, exist_ok=True)
    metrics_path.parent.mkdir(parents=True, exist_ok=True)
    model_path.write_text(json.dumps(card, indent=2), encoding="utf-8")
    metrics_path.write_text(json.dumps(metrics, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
