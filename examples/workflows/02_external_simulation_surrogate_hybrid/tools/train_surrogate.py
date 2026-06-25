#!/usr/bin/env python3
"""Fake surrogate trainer that writes a model card and metrics file."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", default="outputs/summary_results.csv")
    parser.add_argument("--model", default="outputs/surrogate.json")
    parser.add_argument("--metrics", default="outputs/model_metrics.json")
    args = parser.parse_args()

    input_path = Path(args.input)
    input_bytes = input_path.read_bytes()
    with input_path.open(encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    metric_summary = {
        "rmse": 0.0,
        "r2": 1.0,
    }
    card = {
        "model": "linear-fixture",
        "features": [
            "people_density",
            "lighting_power_density",
            "equipment_power_density",
            "cooling_cop",
        ],
        "target": "annual_electricity",
        "target_quantity": "Energy",
        "target_unit": "kWh",
        "train_test_split": {
            "train": 0.8,
            "test": 0.2,
            "seed": 42,
        },
        "metrics": metric_summary,
        "residual_distribution": {
            "mean": 0.0,
            "p95_abs": 0.0,
        },
        "training_data_hash": hashlib.sha256(input_bytes).hexdigest(),
        "training_rows": len(rows),
        "status": "fixture",
    }
    model_path = Path(args.model)
    metrics_path = Path(args.metrics)
    model_path.parent.mkdir(parents=True, exist_ok=True)
    metrics_path.parent.mkdir(parents=True, exist_ok=True)
    model_path.write_text(json.dumps(card, indent=2), encoding="utf-8")
    metrics = {
        **metric_summary,
        "residual_distribution": card["residual_distribution"],
        "training_data_hash": card["training_data_hash"],
        "model_artifact_hash": hashlib.sha256(model_path.read_bytes()).hexdigest(),
        "status": "fixture",
    }
    metrics_path.write_text(json.dumps(metrics, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
