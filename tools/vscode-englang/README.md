# EngLang VS Code Extension

This extension provides VS Code support for EngLang editing and local workflow
checks. It intentionally uses the shipped EngLang executables instead of
embedding compiler logic in JavaScript.

## Features

- `.eng` language registration and syntax highlighting for workflow keywords,
  schema/types, units, built-in functions, with-block options, and literals
- stable file diagnostics from the EngLang CLI checker, with code links,
  token-precise ranges, option-value underlines, and legacy/deprecated tags in VS Code Problems
- optional live editor diagnostics, hover, completion, document/workspace
  symbols, semantic same-symbol highlights, and folding from the current
  unsaved buffer plus every modified open EngLang import in the workspace
- debounced diagnostics for unsaved buffers after a short typing pause,
  including open dependent EngLang files when an imported buffer changes
- debounced role-aware color refresh and stale decoration clearing across open
  editors when a modified EngLang import changes or closes
- automatic dependent Problems and color refresh when an open import is saved
  or a closed workspace `.eng` import is created, changed, or deleted by Git,
  formatters, or other disk tools; generated trees are ignored
- a clickable EngLang Problems status bar item showing file/live mode,
  current error/warning/info/hint counts, and a Tooling Status shortcut
- role-aware highlighting for unsaved buffers, covering roles such as
  variables, parameters, properties, built-in workflow helpers, module
  namespaces, quantities, units, reports, validations, and side-effect/external
  workflow boundaries
- packaged role-coloring metadata so themes can color EngLang code
  consistently without custom rules
- optional `EngLang Dark` and `EngLang Light` color themes with explicit
  semantic colors for every EngLang semantic selector contributed by the
  extension
- subtle review-risk line and overview-ruler markers for high and medium risks,
  with a gutter warning icon reserved for high-risk side effects
- highlight-token inspection command for checking how the current file is
  colored
- Problems-at-cursor inspection command, available from the Command Palette or
  `.eng` editor context menu, for checking the diagnostic source, code,
  severity, and exact range under the caret
- hover from compiler review metadata
- position-aware completion from compiler/editor metadata
- compiler-backed go-to-definition for current-file and static-import symbols,
  preferring every modified open EngLang buffer over disk
- safe current-file semantic rename plus unsaved-aware, static-import-aware
  workspace rename for importable declarations and verified references
- compiler-backed same-symbol read/write highlighting in the current file, excluding
  strings, comments, literals, units, and same-named locals in other function scopes
- standard Find All References results for compiler-resolved occurrences in the
  current unsaved file and workspace files that resolve the same static-import
  declaration, with declaration inclusion controlled by VS Code
- workspace symbol search across `.eng` files in the open workspace, preferring
  every modified open EngLang buffer over its saved file
- compiler-owned snippets from generated editor metadata, plus non-overlapping static snippets from `snippets/eng.json`
- quick fixes for `:=`, boolean `==`, stale `struct Args`, removable `script` wrapper
  migration diagnostics, ambiguous unit-to-quantity annotations, safe
  missing-unit suffix fixes for unit arithmetic diagnostics, unterminated/empty string
  interpolation closures, unresolved interpolation literal conversions, interpolation display-unit removals, command target
  parenthesizing, unknown stdlib module replacements, planned/internal stdlib
  import removal, schema column annotation migrations, required file-mutation
  `with` options, invalid network URL/body-method/retry/timeout/body-size policies,
  legacy network `fixture` option aliases that should be rewritten to
  `offline_response`, legacy response `.hash`/`.status` aliases,
  direct `read json` field-access promotion edits,
  HeatRate TimeSeries sum-to-integrate repairs,
  statement-only binding prefix removals for report, validation, side-effect,
  block/header, import/use/connect/return, and workflow-option diagnostics,
  process binding conflicts and command/env/cwd values, pinned
  response SHA-256 mismatches, sampling count/seed/range-unit values, missing repro-profile
  sampling seeds, simulation/solver option value repairs and missing-option
  insertion into attached or new `with` blocks, model source-chain
  starter-code repairs, model option value repairs for invalid test splits, seeds, hidden layers, and epochs,
  unsupported regression algorithm repairs, unsupported command-style call conversions,
  legacy `select_first_row` migration edits,
  uncertainty constructor argument repairs, direct uncertainty comparison repairs,
  uncertainty propagation option/seed repairs, uncertainty source
  definition/conversion repairs, and golden test-block/expected path wrappers.
  Quick fixes come from `eng-lsp` using the current unsaved buffer. The extension
  accepts only complete edits for the matching current-file diagnostic and does
  not duplicate compiler repair rules in JavaScript.
