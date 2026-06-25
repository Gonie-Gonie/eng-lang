# Documentation Authoring

Markdown is the source of truth for hand-written documentation. Generated JSON
metadata under build/docs may feed reference bundles, and OODocs may publish
curated DOCX/PDF/HTML artifacts.

Rules:

- Keep first-user material under docs/user and docs/workflows.
- Keep lookup material under docs/reference.
- Keep implementation tracks under docs/internal.
- Keep historical long-form plans under docs/archive/old-plans.
- Mark non-runnable EngLang snippets as partial, future, or unchecked.
- Do not make EngLang runtime commands depend on Python or OODocs.
