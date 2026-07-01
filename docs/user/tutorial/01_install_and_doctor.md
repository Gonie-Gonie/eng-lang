# 01 Install And Doctor

## Goal

Verify that your EngLang package or repository build can run the supported
user workflows.

## What You Will Build

No new source file is required. You will verify the executable, run the package
doctor, and smoke-test the example suite.

## Run Commands

From a portable package:

```bat
eng.exe doctor
eng.exe test examples
```

From the repository:

```bat
cargo run -p eng_cli -- doctor
cargo run -p eng_cli -- test examples
```

## Expected Artifacts

doctor should report package readiness. test examples should run the official
examples and composite native workflow programs that define the supported
public scope.

## Explanation

doctor is the fastest check for a broken package layout. test examples is a
behavior check: it proves that supported source files still compile, run, and
produce the expected reviewable artifacts.

## Common Mistakes

- Running commands from a directory where relative example paths do not exist.
- Mixing a package eng.exe with repository-relative paths from another clone.
- Treating advanced or internal examples as first-user package support.

## What To Inspect

After test examples, inspect failing example names before editing source.
Failures usually point to missing package files, changed relative paths, or a
real behavior regression.
