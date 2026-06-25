from __future__ import annotations

from collect_docs_sources import REPO_ROOT, build_combined_markdown


OUTPUT = REPO_ROOT / "artifacts" / "docs" / "englang-release-notes.md"


def main() -> None:
    candidates = [
        REPO_ROOT / "docs" / "release" / "release-state.md",
        REPO_ROOT / "docs" / "current" / "feature_maturity_matrix.md",
        REPO_ROOT / "docs" / "release" / "v0.1.0.md",
    ]
    sources = [path for path in candidates if path.exists()]
    output = build_combined_markdown(sources, OUTPUT)
    print(output)


if __name__ == "__main__":
    main()
