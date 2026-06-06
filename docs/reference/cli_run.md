# `eng run` Reference

`eng run` executes one file entry point through bytecode and the native VM seed.

## Basic Run

```bat
target\debug\eng.exe run examples\04_plotting\main.eng
```

Output:

```text
bytecode: build\main.engbc
result:   build\result\result.engres
review:   build\result\review.json
plot:     build\result\plots\timeseries.svg
report:   build\result\report.html
```

## List Entries

```bat
target\debug\eng.exe entries examples\04_plotting\main.eng
```

Output:

```text
examples\04_plotting\main.eng:8: script main(args: Args) -> Report
```

## Select an Entry

```bat
target\debug\eng.exe run examples\04_plotting\main.eng --entry main
```

Default rule:

```text
1. `--entry <name>` wins.
2. `script main` is the default when present.
3. A single non-main entry can run.
4. Multiple non-main entries require `--entry`.
5. No entry fails with E-ENTRY-NOT-FOUND-001.
```

## Open Report

```bat
target\debug\eng.exe run examples\04_plotting\main.eng --open-report
```

This attempts to open `build\result\report.html`.

## Missing Entry Example

`examples\05_error_messages\missing_entry.eng` is intentionally declaration-only:

```eng
L = 1 m
```

`check` can inspect it:

```bat
target\debug\eng.exe check examples\05_error_messages\missing_entry.eng
```

`run` fails because file execution requires an entry point:

```bat
target\debug\eng.exe run examples\05_error_messages\missing_entry.eng
```

Expected diagnostic:

```text
E-ENTRY-NOT-FOUND-001
```

## Artifact Review

After a successful run:

```bat
type build\main.engbc
type build\result\result.engres
target\debug\eng.exe view build\result\result.engres
```

Inspect:

```text
.engbc
  ENGBYTECODE 1
  entry
  objects
  instructions

result.engres
  format = engres-v1
  entry
  object_store
  typed_payload
  provenance
```
