from __future__ import annotations

import argparse
from pathlib import Path


USER_GUIDE_PARTS = [
    "user/index.md",
    "user/getting_started.md",
    "user/tutorial/01_install_and_doctor.md",
    "user/tutorial/02_first_unit_calculation.md",
    "user/tutorial/03_args_and_files.md",
    "user/tutorial/04_schema_promote_csv.md",
    "user/tutorial/05_timeseries_statistics.md",
    "user/tutorial/06_plot_report_review.md",
    "user/tutorial/07_validation_and_diagnostics.md",
    "user/tutorial/08_side_effects_and_artifacts.md",
    "user/tutorial/09_functions_imports_const.md",
    "user/tutorial/10_uncertainty_basics.md",
    "user/tutorial/11_native_ide_review.md",
    "user/tutorial/12_composite_workflow.md",
    "user/howto/README.md",
    "user/howto/read_csv_and_plot.md",
    "user/howto/create_report.md",
    "user/howto/run_external_command.md",
    "user/howto/save_artifacts.md",
    "user/howto/review_llm_generated_code.md",
    "user/howto/use_native_ide.md",
    "user/concepts/README.md",
    "user/concepts/semantic_workflow.md",
    "user/concepts/units_quantities_axes.md",
    "user/concepts/schema_boundary.md",
    "user/concepts/time_series.md",
    "user/concepts/reviewability.md",
    "user/concepts/uncertainty.md",
    "user/concepts/side_effects.md",
]


def assemble_user_guide_markdown(docs_root: Path) -> Path:
    build_path = docs_root.parent / "build" / "docs" / "oodocs" / "user_guide.md"
    build_path.parent.mkdir(parents=True, exist_ok=True)

    sections = []
    for relative in USER_GUIDE_PARTS:
        path = docs_root / relative
        if not path.exists():
            raise FileNotFoundError(f"missing user guide source: {path}")
        text = path.read_text(encoding="utf-8").strip()
        sections.append(f"<!-- source: docs/{relative} -->\n\n{text}\n")

    build_path.write_text("\n\n".join(sections), encoding="utf-8")
    return build_path


def build_document(version: str):
    from oodocs import DocumentSettings, from_markdown_file

    docs_root = Path(__file__).resolve().parents[1]
    markdown_path = assemble_user_guide_markdown(docs_root)
    is_preview = "preview" in version
    subtitle_prefix = "Preview portable Windows user guide" if is_preview else "Portable Windows user guide"
    return from_markdown_file(
        markdown_path,
        title="EngLang User Guide",
        numbered=True,
        toc=True,
        settings=DocumentSettings(
            metadata_author="EngLang",
            subtitle=f"{subtitle_prefix} v{version}",
        ),
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pdf")
    parser.add_argument("--version", default="dev")
    parser.add_argument("--assemble-markdown", action="store_true")
    args = parser.parse_args()

    if args.assemble_markdown and args.pdf is None:
        docs_root = Path(__file__).resolve().parents[1]
        print(assemble_user_guide_markdown(docs_root))
        return

    if args.pdf is None:
        parser.error("--pdf is required unless --assemble-markdown is used")

    pdf_path = Path(args.pdf)
    pdf_path.parent.mkdir(parents=True, exist_ok=True)
    document = build_document(args.version)
    document.save_pdf(pdf_path)


if __name__ == "__main__":
    main()
