#!/usr/bin/env python3
"""Collect fake simulator JSON files into a CSV summary and collection manifest."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path


def file_record(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {
        "path": str(path).replace("\\", "/"),
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
    }


def sample_case_ids(path: str) -> list[str]:
    with Path(path).open(encoding="utf-8") as handle:
        return [row["case_id"] for row in csv.DictReader(handle)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+")
    parser.add_argument("--samples", default="samples/design_samples.csv")
    parser.add_argument("--cases", nargs="+")
    parser.add_argument("--out", default="outputs/summary_results.csv")
    parser.add_argument("--manifest", default="outputs/result_collection_manifest.json")
    args = parser.parse_args()

    if args.inputs is None:
        args.inputs = [
            "outputs/case_001/result.json",
            "outputs/case_002/result.json",
            "outputs/case_003/result.json",
        ]

    rows = []
    missing_inputs = []
    input_records = []
    for path_text in args.inputs:
        path = Path(path_text)
        if not path.exists():
            missing_inputs.append(path_text)
            continue
        input_records.append(file_record(path))
        rows.append(json.loads(path.read_text(encoding="utf-8")))

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = [
        "case_id",
        "annual_electricity",
        "annual_cooling",
        "peak_cooling",
        "unmet_hours",
    ]
    with out.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow({key: row[key] for key in fieldnames})

    expected_cases = args.cases if args.cases is not None else sample_case_ids(args.samples)
    result_cases = [row["case_id"] for row in rows]
    missing_case_ids = [case for case in expected_cases if case not in result_cases]
    extra_case_ids = [case for case in result_cases if case not in expected_cases]
    manifest = {
        "samples": args.samples,
        "expected_case_ids": expected_cases,
        "result_case_ids": result_cases,
        "missing_case_ids": missing_case_ids,
        "extra_case_ids": extra_case_ids,
        "missing_inputs": missing_inputs,
        "input_files": input_records,
        "output_file": file_record(out),
        "row_count": len(rows),
        "failed_case_count": len(missing_case_ids) + len(missing_inputs),
        "metrics": {
            "annual_electricity_kwh_sum": sum(
                float(row["annual_electricity"]) for row in rows
            ),
            "peak_cooling_kw_max": max(
                (float(row["peak_cooling"]) for row in rows), default=0.0
            ),
            "unmet_hours_sum": sum(float(row["unmet_hours"]) for row in rows),
        },
        "status": "complete"
        if not missing_case_ids and not missing_inputs and not extra_case_ids
        else "incomplete",
    }
    manifest_path = Path(args.manifest)
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    if manifest["status"] != "complete":
        raise SystemExit(f"result collection incomplete: {manifest_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