- quick fixes for simple same-block `where` local ordering mistakes where a
  later local definition can be moved before its first use
- quick fixes for simple escaped `where` locals where a reused local binding
  can be promoted to a top-level binding
- commands to check, run the current file or a bundled example with saved
  artifacts, open a current-file review panel, open current-file review data,
  open the latest generated report, and inspect last-run review data, result
  data, report data, generated output lists, run logs, run graphs, reproducibility locks,
  process results, cache records, test results, plot data, plot output
  lists, and plot SVGs
- `EngLang: Switch Diagnostics Mode...` for choosing quieter saved-file checks
  or live unsaved-buffer checks from the Command Palette
- `EngLang: Refresh Problems` for rerunning the active-file linter from the
  Command Palette or `.eng` editor context menu
- `EngLang: Copy Problem at Cursor` for copying the current or nearest same-line
  diagnostic details without opening a separate inspector view
- `EngLang: Copy Highlight Token at Cursor` for copying the current or nearest
  same-line role-aware highlight token details without opening a separate inspector view
- `EngLang: Show Tooling Status` for inspecting a summary-first JSON status view
  with the active check/run and live editor tool paths, configured-path/source labels,
  diagnostics mode, the `eng/file` or `eng/live` Problems source label,
  current-file Problems counts and range status, cursor diagnostic inspection and
  copy commands, Problems diagnostics toggles, the first-pass plus role-aware
  highlighting model, highlight inspection and copy commands, highlight coverage
  summary, theme fallback scope-map coverage, native workflow source/docs and per-workflow native primitive evidence, latest
  zero-process artifact evidence, local VSIX package freshness, install
  freshness, install preflight guidance, and extension version
- `EngLang: Switch Execution Profile...` for choosing the `normal`, `safe`, or
  `repro` profile used by `EngLang: Run Current File`

## Install From Portable Package

1. Extract the EngLang portable zip.
2. In VS Code, run `Extensions: Install from VSIX...`.
3. Select `tools/englang-vscode-<version>.vsix` from the extracted
   EngLang folder.
4. Open the extracted folder or any EngLang project folder.
5. Open a `.eng` file and run `EngLang: Check Current File`.

The packaged VSIX contains the EngLang command-line and editor tooling, so no
Rust setup is required for diagnostics or live editor checks. The default
diagnostics mode uses stable file checks. In Settings, switch EngLang diagnostics
to live editor checks, or run `EngLang: Switch Diagnostics Mode...`, to update
Problems while typing. In `settings.json`, set:

```json
{
  "englang.diagnosticsMode": "live",
  "englang.liveDiagnosticsDelayMs": 350
}
```

## Install From Source

To build and install the extension from the current checkout:

```bat
.\dev.bat vscode-install
```

This builds a release `eng.exe` and `eng-lsp.exe`, packages
`dist\local-vscode\tools\englang-vscode-<version>.vsix`, and installs it with
the VS Code `code` CLI. Run `.\dev.bat vscode-status` first to see the built
VSIX path, installed EngLang extension folders, running VS Code processes,
Package freshness and Install freshness results, and whether reinstall is currently blocked. Close all VS Code windows before reinstalling EngLang;
VS Code can lock the existing extension folder while it is running, so
`vscode-install` checks for that before starting the release build. If a built
VSIX already exists, the preflight error includes its path so you can install it
manually after closing or reloading VS Code. The wrapper runs the CLI with an
ignored temporary user-data directory for VS Code CLI logs while installing into
the normal user extension directory. Reload VS Code after installation. The VSIX
remains available at the generated `dist\local-vscode\tools` path.

To build the VSIX without installing it:

```bat
.\dev.bat vscode-package
```

For focused extension validation without creating a VSIX, use:

```bat
.\dev.bat vscode-smoke
.\dev.bat vscode-test
```

`vscode-smoke` checks generated grammar/editor metadata plus the extension
JavaScript contract and smoke programs, including fake saved-file `eng.exe`
success, malformed-output, and stale-result diagnostics cases. `vscode-test`
additionally builds the debug `eng-lsp` and checks full semantic fallback
coverage across the example and grammar-fixture snapshots. Packaging retains
the same checks against the release binary before it writes the VSIX.

