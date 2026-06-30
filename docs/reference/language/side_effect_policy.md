# Side Effect And General Programming Policy

Status: GP-1 path policy, GP-2 read-only I/O, GP-3 write/export hardening,
GP-4 constrained copy/move/delete file operations, GP-5 structured runtime
log messages, GP-6 explicit external process execution, GP-7 test/assert/
golden support, and safe/normal/repro profile basics are implemented for
the current public package. Broader filesystem mutation outside generated-output boundaries,
network, workspace-wide test discovery, and full process sandboxing remain
planned tracks, not supported behavior.

EngLang is not trying to become a fully general replacement for Python, MATLAB,
or R. Real engineering workflows still need practical file, path, config,
logging, process, and test support. The policy is to provide those operations
as typed, explicit, and provenance-aware language/runtime features.

## One-Line Rule

```text
EngLang can manipulate the outside world, but it must show what changed,
what was read, and what external state influenced the result.
```

## Effect Categories

| Category | Examples | Initial Policy |
|---|---|---|
| Pure path manipulation | `join`, `parent`, `stem`, `extension` | Pure, allowed in args defaults and const when inputs are pure |
| Environment-dependent read | `exists`, `env`, `today`, `now` | Allowed in normal profile, recorded in provenance |
| Read-only I/O | `read text`, `read json`, `read toml` | UTF-8 raw string reads; source hash is recorded; direct JSON field access is rejected |
| Typed data boundary | `promote csv/json/toml as Schema` | Preferred for engineering data |
| Write/export | `write text`, `write json`, `export summary to csv` | Explicit target required; changed overwrite requires `overwrite = true`; generated outputs are manifest-recorded |
| File operations | `copy`, `move`, `delete`, `mkdir`, `list` | Copy/move/delete seed implemented under explicit output boundaries; broader operations planned |
| Runtime messages | `print`, `log info`, `log warn`, `log debug`, `log error` | CLI/debug output plus structured `run_log.json` metadata |
| External process | `result = run command ... with { ... }` | Explicit `ProcessResult`; command/cwd/args/exit/stdout/stderr recorded |
| Test checks | `test { assert ...; golden ... }` | Runtime verification plus structured `test_results.json` metadata |
| Network | `download url(...) to file(...)` | Long-term only; repro profile requires hash/cache |

## Types

The initial type vocabulary should stay small:

```text
FilePath
DirectoryPath
CsvFile
JsonFile
TomlFile
TextFile
ReportFile
PlotFile
ProcessResult
```

Path helpers are pure unless they query the filesystem:

```eng partial
summary_file = join(args.output, "summary.csv")
parent_dir = parent(args.input)
name = stem(args.input)
```

Environment-dependent helpers should be visible in review/provenance:

```eng partial
input_exists = exists args.input
print "input exists = {input_exists}"
```

Implemented behavior in the current package scope:

```text
- file("...") and dir("...") are accepted in args defaults.
- join(...), parent(...), stem(...), and extension(...) are typed pure path helpers.
- exists path_expression is typed as Bool and resolved relative to the source file.
- read text/json/toml path_expression is typed as String and resolved relative
  to the source file.
- review.json includes top-level environment_dependencies.
- result.engres and report_spec.json include provenance.environment_dependencies.
- read-only I/O dependencies record the resolved source path and source hash
  when the source file is present.
- write text/json output statements are top-level workflow statements.
- export/write outputs are written under `build/result`.
- an identical existing output is accepted as an idempotent rerun.
- replacing different existing contents requires `with { overwrite = true }`.
- output_manifest.json records generated file paths and content hashes.
- copy can copy a source-relative UTF-8 text file or generated output file into
  `build/result`.
- move operates on generated output paths under `build/result`.
- delete operates on generated output paths under `build/result`.
- move and delete require `with { confirm = true }`.
- delete dir(...) also requires `with { recursive = true }`.
- file operations are recorded in review.json and output_manifest.json.
- print output remains a lightweight CLI/debug stream.
- log debug/info/warn/error statements are type-checked with the same
  interpolation policy as print.
- saved runs write `run_log.json` with level, message, source line, and index
  metadata for IDEs and tools.
- output_manifest.json records the saved run-log artifact.
- run command statements bind `ProcessResult` values.
- process options support `args`, `cwd`, and `allow_failure`.
- process options support `expected_outputs` for files that must exist after
  the command exits; saved runs record existence and hashes in
  `process_results.json` and `output_manifest.json`.
- saved runs write `process_results.json` with command, args, cwd, exit code,
  stdout, stderr, duration, expected outputs, status, and line metadata.
- output_manifest.json records the saved process-results artifact.
- test blocks group runtime assertions and golden artifact comparisons.
- assert operands are checked for compatible quantity dimensions.
- golden checks compare generated artifacts with source-relative expected files.
- saved runs write `test_results.json` with pass/fail metadata.
- output_manifest.json records the saved test-results artifact.
```

