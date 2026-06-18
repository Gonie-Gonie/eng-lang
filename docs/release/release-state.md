# Release State Audit

Last checked: 2026-06-18, Asia/Seoul.

This file records what has actually been published, what only exists as a git
tag or local release gate, and what should not be described as a release.

Checks used for this audit:

```text
git tag --list --sort=creatordate
git ls-remote --tags origin
GitHub Releases API: https://api.github.com/repos/Gonie-Gonie/eng-lang/releases
```

## Short Answer

`v0.1.0` was not released.

The first public release was `v0.1-preview`. That release used workspace package
version `0.1.0-preview`, but the user-facing tag, release note, package name,
and GitHub Release page were all `v0.1-preview`, not `v0.1.0`.

As of this audit, the only published GitHub Release page visible from the public
GitHub API is:

```text
tag:          v0.1-preview
name:         EngLang v0.1-preview
kind:         prerelease
published_at: 2026-06-08T01:31:27Z
url:          https://github.com/Gonie-Gonie/eng-lang/releases/tag/v0.1-preview
```

## Terms

Use these terms consistently:

- Workspace version: the Cargo/package version in `Cargo.toml`, for example
  `0.1.0-preview` or `1.0.0`.
- Public label: the user-facing release label, for example `v0.1-preview` or
  `v1.0.0`.
- Git tag: a repository ref. A tag alone is traceability, not a published
  release package.
- GitHub Release: a published GitHub release page with attached assets or an
  explicit asset-publishing decision.
- Release-ready documentation: release notes and checklist entries that describe
  a planned or locally validated slice. This is not the same as publication.

## Observed State

| Candidate | Git tag state | GitHub Release state | Meaning |
| --- | --- | --- | --- |
| `v0.1.0` | No local or remote tag found | No published release found | Not released. Do not claim `0.1.0` as a public release. |
| `0.1.0-preview` | Workspace version for first preview, not a public tag | No direct release label | Internal package version behind `v0.1-preview`. |
| `v0.1-preview` | Local and remote tag exist | Published GitHub prerelease on 2026-06-08 | First public preview release. |
| `v0.2-preview` to `v0.6-preview` | Local and remote tags exist | No published GitHub Release pages found by public API | Historical tagged preview slices; release notes/checklists describe local readiness, not current public publication pages. |
| `v0.7-preview` to `v0.9-preview` | No matching remote preview tags found | No published GitHub Release pages found by public API | Release-note labels exist, but the remote tags are `v0.7-alpha`, `v0.8-alpha`, and `v0.9-alpha`. Treat as historical naming mismatch, not published preview releases. |
| `v0.7-alpha` to `v0.9-alpha` | Local and remote commit tags exist | No published GitHub Release pages found by public API | Historical alpha/readiness tags. Do not present as public release packages. |
| `v1.0-stable` | Local and remote commit tag exists | No published GitHub Release page found by public API | Historical readiness tag. Superseded by the SemVer public label rule. |
| `v1.0.0` | Local and remote commit tag exists | No published GitHub Release page found by public API | Current stable-core public line in docs and package naming, but publication is not complete until a GitHub Release page and assets are published. |
| `v1.0.1` and `v1.0.2` | Local and remote commit tags exist | No published GitHub Release pages found by public API | Historical patch tags. Do not imply public package publication unless release pages/assets are later created. |

## Current Naming Rule

For public user-facing releases:

```text
Preview line: v0.x-preview
Stable line:  v1.0.0, v1.0.1, ...
```

Do not use `v0.1.0` for the first preview. If referring to the package version
behind that preview, write `workspace version 0.1.0-preview`.

Do not use `v1.0-stable` for a new user-facing release. Use SemVer tags such as
`v1.0.0` and publish a GitHub Release page with the generated assets.

## Publication Rule

A release is complete only after all of these are true:

```text
1. The release note exists and names the intended public label.
2. .\dev.bat release-check passes locally.
3. The git tag exists locally and on origin.
4. A GitHub Release page is published for that exact tag.
5. The expected zip, checksum, user guide PDF, and release manifest are attached
   or the release note explicitly states why an asset is intentionally omitted.
6. This audit file is updated with the observed publication state.
```

Local package generation, `release-check`, or a pushed tag can say a release is
ready. They should not be worded as published unless the GitHub Release page and
asset step is complete.
