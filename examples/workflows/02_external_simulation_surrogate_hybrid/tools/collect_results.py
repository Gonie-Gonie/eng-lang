#!/usr/bin/env python3
"""Collect fake simulator JSON files into a CSV summary."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+")
    parser.add_argument("--out", default="outputs/summary_results.csv")
    args = parser.parse_args()

    if args.inputs is None:
        args.inputs = [
            "outputs/case_001/result.json",
            "outputs/case_002/result.json",
            "outputs/case_003/result.json",
        ]

    rows = [json.loads(Path(path).read_text(encoding="utf-8")) for path in args.inputs]
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=[
                "case_id",
                "annual_electricity",
                "annual_cooling",
                "peak_cooling",
            ],
        )
        writer.writeheader()
        for row in rows:
            writer.writerow({key: row[key] for key in writer.fieldnames})
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

