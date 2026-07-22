# Reviewability As A Language Feature

Status: current public artifact family is stable for the v0.1.0 package scope.
`review.json.review_document`, `eng review <file.eng>`, and the IDE Review
inspector are the first supported normalized Review IR slice. `eng review
--against` and `eng review diff <old> <new>` emit the same CLI item-level
semantic diff payload. Saved runs enrich the same ReviewDocument rows with
runtime values and statuses, and the native IDE exposes baseline selection plus
section-hash and item-level comparison. Runtime-generated `report.html`
validates the final saved document and displays the same fingerprint, core
runtime evidence, TimeSeries/coverage counts, generated-file and DB-write
side-effect evidence, and validation outcomes.

## Core Principle

```text
Every engineering computation must produce a reviewable semantic trace.
```

EngLang should not ask reviewers to trust generated code line by line. It
should expose meaning-level artifacts: inputs, units, schemas, axes,
calculations, validations, side effects, external boundaries, assumptions,
fallbacks, and risks.

## Current Artifact Baseline

The current package writes the artifact family that Review IR normalizes:

```text
build/result/result.engres
build/result/review.json
build/result/report.html
build/result/report_spec.json
build/result/run_plan.json
build/result/run_log.json
build/result/process_results.json
build/result/test_results.json
build/result/output_manifest.json
```

These artifacts are the public proof path for typed data boundaries,
TimeSeries statistics, plots, reports, validations, explicit side effects, and
package/IDE inspection.

`review.json.review_document` is the current compiler-owned normalized view. It
records:

```text
format
status
workflow_signature
semantic_hash
semantic_hash_scope            runtime-enriched documents only
section_hashes
runtime_evidence               saved-run documents only
root_contract
workflow_modules
inputs
schemas
config_promotions
units_quantities
time_axes
symbols
derived_values
calculations
table_transforms
report_outputs
validations
side_effects
external_boundaries
caches
fallbacks
risks
```

## Review IR Target

Review IR is becoming the shared semantic document between compiler, runtime,
report generation, CLI review commands, and IDE panels.

Initial node families:

```text
ReviewDocument
ReviewWorkflowModule
ReviewInput
ReviewSymbol
ReviewCalculation
ReviewWhereExpansion
ReviewValidation
ReviewSideEffect
ReviewExternalBoundary
ReviewFallback
ReviewUncertainty
ReviewRisk
ReviewSemanticDiff
```

The current compiler slice fills static meaning: source spans, declarations,
quantities, schemas, time axes, report-output candidates, calculation trace
fields, commands, options, validation expressions, process declarations,
side-effect boundaries, fallbacks, risk levels, and semantic section hashes.
Saved runs currently fill matching core input, schema, scalar, materialized
table, TimeSeries, coverage, source-derived time-axis, calculation, transform,
output, and validation rows with values, statuses, and source/hash evidence.
Table, TimeSeries, and coverage values are projected consistently across their
matching units/quantity, symbol, derived-value, and calculation rows.
Generated-file side effects add artifact kind/path/hash evidence. Native
SQLite write side effects add database and manifest paths/hashes, transaction
and schema status, table schemas, and row counts. Native model, model-card,
metric, and prediction rows add computed metrics, coefficients, train/test
counts, schema/case IDs, and dedicated hashes. Specialized solver, assembly,
and non-write DB record families remain follow-up runtime projection work.

## Root Workflow Review Contract

Every root `.eng` run should be able to answer:

```text
what inputs were declared
which source files and hashes were used
which schemas promoted external data
which units and quantities were inferred or declared
which calculations were performed
which validations passed, warned, or failed
which plots and report sections were requested
which files were written, copied, moved, or deleted
which external processes ran
which tests and golden checks passed
which assumptions, fallbacks, and risks require review
```

This contract should hold for successful runs and failing runs. Failure
artifacts are more valuable than a silent abort.

## Opaque Boundaries

External processes, network calls, database writes, future solver adapters, and
future domain-specific file writers are opaque boundaries. Opaque does not mean
unreviewable. A boundary should record:

```text
boundary kind
tool or target
arguments or schema
working directory or endpoint
input artifacts and hashes
expected outputs when known
observed outputs and hashes
status
success or failure when runtime has executed
risk level
profile policy
source span
```

Domain adapters such as weather APIs, EPW writers, or EnergyPlus-like tools
belong on top of this generic boundary model, not in the core language
identity.

## Fallback And Risk Visibility

Fallbacks should be first-class review facts. Examples include:

```text
unit inference ambiguity
missing data interpolation
external process allowed failure
unsafe or non-reproducible profile dependency
uncertainty independence assumption
linearized uncertainty propagation
solver fallback or nonconvergence
domain adapter output validation warning
```

