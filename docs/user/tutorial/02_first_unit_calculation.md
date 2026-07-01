# 02 First Unit Calculation

## Goal

Write a small unit-aware calculation and see how EngLang keeps physical
quantity meaning attached to values.

## What You Will Build

A simple heat-rate calculation:

```eng partial
m_dot = 2 kg/s
cp = 4180 J/kg/K
delta_T = 5 K
Q = m_dot * cp * delta_T
print "Q = {Q: .2 kW}"
```

## Source File

Create a small file such as scratch/first_units.eng in your working directory.

## Run Command

```bat
eng.exe run scratch/first_units.eng --save-artifacts
```

## Expected Artifacts

`build/result` should contain result.engres and review metadata. The printed
value should be convertible to kW.

## Explanation

EngLang treats kg/s, J/kg/K, K, and kW as part of the calculation contract. A
value is not just a number; it carries quantity and unit evidence that later
reports and review artifacts can inspect.

## Common Mistakes

- Printing a value in a unit that is not compatible with its quantity.
- Treating degC temperature differences like absolute temperatures. Use Kelvin
  for temperature deltas.
- Removing units early and expecting review artifacts to recover them.

## What To Inspect

Open review.json and confirm that the calculation appears with unit evidence.
If a unit mismatch fails, read the diagnostic before changing the formula.
