# Expected Output Manifest Summary

The saved run should include:

```text
outputs/case_001/input.txt
outputs/case_002/input.txt
outputs/case_003/input.txt
outputs/case_001/result.json
outputs/case_001/simulator.log
outputs/case_002/result.json
outputs/case_002/simulator.log
outputs/case_003/result.json
outputs/case_003/simulator.log
outputs/case_001/case_manifest.json
outputs/case_002/case_manifest.json
outputs/case_003/case_manifest.json
outputs/summary_results.csv
outputs/result_collection_manifest.json
outputs/surrogate.json
outputs/model_metrics.json
outputs/predictions.csv
outputs/db_write_manifest.json
outputs/workflow_summary.csv
outputs/model_card.json
```

The per-case artifacts should be classified as `case_input`, `case_result`, and
`case_manifest` generated files. The patched case inputs, simulator logs,
collected summary CSV, and result collection manifest should have expected output
records with hashes in `process_results.json` and output-manifest artifact
records.

The manifest should also include normal EngLang runtime artifacts when the
example is run with `--save-artifacts`.
