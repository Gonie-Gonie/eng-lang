#!/usr/bin/env python3
"""Fake surrogate trainer that writes a model artifact, metrics, and model card."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path


FEATURES = [
    "people_density",
    "lighting_power_density",
    "equipment_power_density",
    "cooling_cop",
]


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", default="outputs/summary_results.csv")
    parser.add_argument("--model", default="outputs/surrogate.json")
    parser.add_argument("--metrics", default="outputs/model_metrics.json")
    parser.add_argument("--card", default="outputs/model_card.json")
    args = parser.parse_args()

    input_path = Path(args.input)
    input_bytes = input_path.read_bytes()
    with input_path.open(encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    training_rows = len(rows)
    test_rows = 1 if training_rows > 1 else 0
    train_rows = training_rows - test_rows
    training_data_hash = hashlib.sha256(input_bytes).hexdigest()
    split = {
        "train_fraction": 0.8,
        "test_fraction": 0.2,
        "seed": 42,
        "train_rows": train_rows,
        "test_rows": test_rows,
    }
    metric_summary = {
        "rmse": 0.0,
        "r2": 1.0,
    }
    residual_distribution = {
        "mean": 0.0,
        "p95_abs": 0.0,
        "max_abs": 0.0,
    }
    model_payload = {
        "model": "linear-fixture",
        "model_kind": "surrogate_regression_fixture",
        "algorithm": "linear_regression_fixture",
        "features": FEATURES,
        "target": "annual_electricity",
        "target_quantity": "Energy",
        "target_unit": "kWh",
        "prediction_outputs": {
            "predicted_annual_electricity": {
                "source_target": "annual_electricity",
                "quantity": "Energy",
                "unit": "kWh",
                "coefficients": {
                    "intercept": 9000.0,
                    "people_density": 10000.0,
                    "lighting_power_density": 110.0,
                    "equipment_power_density": 140.0,
                    "cooling_cop": -400.0,
                },
            },
            "predicted_peak_cooling": {
                "source_target": "peak_cooling",
                "quantity": "HeatRate",
                "unit": "kW",
                "coefficients": {
                    "intercept": 8.0,
                    "people_density": 12.0,
                    "lighting_power_density": 0.18,
                    "equipment_power_density": 0.25,
                    "cooling_cop": -0.55,
                },
            },
        },
        "training_data_hash": training_data_hash,
        "training_rows": training_rows,
        "status": "fixture",
    }

    model_path = Path(args.model)
    metrics_path = Path(args.metrics)
    card_path = Path(args.card)
    write_json(model_path, model_payload)
    model_artifact_hash = hashlib.sha256(model_path.read_bytes()).hexdigest()

    metrics = {
        **metric_summary,
        "residual_distribution": residual_distribution,
        "training_data_hash": training_data_hash,
        "model_artifact_hash": model_artifact_hash,
        "status": "fixture",
    }
    write_json(metrics_path, metrics)

    card = {
        "model_card": "linear-fixture annual_electricity surrogate",
        "model": "linear-fixture",
        "model_kind": "surrogate_regression_fixture",
        "features": FEATURES,
        "target": "annual_electricity",
        "target_quantity": "Energy",
        "target_unit": "kWh",
        "train_test_split": split,
        "metrics": metric_summary,
        "prediction_outputs": model_payload["prediction_outputs"],
        "residual_distribution": residual_distribution,
        "training_data_hash": training_data_hash,
        "model_artifact_hash": model_artifact_hash,
        "training_rows": training_rows,
        "model_path": args.model,
        "metrics_path": args.metrics,
        "status": "fixture",
    }
    write_json(card_path, card)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
