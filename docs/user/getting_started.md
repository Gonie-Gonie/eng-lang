# Getting Started

This page assumes a portable Windows package or a repository build that exposes
eng.exe. From a package, run commands in the extracted package directory. From
the repository, run commands from the repository root after building.

## Check The Package

```bat
eng.exe doctor
```

Expected result: the command reports that the package assets and example paths
are usable. If a package README gives a different executable path, use that
path consistently in the commands below.

## Run A First Example

```bat
eng.exe run examples/official/01_csv_plot/main.eng --save-artifacts
```

Inspect these outputs:

- build/result/result.engres
- build/result/review.json
- build/result/report.html
- generated plot and summary artifacts listed in the review output

The point of the first run is not only the numeric result. Check that units,
schema promotion, TimeSeries metadata, plot intent, and report evidence are all
visible in generated artifacts.

## Validate Examples

```bat
eng.exe test examples
```

This command is the user-facing smoke path for the supported example set. It
should pass before you trust a local package, CI artifact, or edited example.

## Open The Native IDE

```bat
eng.exe ide examples/official/01_csv_plot/main.eng
```

Use the IDE when you need to inspect diagnostics, runtime objects, schema rows,
unit conversions, artifacts, and review metadata without manually opening each
output file.

## What To Read Next

- Continue with [01 install and doctor](tutorial/01_install_and_doctor.md).
- Use [how-to guides](howto/README.md) when you already know the task.
- Use [concepts](concepts/README.md) when you need the language model behind
  the workflow.
