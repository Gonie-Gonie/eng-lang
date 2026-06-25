#!/usr/bin/env python3
"""Write a database side-effect manifest instead of touching a real database."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path


RESULT_SCHEMA = [
    "case_id",
    "annual_electricity",
    "annual_cooling",
    "peak_cooling",
    "unmet_hours",
]
PREDICTION_SCHEMA = [
    "case_id",
    "predicted_annual_electricity",
    "predicted_peak_cooling",
    "prediction_confidence",
]


def file_record(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {
        "path": str(path).replace("\\", "/"),
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
    }


def load_rows(path: str) -> tuple[list[str], int, dict[str, object]]:
    source = Path(path)
    with source.open(encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        rows = list(reader)
        return list(reader.fieldnames or []), len(rows), file_record(source)


def schema_diagnostic(table: str, expected: list[str], actual: list[str]) -> dict[str, object] | None:
    missing = [column for column in expected if column not in actual]
    extra = [column for column in actual if column not in expected]
    if not missing and not extra:
        return None
    return {
        "code": "E-DB-SCHEMA-MISMATCH",
        "table": table,
        "expected_schema": expected,
        "actual_schema": actual,
        "missing_columns": missing,
        "extra_columns": extra,
        "severity": "error",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--database", default="outputs/surrogate_results.sqlite")
    parser.add_argument("--results", default="outputs/summary_results.csv")
    parser.add_argument("--predictions", default="outputs/predictions.csv")
    parser.add_argument("--out", default="outputs/db_write_manifest.json")
    args = parser.parse_args()

    result_fields, result_count, result_file = load_rows(args.results)
    prediction_fields, prediction_count, prediction_file = load_rows(args.predictions)
    diagnostics = [
        diagnostic
        for diagnostic in [
            schema_diagnostic("simulation_results", RESULT_SCHEMA, result_fields),
            schema_diagnostic("predictions", PREDICTION_SCHEMA, prediction_fields),
        ]
        if diagnostic is not None
    ]
    schema_status = "ok" if not diagnostics else "mismatch"
    transaction_status = "committed-fixture" if not diagnostics else "rolled-back-fixture"
    manifest = {
        "format": "db-write-manifest-v1",
        "database": args.database,
        "transaction_status": transaction_status,
        "schema_status": schema_status,
        "schema_mismatch_diagnostics": diagnostics,
        "transaction": {
            "status": transaction_status,
            "mode": "single_manifest_transaction",
            "table_count": 2,
        },
        "tables_written": ["simulation_results", "predictions"],
        "source_files": [result_file, prediction_file],
        "tables": [
            {
                "name": "simulation_results",
                "mode": "upsert",
                "key": ["case_id"],
                "schema": result_fields,
                "expected_schema": RESULT_SCHEMA,
                "row_count": result_count,
            },
            {
                "name": "predictions",
                "mode": "append",
                "key": [],
                "schema": prediction_fields,
                "expected_schema": PREDICTION_SCHEMA,
                "row_count": prediction_count,
            },
        ],
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    if diagnostics:
        raise SystemExit(f"database schema mismatch: {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
