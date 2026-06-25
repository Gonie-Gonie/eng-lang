#!/usr/bin/env python3
"""Write a database side-effect manifest instead of touching a real database."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path


def load_rows(path: str) -> tuple[list[str], int]:
    with Path(path).open(encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        rows = list(reader)
        return list(reader.fieldnames or []), len(rows)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--database", default="outputs/surrogate_results.sqlite")
    parser.add_argument("--results", default="outputs/summary_results.csv")
    parser.add_argument("--predictions", default="outputs/predictions.csv")
    parser.add_argument("--out", default="outputs/db_write_manifest.json")
    args = parser.parse_args()

    result_fields, result_count = load_rows(args.results)
    prediction_fields, prediction_count = load_rows(args.predictions)
    manifest = {
        "database": args.database,
        "transaction_status": "committed-fixture",
        "schema_status": "ok",
        "tables": [
            {
                "name": "simulation_results",
                "mode": "upsert",
                "key": ["case_id"],
                "schema": result_fields,
                "row_count": result_count,
            },
            {
                "name": "predictions",
                "mode": "append",
                "key": [],
                "schema": prediction_fields,
                "row_count": prediction_count,
            },
        ],
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
