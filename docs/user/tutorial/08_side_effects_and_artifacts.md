# 08 Side Effects And Artifacts

## Goal

Write output files and call external processes while keeping side effects
visible.

## What You Will Build

Use official examples for supported file and process patterns:

- examples/official/12_write_output_manifest/main.eng
- examples/official/15_process_result/main.eng

Example output statements:

```eng partial
write text "outputs/run_note.txt", notes_text
with {
    overwrite = true
}

write json "outputs/energy.json", E_coil
with {
    overwrite = true
}
```

## Execute The Steps

```bat
eng.exe run examples/official/12_write_output_manifest/main.eng --save-artifacts
eng.exe run examples/official/15_process_result/main.eng --save-artifacts
```

## Expected Artifacts

Runs should record generated files, process results, logs, and output manifests
where the workflow writes or calls outside EngLang.

## Explanation

Real engineering workflows create files, run tools, and sometimes prepare
database writes. EngLang supports those boundaries only when they are explicit
enough to review.

## Common Mistakes

- Running an external tool without recording arguments, status, and outputs.
- Overwriting files implicitly.
- Treating a generated file as trusted without checking its manifest entry.

## What To Inspect

Inspect output manifests, process result records, and side-effect entries in
review.json. For a saved run, inspect each side effect's `runtime_result`:
generated files expose artifact path and content hash, while native SQLite
writes expose database/manifest hashes, transaction/schema status, table
schema, and row count. Check that each generated path is expected and
reproducible.
