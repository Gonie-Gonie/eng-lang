#!/usr/bin/env python3
"""Fake input patcher for the external simulation workflow."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path


def sample_for_case(samples_path: str, case_id: str) -> dict[str, str]:
    with Path(samples_path).open(encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("case_id") == case_id:
                return row
    raise SystemExit(f"sample case not found: {case_id}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="model/base_model_placeholder.txt")
    parser.add_argument("--case", required=True)
    parser.add_argument("--sample")
    parser.add_argument("--samples")
    parser.add_argument("--out")
    args = parser.parse_args()

    base = Path(args.base).read_text(encoding="utf-8")
    if args.out is None:
        args.out = f"outputs/{args.case}/input.txt"
    if args.samples is None and args.sample is None:
        args.samples = "samples/design_samples.csv"

    if args.samples:
        sample = sample_for_case(args.samples, args.case)
    elif args.sample:
        try:
            sample = json.loads(args.sample)
        except json.JSONDecodeError:
            sample = {"raw_sample_row": args.sample}
    else:
        raise SystemExit("provide --samples or --sample")

    text = base + "\n" + json.dumps({"case_id": args.case, "sample": sample}, indent=2)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
