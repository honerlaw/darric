use crate::error::{AppError, Result};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

// large-v3-turbo replaces small.en-tdrz: tinydiarize existed only to emit
// [SPEAKER_TURN] tokens for speaker attribution, which is now derived from the
// originating device instead. Turbo is multilingual and markedly more accurate
// at comparable speed on Apple Silicon with Metal.
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin";
const MODEL_FILENAME: &str = "ggml-large-v3-turbo.bin";

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

    log::info!("[model] not found — downloading {MODEL_FILENAME}");
    app.emit("model_download_start", ()).ok();

    match download(app, &path).await {
        Ok(()) => {
            log::info!("[model] download complete → {}", path.display());
            app.emit("model_download_done", ()).ok();
            Ok(path)
        }
        Err(e) => {
            // `model_download_start` has no terminal counterpart on the failure
            // path without this. The UI would leave its progress indicator
            // pinned at the last reported percentage and the Record button
            // disabled for the rest of the session, because nothing ever tells
            // it the download stopped.
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

    let tmp_path = path.with_extension("tmp");
    let mut file = tokio::fs::File::create(&tmp_path).await?;
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
            let pct = u32::try_from(ratio).unwrap_or(100);
            if pct >= last_reported_pct + 5 {
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
    tokio::fs::rename(&tmp_path, path).await?;

    Ok(())
}
