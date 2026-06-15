# Breaking Change Policy

EngLang `1.0.0` stabilizes a narrow core. A change is breaking when it changes
documented stable-core behavior in a way that can make an existing supported
workflow fail, produce different artifact contracts, or require source edits.

## Breaking Changes

- Removing or changing accepted stable syntax.
- Changing the meaning of a stable unit, quantity kind, Args type, or supported
  TimeSeries/statistics operation.
- Removing required fields from stable artifacts such as `review.json`,
  `result.engres`, `report_spec.json`, PlotSpec, run log, process results, test
  results, output manifest, `.engpkg`, or `.lock`.
- Changing the type or meaning of an existing stable artifact field.
- Changing CLI/package commands used by the stable smoke path.
- Changing standalone package layout in a way that breaks `run.bat`, Args help,
  dependency copying, or package-smoke behavior.

## Non-Breaking Changes

- Adding optional artifact fields.
- Adding warnings or diagnostics for previously accepted but questionable
  source, as long as valid stable workflows still pass.
- Adding new units, quantities, examples, snippets, docs, IDE panels, or
  internal-track metadata.
- Improving report/IDE presentation without changing stable artifact contracts.
- Changing supported/internal/planned tracks when the stable-core workflow remains
  compatible and release notes call out the change.

## Deprecation Rule

For stable-core behavior, prefer a deprecation warning for at least one minor
release before removal. Immediate breaking changes should be reserved for
security, data-loss, or severe correctness issues and must be called out in the
release notes.

## Versioning

```text
1.0.x  patch fixes without stable contract changes
1.x.0  compatible additions and deprecations
2.0.0  intentional breaking changes to stable-core behavior
```

Supported, internal, and planned tracks can evolve faster than the stable core,
but their release notes must keep the boundary visible.
