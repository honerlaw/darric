use crate::error::{AppError, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::error::Error as StdError;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

// large-v3-turbo replaces small.en-tdrz: tinydiarize existed only to emit
// [SPEAKER_TURN] tokens for speaker attribution, which is now derived from the
// originating device instead. Turbo is multilingual and markedly more accurate
// at comparable speed on Apple Silicon with Metal.
//
// Served from this repository's `models` GitHub Release rather than from
// Hugging Face: some corporate networks block huggingface.co outright, while
// anything that can install the app already reaches GitHub's release assets.
// The asset is a byte-exact mirror of ggerganov/whisper.cpp's file (MIT), and
// `MODEL_SHA256` pins it, so a swapped or truncated download is refused.
const MODEL_URL: &str =
    "https://github.com/honerlaw/darric/releases/download/models/ggml-large-v3-turbo.bin";
const MODEL_FILENAME: &str = "ggml-large-v3-turbo.bin";
/// SHA-256 of the exact bytes `MODEL_URL` serves. Matches Hugging Face's LFS
/// object hash for the upstream file (`x-linked-etag`).
const MODEL_SHA256: &str = "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";

/// Sentinel for `DOWNLOAD_PCT` meaning "no download in flight".
const NOT_DOWNLOADING: i64 = -1;

/// Emit a progress event every this many percent. One event per percent is
/// nothing next to a multi-minute download, and a bar that only moves every 5%
/// of ~1.6 GB looks stalled for ~80 MB at a time.
const PROGRESS_STEP: u32 = 1;

/// Live download percentage, readable without awaiting anything. Events are the
/// update path for a frontend that is already listening; this is what a
/// frontend that mounted mid-download reads to catch up. See
/// `commands::model::model_download_state`.
static DOWNLOAD_PCT: AtomicI64 = AtomicI64::new(NOT_DOWNLOADING);

/// Serialises downloads across callers. Both `lib.rs`'s startup pre-load and
/// `sessions::load_transcriber` call `ensure_model`, and without this they each
/// stream the model into the same `.tmp` path at independent offsets, rename
/// the interleaved result into place, and cache it forever — `ensure_model`
/// accepts any existing file on an `exists()` check with no validation, so the
/// install is then permanently broken with no in-app recovery.
static DOWNLOAD_LOCK: Mutex<()> = Mutex::const_new(());

/// Where a download writes before it is complete. Derived in one place so the
/// write path and the failure cleanup cannot disagree about it.
fn tmp_path(path: &Path) -> PathBuf {
    path.with_extension("tmp")
}

/// Current download percentage, or `None` when no download is in flight.
pub fn download_progress() -> Option<u32> {
    u32::try_from(DOWNLOAD_PCT.load(Ordering::Relaxed)).ok()
}

/// The directory every model file lives in — the whisper model downloaded by
/// [`ensure_model`] and the VAD model `transcription::vad` writes out.
pub fn model_dir() -> PathBuf {
    let home = std::env::var("HOME").map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from);
    home.join("Library/Application Support/darric")
}

pub fn default_model_path() -> PathBuf {
    model_dir().join(MODEL_FILENAME)
}

pub async fn ensure_model(app: &AppHandle) -> Result<PathBuf> {
    let path = default_model_path();
    if path.exists() {
        log::info!("[model] cached at {}", path.display());
        return Ok(path);
    }

    let _guard = DOWNLOAD_LOCK.lock().await;

    // Re-checked after acquiring the lock: a concurrent caller may have
    // finished the whole download while this one waited, in which case there is
    // nothing left to do.
    if path.exists() {
        log::info!("[model] downloaded by a concurrent caller while waiting");
        return Ok(path);
    }

    log::info!("[model] not found — downloading {MODEL_FILENAME}");
    DOWNLOAD_PCT.store(0, Ordering::Relaxed);
    app.emit("model_download_start", ()).ok();

    let result = download(app, &path).await;
    DOWNLOAD_PCT.store(NOT_DOWNLOADING, Ordering::Relaxed);

    match result {
        Ok(()) => {
            log::info!("[model] download complete → {}", path.display());
            app.emit("model_download_done", ()).ok();
            Ok(path)
        }
        Err(e) => {
            // Two things have to happen here. The partial file is up to ~1.6 GB
            // that nothing else ever reclaims, and `model_download_start` has no
            // terminal counterpart on this path without the event — the UI would
            // leave its indicator pinned at the last percentage and the Record
            // button disabled for the rest of the session.
            tokio::fs::remove_file(tmp_path(&path)).await.ok();
            log::error!("[model] download failed: {e}");
            app.emit("model_download_error", e.to_string()).ok();
            Err(e)
        }
    }
}

