# Artifact Schemas

These schema files document the current stable-core artifact contracts used by
the release gate. Supported and internal tracks may add optional fields, but
stable required fields are covered by the breaking-change policy.

```text
review.schema.json       build/result/review.json
report_spec.schema.json  build/result/report_spec.json
result.schema.json       build/result/result.engres
plotspec.schema.json     build/result/plots/plot_spec.json
output_manifest.schema.json build/result/output_manifest.json
run_log.schema.json      build/result/run_log.json
process_results.schema.json build/result/process_results.json
test_results.schema.json build/result/test_results.json
engpkg.schema.json       normalized key/value view of dist/*/*.engpkg
```

`.\dev.bat artifacts-check` validates official example artifacts against these
schema headers and the golden baselines in `tests/golden/artifacts`.

The current schemas are intentionally structural baselines. They protect format
headers, version numbers, required top-level sections, and release-critical
counts without freezing volatile values such as hashes or generated paths.
Domain/component assembly fields are schema-checked enough for IDE/report
tooling to rely on `component_graph`, `assembly_summary.domain_plans`,
`assembly_summary.solver_preview`, and runtime
`typed_payload.component_solutions` while the track remains internal.

For the full standalone package field contract, see
[Standalone package reference](../reference/standalone_package.md).
