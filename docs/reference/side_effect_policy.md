# Side Effect And General Programming Policy

Status: GP-1 path policy, GP-2 read-only I/O, and GP-3 write/export hardening
are implemented for `v0.5-preview`. Broader filesystem mutation, process,
network, and test support remain planned tracks, not supported preview
behavior.

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
| Read-only I/O | `read text`, `read json`, `read toml` | UTF-8 raw string reads; source hash is recorded |
| Typed data boundary | `promote csv/json/toml as Schema` | Preferred for engineering data |
| Write/export | `write text`, `write json`, `export summary to csv` | Explicit target required; changed overwrite requires `overwrite = true`; generated outputs are manifest-recorded |
| File operations | `copy`, `move`, `delete`, `mkdir`, `list` | Destructive operations require explicit confirmation/options |
| External process | `run command ... with { ... }` | Experimental; command/cwd/args/exit/stdout/stderr recorded |
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

Implemented behavior through `v0.5-preview`:

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
- read text/json/toml returns UTF-8 raw text in the current preview
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
- output targets are constrained under build/result in the current preview
- output files appear in review metadata and output_manifest.json
- export is preferred over ad hoc write for reproducible artifacts
- `write json` writes scalar quantities as JSON objects with value/unit metadata
- `output_manifest.json` records generated artifact kind, path, and hash
```

## Destructive Operation Policy

Destructive operations must be visibly intentional:

```eng partial
delete file("build/temp.csv")
with {
    confirm = true
}

delete dir("build/tmp")
with {
    recursive = true
    confirm = true
}
```

Rules:

```text
- copy is lower risk but still recorded when it changes outputs
- move records both source and destination
- delete file requires confirm
- delete directory requires recursive=true and confirm=true
- repro/safe profiles may warn or reject move/delete
```

## Process Policy

External tools are needed for adapters such as EnergyPlus, FMU/FMI, and legacy
solvers, but they must not be hidden:

```eng partial
result = run command "energyplus.exe"
with {
    args = ["-w", args.weather, "-d", args.output, args.idf]
    cwd = dir("runs/case01")
}

validate result.exit_code == 0
```

`ProcessResult` should include:

```text
exit_code
stdout
stderr
command
args
cwd
duration
```

## Profiles

Runtime profiles should control the side-effect envelope:

| Profile | Policy |
|---|---|
| `safe` | file write/delete/network/process forbidden |
| `normal` | allowed with provenance |
| `repro` | external effects restricted; env/time/network/process require strong metadata |

The CLI already has a `--profile` shape for packaged execution. Do not add a
language feature that silently changes the active profile inside source code
until profile semantics are stable.

## Stdlib Module Plan

Bundled stdlib modules should grow in this order:

```text
eng.path     FilePath, DirectoryPath, join, parent, stem, extension, exists
eng.io       read/write text/json/toml, source hash helpers
eng.fs       copy, move, delete, mkdir, list
eng.config   promote toml/json as schema
eng.log      print/log/warn and unit-aware formatting helpers
eng.process  run command, ProcessResult
eng.test     test/assert/golden support, later
eng.net      download/cache/hash, later
```

## Implementation Phases

| Phase | Scope | Public Claim |
|---|---|---|
| GP-1 | path types/helpers, `exists`, side-effect docs | implemented in v0.3 |
| GP-2 | read text/json/toml, source hashes | implemented in v0.4 |
| GP-3 | write text/json, export hardening, output manifest | implemented in v0.5 |
| GP-4 | copy/move/delete and side-effect manifest | v0.6 target |
| GP-5 | print/log/warn formatting and run log artifact | v0.7 target |
| GP-6 | external process and `ProcessResult` | v0.8 experimental |
| GP-7 | network/download/cache/hash | optional, after process policy |
| GP-8 | test block/assert/golden support | v0.9 target |
| GP-9 | IDE side-effect panels and output file navigation | grows across phases |

The current implementation deliberately stops at path helpers, `exists`,
read-only source-hashed inputs, and generated output writes. Copy, move,
delete, process, and network effects remain outside the public preview until
their policy slices land.
