# Version And Track Workflow

EngLang separates public release versions from long-term development tracks.
Contributors should start from the current public preview scope or the relevant
track, not from a broad feature list.

## Standard Workflow

```text
1. Pick the current public preview scope or development track.
2. Read `docs/current/version_plan.md`, `docs/current/status.md`, and `docs/current/tracks.md`.
3. Open the v9 master plan only for long-term design sections.
4. Create or select an issue with a preview or track target.
5. Implement code, tests, examples, and docs together.
6. Run dev.bat ci.
7. Commit and push at a reviewable unit.
8. Update roadmap/release notes when the public preview state changes.
9. After a public preview, run a gap audit before promoting any feature to stable.
```

Before marking a feature done, check
[feature_maturity_matrix.md](../current/feature_maturity_matrix.md). An official
example passing is not enough by itself.

## Issue Format

Example:

```text
Title:
  data-boundary: validate CSV headers against schema columns

Labels:
  area:compiler
  area:schema
  track:data-boundary

Definition of Done:
  - schema symbol table includes required columns
  - CSV header reader reports missing column
  - source span diagnostic is emitted
  - official CSV example still passes
  - docs updated
```

## Commit Cadence

Use small commits that can pass CI independently.

Good commit units:

```text
- add one compiler data structure and tests
- add one diagnostic and one error example
- add one artifact field and report/review output
- update docs for a completed implementation slice
```

Avoid combining:

```text
- parser changes + runtime changes + unrelated docs
- broad roadmap rewrites + source code refactors
- generated artifacts + source edits
```

## Milestone Gap Audit

After a public preview is tagged, compare the implemented behavior
against the master plan and write down seed-only areas before moving on.

The audit should classify each item as:

```text
Prototype
Preview
Supported
Stable
Experimental
Planned
Deferred
```

Definitions:

```text
Prototype
  Internal spike or seed. Do not present as a release feature.

Preview
  Usable through official examples or package paths with explicit limitations.

Supported
  Documented, tested, has diagnostics or IDE metadata where relevant, and is
  part of the current public preview contract.

Stable
  Public behavior with a breaking-change policy.

Experimental
  May exist on main, but is not release-supported.

Planned
  Intended future work.
```

Completion policy:

```text
A feature is not complete merely because an example passes.
A feature is complete only when its language rule, compiler check,
runtime or check behavior, diagnostic, IDE metadata, official example,
and documentation are aligned for the stated scope.
```

The historical gap audit remains in [gap audit](05_historical_stable_core_gap_audit.md), but
current planning should use [tracks](../current/tracks.md).

## Backfill Policy

If a missed item is found after a preview release:

```text
1. Add the missing item as a normal main-branch commit.
2. Do not move existing tags.
3. Mention it in docs/roadmap.md or the relevant track doc.
4. Keep the next active preview unchanged unless the user asks for a patch label.
```
