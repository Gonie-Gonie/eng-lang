# Reviewability As A Language Feature

Status: current public artifact family is stable for the v0.1.0 package scope.
The explicit Review IR, semantic diff, and `eng review` command described here
are implementation targets unless a status document marks a narrower slice as
supported.

## Core Principle

```text
Every engineering computation must produce a reviewable semantic trace.
```

EngLang should not ask reviewers to trust generated code line by line. It
should expose meaning-level artifacts: inputs, units, schemas, axes,
calculations, validations, side effects, external boundaries, assumptions,
fallbacks, and risks.

## Current Artifact Baseline

The current package already writes the artifact family that Review IR should
normalize:

```text
build/result/result.engres
build/result/review.json
build/result/report.html
build/result/report_spec.json
build/result/run_log.json
build/result/process_results.json
build/result/test_results.json
build/result/output_manifest.json
```

These artifacts are the public proof path for typed data boundaries,
TimeSeries statistics, plots, reports, validations, explicit side effects, and
package/IDE inspection.

## Review IR Target

Review IR should be the shared semantic document between compiler, runtime,
report generation, CLI review commands, and IDE panels.

Initial node families:

```text
ReviewDocument
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

The compiler can fill static meaning: source spans, declarations, quantities,
schemas, commands, options, validation expressions, process declarations, and
side-effect boundaries. Runtime can fill values, statuses, hashes, process exit
codes, generated outputs, test results, and profile diagnostics.

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

This is planned until Review IR has a stable enough internal shape.

## CLI And IDE Targets

The target CLI shape is:

```text
eng review <file.eng>
eng review diff <old-review.json> <new-review.json>
```

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

The current IDE already inspects several artifact families. The next cleanup is
to make those panels consume a normalized Review IR instead of parallel
metadata shapes.

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

