from __future__ import annotations

import argparse
from pathlib import Path

from oodocs import DocumentSettings, from_markdown_file


def build_document(version: str):
    docs_root = Path(__file__).resolve().parents[1]
    markdown_path = docs_root / "guide" / "language_grammar.md"
    return from_markdown_file(
        markdown_path,
        title="EngLang Language Grammar Guide",
        numbered=True,
        toc=True,
        settings=DocumentSettings(
            metadata_author="EngLang",
            subtitle=f"Practical grammar and command policy v{version}",
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
