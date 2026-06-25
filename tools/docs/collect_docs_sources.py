from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def markdown_sources(*roots: str) -> list[Path]:
    files: list[Path] = []
    for root in roots:
        directory = REPO_ROOT / root
        if directory.exists():
            files.extend(sorted(directory.rglob("*.md")))
    return files


def build_combined_markdown(paths: list[Path], output: Path) -> Path:
    output.parent.mkdir(parents=True, exist_ok=True)
    sections = []
    for path in paths:
        rel = path.relative_to(REPO_ROOT).as_posix()
        sections.append(f"<!-- source: {rel} -->\n\n{path.read_text(encoding='utf-8').strip()}\n")
    output.write_text("\n\n".join(sections), encoding="utf-8")
    return output
