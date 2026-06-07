# v1.0.3 Hardening Register

The active target is `v1.0.3`: a hardening release candidate focused on making
the repository easier to use, test, and continue from before v1.1/v1.2 feature
work becomes the main priority.

## Goals

- Make the native tester IDE usable for real user testing.
- Curate user-facing documentation into package PDF assets.
- Normalize repository status language around Supported, Preview, Experimental,
  and Planned.
- Reduce LLM context loading by adding current-status and load-map documents.
- Clarify that Python is optional documentation tooling, not part of the core
  execution path.
- Clarify `degC` as the current canonical spelling and support `°C` as a
  user-facing alias for `AbsoluteTemperature`.

## Hardening Items

| Area | Status | Notes |
|---|---|---|
| Native IDE layout | Implemented on main | Explorer, Code/Result split, Inspector/Completions/Runtime sidebar, bottom panels |
| Native IDE scrolling | Implemented on main | Code and Result panes scroll independently; horizontal code scroll only for long lines |
| Native IDE user settings | Implemented on main | Settings controls light/dark theme, density, font sizes, window presets, soft wrapping, and panel defaults saved under `build/ide/settings.json` |
| Native IDE completion | Implemented on main | Current-file variables, identifiers, keywords, quantities, units, snippets, and Tab accept |
| Native IDE auto-pairs | Implemented on main | Parentheses, brackets, braces, single quotes, and double quotes |
| Native IDE plot preview | Implemented on main | Grid, ticks, zero baseline, line/scatter/bar/histogram rendering paths |
| Native IDE runtime summary | Implemented on main | Uncertainty and ML artifact summaries from `result.engres` |
| Curated PDF user docs | Implemented on main | Package ships curated PDF instead of developer markdown tree |
| Portable Python docs tooling | Implemented on main | Repo-local optional documentation toolchain only |
| Current status layer | Implemented on main | `docs/current/status.md` |
| Feature maturity matrix | Implemented on main | `docs/current/feature_maturity_matrix.md` |
| LLM context/load map | Implemented on main | `LLM_CONTEXT.md` and `docs/llm/load_map.yml` |
| README link cleanup | Implemented on main | README points to current-status layer and short doc index |
| Master plan cleanup | Implemented on main | Active pointer plus current v9 plan; historical plans are left to git history |
| `degC`/`°C` policy | Implemented on main | `degC` remains canonical; `°C` is supported as an AbsoluteTemperature alias with tests |
| Official vs legacy examples | Implemented on main | `examples/README.md` defines official, compatibility regression, diagnostic, and data-quality namespaces; IDE and CLI smoke surface official examples first |
| IDE variable/unit/schema inspector depth | Implemented on main | Variables show quantity, display/canonical unit, dimension, source, expression, and unit path; schemas and CSV promotions have dedicated inspector sections |

## Release Gate Additions

Before tagging or publishing `v1.0.3`, manually confirm:

- The packaged `eng-ide.exe` opens without a console window.
- Settings can switch Light/Dark theme, adjust UI/code font sizes, apply window
  presets, and save preferences under `build/ide/settings.json`.
- The Result pane does not cover or push the right sidebar.
- Code editing scrolls vertically by default and horizontally only for long
  lines.
- Completion suggestions appear while typing and `Tab` applies the first
  suggestion.
- Official CSV+plot, integrated HVAC, uncertainty, and data-driven modeling
  examples run from the IDE.
- Explorer shows Official Examples before compatibility regression,
  diagnostic, and data-quality fixtures.
- Variables inspector shows canonical units, dimensions, derivation steps,
  schemas, and CSV promotion summaries for official examples.
- The package docs folder contains curated PDF assets, not the full developer
  markdown tree.
