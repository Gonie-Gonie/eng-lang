#!/usr/bin/env python3
"""Write a deterministic case manifest for an opaque external simulation case."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


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
    parser.add_argument("--sample-row", required=True)
    parser.add_argument("--case-dir", required=True)
    parser.add_argument("--input", required=True)
    parser.add_argument("--result", required=True)
    parser.add_argument("--patch-status", required=True)
    parser.add_argument("--simulation-status", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    result = json.loads(Path(args.result).read_text(encoding="utf-8"))
    manifest = {
        "case_id": args.case,
        "sample_row_hash": hashlib.sha256(args.sample_row.encode("utf-8")).hexdigest(),
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
        },
        "failure_reason": None,
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