## Read Policy

Raw reads are useful for scripts, but engineering data should use typed
promotion where possible:

```eng partial
notes = read text file("notes.txt")
raw_config = read toml file("case.toml")
config = promote toml file("case.toml") as CaseConfig
data = promote csv args.input as SensorData
```

Rules:

```text
- read text/json/toml returns UTF-8 raw text in the current runtime
- structured JSON/TOML values are deferred to typed promote/object support
- imported const/function files may not hide read effects
- source hashes are recorded when data affects runtime artifacts
- typed promote remains the recommended data boundary for official examples
```

## Write And Export Policy

Writes must name their target:

```eng partial
write text "outputs/log.txt", "finished"
write json "outputs/summary.json", E_coil

export summary to csv "summary.csv" {
    E_coil as kWh with ".2"
}
with {
    overwrite = true
}
```

Rules:

```text
- overwrite default is false for changed existing output contents
- an identical existing file is accepted so official examples can be rerun
- output targets are constrained under build/result in the current runtime
- output files appear in review metadata and output_manifest.json
- export is preferred over ad hoc write for reproducible artifacts
- `write json` writes scalar quantities as JSON objects with value/unit metadata
- `output_manifest.json` records generated artifact kind, path, and hash
```

## Destructive Operation Policy

Destructive operations must be visibly intentional:

```eng partial
delete "outputs/temp.csv"
with {
    confirm = true
}

delete dir("outputs/tmp")
with {
    recursive = true
    confirm = true
}
```

Rules:

```text
- copy is lower risk but still recorded when it changes outputs
- copy targets are constrained under `build/result`
- move records both source and destination and is constrained under `build/result`
- delete file requires confirm
- delete directory requires recursive=true and confirm=true
- `safe` rejects copy/move/delete before execution
- `repro` records move/delete profile diagnostics in runtime artifacts
```

## Runtime Message Policy

Use `print` for direct CLI/debug feedback and `log <level>` when a message
should be structured for tools:

```eng partial
print "Loaded {sensor.rows} rows from {args.input}"
log info "Q mean = {mean_Q: .2 kW}"
log warn "review high load case"
log debug "daily energy raw = {E_day: .2 kWh}"
log error "operator acknowledgement required"
```

Rules:

```text
- supported log levels are debug, info, warn, and error
- `warn "..."` is not a separate command; use `log warn "..."`
- log interpolation is type- and unit-checked like print
- CLI output prefixes structured log messages as [level] message
- saved runs write build/result/run_log.json
- run_log.json is a runtime artifact, not a replacement for report/export
```

## Process Policy

External tools are needed for adapters such as EnergyPlus, FMU/FMI, and legacy
solvers, but they must not be hidden:

```eng partial
result = run command "energyplus.exe"
with {
    args = ["-w", args.weather, "-d", args.output, args.idf]
    cwd = dir("runs/case01")
    expected_outputs = ["eplusout.sql", "eplusout.err"]
}

log info "EnergyPlus process was executed"
```

`ProcessResult` should include:

