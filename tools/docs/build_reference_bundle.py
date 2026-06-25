from __future__ import annotations

from collect_docs_sources import REPO_ROOT, build_combined_markdown, markdown_sources


OUTPUT = REPO_ROOT / "artifacts" / "docs" / "englang-reference.md"


def main() -> None:
    sources = markdown_sources("docs/reference")
    output = build_combined_markdown(sources, OUTPUT)
    print(output)


if __name__ == "__main__":
    main()
