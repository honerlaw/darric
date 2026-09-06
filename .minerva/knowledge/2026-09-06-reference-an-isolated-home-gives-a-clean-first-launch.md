# Launching the debug binary with an isolated `HOME` reproduces a first launch

**Date**: 2026-09-06
**Type**: reference
**Summary**: `default_model_path` and the SQLite database both hang off `$HOME`, so `HOME=<scratch dir> target/debug/darric` runs the startup model download against an empty cache without touching the real one
**Context**: .minerva/work/2026-09-06-whisper-model-github-release (see git history if the worktree has been cleaned up)

## Context

The startup model download runs from `setup()` whenever
`~/Library/Application Support/darric/ggml-large-v3-turbo.bin` is missing. Exercising it end to
end normally means deleting or moving the real 1.6 GB cache — which also breaks any sibling
session's whisper tests that load the same file, and leaves the real app without a model until
the download completes.

## Finding

Both `model::default_model_path` and `db::path` resolve under `$HOME`, so running the debug
binary as `HOME=<empty scratch dir> RUST_LOG=darric_lib=info target/debug/darric` gives a
process with no cached model and a fresh database. It logs the whole download (`[model] not
found — downloading`, per-percent progress, `checksum verified`, `download complete`) to stderr
and leaves the downloaded file under the scratch `HOME` for hashing. With the real app already
running, the MCP server bind fails with "port busy" and is reported, but the download proceeds
regardless. A 1.55 GB download from the GitHub release took about 90 s.

## Implications

- Use this for any change to the download path, or to a second model asset, instead of
  disturbing the real cache; delete the scratch copy afterwards.
- The webview may not come up when launched this way (no Vite dev server); the download and
  the log lines do not depend on it.

## Related

- [[2026-09-06-decision-whisper-model-served-from-the-models-github-release]] — the download this technique verified
- [[2026-09-05-constraint-tauri-events-from-setup-reach-no-webview]] — why the log, not a UI event, is the observable here
- [[2026-09-05-reference-model-rs-download-paths-have-no-tests]] — the network paths this exercises are the ones no unit test reaches
- [[2026-09-06-bug-webpki-only-roots-rejected-zscaler-tls-inspection]] — see also
