# CLI Specification

The core user-facing CLI is `eng.exe`. Portable tester IDE releases also ship
`eng-ide.exe` as a native GUI companion.

## Commands

```text
eng.exe doctor
eng.exe new <project_name>
eng.exe check <file.eng> [--review]
eng.exe review <file.eng> [--json]
eng.exe fmt <file.eng> [--check|--write]
eng.exe ide-check <file.eng>
eng.exe jit-plan <file.eng> [--backend <name>]
eng.exe jit-bench <file.eng> [--iterations N] [--backend <name>] [--<arg> <value>...]
eng.exe run <file.eng> [--open-report] [--save-artifacts] [--skip-unchanged] [--<arg> <value>...]
eng.exe cache invalidate [--manifest build/result/cache_manifest.json] [--all|--owner-kind <kind>|--owner-name <name>|--cache-key-hash <hash>] [--dry-run]
eng.exe build <file.eng> [--standalone] [--profile repro]
eng.exe view <result.engres>
eng.exe test <project_or_examples>
eng-ide.exe
eng-ide.exe --smoke
eng-lsp.exe --smoke
eng-lsp.exe --snapshot <file.eng>
eng-lsp.exe --snapshot-check <file.eng>
```

## `eng doctor`

Checks the local EngLang environment.

Current checks:

```text
Runtime
Standard library
Unit registry
Plot renderer
Report generator
Write permission
Example files
```

Success prints `Ready.` and returns exit code 0.

## `eng check <file.eng> [--review]`

Checks source and writes optional review metadata. It does not execute the
top-level workflow.

Current diagnostics:

