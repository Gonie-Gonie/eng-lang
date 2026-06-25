#!/usr/bin/env python3
"""Fake deterministic external simulator."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument("--input")
    parser.add_argument("--out")
    args = parser.parse_args()

    if args.input is None:
        args.input = f"outputs/{args.case}/input.txt"
    if args.out is None:
        args.out = f"outputs/{args.case}/result.json"

    metrics_by_case = {
        "case_001": {
            "annual_electricity": 11800.0,
            "annual_cooling": 4200.0,
            "peak_cooling": 12.8,
        },
        "case_002": {
            "annual_electricity": 12800.0,
            "annual_cooling": 4550.0,
            "peak_cooling": 14.2,
        },
        "case_003": {
            "annual_electricity": 13950.0,
            "annual_cooling": 4980.0,
            "peak_cooling": 15.6,
        },
    }
    metrics = metrics_by_case.get(
        args.case,
        {
            "annual_electricity": 12800.0,
            "annual_cooling": 4550.0,
            "peak_cooling": 14.2,
        },
    )
    result = {
        "case_id": args.case,
        **metrics,
        "input_hash_hint": Path(args.input).name,
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
