# Side Effect And General Programming Policy

Status: design policy for `v0.2-preview`; runtime implementation is planned by
track, not claimed as supported preview behavior.

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
| Read-only I/O | `read text`, `read json`, `read toml` | Planned after path helpers; source hash should be recorded |
| Typed data boundary | `promote csv/json/toml as Schema` | Preferred for engineering data |
| Write/export | `write text`, `write json`, `export summary to csv` | Explicit target required; overwrite defaults to false |
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
if not exists args.input {
    error "Input file not found: {args.input}"
}
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
- import-time read is forbidden for executable workflow effects
- source hashes are recorded when data affects runtime artifacts
- typed promote remains the recommended data boundary for official examples
```

## Write And Export Policy

Writes must name their target:

```eng partial
write text file("build/log.txt"), "finished"
write json file("build/summary.json"), summary

export summary to csv join(args.output, "summary.csv")
with {
    overwrite = true
}
```

Rules:

```text
- overwrite default is false
- missing parent directory is an error unless mkdir/parents policy says otherwise
- output files should appear in report/review artifact metadata
- export is preferred over ad hoc write for reproducible artifacts
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
| GP-1 | path types/helpers, `exists`, side-effect docs | v0.3 target |
| GP-2 | read text/json/toml, source hashes | v0.4 target |
| GP-3 | write text/json, export hardening, output manifest | v0.5 target |
| GP-4 | copy/move/delete and side-effect manifest | v0.6 target |
| GP-5 | print/log/warn formatting and run log artifact | v0.7 target |
| GP-6 | external process and `ProcessResult` | v0.8 experimental |
| GP-7 | network/download/cache/hash | optional, after process policy |
| GP-8 | test block/assert/golden support | v0.9 target |
| GP-9 | IDE side-effect panels and output file navigation | grows across phases |

The first implementation slice should be path helpers and review-visible
environment dependency metadata, not broad filesystem mutation.