async fn download(app: &AppHandle, path: &Path) -> Result<()> {
    let response = reqwest::get(MODEL_URL).await.map_err(|e| {
        AppError::Audio(format!(
            "model download request failed: {}",
            error_chain(&e)
        ))
    })?;

    if !response.status().is_success() {
        return Err(AppError::Audio(format!(
            "model download failed: HTTP {}",
            response.status()
        )));
    }

    let total = response.content_length().unwrap_or(0);
    log::info!("[model] {} MB to download", total / 1_048_576);

    let tmp = tmp_path(path);
    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_reported_pct = 0u32;
    let mut hasher = Sha256::new();

    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| AppError::Audio(format!("download error: {}", error_chain(&e))))?;
        downloaded += chunk.len() as u64;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;

        // `checked_div` yields `None` when `total` is 0 (server sent no
        // Content-Length), which is exactly the case the progress log skips.
        if let Some(ratio) = (downloaded * 100).checked_div(total) {
            // Clamped because a server that under-reports Content-Length would
            // otherwise push a percentage above 100 all the way into the
            // progress bar's width and its `aria-valuenow`.
            let pct = u32::try_from(ratio).unwrap_or(100).min(100);
            DOWNLOAD_PCT.store(i64::from(pct), Ordering::Relaxed);
            if pct >= last_reported_pct + PROGRESS_STEP {
                last_reported_pct = pct;
                log::info!(
                    "[model] {}% ({}/{} MB)",
                    pct,
                    downloaded / 1_048_576,
                    total / 1_048_576
                );
                app.emit("model_download_progress", pct).ok();
            }
        }
    }

    file.flush().await?;
    drop(file);

    // Both checks run before the rename so that a bad download never reaches
    // the cached path: `ensure_model` accepts any existing file there, so a
    // corrupt one would be trusted on every later launch with no way to recover
    // from inside the app. Returning early here lets `ensure_model`'s failure
    // path remove the `.tmp` and report the error.
    check_length(total, downloaded)?;
    verify_digest(MODEL_SHA256, &format!("{:x}", hasher.finalize()))?;
    log::info!("[model] checksum verified: sha256 {MODEL_SHA256}");

    tokio::fs::rename(&tmp, path).await?;

    Ok(())
}

/// An error's `Display` followed by every `source()` down the chain, joined
/// with `: `.
///
/// reqwest's `Display` stops at its own layer — "error sending request for url
/// (…)" — and keeps the cause (a TLS handshake rejected by an unknown issuer, a
/// refused connection, a DNS failure) in `source()`. That cause is the useful
/// part of the diagnosis, so the log and the error bar carry the full chain.
fn error_chain(error: &dyn StdError) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}

/// Rejects a download whose length disagrees with the server's Content-Length.
///
/// A stream can end short without a transport error, which the checksum would
/// also catch — but this message says what actually happened. `total == 0`
/// means the server sent no Content-Length, so there is nothing to compare.
fn check_length(total: u64, downloaded: u64) -> Result<()> {
    if total == 0 || downloaded == total {
        return Ok(());
    }
    let what = if downloaded < total {
        "truncated"
    } else {
        "longer than advertised"
    };
    Err(AppError::Audio(format!(
        "model download {what}: got {downloaded} of {total} bytes"
    )))
}

/// Accepts a download only when its lower-hex SHA-256 is exactly `expected_hex`.
fn verify_digest(expected_hex: &str, actual_hex: &str) -> Result<()> {
    if actual_hex == expected_hex {
        Ok(())
    } else {
        Err(AppError::Audio(format!(
            "model download failed checksum: expected sha256 {expected_hex}, got {actual_hex}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";

    fn audio_message(result: Result<()>) -> String {
        match result {
            Err(AppError::Audio(msg)) => msg,
            other => panic!("expected an AppError::Audio, got {other:?}"),
        }
    }

    #[test]
    fn verify_digest_accepts_a_matching_hash() {
        assert!(verify_digest(DIGEST, DIGEST).is_ok());
    }

    #[test]
    fn verify_digest_rejects_a_different_hash() {
        let msg = audio_message(verify_digest(DIGEST, &"0".repeat(64)));
        assert!(msg.contains("checksum") && msg.contains(DIGEST));
    }

    #[test]
    fn verify_digest_is_case_sensitive_because_the_digest_is_formatted_lowercase() {
        assert!(verify_digest(DIGEST, &DIGEST.to_uppercase()).is_err());
    }

    #[test]
    fn pinned_hash_is_a_lowercase_sha256_hex_string() {
        assert_eq!(MODEL_SHA256.len(), 64);
        assert!(MODEL_SHA256
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    }

    #[derive(Debug)]
    struct Nested {
        message: &'static str,
        cause: Option<Box<Self>>,
    }

    impl std::fmt::Display for Nested {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.message)
        }
    }

    impl StdError for Nested {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.cause
                .as_deref()
                .map(|cause| cause as &(dyn StdError + 'static))
        }
    }

    #[test]
    fn error_chain_joins_every_source_with_a_colon() {
        let leaf = Nested {
            message: "invalid peer certificate: UnknownIssuer",
            cause: None,
        };
        let mid = Nested {
            message: "client error (Connect)",
            cause: Some(Box::new(leaf)),
        };
        let top = Nested {
            message: "error sending request",
            cause: Some(Box::new(mid)),
        };
        assert_eq!(
            error_chain(&top),
            "error sending request: client error (Connect): invalid peer certificate: UnknownIssuer"
        );
    }

    #[test]
    fn error_chain_of_a_leaf_is_its_display() {
        let leaf = Nested {
            message: "refused",
            cause: None,
        };
        assert_eq!(error_chain(&leaf), "refused");
    }

    #[test]
    fn check_length_skips_when_the_server_sent_no_content_length() {
        assert!(check_length(0, 12_345).is_ok());
    }

    #[test]
    fn check_length_accepts_an_exact_match() {
        assert!(check_length(1_624_555_275, 1_624_555_275).is_ok());
    }

    #[test]
    fn check_length_names_a_short_stream_as_truncated() {
        let msg = audio_message(check_length(100, 60));
        assert!(msg.contains("truncated") && msg.contains("60 of 100"));
    }

    #[test]
    fn check_length_names_an_over_long_stream_distinctly() {
        let msg = audio_message(check_length(100, 140));
        assert!(msg.contains("longer than advertised") && msg.contains("140 of 100"));
    }
}
