#!/usr/bin/env python3
"""Write a deterministic case manifest for an opaque external simulation case."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path


def sample_for_case(samples_path: str, case_id: str) -> dict[str, str]:
    with Path(samples_path).open(encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("case_id") == case_id:
                return row
    raise SystemExit(f"sample case not found: {case_id}")


def sample_hash_payload(sample: dict[str, str]) -> str:
    return "|".join(f"{key}={sample[key]}" for key in sorted(sample))


def file_record(path: str) -> dict[str, object]:
    data = Path(path).read_bytes()
    return {
        "path": path,
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument("--sample-row")
    parser.add_argument("--samples")
    parser.add_argument("--case-dir")
    parser.add_argument("--input")
    parser.add_argument("--result")
    parser.add_argument("--patch-status", default="success")
    parser.add_argument("--simulation-status", default="success")
    parser.add_argument("--out")
    args = parser.parse_args()

    if args.samples is None and args.sample_row is None:
        args.samples = "samples/design_samples.csv"
    if args.case_dir is None:
        args.case_dir = f"outputs/{args.case}"
    if args.input is None:
        args.input = f"{args.case_dir}/input.txt"
    if args.result is None:
        args.result = f"{args.case_dir}/result.json"
    if args.out is None:
        args.out = f"{args.case_dir}/case_manifest.json"

    result = json.loads(Path(args.result).read_text(encoding="utf-8"))
    if args.samples:
        sample = sample_for_case(args.samples, args.case)
        sample_hash_source = sample_hash_payload(sample)
    elif args.sample_row:
        sample = {"raw_sample_row": args.sample_row}
        sample_hash_source = args.sample_row
    else:
        raise SystemExit("provide --samples or --sample-row")

    manifest = {
        "case_id": args.case,
        "sample": sample,
        "sample_row_hash": hashlib.sha256(sample_hash_source.encode("utf-8")).hexdigest(),
        "case_dir": args.case_dir,
        "generated_input_file": file_record(args.input),
        "processes": [
            {
                "name": "patch_input",
                "command": "python tools/patch_input.py",
                "status": args.patch_status,
            },
            {
                "name": "external_simulation",
                "command": "python tools/run_external_sim.py",
                "status": args.simulation_status,
            },
        ],
        "result_files": [file_record(args.result)],
        "metrics": {
            "annual_electricity_kwh": result["annual_electricity"],
            "annual_cooling_kwh": result["annual_cooling"],
            "peak_cooling_kw": result["peak_cooling"],
            "unmet_hours": result["unmet_hours"],
        },
        "failure_reason": None,
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