```text
exit_code
success
stdout
stdout_hash
stderr
stderr_hash
command
tool_version
args
cwd
duration
status
line
```

Implemented rules:

```text
- `run command` must bind a result, for example `result = run command "cmd"`
- args are string arrays such as `args = ["/C", "echo", "ok"]`
- cwd is a typed path expression resolved source-relative when relative
- tool_version is an explicit string metadata option recorded in review and
  process artifacts
- stdout and stderr are recorded with stable hashes for compact comparison
- expected outputs are path-expression arrays resolved relative to the process
  cwd and recorded with existence status plus file hashes when readable
- non-zero exit codes fail the run by default
- `with { allow_failure = true }` records a failed process as a ProcessResult
- process results are written to build/result/process_results.json on saved runs
```

## Test Policy

Tests should be close enough to examples and workflows that users can trust a
run without reading every artifact manually:

```eng partial
test "summary values" {
    assert mean_Q > 0 kW
    assert E_coil == 1.26 kWh within 0.02 kWh
    golden "summary.csv" matches file("golden/summary.csv")
}
```

Rules:

```text
- `test` blocks are top-level workflow declarations
- `assert` is valid only inside a test block
- quantity comparisons require compatible dimensions
- golden expected files use source-relative file("...") paths
- generated artifacts are compared from build/result
- saved runs write build/result/test_results.json
- failed tests fail the run after artifacts are written
```

## Profiles

Runtime profiles should control the side-effect envelope:

| Profile | Policy |
|---|---|
| `safe` | Explicit workflow export/write/file-operation/process effects are rejected before execution |
| `normal` | Default profile; supported effects are allowed with provenance and artifact records |
| `repro` | Supported effects are allowed, with profile diagnostics for environment dependencies, process runs, and move/delete mutations |

Run profiles are selected through the CLI:

```powershell
eng run main.eng --profile safe
eng run main.eng --profile normal
eng run main.eng --profile repro
```

Profile records are written to `result.engres`, `run_log.json`, and
`output_manifest.json`. Do not add a language feature that silently changes the
active profile inside source code.

## Stdlib Module Plan

Bundled stdlib modules should grow in this order:

```text
eng.path     FilePath, DirectoryPath, join, parent, stem, extension, exists
eng.io       read/write text/json/toml, source hash helpers
eng.fs       copy, move, delete, mkdir, list
eng.config   promote toml/json as schema
eng.log      print/log <level> and unit-aware formatting helpers
eng.process  run command, ProcessResult
eng.test     test/assert/golden support
eng.net      offline/fixture network boundary records now; live download/cache/hash later
eng.cache    explicit cache-key records and hit/miss manifests now; replay/invalidation later
```

## Implementation Phases

| Phase | Scope | Public Claim |
|---|---|---|
| GP-1 | path types/helpers, `exists`, side-effect docs | implemented in v0.3 |
| GP-2 | read text/json/toml, source hashes | implemented in v0.4 |
| GP-3 | write text/json, export hardening, output manifest | implemented in v0.5 |
| GP-4 | copy/move/delete and side-effect manifest | implemented in v0.6 |
| GP-5 | print/log level formatting and run log artifact | implemented in v0.7 |
| GP-6 | external process and `ProcessResult` | implemented in v0.8 |
| GP-7 | test block/assert/golden support | implemented in v0.9 |
| GP-8 | safe/normal/repro profile basics | implemented in v1.0 |
| GP-9 | network/download/cache/hash | optional, after test policy |
| GP-10 | IDE side-effect panels and output file navigation | grows across phases |

The current implementation deliberately stops at path helpers, `exists`,
read-only source-hashed inputs, generated output writes, constrained
copy/move/delete operations under `build/result`, structured runtime message
artifacts, explicit external process records, local test/assert/golden records,
and CLI-selected safe/normal/repro profile basics. Broader filesystem access,
network effects, workspace-wide test discovery, and full process sandboxing
remain outside the supported surface until their policy slices land.
