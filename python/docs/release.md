# Python Release Automation

Canonical reference for Python wheel/sdist release automation for `rehuman`.

## Trigger Model

Automation is tag/release-driven and supports both automatic and manual runs.

### Workflow 1: Build + Attach Artifacts

Workflow: `.github/workflows/python-release-artifacts.yml`

- Auto trigger: GitHub Release `published`
- Manual trigger: `workflow_dispatch` with required `tag`

Behavior:

- Resolves tags in either format: `vX.Y.Z` or `X.Y.Z`
- Fails early if the tag does not exist or the corresponding GitHub release is missing
- Validates tag version against:
  - root `Cargo.toml` package version
  - `python/Cargo.toml` package version
  - `python/pyproject.toml` project name (`rehuman`)
- Builds and uploads release artifacts:
  - Wheels for Linux/macOS/Windows matrix
  - One source distribution (`sdist`)
  - Deterministic `SHA256SUMS`
- Re-run behavior: assets are replaced (`gh release upload --clobber`)

### Workflow 2: Publish to PyPI

Workflow: `.github/workflows/python-pypi-publish.yml`

- Auto trigger: GitHub Release `published` (stable releases only)
- Manual trigger: `workflow_dispatch` with required `tag`

Behavior:

- Skips auto-publish for prereleases
- Manual prerelease publish requires `allow_prerelease=true`
- Prerelease status is read from GitHub Release metadata (`prerelease` field)
- Downloads artifacts from GitHub Release assets (never rebuilds)
- Requires at least one `.whl` and one `.tar.gz`
- Publishes with idempotent mode (`skip-existing: true`)
- Verifies the expected version appears on PyPI after publish

## Artifact Matrix

Current Python wheel targets:

- Linux: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- macOS: `x86_64-apple-darwin`, `aarch64-apple-darwin`
- Windows: `x86_64-pc-windows-msvc`

Known limitation:

- Windows ARM64 wheels are not built yet.

## Why sdist Is Included

The source distribution (`.tar.gz`) provides a fallback path for environments where
no compatible prebuilt wheel exists.

## One-Time PyPI Trusted Publisher Setup

Configure in PyPI project settings for `rehuman`:

1. Add Trusted Publisher
2. Set GitHub owner + repository
3. Set workflow file: `python-pypi-publish.yml`
4. Environment can remain unset for immediate publish-on-release

No `PYPI_API_TOKEN` secret is required for Trusted Publishing.

## Security/Permissions Note

Because this pipeline publishes immediately on stable GitHub Release events, any
maintainer with permission to publish a release effectively has permission to
trigger PyPI publication for that release.

## Manual Re-run Examples

Build/re-attach release artifacts for existing tag:

- Dispatch `python-release-artifacts.yml`
- Input `tag: v0.1.2` (or `0.1.2`)

Re-publish existing release artifacts to PyPI (idempotent):

- Dispatch `python-pypi-publish.yml`
- Input `tag: v0.1.2` (or `0.1.2`)
- Optional prerelease override: `allow_prerelease: true`

## Troubleshooting

- `No tag found`: ensure tag exists in repo and matches input spelling
- `No release found`: create/publish GitHub Release for the tag first
- Version mismatch errors: sync versions in root and `python/` Cargo manifests
- `No wheel/sdist assets`: run the artifact workflow before publish workflow
- PyPI verification timeout: retry publish workflow after a short delay

## Future Hardening TODOs

- Evaluate workspace-level version inheritance to reduce duplicate version bumps.
- Consider adding Windows ARM64 wheel builds once runner support is stable.
