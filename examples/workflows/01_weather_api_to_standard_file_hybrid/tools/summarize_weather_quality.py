#!/usr/bin/env python3
"""Compute fixture weather coverage summary from typed source CSV files."""

from __future__ import annotations

import argparse
import csv
from datetime import datetime
from pathlib import Path


def parse_time(value: str) -> datetime:
    return datetime.fromisoformat(value)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--station-map", default="data/station_map_sample.csv")
    parser.add_argument("--weather", default="data/sample_weather_hourly.csv")
    parser.add_argument("--station", default="STN001")
    parser.add_argument("--expected-records", type=int, default=2)
    parser.add_argument("--out", default="outputs/weather_quality_summary.txt")
    args = parser.parse_args()

    with Path(args.station_map).open(encoding="utf-8") as handle:
        stations = list(csv.DictReader(handle))
    with Path(args.weather).open(encoding="utf-8") as handle:
        weather_rows = list(csv.DictReader(handle))

    times = [parse_time(row["time"]) for row in weather_rows if row.get("time")]
    gaps = [
        (right - left).total_seconds() / 3600.0
        for left, right in zip(times, times[1:])
    ]
    missing_cells = sum(
        1
        for row in weather_rows
        for key, value in row.items()
        if key != "time" and value == ""
    )
    selected = next(
        (station for station in stations if station.get("station_id") == args.station),
        None,
    )
    if selected is None:
        raise SystemExit(f"unknown station {args.station}")

    actual_records = len(weather_rows)
    lines = [
        f"selected_station_id={args.station}",
        f"station_rows={len(stations)}",
        f"expected_records={args.expected_records}",
        f"actual_records={actual_records}",
        f"missing_records={max(args.expected_records - actual_records, 0)}",
        f"missing_cells={missing_cells}",
        f"max_gap_hours={max(gaps) if gaps else 0:g}",
    ]

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
