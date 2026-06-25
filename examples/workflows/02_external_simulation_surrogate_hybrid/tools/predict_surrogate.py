#!/usr/bin/env python3
"""Fake surrogate predictor that writes typed prediction rows and a manifest."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path


REQUIRED_SAMPLE_COLUMNS = [
    "case_id",
    "people_density",
    "lighting_power_density",
    "equipment_power_density",
    "cooling_cop",
]
PREDICTION_SCHEMA = [
    "case_id",
    "predicted_annual_electricity",
    "predicted_peak_cooling",
    "prediction_confidence",
]
PREDICTION_OUTPUTS = [
    {
        "column": "predicted_annual_electricity",
        "source_target": "annual_electricity",
        "quantity": "Energy",
        "unit": "kWh",
    },
    {
        "column": "predicted_peak_cooling",
        "source_target": "peak_cooling",
        "quantity": "HeatRate",
        "unit": "kW",
    },
    {
        "column": "prediction_confidence",
        "source_target": "confidence",
        "quantity": "Ratio",
        "unit": "1",
    },
]


def file_record(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {
        "path": str(path).replace("\\", "/"),
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
    }


def linear_value(row: dict[str, str], coefficients: dict[str, float]) -> float:
    value = float(coefficients.get("intercept", 0.0))
    for name, coefficient in coefficients.items():
        if name == "intercept":
            continue
        value += float(row[name]) * float(coefficient)
    return value


def prediction_for(row: dict[str, str], model: dict[str, object]) -> dict[str, object]:
    outputs = model.get("prediction_outputs", {})
    annual_output = outputs.get("predicted_annual_electricity", {}) if isinstance(outputs, dict) else {}
    peak_output = outputs.get("predicted_peak_cooling", {}) if isinstance(outputs, dict) else {}
    annual = linear_value(row, annual_output.get("coefficients", {}))
    peak = linear_value(row, peak_output.get("coefficients", {}))
    return {
        "case_id": row["case_id"],
        "predicted_annual_electricity": round(annual, 1),
        "predicted_peak_cooling": round(peak, 1),
        "prediction_confidence": 0.95,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--samples", default="samples/design_samples.csv")
    parser.add_argument("--model", default="outputs/surrogate.json")
    parser.add_argument("--out", default="outputs/predictions.csv")
    parser.add_argument("--manifest", default="outputs/prediction_manifest.json")
    args = parser.parse_args()

    model_path = Path(args.model)
    model = json.loads(model_path.read_text(encoding="utf-8"))
    sample_path = Path(args.samples)
    with sample_path.open(encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        missing_columns = [
            column for column in REQUIRED_SAMPLE_COLUMNS if column not in (reader.fieldnames or [])
        ]
        if missing_columns:
            raise SystemExit(f"prediction sample schema mismatch: {missing_columns}")
        rows = [prediction_for(row, model) for row in reader]

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=PREDICTION_SCHEMA)
        writer.writeheader()
        writer.writerows(rows)

    manifest = {
        "format": "prediction-manifest-v1",
        "status": "complete",
        "model": model.get("model", "unknown"),
        "model_file": file_record(model_path),
        "sample_file": file_record(sample_path),
        "output_file": file_record(out),
        "schema": PREDICTION_SCHEMA,
        "outputs": PREDICTION_OUTPUTS,
        "case_ids": [row["case_id"] for row in rows],
        "row_count": len(rows),
        "schema_mismatch_diagnostics": [],
    }
    manifest_path = Path(args.manifest)
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
