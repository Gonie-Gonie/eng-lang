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
the committed screenshot manifest, dimensions, hashes, and PNG headers.

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

The automated gate protects baseline integrity and compiler/editor contracts;
it does not claim to drive VS Code or perform a pixel-diff in CI.
