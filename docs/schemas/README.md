# Artifact Schemas

These schema files document the current preview artifact contracts used by the
release gate.

```text
review.schema.json       build/result/review.json
report_spec.schema.json  build/result/report_spec.json
result.schema.json       build/result/result.engres
plotspec.schema.json     build/result/plots/plot_spec.json
engpkg.schema.json       normalized key/value view of dist/*/*.engpkg
```

`.\dev.bat artifacts-check` validates official example artifacts against these
schema headers and the golden baselines in `tests/golden/artifacts`.

The current schemas are intentionally structural baselines. They protect format
headers, version numbers, required top-level sections, and release-critical
counts without freezing volatile values such as hashes or generated paths.

For the full standalone package field contract, see
[Standalone package reference](../reference/standalone_package.md).
