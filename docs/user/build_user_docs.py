from __future__ import annotations

import argparse
from pathlib import Path

from oodocs import Chapter, Document, DocumentSettings, Paragraph, Section, bold, code


def paragraph(*parts: object) -> Paragraph:
    return Paragraph(*parts)


def build_document(version: str) -> Document:
    is_preview = "preview" in version
    title = "EngLang User Test Guide" if is_preview else "EngLang User Guide"
    current_line = (
        "The current public line is a pre-release package: typed CSV import, unit-aware calculations, "
        "TimeSeries statistics, structured runtime logs, explicit process-result records, local test-result records, PlotSpec/SVG/report generation, native IDE user testing, and basic packaged execution."
        if is_preview
        else "EngLang 1.0.0 is a stable-core package: typed CSV import, unit-aware calculations, TimeSeries statistics, structured runtime logs, explicit process-result records, local test-result records, PlotSpec/SVG/report generation, native IDE user testing, and basic packaged execution are supported within the documented scope."
    )
    boundary = (
        "The packaged "
        if is_preview
        else "In the stable-core package, "
    )
    boundary_detail = (
        " path and the "
        if is_preview
        else ", "
    )
    kernel_tail = (
        " JIT planning surface are experimental and intended for smoke checks and inspection. The "
        if is_preview
        else " JIT planning surface, and "
    )
    final_boundary = (
        " harness records interpreter baseline timings only and marks JIT execution as unavailable. Backend selection metadata can record a native-preview request, but native execution remains unavailable. This release is not a stable language contract, not a full editor platform, not a general nonlinear solver, not a native JIT runtime, and not a complete domain package ecosystem. Those are future tracks. The public claim for this release is a pre-release data-to-report workflow with a native test IDE."
        if is_preview
        else " harness remain internal and intended for smoke checks and inspection. Backend selection metadata can record a native-preview request, but native execution remains unavailable. The stable contract is intentionally narrow: documented syntax, artifact formats, official examples, package smoke, and the native tester workflow. It is not a full editor platform, not a general nonlinear solver, not a native JIT runtime, and not a complete domain package ecosystem."
    )
    return Document(
        title,
        Chapter(
            "1. What EngLang Is",
            paragraph(
                "EngLang is a native engineering language for workflows where ",
                bold("units, physical quantity kinds, data schemas, plots, reports, and provenance"),
                " should be checked as part of the program rather than as after-the-fact spreadsheet convention.",
            ),
            paragraph(
                current_line,
            ),
        ),
        Chapter(
            "2. Portable Package",
            paragraph(
                "Extract the Windows x64 portable zip and run ",
                code("eng-ide.exe"),
                " for the native IDE or ",
                code("eng.exe"),
                " for command-line checks. The package also includes internal ",
                code("eng-lsp.exe"),
                " for editor-service smoke checks. The target PC does not need Rust, Node, Visual Studio Build Tools, or a browser IDE.",
            ),
            paragraph(
                "Recommended first commands are ",
                code("eng.exe doctor"),
                ", ",
                code("eng-ide.exe --smoke"),
                ", and ",
                code("eng-lsp.exe --smoke"),
                ".",
            ),
            paragraph(
                "The package documentation is intentionally curated. Developer notes, master plans, and release checklists "
                "stay in the repository, while the package ships this PDF and a short README.",
            ),
        ),
        Chapter(
            "3. Native IDE Workflow",
            Section(
                "Open and Edit",
                paragraph(
                    "Use the Explorer to open official examples or create a new ",
                    code(".eng"),
                    " file. The main area is split into scrollable Code on the left and scrollable Result on the right, with a draggable divider and separate Variables, Completions, and Runtime Summary sidebar. While editing, the IDE suggests variables, identifiers, keywords, quantities, units, and snippets from the cursor prefix; press Tab to apply the first suggestion. Parentheses, brackets, braces, and quotes are closed automatically.",
                ),
            ),
            Section(
                "Check and Run",
                paragraph(
                    "Use ",
                    code("Check"),
                    " for compiler diagnostics and ",
                    code("Run"),
                    " to generate result artifacts. Successful runs show the report, result, review, run log, process results, test results, PlotSpec, plot manifest, and SVG paths.",
                ),
            ),
            Section(
                "Inspect Plot Output",
                paragraph(
                    "The IDE renders PlotSpec data directly and still exposes the generated ",
                    code("plots/timeseries.svg"),
                    " and ",
                    code("report.html"),
                    " files for external review. The in-IDE plot view includes grid lines, x/y ticks, zero baseline handling, and line/scatter/bar/histogram rendering.",
                ),
            ),
            Section(
                "Inspect Runtime Output",
                paragraph(
                    "After a successful run, open the ",
                    code("Runtime"),
                    " tab in the right panel. It summarizes uncertainty distributions, propagation methods, "
                    "p05/p50/p95 values, ML train/test counts, metrics, coefficients, leakage status, and loss history directly from ",
                    code("result.engres"),
                    ". It also shows the experimental ",
                    code("eng-kernel-plan-v1"),
                    " kernel plan for the current file, with backend, candidate kind, lowering status, estimated rows, input/output counts, operation-class count, scan count, source, reason, and operation list. This plan is inspection metadata only; execution still uses the normal runtime path.",
                ),
            ),
        ),
        Chapter(
            "4. Recommended User Test",
            paragraph(
                "Start with ",
                code("examples/official/03_integrated_hvac/main.eng"),
                ". It exercises Args defaults, typed CSV promotion, DateTime parsing, missing-value interpolation, schema constraints, "
                "HeatRate calculation, statistics, integration, PlotSpec generation, report output, and simple thermal system metadata.",
            ),
            paragraph(
                "From a command prompt, the equivalent smoke command is ",
                code("eng.exe run examples\\official\\03_integrated_hvac\\main.eng --save-artifacts"),
                ".",
            ),
            paragraph(
                "For the integrated measured-vs-simulated workflow, run ",
                code("eng.exe run examples\\official\\17_measured_vs_simulated\\main.eng --profile repro --save-artifacts"),
                ". It promotes weather and measured CSV inputs, simulates a one-state thermal model into ",
                code("sim.T_zone"),
                ", computes RMSE, records validation and time-alignment metadata, and emits a multi-series PlotSpec.",
            ),
            paragraph(
                "For internal-track smoke testing, open ",
                code("examples/internal/04_uncertainty_core/main.eng"),
                " and verify the in-IDE Runtime uncertainty summary plus the histogram view. For data-driven modeling smoke testing, open ",
                code("examples/internal/05_data_driven_modeling/main.eng"),
                " and verify the in-IDE ML metrics, coefficients, loss history, and parity plot.",
            ),
        ),
        Chapter(
            "5. Expected Artifacts",
            paragraph(
                "A successful run writes ",
                code("build/result/result.engres"),
                ", ",
                code("review.json"),
                ", ",
                code("report_spec.json"),
                ", ",
                code("report.html"),
                ", ",
                code("plots/plot_spec.json"),
                ", ",
                code("plots/plot_manifest.json"),
                ", ",
                code("plots/timeseries.svg"),
                ", ",
                code("run_log.json"),
                ", ",
                code("process_results.json"),
                ", ",
                code("test_results.json"),
                ", and ",
                code("output_manifest.json"),
                ".",
            ),
            paragraph(
                "The result records policy execution, computed statistics, integration provenance, plot hashes, and "
                "system solver boundary metadata. Internal-track examples may also record uncertainty and ML metadata.",
            ),
        ),
        Chapter(
            "6. Current Boundaries",
            paragraph(
                boundary,
                code("eng-lsp.exe"),
                boundary_detail,
                code("eng-kernel-plan-v1"),
                kernel_tail,
                code("eng-jit-bench-v1"),
                final_boundary,
            ),
        ),
        settings=DocumentSettings(
            metadata_author="EngLang",
            subtitle=f"Portable Windows user guide v{version}",
        ),
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pdf", required=True)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()

    pdf_path = Path(args.pdf)
    pdf_path.parent.mkdir(parents=True, exist_ok=True)
    document = build_document(args.version)
    document.save_pdf(pdf_path)


if __name__ == "__main__":
    main()