If the `code` CLI is not on PATH, run `Extensions: Install from VSIX...` in VS
Code and select the generated VSIX. For extension-host development instead of
local installation, open `tools\vscode-englang` in VS Code and launch the
extension development host. After installing, run `EngLang: Show Tooling
Status` to confirm the summary, bundled check/run tool and live editor tool
paths, configured-path/source labels, the current diagnostics mode, the `eng/file` or
`eng/live` Problems source label, current-file Problems counts, source counts,
range status, per-feature live editor routing, current-file highlight token and
overlap status, theme fallback scope-map coverage for role-aware highlighting, and the
local VSIX package/install freshness block when the active workspace is an
EngLang source checkout. If you run directly from source without packaging, set:

```text
englang.runtimePath = C:\path\to\eng.exe
englang.lspPath = C:\path\to\eng-lsp.exe
```

## Current Scope

The extension is a local editor client for the bundled EngLang tooling. It uses
on-demand live editor checks for live Problems, hover, completion, document
symbols, workspace symbols, folding, role-aware color data, same-symbol highlights, definition, formatting,
static-import-aware references, semantic rename, and quick fixes. References,
rename preparation, and rename pass the current buffer plus all modified open
EngLang documents in the same workspace to bounded compiler endpoints. Static
imports resolve open text before disk, so a changed declaration or import does
not require a save. Saved workspace files are included only when their static
import chain resolves to the same declaration; unrelated same-name symbols are
excluded. Local variables, parameters, and members remain current-file results.
Workspace symbol search uses the same modified-document policy, so Ctrl+T does
not fall back to stale saved text. Rename updates an importable declaration and
every verified workspace reference. The operation is rejected as a whole for
conflicts, incomplete semantic coverage, unreadable or truncated workspace
scans, built-ins, and member fields. If any participating buffer is added,
saved, closed, or changed while the request is running, VS Code discards the
whole result instead of applying stale ranges. The default diagnostics
mode runs stable file checks on open/save and manual check. Set
`englang.diagnosticsMode` to `live` to update Problems from the current unsaved
buffer while typing, or run `EngLang: Switch Diagnostics Mode...` and choose
`live`; the command refreshes the active EngLang editor immediately. Switching
back to `file` clears stale live Problems for an unsaved active buffer and
refreshes saved-file Problems after the file is saved. Direct `settings.json`
changes to diagnostics mode or lint toggles also refresh or clear the active
EngLang editor so Problems match the selected settings. Editing an EngLang
buffer clears cached review/highlight data immediately, so hover,
completion, and decorations cannot reuse an older buffer snapshot while
live editor data is unavailable. The extension also stops the stale editor
subprocess when a newer buffer revision invalidates its shared snapshot. If an
older workspace already has `englang.problemsSource` or
`englang.diagnosticsBackend`, the extension still accepts it as a compatibility
alias. New workspaces should use `englang.diagnosticsMode`.
`EngLang: Run Current File`
passes `--profile <englang.executionProfile> --save-artifacts`, so the
generated `build/result` review artifacts are available to the open-artifact
commands immediately after a successful run. Use `EngLang: Switch Execution
Profile...` to choose `normal`, `safe`, or `repro` for the workspace.
`EngLang: Run Example...` lists `examples/official/**/main.eng` and
`examples/workflows/**/main.eng`, opens the selected example, then runs it
through the same profile/artifact path.
`EngLang: Open Current File Review Panel` runs
`eng.exe review <file.eng> --json` and opens a VS Code-native summary of
inputs, symbols, schemas, units/quantities, time axes, derived values,
diagnostics, external boundaries, side effects, table transforms, calculations,
validations, caches, risks, and workflow modules. Line cells in the panel jump
back to the matching source line, and the Last Run Artifacts section opens
available `build/result` outputs directly, with the same availability labels as
the artifact picker. `EngLang: Open Current File Review Data` runs the same
current-file review command and opens the normalized review data directly,
without requiring a prior run. `EngLang: Open Last Run Review Data` opens the
`build/result/review.json` artifact from the last saved run.
`EngLang: Open Last Generated Output...` reads
`build/result/output_manifest.json` and opens any existing file recorded by the
last run, including generated CSV/text outputs and review artifacts that are not
listed as fixed commands.

