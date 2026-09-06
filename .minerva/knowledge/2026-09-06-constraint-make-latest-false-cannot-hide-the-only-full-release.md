# `--latest=false` cannot keep the only full release off GitHub's Latest badge

**Date**: 2026-09-06
**Type**: constraint
**Summary**: GitHub's "latest" is the newest non-prerelease, so when every other release is a prerelease the one full release is latest regardless of `make_latest`
**Context**: .minerva/work/2026-09-06-whisper-model-github-release (see git history if the worktree has been cleaned up)

## Context

Every app release in this repository is a `main-<sha>` prerelease cut by `release.yml`. The
`models` release, which holds the whisper model asset and is not an app release at all, was
created with `gh release create models --latest=false` to keep it from being presented as the
app's latest release.

## Finding

It became the latest release anyway: `gh release list` labelled it `Latest` and
`GET /repos/{owner}/{repo}/releases/latest` returned `models`. GitHub defines the latest release
as the most recent non-draft, non-prerelease release; `make_latest: false` only declines to
_promote_ a release when another full release exists. With every other release marked
prerelease, the only full release is latest by definition and the flag has nothing to act on.

Marking the release a prerelease as well (`gh release edit models --prerelease`) restored the
prior state: no release carries the badge and `releases/latest` is 404 again.

## Implications

- In a repository whose real releases are all prereleases, any auxiliary release (assets,
  models, fixtures) must itself be a prerelease, or it becomes what `releases/latest` and the
  Releases page advertise.
- Check `gh release view <tag> --json isPrerelease` and the `releases/latest` endpoint after
  creating one; `gh release list`'s `Latest` column can lag an edit by a request.

## Related

- [[2026-09-06-decision-whisper-model-served-from-the-models-github-release]] — the release this was discovered on
