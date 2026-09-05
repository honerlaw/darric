use crate::error::{AppError, Result};
use futures_util::StreamExt;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

const MODEL_URL: &str =
    "https://huggingface.co/akashmjn/tinydiarize-whisper.cpp/resolve/main/ggml-small.en-tdrz.bin";
const MODEL_FILENAME: &str = "ggml-small.en-tdrz.bin";

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

    log::info!("[model] not found — downloading ggml-small.en-tdrz.bin (~466MB)");
    app.emit("model_download_start", ()).ok();

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
    #[allow(clippy::cast_precision_loss)]
    {
        log::info!("[model] {:.1} MB to download", total as f64 / 1_048_576.0);
    }

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
                #[allow(clippy::cast_precision_loss)]
                {
                    log::info!(
                        "[model] {}% ({:.1}/{:.1} MB)",
                        pct,
                        downloaded as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    );
                }
                app.emit("model_download_progress", pct).ok();
            }
        }
    }

    file.flush().await?;
    drop(file);
    tokio::fs::rename(&tmp_path, &path).await?;

    log::info!("[model] download complete → {}", path.display());
    app.emit("model_download_done", ()).ok();

    Ok(path)
}
