# Version Roadmap Workflow

The v9 master plan changes how development work is selected. Contributors should start from the version target, not from a broad feature list.

## Standard Workflow

```text
1. Pick the current target version.
2. Read that version's goals, required outputs, tests, and release gate.
3. Open the cross-reference map in the v9 master plan for detailed design sections.
4. Create or select an issue with a version target.
5. Implement code, tests, examples, and docs together.
6. Run dev.bat ci.
7. Commit and push at a reviewable unit.
8. Update roadmap/release notes when the version state changes.
9. After a stable or alpha milestone, run a gap audit before starting the next major feature arc.
```

## Issue Format

Example:

```text
Title:
  v0.3: validate CSV headers against schema columns

Labels:
  area:compiler
  area:schema
  milestone:v0.3-preview

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

After an alpha/stable milestone is tagged, compare the implemented behavior
against the master plan and write down seed-only areas before moving on.

The audit should classify each item as:

```text
Implemented
Seed
Gap
Deferred
```

For v1.0, this register lives in
[v1.0 gap audit](05_v1_0_gap_audit.md).

## v0.1/v0.2 Backfill Policy

If v9 reveals a missed item for an already tagged preview version:

```text
1. Add the missing item as a normal main-branch commit.
2. Do not move existing tags.
3. Mention it as backfill in docs/roadmap.md.
4. Keep the next active milestone unchanged unless the user asks for a patch tag.
```
