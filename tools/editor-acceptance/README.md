# Editor Visual Acceptance

This directory is the bounded visual acceptance surface for the VS Code
extension and the native IDE. It contains no user project data and does not run
the workflow examples.

`vscode-workspace/main.eng` and
`native-workspace/examples/official/01_editor_visual/main.eng` must remain byte
identical. The source covers declarations, imports, quantities, units,
TimeSeries operations, workflow steps, validation, reports, and side effects.
It must produce no diagnostics. `vscode-workspace/diagnostics.eng` is separate
and intentionally produces four dimension diagnostics.

## Automated Contract

Run:

```bat
.\dev.bat editor-visual-check
```

The command builds `eng-lsp` and verifies the bounded workspace definitions,
matching VS Code/native source and data, clean main-source diagnostics, the
semantic token type/modifier coverage, the intentional diagnostic corpus, and
the committed screenshot manifest, dimensions, hashes, and PNG headers. It also
runs the pixel-comparison engine against the accepted captures as a decoder and
zero-diff self-check.

## VS Code Capture

Install the current extension, then open each workspace in a separate window:

```bat
.\dev.bat vscode-install
code --new-window tools\editor-acceptance\vscode-light.code-workspace --goto tools\editor-acceptance\vscode-workspace\main.eng:1:1
code --new-window tools\editor-acceptance\vscode-dark.code-workspace --goto tools\editor-acceptance\vscode-workspace\main.eng:1:1
```

Trust only this bounded repository workspace when VS Code asks. Wait until the
status bar reports `EngLang`, the Problems count is zero, and role-aware colors
have replaced the first-pass syntax colors. Hide the primary and secondary side
bars for a source-focused capture. Record the full application window so the
selected theme, source, status bar, minimap, and overview-ruler markers remain
visible.

## Native IDE Capture

Build and launch the native IDE from its bounded workspace root:

```powershell
cargo build --release -p eng_ide -p eng_lsp
Push-Location tools\editor-acceptance\native-workspace
..\..\..\target\release\eng-ide.exe
Pop-Location
```

Open `examples/official/01_editor_visual/main.eng` if it is not selected, run
`Check`, and require `Errors 0` and `Warnings 0`. Keep the Problems panel open
for the baseline so the clean native diagnostic state is visible.

## Baselines

The inspected captures are stored in `baselines/` and described by
`baseline-manifest.json`. Re-record them only for an intentional editor,
theme, fixture, or layout change. Compare each replacement manually with the
previous image for missing color families, low contrast, clipped text,
overlap, unexpected diagnostics, and stale language mode. Then update the
manifest dimensions and SHA-256 values and rerun `editor-visual-check`.

Before replacing a baseline, place the three new full-window captures in one
directory using these exact names:

```text
vscode-light.png
vscode-dark.png
native-ide-light.png
```

Then run:

```bat
.\dev.bat editor-visual-compare path\to\current-captures
```

The comparison requires identical dimensions, treats an RGB channel delta over
24 as a changed pixel, and fails above a 3% changed-pixel ratio or a mean RGB
channel delta of 3.0. It writes a JSON summary and magenta difference images to
`build/editor-tests/visual-diff`. The thresholds allow small rasterization
noise; they do not make a changed theme, layout, diagnostics state, or missing
color family acceptable.

The automated gate protects baseline integrity and compiler/editor contracts
and can compare supplied captures. It does not drive VS Code or the native IDE,
so capture state and any accepted visual change still require manual review.