When the diagnostics mode is `live`, dirty buffers are checked after a short
typing pause, so Problems can update before the file is saved. Configure that
pause with `englang.liveDiagnosticsDelayMs` (100-5000 ms, default 350 ms).
The EngLang output
panel records whether Problems came from file diagnostics or live-buffer
diagnostics and which tool path was selected. The VS Code Problems source column
uses `eng/file` for saved-file checks and `eng/live` for live-buffer checks.
Checks are ordered per document, so a slower earlier request cannot replace
newer Problems. Starting a newer file check stops the older subprocess, while a
cancelled color-refresh caller does not interrupt another caller sharing the
same current-revision analysis.
The status bar shows the active `.eng` file's EngLang Problems mode and current
error/warning/info/hint counts; click it to open `EngLang: Show Tooling Status`.
Use `EngLang: Refresh Problems` from the Command Palette or `.eng` editor
context menu to run the active-file check immediately. The refresh follows the
selected diagnostics mode: file mode checks the saved file, while live mode can
check the current unsaved buffer.
Saved-file open/save diagnostics
are controlled by `englang.lintOnSave`; live typing diagnostics are controlled by
`englang.lintOnChange`. Use `EngLang: Inspect Problem at Cursor` to open a
focused Problems view of diagnostics covering the caret, nearest same-line
diagnostics, source labels, codes, severity, range text, the underlined source
text, and the full source line for copy-ready reports. Use `EngLang: Copy Problem
at Cursor` to copy that current or nearest same-line diagnostic details directly
to the clipboard. If diagnostics cannot be read from the selected tool, run `EngLang: Show
Tooling Status` to inspect the selected paths. When the selected
tool exits without current editor data, the Problems entry includes a short
`Tool failure:` reason and the EngLang output channel keeps stderr/stdout
details. Set `englang.lintOnChange = false` to disable live typing checks while
keeping live open/save analysis.

Quick fixes are available for common syntax migrations, quantity/unit
annotations, schema column annotations, side-effect confirmations, and invalid
network/process/sampling options such as retry, timeout, body-size, duplicate
process bindings, process command/env/cwd, allow-failure, sample count, sample
seed values, sample range units, deterministic cache keys, cache directories, cache TTL values,
model test splits, model seeds, hidden-layer lists, model epochs, and common
simulation/solver option values such as timestep, duration, tolerance, solver,
max-iteration, and initial values.
Supplied invalid simulation/solver options are replaced at the exact Problems
range. When a required timestep, duration, or solver option is absent, the quick
fix adds it to the attached `with` block or creates a block on the owning
`simulate`/`solve` declaration.
Simple same-block `where` local ordering diagnostics can move the later
definition before its first use.
Uncertainty diagnostics can also repair common constructor mistakes such as
unsupported distribution kind, unsupported propagation method, invalid sample
count, missing constructor arguments, unknown sources, missing source arguments,
and deterministic sources that should be `measured(...)`. Propagation `with`
blocks can repair invalid uncertainty policy, sample-count, and seed option
values, and can insert a reproducible seed for Monte Carlo propagation. A direct
uncertainty comparison can be changed to `mean(...)`; this edit uses the exact
Problems range first, so repeated operand text does not redirect the fix. The
provider only answers Quick Fix requests, so refactor and source-action menus
stay scoped to their own providers.

