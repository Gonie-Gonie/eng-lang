from __future__ import annotations

import argparse
from pathlib import Path

from oodocs import Chapter, Document, DocumentSettings, Paragraph, Section, bold, code


def paragraph(*parts: object) -> Paragraph:
    return Paragraph(*parts)


def build_document(version: str) -> Document:
    return Document(
        "EngLang Language Grammar Guide",
        Chapter(
            "1. Execution Shape",
            paragraph(
                "EngLang executes one source file as a ",
                bold("top-level workflow"),
                ". There is no public entry selector and no script-main execution root. "
                "Use one root ",
                code("args { ... }"),
                " block for CLI inputs and write workflow statements at top level.",
            ),
            paragraph(
                "Imported files may contribute functions and importable constants. Imported executable top-level bodies are not run or merged into the caller workflow.",
            ),
        ),
        Chapter(
            "2. Core Top-Level Forms",
            paragraph(
                "Supported declaration families include ",
                code("use \"file.eng\""),
                ", ",
                code("args { ... }"),
                ", ",
                code("const name: Quantity = expression"),
                ", explicit typed declarations, fast bindings, schemas, functions, systems, domains, components, ",
                code("print"),
                ", ",
                code("export summary to csv"),
                ", and ",
                code("report { ... }"),
                ".",
            ),
            paragraph(
                code("struct Args"),
                " and ",
                code("script"),
                " blocks are rejected compatibility syntax. The root argument syntax is exactly ",
                code("args { ... }"),
                ".",
            ),
        ),
        Chapter(
            "3. Calls And Commands",
            paragraph(
                "General user-defined and library function calls stay parenthesized, for example ",
                code("heat_loss(UA, dT)"),
                " or ",
                code("mean(Q, axis=Time)"),
                ". Parenthesis-light syntax is reserved for built-in workflow verbs.",
            ),
            Section(
                "Command Verbs",
                paragraph(
                    "The current command-style verbs are ",
                    code("integrate"),
                    ", ",
                    code("mean"),
                    ", ",
                    code("max"),
                    ", ",
                    code("min"),
                    ", ",
                    code("duration"),
                    ", ",
                    code("plot"),
                    ", ",
                    code("show"),
                    ", and ",
                    code("validate"),
                    ".",
                ),
                paragraph(
                    "Examples lower to canonical call strings: ",
                    code("integrate Q over Time"),
                    " becomes ",
                    code("integrate(Q, over=Time)"),
                    ", and ",
                    code("mean Q over Time"),
                    " becomes ",
                    code("mean(Q, axis=Time)"),
                    ".",
                ),
                paragraph(
                    "Complex command targets must be parenthesized. ",
                    code("integrate Q1 + Q2 over Time"),
                    " is rejected with ",
                    code("E-CMD-AMBIG-001"),
                    "; write ",
                    code("integrate (Q1 + Q2) over Time"),
                    ".",
                ),
            ),
        ),
        Chapter(
            "4. Local Context Blocks",
            Section(
                "where",
                paragraph(
                    code("where"),
                    " introduces local calculations for the immediately preceding owner expression or command. Where-local names are visible to that owner and to later locals in the same block only.",
                ),
                paragraph(
                    "Using a where-local outside its owner raises ",
                    code("E-NAME-LOCAL-001"),
                    ". Referencing a later local in the same where block raises ",
                    code("E-WHERE-FWD-001"),
                    ".",
                ),
            ),
            Section(
                "with",
                paragraph(
                    code("with"),
                    " introduces options for the immediately preceding owner expression or command. It is used for method, backend, display, solver, and artifact options rather than calculations.",
                ),
                paragraph(
                    "Unknown options raise ",
                    code("E-WITH-OPTION-001"),
                    ". Incompatible display-unit options raise ",
                    code("E-WITH-UNIT-001"),
                    " when the owner type is known.",
                ),
            ),
        ),
        Chapter(
            "5. Output Policy",
            paragraph(
                code("print"),
                " is for debugging and CLI output. Format expressions are type-checked and requested units must be compatible, such as ",
                code("print \"E = {E: .2 kWh}\""),
                ".",
            ),
            paragraph(
                code("export summary to csv \"summary.csv\" { ... }"),
                " writes durable scalar summary artifacts. Report, show, and export commands are the reproducible artifact path.",
            ),
        ),
        Chapter(
            "6. Review Metadata",
            paragraph(
                code("eng check --review"),
                " records ",
                code("command_styles"),
                ", ",
                code("where_blocks"),
                ", and ",
                code("with_blocks"),
                " so surface syntax remains reviewable while downstream compiler/runtime paths consume canonical expressions.",
            ),
            paragraph(
                "The official example ",
                code("examples/official/09_command_where_with/main.eng"),
                " exercises command-style integration/statistics, where locals, with options, print/export, and plot/report output.",
            ),
        ),
        settings=DocumentSettings(
            metadata_author="EngLang",
            subtitle=f"Grammar and command policy v{version}",
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