```text
E-SYNTAX-DECL-001      := is not EngLang syntax
E-PUBLIC-ANNOTATION-001 schema columns require explicit quantity/unit annotations
E-DIM-ADD-001          Length + DimensionlessNumber is invalid
E-DIM-ADD-002          DimensionlessNumber + power quantity is invalid
E-DIM-ADD-003          AbsoluteTemperature + DimensionlessNumber is invalid
E-DIM-ADD-004          other physical quantity + DimensionlessNumber is invalid
E-RESERVED-KEYWORD-001 reserved keyword binding is invalid
W-QTY-AMBIG-001        ambiguous quantity warning
E-SCHEMA-PROMOTE-001   unknown schema in promote csv
E-SCHEMA-CSV-001       CSV source cannot be read
E-SCHEMA-CSV-002       CSV source missing required columns
E-SCHEMA-MISSING-001   missing policy references unknown column
E-CONFIG-SOURCE-001    JSON/TOML config source cannot be read or parsed
E-CONFIG-MISSING-FIELD config source is missing a required schema field
E-CONFIG-UNKNOWN-FIELD config source contains a field outside the schema
E-CONFIG-TYPE-MISMATCH config field type does not match the schema
E-CONFIG-NULL-NOT-OPTIONAL config field is null but schema field is not Optional[T] or T?
E-TABLE-UNKNOWN-COLUMN table filter predicate references a column outside the promoted schema
E-TABLE-PREDICATE-TYPE table filter predicate compares a schema column with an incompatible literal
E-TABLE-JOIN-KEY-MISMATCH join key does not compare columns from the joined tables
E-TABLE-SCHEMA-MISMATCH table join key columns have incompatible schema types
E-NET-INVALID-URL     network boundary URL is not absolute HTTP(S)
E-NET-RETRY-POLICY    network retry value is not an integer from 0 to 5
E-NET-TIMEOUT         network timeout value is not a positive finite duration
E-NET-BODY-SIZE-LIMIT network response body size limit is not a positive whole-byte size
E-NET-HASH-MISMATCH   network response SHA-256 does not match expected_sha256
E-NET-UNPINNED-REPRO  repro profile network boundary lacks a pinned response file or expected_sha256
E-CACHE-HASH-MISMATCH cache record observed hash does not match expected hash
E-CACHE-DIR           cache_dir is not a safe relative directory
E-CACHE-KEY-NONDETERMINISTIC cache_key contains nondeterministic or secret-dependent input
E-CACHE-TTL           cache_ttl is not a positive finite duration
E-CACHE-UNHASHED-REPRO repro profile cache record has no observed hash
W-CACHE-STALE         cache directory contains an entry outside the current cache manifest
E-ARGS-UNKNOWN-001     CLI Args flag does not match `args { ... }`
E-ARGS-REQUIRED-001    required Args field was not provided for run
E-ARGS-TYPE-001        Args value cannot be converted to the declared type
E-ARGS-CSV-001         CSV promotion references an Args field without a value
E-ARGS-CONFIG-001      config promotion references an Args field without a value
E-SCRIPT-001           `script` blocks are not supported as execution roots
W-STATS-SUM-001        HeatRate summed over Time should use integrate
E-EQ-BOOL-001          physical equation used == instead of eq
E-EQ-UNIT-001          physical equation dimensions do not match
E-UNC-SOURCE-001      missing or unknown uncertainty source reference
E-UNC-SOURCE-002      referenced binding is not an uncertainty source
E-UNC-ARGS-001        missing or malformed required uncertainty argument
E-UNC-ARGS-002        invalid numeric/range/count/transform uncertainty argument
E-UNC-ARGS-003        unsupported uncertainty option
E-DOMAIN-CONTRACT-001  domain has no across variable
E-DOMAIN-CONTRACT-002  domain has no through variable
E-DOMAIN-CONTRACT-003  domain has no conservation contract
E-DOMAIN-VAR-001       domain variable uses an unknown quantity kind
E-PORT-DOMAIN-001      component port references an unknown domain
E-PORT-DOMAIN-002      generic domain reference has wrong argument count
E-CONNECT-ENDPOINT-001 connection endpoint is not Component.port
E-CONNECT-UNKNOWN-PORT connection endpoint does not resolve to a port
E-CONNECT-DOMAIN-MISMATCH connected ports have incompatible domains
E-CONNECT-MEDIUM-MISMATCH connected generic ports have incompatible Medium arguments
E-CONNECT-FRAME-001    connected generic ports have incompatible Frame arguments
E-CONNECT-AXIS-001     connected generic ports have incompatible Axis arguments
E-ASSEMBLY-UNDERDETERMINED component assembly has fewer equations than unknowns
E-ASSEMBLY-OVERDETERMINED  component assembly has more equations than unknowns
W-ASSEMBLY-ALGEBRAIC-LOOP component assembly has an algebraic dependency loop
E-PROCESS-001          run command is supported only at top level
E-PROCESS-BINDING-001  run command must bind a ProcessResult
E-PROCESS-CMD-001      run command requires a command string
E-PROCESS-BINDING-002  ProcessResult binding conflicts with an existing binding
E-PROCESS-ENV-001      process env option must be an inline portable-name object
E-PROCESS-CWD-001      process cwd option must be a path expression
E-PROCESS-TIMEOUT      process timeout option must be a positive duration
E-PROCESS-RETRY-POLICY process retry option must be an integer from 0 to 5
E-PROCESS-ALLOW-FAILURE process allow_failure option must be true or false
E-SAMPLING-COUNT-INVALID sample count must be a positive integer
E-SAMPLING-RANGE-UNIT generated sample ranges must use compatible units
E-SAMPLING-SEED-INVALID sample seed option must be a non-negative integer
E-SAMPLING-SEED-MISSING repro profile requires seeded random or LHS sampling
E-CASE-ID-DUPLICATE sample/case table contains a duplicate case_id
E-CASE-DIR-COLLISION multiple case IDs resolve to the same case directory
E-CASE-OUTPUT-MISSING case step did not create an expected case output
E-CASE-STEP-FAILED case step reported a failed status
W-CASE-SKIPPED-CACHE case was skipped from case cache
E-DB-CONNECT          SQLite connection or table target cannot be resolved
E-DB-SCHEMA-MISMATCH DB write source/table schema does not match
E-DB-KEY-MISSING     DB upsert key is missing or outside the source table schema
E-DB-TRANSACTION-FAILED SQLite write transaction failed
E-DB-SAFE-PROFILE    safe profile rejects native DB write side effects
W-PROFILE-REPRO-DB   repro profile records DB write hashes and manifest metadata
E-MODEL-FEATURE-MISSING model spec/card has no feature contract
E-MODEL-TARGET-MISSING model spec/card has no target contract
E-MODEL-CARD-MISSING model artifact process has no model card output
W-MODEL-EXTRAPOLATION prediction manifest reported schema/extrapolation warning
E-PATH-INVALID         generated output path is empty
E-PATH-TRAVERSAL       generated output path contains a parent-directory segment
E-PATH-OUTSIDE-OUTPUT-ROOT generated output path is absolute/rooted
E-IO-JSON-PARSE        read json source is not valid JSON
E-IO-TOML-PARSE        read toml source is not valid TOML
E-IO-JSON-FIELD-ACCESS-001 read json values do not support direct field access
E-TEST-001             test block syntax is invalid
E-TEST-NAME-001        test block name is missing or invalid
E-ASSERT-001           assert is outside a test block
E-ASSERT-002           assert expression syntax is invalid
E-ASSERT-UNIT-001      assert operands use incompatible units
E-ASSERT-TOL-001       tolerance is only valid with equality-style checks
E-GOLDEN-001           golden check syntax is invalid
E-GOLDEN-002           golden check expected path must use file("...")
E-STDLIB-MODULE-UNKNOWN use/import names an unknown `eng.*` stdlib module
W-STDLIB-MODULE-PLANNED use/import names a planned stdlib module
W-STDLIB-MODULE-INTERNAL use/import names an internal stdlib module
```

