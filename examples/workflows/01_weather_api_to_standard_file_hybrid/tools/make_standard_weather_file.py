#!/usr/bin/env python3
"""Fixture standard-weather writer for the workflow example."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    payload = json.loads(Path(args.input).read_text(encoding="utf-8"))
    lines = [
        "STANDARD-WEATHER-FIXTURE",
        f"station_id={payload.get('station_id', 'unknown')}",
        f"year={payload.get('year', 'unknown')}",
        "time,dry_bulb_degC,relative_humidity,wind_speed_mps,global_horizontal_W_m2",
    ]
    for record in payload.get("records", []):
        lines.append(
            ",".join(
                [
                    str(record.get("time", "")),
                    str(record.get("dry_bulb_degC", "")),
                    str(record.get("relative_humidity", "")),
                    str(record.get("wind_speed_mps", "")),
                    str(record.get("global_horizontal_W_m2", "")),
                ]
            )
        )
    Path(args.out).write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