Hover is computed from the current unsaved buffer, so quantity, unit, and
role/status labels stay aligned with live diagnostics and role-aware highlighting.
Top-level state-space vector types and their members expose role-specific hover,
completion, outline, reference, and rename metadata from the same compiler spans.
Typed system vectors and linear operators use compiler-owned type/expression
ranges for solver colors and precise Problems underlines.
Import targets and const name/type/unit declarations also use compiler-owned
ranges for namespace/readonly colors, outline selection, and target/expression
Problems underlines, including CRLF and UTF-16 source positions.
Function parameter and return type/unit annotations use the same compiler-owned
ranges for quantity/unit colors and unknown-type underlines. Function,
parameter, and local outline selections stay on their exact declaration names
even when names repeat on one signature line.
Block and inline function returns also keep compiler-owned expression ranges, so
duplicate, unresolved, dimension-mismatch, and return-side-effect Problems
underline the expression rather than the function header.
Component assembly balance and algebraic-loop Problems underline the first
source component name. Unconnected-port Problems underline the port name, and
unknown or invalid generic port domains underline the complete domain reference.
Trailing `#` and `//` comments are excluded from both the parsed domain and its
underline, including CRLF and non-BMP source files.
Invalid network URL Problems underline the complete request/download URL operand.
Declared URL aliases are resolved before validation; unsupplied runtime
`args.*` URLs do not create malformed-URL false positives, and trailing comments
are excluded from URL and download-target ranges.
`where`/`with` openers, `where` locals, and `with` option keys now use exact
compiler-owned ranges for semantic colors and outline selection. Inline option
list/enum values and `file`/`dir`/`join` helpers are limited to their own value
range, so matching text in an earlier option or string is not recolored.
Simple `write` source identifiers also use exact compiler-owned expression
ranges. A binding such as `records` is shown once as a workflow/output variable
at that source position, while `records` in `promote json records` remains a
workflow keyword.
Fast-binding aliases and model workflow operands use the same exact-range
policy. `model_alias = model`, `train regression records`, and `predict model
using records` keep resolved variable colors without changing grammar keywords
or dotted member fields that use the same spelling. Dotted ML operands and
features color `args`, receivers, and members independently; dotted `apply`
targets additionally color the final segment as a workflow-step function.
Inline ML arguments and attached `with` options share compiler-owned key/value
spans, and every feature path has its own range. Problems therefore underline
the malformed ML value or feature itself, while option semantic colors do not
repaint a binding declaration named `target` or `algorithm`. Trailing option
comments are outside both Problems and highlight ranges.
Sampling bindings and distribution keys also use exact compiler-owned ranges for
semantic colors and Outline selection. When a sample declaration omits `count`
or every `uniform(lower, upper)` parameter, Problems underline its complete
`sample <method>` RHS; malformed supplied options continue to underline their
own values.
Simulation and algebraic/component-solver Problems follow the same source-owned
policy. Unknown targets underline the target name, missing required options
underline the owning `simulate`/`solve` RHS, and malformed supplied options
underline only their exact value, including CRLF and non-BMP source positions.
Class and object Problems now follow that policy as well: defaults and object
assignments underline their value, validation and method-return errors underline
their expression, and method calls underline the receiver, method, or argument
that failed. Class method return quantities and units receive independent
semantic colors. Outline selects exact class, field, validation, method, object,
and explicit object-field occurrences; inherited copy-with fields are not shown
as links back to the source object, and evaluated object validation results stay
in validation metadata rather than appearing as non-source Outline children.
Command-style targets and clause names/values now use exact compiler-owned
ranges for semantic colors, and command Outline entries select the target.
Assertion Outline children select the comparison operator. Direct uncertainty
comparisons underline the uncertain operand, percentile unit mismatches the
incompatible threshold, invalid probability forms the complete call, and
generic validation unit mismatches the right operand.
Uncertainty constructors now preserve exact positional, source, and named
key/value ranges too. `E-UNC-SOURCE-*` and `E-UNC-ARGS-*` Problems underline the
owning source or malformed value, including a nested call, while source paths,
option keys, distribution/method literals, and dotted option values receive
role-aware uncertainty colors only inside that argument. A declaration named
`method` or `kind` is therefore not repainted by its same-spelled option key.
Unquoted dotted imports remain one namespace token, so `eng.stats` does not
compete with shorter variable/property overlays.
Numeric fallback also respects identifier boundaries: percentile helpers,
solver literals, hash option names, and numbered bindings are not split into
identifier plus number colors. Decimal exponents, hyphenated workflow literals,
and compiler-owned composite units remain single semantic ranges without an
embedded operator color. The packaged LSP coverage gate checks all examples and
grammar fixtures and rejects every overlapping semantic-token pair. Strings are
emitted as fragments around interpolation parameters/properties, format precision
numbers, display units, and quoted import namespaces, preserving those nested
roles without overlapping ranges.