`--review` writes:

```text
build/check/<source-stem>.review.json
```

Review JSON includes:

```text
review_schema_version
review_document
syntax_summary
quantity_completion_count
diagnostics
variable_table
warning_list
plot_manifest
workflow
args_summary
arg_values
inferred_declarations
expected_types
hover_hints
type_info
unit_derivations
unit_conversion_table
axis_info
stats_info
integrations
system_summary
domain_summary
component_summary
connection_summary
assembly_summary
schema_summary
schemas
csv_promotions
```

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check failure
```

## `eng review <file.eng> [--json]`

Builds the compiler-owned review artifact and prints the normalized
`review_document` projection. The default output is a reviewer summary with:

```text
review status
workflow signature
input/symbol/calculation/validation counts
side-effect/external-boundary/fallback/risk counts
external boundary rows
fallback rows
risk rows
```

`--json` prints only `review_document` as formatted JSON. This command does not
execute the workflow, so runtime hashes, process exits, and generated file
status still belong to `eng run --save-artifacts`.

## `eng fmt <file.eng> [--check|--write]`

Formats an EngLang source file using the source-preserving formatter used by
the official example gate.

Current behavior:

```text
- no flag: writes formatted source to stdout
- --check: returns exit code 2 if the file would change
- --write: rewrites the file in place when formatting changes are needed
- --check and --write are mutually exclusive
```

The formatter normalizes block indentation and trailing whitespace while
preserving comments and source text inside strings. It does not perform AST
rewrites or semantic lowering.

Exit code:

```text
0 success
1 IO/tooling failure
2 usage error or source would change under --check
```

## `eng ide-check <file.eng>`

Prints the same review JSON used by `eng check --review` to stdout instead of
writing it under `build/check`.

This command is intended for IDE tools and extensions that need diagnostics,
hover hints, type information, symbols, Args metadata, schema metadata, and
completion counts without managing generated review files.

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check failure
```

## `eng jit-plan <file.eng>`

Prints internal `eng-kernel-plan-v1` JSON for runtime optimization track hot-kernel planning.
This command does not compile native code and does not change runtime
execution. Its current backend is `interpreter-fallback`.

Supported backend requests are `auto`, `interpreter-fallback`, and
`native-preview`. `native-preview` records a request but still selects
`interpreter-fallback` with `backend_selection.status = not_available`.

Each candidate includes source, reason, lowering status, operation list, and a
coarse planning estimate:

```text
estimated_rows
input_count
output_count
operation_count
scan_count
complexity
notes
```

These estimates are for inspection and benchmark selection only. They are not
measured performance data.

Current candidate kinds:

```text
timeseries_arithmetic
timeseries_integrate
statistics_fusion
system_residual
```

Example:

```bat
eng.exe jit-plan examples\official\01_csv_plot\main.eng
```

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check failure
```

## `eng jit-bench <file.eng>`

Runs an internal `eng-jit-bench-v1` benchmark harness for runtime optimization track planning.
The harness measures the current interpreter/runtime path for a small number of
iterations and includes the current `eng-kernel-plan-v1` metadata in the same
JSON output.

Current behavior:

```text
- default iterations: 3
- allowed iterations: 1..100
- `--backend <name>` records backend selection metadata
- other `--<arg> <value>` flags are forwarded as Eng Args overrides
- `benchmark_targets` records which internal target families were observed in
  the current source's kernel plan