Risk entries should use stable categories such as:

```text
data_quality
unit_or_quantity
external_boundary
reproducibility
uncertainty
solver_or_numeric
side_effect
claim_boundary
```

Risk levels start as `low`, `medium`, or `high`. Pure checked declarations stay
low; missing-data policy, uncertainty, reproducibility, and solver metadata are
medium by default; external processes and filesystem mutation are high.

## Semantic Diff Target

Code review should compare meaning, not only text. A semantic diff should show
changes in:

```text
input schemas
units and quantities
calculation graph
validation thresholds
with-option policy
external boundaries
generated artifact targets
risk and fallback entries
```

`eng review` produces the compiler-owned static document. A saved run starts
from that document, adds `runtime_result` to matching normalized rows in
inputs, schemas, units/quantities, symbols, time axes, calculations, table
transforms, report outputs, and validations, and records aggregate
`runtime_evidence`. Generated-file and DB-write side effects receive the
same nested `runtime_result` contract in their existing normalized rows;
native model, model-card, metric, and prediction bindings replace generic
unavailable scalar fallbacks with discriminated runtime results containing
computed metrics, coefficients, train/test counts, hashes, output schema, and
case IDs. Boundaries, caches, and fallbacks retain their current enrichment
shapes.

The runtime finalizer compares each normalized section with the static
baseline. It preserves static hashes for unchanged sections, refreshes only
changed section hashes, and recomputes `semantic_hash` with
`semantic_hash_scope = runtime_enriched`. `symbols` and `config_promotions` are
part of the section-hash contract, so value or promotion changes cannot be
hidden from semantic diff.

`eng review --against` uses those hashes for a native meaning-level comparison
without relying on line-by-line source diffs. The payload includes
`section_changes[]` with added, removed, and changed array entries. The
standalone `eng review diff` command and the native IDE Review panel compare
saved ReviewDocuments through the same compiler-owned native diff engine.

## CLI And IDE Targets

The current CLI shape is:

```text
eng review <file.eng>
eng review <file.eng> --json
eng review <file.eng> --output build/review_static
eng review <file.eng> --against build/previous/review.json
eng review diff <old-review.json> <new-review.json>
eng review diff <old-review.json> <new-review.json> --json --output build/review_diff
```

The direct diff command accepts either full `review.json` artifacts or bare
`review_document` JSON. `--output` writes `semantic_diff.json` under the
selected directory; `--json` keeps stdout machine-readable.

The IDE should present a review cockpit with:

```text
variable and unit table
schema and data-boundary panel
TimeSeries and plot panels
validation panel
side-effect and process panel
uncertainty panel
risk and fallback panel
semantic diff panel
```

Runtime report generation consumes either a full `review.json` wrapper or a
bare validated ReviewDocument through the compiler-owned extractor. The saved
run path passes the final enriched wrapper, so the HTML Review fingerprint
matches `review.json.review_document.semantic_hash`. Its Runtime Review
table displays normalized result/evidence/status rows, including side-effect
artifact and SQLite table/row evidence, computed model metrics and hashes,
prediction row/schema evidence, and aggregate side-effect/model/prediction
counts.
Its Validations table uses the normalized full expression and runtime result
instead of the parallel ReportSpec validation list. Specialized solver,
assembly, quality, uncertainty, and non-write DB panels continue to use their
existing typed ReportSpec records until corresponding ReviewDocument rows are
complete.
The dedicated native IDE Uncertainty panel combines those static review records
with typed runtime result arrays: it displays scalar and TimeSeries runtime
results separately from `timeseries_uncertainty_plans[]`, whose
`execution_status = not_executed` remains visible.

The current native IDE Review inspector consumes `review_document` directly for
root counts, semantic hashes, runtime inputs and values, variables/symbols,
unit derivations, schemas, time axes, table transforms, calculation traces,
report outputs, validations, side effects, external boundaries, fallbacks, and
risks. The Model Review inspector also consumes the normalized model and
prediction runtime rows alongside the detailed model-card arrays. Its Semantic
Diff section accepts a full `review.json` artifact or bare
`review_document`, shows section-hash and item-level changes, and automatically
recomputes against the selected baseline after a later run updates the current
ReviewDocument. The VS Code current-file Review panel uses the source-path and
source-hash-matched last-run document when available, including runtime result
and status columns; otherwise it falls back to the fresh static `eng review`
document.

## Completion Checklist

A reviewability slice is complete only when these agree:

```text
compiler-owned semantic record
runtime-updated values and statuses
artifact schema field
HTML report projection
IDE projection
diagnostic or warning path when required
official example or regression fixture
snapshot or artifact test
status and maturity documentation
```
