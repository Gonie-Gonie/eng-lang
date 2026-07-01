# 05 TimeSeries Statistics

## Goal

Calculate and summarize a unit-aware TimeSeries.

## What You Will Build

The official CSV example derives coil heat rate and energy:

```eng partial
cp = 4180 J/kg/K
Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)
E_coil = integrate(Q_coil, over=Time)

report {
    summarize Q_coil by [mean, time_weighted_mean, max, median, std, p90, p95]
    show E_coil
}
```

## Source File

Use examples/official/01_csv_plot/main.eng.

## Run Command

```bat
eng.exe run examples/official/01_csv_plot/main.eng --save-artifacts
```

## Expected Artifacts

The report should include summary statistics and an integrated energy value.
The review metadata should preserve TimeSeries axis information.

## Explanation

EngLang's TimeSeries operations are intended to keep the time axis, physical
quantity, and unit conversions visible. Statistics should be reviewable instead
of hidden in spreadsheet cells or post-processing scripts.

## Common Mistakes

- Summarizing before confirming the time axis and row count.
- Comparing values in mixed display units without checking canonical units.
- Reporting a statistic without preserving how it was calculated.

## What To Inspect

Inspect the TimeSeries and summary sections in the IDE or review.json. Check
that row counts, units, and statistic names match the report.