- `jit.status` is `not_available`
- comparison_policy is `no-speedup-claim`
```

Example:

```bat
eng.exe jit-bench examples\official\01_csv_plot\main.eng --iterations 1
```

The repository benchmark catalog lives under `benchmarks/` (`B01_csv_heat_rate`
through `B06_nonlinear_solver`). Each case has local input data, `main.eng`,
and `expected.json`; `.\dev.bat jit-check` runs the catalog through
`jit-bench --iterations 1` and verifies target coverage, measured interpreter
timing, generated artifacts, and result correctness fragments.

Exit code:

```text
0 success
1 IO/tooling failure
2 compile/check/runtime setup failure
```

## `eng-ide.exe`

Launches the Tauri/WebView portable tester IDE.

Current tester IDE features:

```text
- Explorer for examples, stdlib, and docs
- source editor with multi-file tabs
- live check_source diagnostics for unsaved edits
- toolbar diagnostic counts and Problems panel
- caret completion insertion for symbols, keywords, quantity kinds, units, and starter snippets
- compiler-derived symbol metadata
- save/check/run commands
- generated report and plot opening
- Terminal tab with `clear`, `reset`, `check`, `run`, and one-line top-level commands
- Variables table populated after successful runs
- Variables/Plot/Run inspector tabs with an in-IDE PlotSpec viewer beside the variable table
```

`eng-ide.exe --smoke` checks the non-GUI path for release packages. It verifies
that examples are discoverable, compiler completion metadata is available, and
the official domain/component track example produces domain, component,
connection, and assembly metadata.

## `eng-lsp.exe`

Starts the stdio LSP server when no flags are supplied. The release package
also supports advanced editor-tooling smoke and metadata JSON commands:

```bat
eng-lsp.exe --smoke
eng-lsp.exe --snapshot examples\internal\06_domain_port\main.eng
eng-lsp.exe --snapshot-check examples\official\01_csv_plot\main.eng
```

`--smoke` verifies editor metadata extraction for the official CSV workflow and
the official domain/component track metadata. `--snapshot` emits
`eng-lsp-snapshot-v1` JSON with diagnostics, completion items, and hover items.
Domain/component files include hover `kind`/`status` metadata and completion
labels such as `Thermal`, `RoomBoundary`, `RoomBoundary.heat`,
`component_graph`, and `connection_set_1.across_T_1`.

## `eng run <file.eng> [--open-report] [--save-artifacts] [--skip-unchanged] [--<arg> <value>...]`

Runs the file's top-level workflow through bytecode v1 and the native VM.
By default, result/review/report/run-log/process-results/test-results/plot/
output-manifest payloads remain runtime objects in memory. `--save-artifacts`
writes those objects to disk.
`--skip-unchanged` compares the current run input lock against
`build/result/run_lock.json`. `static_run_plan.json` is generated before
bytecode execution from the checked semantic program. When source, profile, CLI
args, and dependency hashes match the prior lock, `run_plan.json` records
`rerun_decision.decision = skip` and `rerun_status = skipped`; otherwise it
records a normal executed rerun status. The skip path also verifies saved
artifact hashes from the prior lock before reusing the saved result.
Explicit `export`, `write`, and constrained `copy/move/delete/mkdir` statements
write or mutate files under `build/result` and are recorded in
`output_manifest.json`. Explicit `run command` statements execute during the
run and are captured in `process_results.json` when artifacts are saved. Named
`test` blocks run after generated artifacts are available and are captured in
`test_results.json`.

Execution model:

```text
1. Root `args { ... }` declares CLI-bindable arguments.
2. Top-level statements form the executable workflow.
3. `script` blocks are rejected with E-SCRIPT-001.
```

Saved artifacts:

```text
build/
  <source-stem>.engbc
  result/
    result.engres
    review.json
    report.html
    report_spec.json
    static_run_plan.json
    run_plan.json
    run_lock.json
    run_log.json
    process_results.json
    test_results.json
    output_manifest.json
    plots/
      plot_spec.json
      plot_manifest.json
      timeseries.svg
