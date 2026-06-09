# Official Example 12: Write And Output Manifest

This mini workflow demonstrates the v0.5 write/export hardening slice:

- `export summary to csv` with explicit overwrite policy
- `write text` for a small text artifact
- `write json` for a scalar quantity artifact
- `build/result/output_manifest.json` listing generated output files and hashes

Run:

```bat
eng.exe run examples\official\12_write_output_manifest\main.eng --save-artifacts
```

Expected output files include:

```text
build/result/outputs/summary.csv
build/result/outputs/run_note.txt
build/result/outputs/energy.json
build/result/output_manifest.json
```