Role-aware highlighting also works on unsaved edits, so token colors do
not have to wait for a file save. Sample-table member completions include
runtime metadata such as `sample_count`, `row_hash_count`, and `row_preview`.
The extension declares EngLang-specific
role categories and theme fallback hints for units, quantities, axes, time
series, validation/report roles, side effects, external boundaries, inputs,
state, outputs, state-space vector types and members, built-in workflow helper functions, solver and
uncertainty policy
literals, module namespaces, model artifacts, DB/cache records, workflow steps,
string interpolation variables/properties, format precision, format units,
and review risks, so themes without EngLang-specific rules still receive stable
color hints. For stronger role separation, choose `EngLang Dark` or `EngLang
Light` from VS Code's Color Theme picker; the bundled themes split unit,
quantity, TimeSeries, workflow, validation, report, side-effect, external,
solver, model, DB, and cache role families across related colors. They define
direct colors for every EngLang role-aware selector contributed by the extension. Set
`englang.semanticHighlighting.enabled = false` to fall back to first-pass syntax
colors only; changing this setting refreshes the current editor colors and
planned/internal symbol markers immediately. Maintainer-facing color mapping
rules live in `docs/internal/editor/token_scopes.md`.
`EngLang: Inspect Highlight Tokens` opens a highlight data view with a plain
status summary, legend, selector/type/detail counts, domain coverage summary for
keywords, workflow words, options, units, constants, and operators, overall
scope-map coverage from the generated semantic legend, representative source-text
samples,
normalized highlight rows with primary selector, theme fallback coverage state,
direct selector coverage, status text that names both theme fallback-scope and
direct-selector mapping gaps, inspector panel hints, overlapping highlight ranges,
theme fallback scopes, and advanced highlight data for debugging theme or scope
mismatches. If no highlight data is available, the warning can
open
`EngLang: Show Tooling Status` so the selected live editor tool path is visible.
`EngLang: Inspect Highlight Token at Cursor` opens a cursor status summary, the
token under the caret when one exists, the nearest highlight tokens, copy-ready
text/range/selector/theme-coverage fields plus panel hints for the selected token,
line overlap rows, and the other highlight tokens on the same line. `EngLang:
Copy Highlight Token at Cursor` copies the current or nearest same-line role-aware
highlight token details directly to the clipboard; when the caret is between
tokens, the copied value is the nearest same-line highlight token details.

Review-risk decorations add a subtle left border and overview-ruler mark for
high and medium review risks without changing source text. High-risk
side-effect and external-boundary lines also receive a gutter warning icon;
high solver, data-quality, and other review categories do not. Set
`englang.reviewRiskDecorations.enabled = false` to hide those markers while
keeping diagnostics and role-aware highlighting enabled.

Evaluated class-object validation rules add one compact `validation passed` or
`validation failed` result at the end of each object declaration line. Hovering
the result shows the class, rule, and observed operands; multiple rules on the
same object are grouped, and any failure takes precedence. Rule declarations
and runtime-pending `validate` commands are intentionally not marked as passed.
Set `englang.validationDecorations.enabled = false` to hide these results while
keeping Problems diagnostics available.

After `EngLang: Run Current File`, explicit `align` and `resample` commands whose
latest source-matched native run produced `partial` or `unavailable` output get
one compact warning at the end of the command line and an overview-ruler mark.
Hovering the warning shows the source/target series, strategy, method, output
count, and runtime reason. Fully materialized output and automatic pairwise
comparison metadata are intentionally not marked. The extension verifies both
`source_path` and `source_hash` from `build/result/report_spec.json`, and clears
the marker as soon as any EngLang source in the workspace changes. Set
`englang.timeAlignmentDecorations.enabled = false` to hide these run-result
warnings without disabling Problems or role-aware highlighting.

The same run loads `build/result/review.json` for unresolved fill/imputation
outcomes and other medium/high runtime fallbacks. Partial or deferred explicit
`fill missing` commands get a compact warning with filled/missing/skipped
counts; gapped coverage gets `fill policy required` when no matching explicit
fill command already owns that source. Applied interpolation and complete
coverage remain unmarked. The extension verifies the review artifact's
`source_path` and `source_hash`, discards it when any EngLang source changes
during the run, and clears markers on the next source edit. Set
`englang.fallbackDecorations.enabled = false` to hide these warnings.

Completion uses the current unsaved buffer and compiler-owned editor metadata.
JavaScript does not maintain a separate keyword, type, quantity, or unit table.
If live completion is unavailable, the extension falls back to the generated
completion catalog from `generated/editor/englang-editor-metadata.json`. The
same generated metadata also supplies the highlight legend and syntax catalog
used by editor contract checks. Builtin first-paint roles come from
`syntax_catalog.workflow_builtin_groups`, rather than a separate JavaScript or
grammar list. Generic type completions keep their public labels
visible, but insert editable snippets such as `Array[T]` and
`LinearOperator[From -> To]` so type arguments can be replaced immediately.
The generated completion metadata also carries plain insert text and VS Code
snippet insert text for common helper and workflow completions such as
`file(...)`, `read text`, `http get`, `check coverage`, `fill missing`, and
`sample uniform`. Internal spellings such as `fill_missing` are not offered as
public completion labels. Static snippets remain
only for larger examples whose prefixes do not duplicate generated completion
labels.