```

For the domain/component metadata track, `result.engres` includes
`typed_payload.component_solutions`. This path assembles homogeneous connection
constraints into residuals, solves square linear residual graphs with the dense
linear solver, and records convergence status plus failure/limitation artifacts
for skipped non-square or overdetermined graphs. `review.json` and
`report_spec.json` also include `assembly_summary.domain_plans` and
`assembly_summary.solver_preview` so tools can identify `multi_domain_preview`
graphs and the explicit
nonlinear/DAE/delay/Predictor/adapter limitation statuses. This is not a production
multi-domain solver claim.

`--open-report` implies `--save-artifacts` and attempts to open the generated
`report.html` with the OS default browser.

Args flags are matched against root `args { ... }` fields. Defaults are used
when available, primitive typed values are normalized, and resolved values are
recorded in `arg_values`.

Current typed Args conversion:

```text
String/Path/CsvFile/DirectoryPath  recorded as text/path
Bool/Boolean         true/false, yes/no, on/off, 1/0 -> true/false
Int/Integer          whole-number signed integer
Count/usize/u32/u64  non-negative whole-number count
Float/Number         finite numeric value
Duration             s, min, h -> normalized seconds such as `600 s`
```

```bat
eng.exe run examples\official\01_csv_plot\main.eng --save-artifacts --input data/sensor.csv
```

## `eng cache invalidate`

Invalidates cache paths listed in a cache manifest. By default, the command
reads `build/result/cache_manifest.json`.

It requires `--all` or a selector such as `--owner-kind`, `--owner-name`, or
`--cache-key-hash`. Use `--dry-run` to list matching paths without deleting
them. Paths outside the current working directory are refused.

## `eng build <file.eng> --standalone --profile repro`

Creates a runnable standalone package bundle:

```text
dist/
  <model>-standalone/
    eng.exe
    run.bat
    ARGS_HELP.txt
    <model>.engpkg
    <model>.lock
    <model>.engbc
    <model>.review.html
    source/
      <file.eng>
```

For CSV promotions that use relative paths, the referenced CSV files are copied
into the bundle at the same relative path from `source/<file.eng>`. Running
`run.bat` executes the bundled `eng.exe run source\<file.eng> --save-artifacts`
and forwards extra Args flags. It creates normal `build/result` artifacts inside
the bundle.

The `.engpkg` records package format, runtime ABI, repro profile, runner,
engine, source and artifact roots, source, bytecode, source hash, bytecode hash,
workflow signature, Args schema, Args field count, Args help
path, dependency count, dependency paths, and dependency hashes. The lock file
records runtime/compiler/package/bytecode/result/report/plot format versions,
source and bytecode hashes, workflow signature, dependency count, dependency hashes, and
`profile = repro`.

See [Standalone package reference](standalone_package.md) for the
full bundle layout, manifest and lock field tables, hash semantics, and the
reserved `model.exe`/AOT boundary.

## `eng view <result.engres>`

Prints the result path, the sibling `report.html` and `report_spec.json` paths, and the plot manifest path when it exists.

The long-term result viewer will be connected to the typed `.engres` payload.

## `eng new <project_name>`

Creates a starter EngLang project:

```text
<project_name>/
  main.eng
  data/
    sensor.csv
```

## `eng test <project_or_examples>`

Runs official smoke checks:

```text
- official user-test examples check first
- compatibility regression examples check after official examples
- unit mismatch example produces errors
- ambiguous power example produces a warning
- HeatRate sum example produces W-STATS-SUM-001
- physical equation using == produces E-EQ-BOOL-001
- equation unit mismatch produces E-EQ-UNIT-001
- missing CSV column example produces errors
- missing uncertainty source example produces E-UNC-SOURCE-001
- invalid uncertainty argument example produces E-UNC-ARGS-001/002/003
- `script` execution-root syntax produces E-SCRIPT-001
- official plotting example produces report and PlotSpec artifacts
- official histogram example produces binned PlotSpec artifacts
- Args CLI binding produces CSV run artifacts
- typed Args values are normalized and invalid typed Args values produce E-ARGS-TYPE-001
- official CSV example produces runtime optimization track kernel candidates
- bad DateTime and bad numeric CSV fixtures record parse_failures
- numeric missing interpolation fixture executes
- constraint violation fixture records upper-bound policy violation
- official simple system example produces system report artifacts
- official run-log example produces `run_log.json` and level metadata
- official process-result example produces `process_results.json` and ProcessResult metadata
- official test/assert/golden example produces `test_results.json` and passing check metadata
```
