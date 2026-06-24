#!/usr/bin/env python3
"""Fake deterministic external simulator."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument("--input", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    result = {
        "case_id": args.case,
        "annual_electricity": 12800.0,
        "annual_cooling": 4550.0,
        "peak_cooling": 14.2,
        "input_hash_hint": Path(args.input).name,
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
