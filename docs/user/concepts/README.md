# Concepts

These concepts explain why EngLang workflows are structured the way they are.

## Semantic Engineering Workflow

An EngLang program should keep engineering meaning attached to computation:
units, physical quantities, schemas, axes, validation policy, generated files,
and review evidence are part of the workflow contract.

## Units, Quantities, And Axes

Units are not formatting labels. They constrain calculations and make reports
reviewable. Time axes are also semantic: a TimeSeries calculation should retain
row count, index meaning, statistics, and integration evidence.

## Schema Boundary

CSV and other external data should cross into EngLang through an explicit
schema. The schema records column names, quantity kinds, units, index fields,
constraints, and missing-data policy.

## Reviewability

The language should help reviewers answer:

- What inputs were used?
- Which assumptions and units were applied?
- What files and processes were touched?
- Which diagnostics or validation results matter?
- Can the report claims be traced to typed evidence?

## Side Effects

File writes, process calls, and database-like operations are allowed only when
they are explicit enough to inspect. Hidden side effects make engineering
results hard to reproduce.

## Uncertainty

Uncertainty should be explicit, measured, and reviewable. Current support is a
narrow workflow track, not a blanket claim that arbitrary probabilistic
propagation is stable.

## Documentation Source Of Truth

Hand-written Markdown and generated metadata are the documentation source of
truth. OODocs may publish curated bundles, but it must not become a dependency
of eng.exe or the language reference model.
