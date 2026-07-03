# Official Examples

This folder is the release-facing core workflow namespace. Open it first when
learning EngLang or validating a portable package.

## Core Workflow Examples

Open these first:

```text
01_csv_plot
  Typed CSV promote, unit-aware calculations, TimeSeries summary statistics,
  integration metadata, PlotSpec/SVG output, report, review, and standalone
  packaging smoke.

07_functions_imports
  Top-level execution, static file import, importable const values,
  function-local bindings, unit-checked parameters, dimension-checked returns,
  CLI print, and explicit summary CSV export.

08_print_export_summary
  Mini scalar summary path for args, reusable const, unit-aware print
  interpolation, and explicit one-row summary CSV output.

09_command_where_with
  Parenthesis-light built-in workflow verbs, scoped where locals, with option
  blocks, statistics/integration, print/export output, and plot display
  options.

10_path_policy
  Typed path arguments, pure path helpers, runtime exists checks, and
  environment dependency provenance.

11_read_only_io
  Read-only text/json/toml inputs, source-hash provenance, and a workflow that
  combines typed CSV data with auxiliary configuration files.

12_write_output_manifest
  Explicit summary CSV export, write text/json outputs, overwrite policy, and
  output manifest generation.

14_run_log
  Structured runtime messages with print plus log info/debug/warn/error and
  generated run_log.json artifacts.

15_process_result
  External process surface with run command, ProcessResult, and generated
  process_results.json artifacts.

16_test_assert_golden
  Local workflow verification with named test blocks, unit-aware assert
  statements, golden artifact comparison, and test_results.json.
```

## Additional Review Examples

```text
13_file_operations
  Explicit copy/move/delete/mkdir filesystem mutation surface, confirmation
  metadata, generated-output mutation boundaries, and output manifest records.

19_class_object
  Typed class declarations, object literals, nested references, validation,
  metadata methods, immutable copy-with, field access metadata, object report
  sections, and IDE/LSP metadata. This is not runtime object dispatch.
```

Advanced solver smoke fixtures live under `examples/advanced_solver`. They are
implementation regression coverage, not first-user tutorial content.

Compatibility regression examples live under `examples/compat`. Diagnostic and
data-quality fixtures live under `examples/diagnostics`. Internal implementation
fixtures live under `examples/internal`.
