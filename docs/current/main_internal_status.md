# Main Internal Status

This page summarizes implementation tracks on `main` that are useful for
contributors but are not the first public package story.

The public package line is still `v0.1.0`. Later work on `main` may be
implemented and tested without being part of the published release assets.

## Internal And Narrow Tracks

| Area | Main status | Public positioning |
|---|---|---|
| Solver algorithms | Dense linear, fixed-point, Newton, DAE, adaptive ODE, dynamic component, and behavior-node tracks exist with targeted tests and artifacts. | Supporting implementation track. Not a general solver platform. |
| State-space | Typed-block fixed-step workflows and additional internal runtime fixtures exist. | Use only for the documented narrow scope. |
| Domain/component | Component metadata, residual assembly, constrained Thermal and Thermal/Fluid paths, report/review/IDE artifacts, and diagnostics exist. | Not production multi-domain simulation. |
| JIT/AOT | Kernel planning, executable interpreter kernel IR, source-system/component residual and partial-Jacobian samples, single Newton-step samples, `jit-plan`, `jit-bench`, and benchmark catalog coverage exist. | No native speedup claim or integrated JIT solver loop. |
| LSP/VS Code | Persistent stdio document sync, diagnostics, cancellable editor requests, direct protocol semantic tokens, and compiler-derived user-function, class-method, and structured built-in API signature help exist alongside compatibility CLI endpoints and optional VS Code source. | Implemented internal editor service, without a public cross-release compatibility guarantee. |
| Uncertainty | Deterministic samples, diagnostics, propagation metadata, and histogram artifacts exist. | Internal track. |
| Data-driven modeling | Train/test, metrics, model specs/cards, prediction manifests, parity/residual plots, and diagnostics exist. | Internal track. |
| Class/domain objects | Typed fields/defaults, object literals, validation, metadata methods, copy-with, IDE/LSP metadata, and report artifacts exist. | Narrow supported authoring surface, not runtime object dispatch. |

## Solver-Centered Detail

For solver implementation detail, read:

- [Solver track docs](../internal/solver/README.md)
- [Solver-centered implementation plan](../internal/solver/solver_centered_plan.md)
- [Generic solver completion plan](../internal/solver/generic_solver_completion_plan.md)

Those documents are implementation ledgers. They should not be copied into the
README, user guide, or first-user examples as product claims.

## Acceptance Rule

An internal track becomes public-facing only when all of these line up for its
stated scope:

- source language rule
- parser and semantic checks
- runtime/check behavior
- diagnostics and failure artifacts
- report/review metadata
- IDE visibility when relevant
- official example or clearly named internal fixture
- tests
- current status and feature matrix entry
