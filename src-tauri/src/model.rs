use crate::error::{AppError, Result};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

// large-v3-turbo replaces small.en-tdrz: tinydiarize existed only to emit
// [SPEAKER_TURN] tokens for speaker attribution, which is now derived from the
// originating device instead. Turbo is multilingual and markedly more accurate
// at comparable speed on Apple Silicon with Metal.
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin";
const MODEL_FILENAME: &str = "ggml-large-v3-turbo.bin";

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

pub fn default_model_path() -> PathBuf {
    let home = std::env::var("HOME").map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from);
    home.join("Library/Application Support/darric")
        .join(MODEL_FILENAME)
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
    let response = reqwest::get(MODEL_URL)
        .await
        .map_err(|e| AppError::Audio(format!("model download request failed: {e}")))?;

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

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Audio(format!("download error: {e}")))?;
        downloaded += chunk.len() as u64;
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
    tokio::fs::rename(&tmp, path).await?;

    Ok(())
}
