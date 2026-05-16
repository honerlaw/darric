pub mod speaker_tracker;

use crate::error::{AppError, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct TranscriptSegment {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

pub struct Transcriber {
    ctx: WhisperContext,
}

// WhisperContext uses a raw pointer internally but whisper-rs marks it Send.
// We wrap it in Transcriber so the type is explicit.
unsafe impl Send for Transcriber {}
unsafe impl Sync for Transcriber {}

impl Transcriber {
    pub fn new(model_path: &str) -> Result<Self> {
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(model_path, params)
            .map_err(|e| AppError::Transcription(e.to_string()))?;
        Ok(Self { ctx })
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn transcribe(&self, samples: &[f32]) -> Result<Vec<TranscriptSegment>> {
        let energy: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
        let rms = energy.sqrt();
        log::debug!("[whisper] input rms={:.5} ({} samples)", rms, samples.len());
        if rms < 0.0001 {
            log::info!("[whisper] very low energy (rms={rms:.5}) — likely silence");
        }

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| AppError::Transcription(e.to_string()))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_tdrz_enable(true);

        state
            .full(params, samples)
            .map_err(|e| AppError::Transcription(e.to_string()))?;

        let n = state
            .full_n_segments()
            .map_err(|e| AppError::Transcription(e.to_string()))?;
        log::debug!("[whisper] {n} segment(s) produced");

        let mut segments = Vec::new();
        for i in 0..n {
            let raw = state
                .full_get_segment_text(i)
                .map_err(|e| AppError::Transcription(e.to_string()))?;

            // Strip TDRZ speaker-turn token that whisper inserts in the raw text
            let text = raw.replace("[SPEAKER_TURN]", "").trim().to_string();
            if text.is_empty() {
                continue;
            }

            let t0 = state
                .full_get_segment_t0(i)
                .map_err(|e| AppError::Transcription(e.to_string()))?;
            let t1 = state
                .full_get_segment_t1(i)
                .map_err(|e| AppError::Transcription(e.to_string()))?;

            log::debug!("[whisper] segment {i} [{t0}–{t1} cs]: {text:?}");
            segments.push(TranscriptSegment {
                text,
                start_ms: t0 * 10,
                end_ms: t1 * 10,
            });
        }

        Ok(segments)
    }
}