Format Document, Format Selection, and closing-brace on-type formatting use the
current unsaved buffer, so VS Code and the command-line formatter share the
compiler-owned formatting rules. On-type formatting replaces only the closing
brace line, ignores braces inside strings/comments, and discards stale or
structurally mismatched formatter results.
JavaScript does not maintain separate indentation or block-formatting rules.

Go-to-definition uses the current unsaved buffer and every modified open
EngLang buffer in the workspace. Static file imports resolve changed, moved, or
new declarations from open text throughout recursive import chains before
reading unchanged files from disk; bundled `use eng.<module>` imports still
resolve to their source files. The result is discarded if any participating
document changes while lookup is running. If live definition lookup is
unavailable, the extension falls back to document symbols from the current
buffer for top-level symbols and nested symbols such as schema fields, class
fields, component ports, and object members. VS Code's workspace symbol search
scans `.eng` files under each open workspace folder. Find All References and
rename use the same open-document precedence plus static-import-resolved saved
files. Unrelated same-name symbols are excluded; broader package/index identity
support is not claimed yet.

## Grammar Maintenance

The generated TextMate grammar lives at `syntaxes/eng.tmLanguage.json`. Edit
`syntaxes/eng.tmLanguage.source.json`, then run:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat vscode-build-grammar
.\dev.bat vscode-grammar-test
```

The source grammar may use `{{...}}` placeholders for compiler-owned keyword,
constant, operator-word, keyword-group, type, unit, option, and highlight-only
legacy workflow alias and legacy unit alias lists. `vscode-build-grammar` expands
those placeholders from `generated/editor/englang-editor-metadata.json`.

The grammar smoke writes token-check output under
`build\editor-tests\textmate_tokens\grammar_smoke.json`.
When VS Code's bundled tokenizer runtime is available, `ide-check` also tokenizes
every `examples/**/*.eng` source and rejects catalog keywords that have no
foreground scope in either bundled theme. It also verifies role-sensitive
first-pass scopes for helpers in argument defaults, missing-policy options, and
compound derivative units. This catches include-priority gaps that isolated
regular-expression checks cannot reproduce.

## Editor Metadata

The extension loads its highlight legend and syntax catalog through
`editorMetadata.js` from `generated/editor/englang-editor-metadata.json`,
generated from `eng-lsp --editor-metadata`. Split generated files are also
written for review: `englang-semantic-legend.json`,
`englang-completions.json`, and `englang-syntax.json`. The same metadata file
provides the static completion fallback used when live completion is
unavailable. Tooling reads the `completion_items` catalog directly; the editor
metadata contract no longer publishes a duplicate completion seed alias.
`syntax_catalog.workflow_builtin_groups` supplies the shared model, uncertainty,
TimeSeries, solver, validation, boundary, path, temporal, deprecated, and
workflow-step first-paint lists used by the generated grammar and native IDE.
`syntax_catalog.legacy_workflow_builtin_aliases` and
`syntax_catalog.legacy_workflow_option_aliases` contain highlight-only compatibility
spellings that are not added back to completions. `syntax_catalog.units` contains
compiler unit labels; `syntax_catalog.legacy_unit_aliases` contains
highlight-only compatibility aliases such as byte-size units and `%`.
`syntax_catalog.model_fields`, `syntax_catalog.prediction_table_fields`,
`syntax_catalog.coverage_result_fields`,
`syntax_catalog.time_alignment_result_fields`, `syntax_catalog.table_fields`,
and `syntax_catalog.case_run_result_table_fields`, along with the
HTTP/sample/DB/case field catalogs, are compiler-owned public member API
catalogs used by TextMate public-member highlighting and local completion
fallback. The TimeSeries alignment catalog exposes the distinct materialization
and axis-comparison statuses, counts, method, tolerance, and step available on
bound native `align`/`resample` outputs. These catalogs describe runtime-backed public fields,
not editor-only placeholders.
Regenerate it after LSP completion, keyword, constant,
operator-word, option, type, unit, public member field, legacy workflow alias,
legacy unit alias, or highlight legend changes:

```bat
.\dev.bat vscode-build-editor-metadata
.\dev.bat ide-check
```
