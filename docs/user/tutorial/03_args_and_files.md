# 03 Args And Files

## Goal

Parameterize a workflow so file paths and simple options are explicit at the
top of the program.

## What You Will Build

A source file with an args block:

```eng partial
args {
    input: CsvFile = file("data/sensor.csv")
    output_dir: Directory = dir("outputs")
}

print "input = {args.input}"
```

## Source File

Use an official example when learning the current syntax:
examples/official/01_csv_plot/main.eng and
examples/official/12_write_output_manifest/main.eng both show file-oriented
workflows.

## Run Command

```bat
eng.exe run examples/official/01_csv_plot/main.eng --save-artifacts
```

## Expected Artifacts

The review artifact should record input files and generated outputs with paths
that can be inspected after the run.

## Explanation

Arguments make workflow boundaries reviewable. A reviewer should be able to see
which file was read, which output directory was used, and which command-line
override changed behavior.

## Common Mistakes

- Hiding important paths inside helper scripts instead of exposing them in args.
- Using relative paths without thinking about the working directory.
- Overwriting outputs without an explicit policy.

## What To Inspect

Inspect review.json for input and output path evidence. If a path fails, use
the diagnostic location rather than changing unrelated source.
