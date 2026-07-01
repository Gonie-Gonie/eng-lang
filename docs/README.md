# EngLang Documentation

This index is organized by reader intent. Start here, then follow the narrower
index for the area you need.

## I Want To Use EngLang

1. [User documentation home](user/index.md)
2. [Getting started](user/getting_started.md)
3. [Install and doctor tutorial](user/tutorial/01_install_and_doctor.md)
4. [CSV to report tutorial](user/tutorial/04_schema_promote_csv.md)
5. [Native IDE guide](user/howto/use_native_ide.md)
6. [Composite workflow examples](workflows/index.md)

## I Want To Look Something Up

1. [Reference index](reference/index.md)
2. [Language reference](reference/language/index.md)
3. [Standard library reference](reference/stdlib/index.md)
4. [CLI reference](reference/cli/index.md)
5. [Artifact reference](reference/artifacts/index.md)
6. [Diagnostics reference](reference/diagnostics/index.md)

## I Want To Contribute

1. [Development index](development/index.md)
2. [Repository layout](development/01_repo_layout.md)
3. [Daily workflow](development/02_daily_workflow.md)
4. [Documentation authoring](development/docs_authoring.md)
5. [Release workflow](release/release-workflow.md)
6. [Current project status](current/status.md)
7. [Feature maturity matrix](current/feature_maturity_matrix.md)
8. [Usability improvement backlog](current/usability_improvement_backlog.md)

## I Am Working On Internal Tracks

1. [Internal docs index](internal/README.md)
2. [Solver internal docs](internal/solver/README.md)
3. [JIT and kernel internals](internal/jit/README.md)
4. [Component and domain internals](internal/component_domain/README.md)
5. [Class object internals](internal/class_object/README.md)
6. [Uncertainty internals](internal/uncertainty_internal/README.md)
7. [Archived long-form plans](archive/old-plans/README.md)

## Documentation Rules

- Public behavior changes must update user docs, reference docs, examples, and
  release notes when applicable.
- Runtime artifact changes must update the artifact reference.
- Core runtime paths must not depend on Python, OODocs, or documentation build
  tooling.
- OODocs is an optional publishing layer for curated release bundles, not the
  source of truth.
- Solver-heavy, JIT, component/domain, class-object, and internal uncertainty
  material belongs under internal docs until a current status document marks it
  public-supported.
