# Examples

This folder is split by release role. Open `examples/official` first when using
EngLang as a user or release tester.

## Official User-Test Examples

`examples/official` is the release-facing namespace. These examples are copied
into portable packages, shown first in the native IDE, and exercised by release
smoke checks.

```text
official/01_csv_plot
  Supported CSV promote, HeatRate statistics, integration, PlotSpec, SVG,
  report, and standalone packaging path.

official/02_simple_system
  Preview physical system/equation metadata and fixed-step ODE preview.

official/03_integrated_hvac
  Recommended v1.0.3 user test. Combines Args, CSV policies, missing-value
  interpolation, statistics, integration, plotting, reports, and system preview.

official/04_uncertainty_core
  Experimental v1.1 seed for deterministic uncertainty summaries, propagation
  metadata, source diagnostics, and histogram artifacts. It is tested on main
  but not release-supported yet.

official/05_data_driven_modeling
  Experimental v1.2 seed. It is tested on main but not release-supported yet.
```

## Compatibility Regression Examples

The top-level numbered examples keep older paths alive and provide focused
regression coverage. They are intentionally not the first user-facing namespace.

```text
01_units
02_csv_plot
04_plotting
06_simple_system
```

## Diagnostic Fixtures

`05_error_messages` contains examples that are expected to produce specific
diagnostics or warnings. Use them when changing parser, semantic, unit, entry,
or equation diagnostics.

## Data-Quality Fixtures

`07_data_quality` contains CSV policy and runtime data-quality fixtures. Some
files intentionally record parse failures, interpolation, constraint
violations, or unsupported conversion failures in generated artifacts.

## Scratch Files

The native IDE may create `examples/scratch/*.eng` during manual testing. Those
files are user work and are not part of the release contract unless explicitly
added and documented.
