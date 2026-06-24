#!/usr/bin/env python3
"""Fake input patcher for the external simulation workflow."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True)
    parser.add_argument("--case", required=True)
    parser.add_argument("--sample", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    base = Path(args.base).read_text(encoding="utf-8")
    sample = json.loads(args.sample)
    text = base + "\n" + json.dumps({"case_id": args.case, "sample": sample}, indent=2)
    Path(args.out).write_text(text + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

