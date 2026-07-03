# Expected Output Manifest Summary

The saved run should include:

```text
outputs/case_001/input.txt
outputs/case_001/input.txt.render_manifest.json
outputs/case_002/input.txt
outputs/case_002/input.txt.render_manifest.json
outputs/case_003/input.txt
outputs/case_003/input.txt.render_manifest.json
...
outputs/case_008/input.txt
outputs/case_008/input.txt.render_manifest.json
outputs/surrogate_results.sqlite
outputs/surrogate_results.sqlite.db_write_manifest.json
outputs/sampling_summary.txt
outputs/workflow_summary.csv
```

The per-case inputs should be classified as `case_input` generated files, with
render manifests classified as `template_render_manifest`. Case inputs are
generated for case_001 through case_008 because the training-design sample count
is eight. The SQLite database target comes from `args.database_target`, and the
database plus DB write manifest should be classified as `db_write`. Native
model, model-card, metric, and prediction records are represented in the output
manifest's `model_artifacts` section as `model://...` artifacts rather than as
external-process or opaque-tool outputs. The sampling summary should record the
native sampler method, seed, sample count, and parameter count used by the
workflow. `process_results.json` should show
`process_count = 0`.

The manifest should also include normal EngLang runtime artifacts when the
example is run with `--save-artifacts`.
