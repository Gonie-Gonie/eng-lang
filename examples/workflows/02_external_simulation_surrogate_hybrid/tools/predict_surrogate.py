#!/usr/bin/env python3
"""Fake surrogate predictor that writes typed prediction rows."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path


def prediction_for(row: dict[str, str]) -> dict[str, object]:
    people_density = float(row["people_density"])
    lighting = float(row["lighting_power_density"])
    equipment = float(row["equipment_power_density"])
    cop = float(row["cooling_cop"])
    annual = 9000.0 + people_density * 10000.0 + lighting * 110.0 + equipment * 140.0 - cop * 400.0
    peak = 8.0 + people_density * 12.0 + lighting * 0.18 + equipment * 0.25 - cop * 0.55
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
    args = parser.parse_args()

    Path(args.model).read_text(encoding="utf-8")
    with Path(args.samples).open(encoding="utf-8") as handle:
        rows = [prediction_for(row) for row in csv.DictReader(handle)]

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=[
                "case_id",
                "predicted_annual_electricity",
                "predicted_peak_cooling",
                "prediction_confidence",
            ],
        )
        writer.writeheader()
        writer.writerows(rows)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
