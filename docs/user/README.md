# EngLang User Documentation Source Notes

If you want to read the user documentation, start at [index.md](index.md).

This file is for contributors maintaining the curated source material for
release-facing user documents. The Markdown files under this directory are the
source of truth for user documentation.

Curated user guide assembly path:

- [index.md](index.md)
- [getting_started.md](getting_started.md)
- [tutorial/01_install_and_doctor.md](tutorial/01_install_and_doctor.md)
  through [tutorial/12_composite_workflow.md](tutorial/12_composite_workflow.md)
- [howto/README.md](howto/README.md)
- [concepts/README.md](concepts/README.md)

`.\dev.bat user-docs-markdown` assembles those Markdown sources into
`build/docs/oodocs/user_guide.md` for publishing checks without requiring
OODocs. The wrapper uses the repo-local portable Python only for optional
documentation publishing; it is not part of the EngLang runtime, workflow
examples, tests, or package smoke path. PDF generation passes the assembled
Markdown to OODocs only when a PDF is requested.

The assembled guide is a build artifact. Do not add a second checked-in
monolithic guide beside the topic files; it duplicates the canonical sources
and becomes stale independently.

The release package must not ship the whole developer documentation tree. It
should ship only polished user artifacts such as:

- EngLang_User_Guide.pdf
- EngLang_Language_Grammar_Guide.pdf
- a short package README.txt
- PACKAGE_ASSETS.txt describing packaged portable/installable assets and
  support boundaries

Development-only material remains in docs/development, docs/architecture,
docs/archive/old-plans, docs/current, and release checklists. Solver-heavy,
experimental, or internal-track material should not become first-user
documentation unless a current status document marks it public and stable.
