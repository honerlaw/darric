pub mod pool;

use crate::error::{AppError, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct TranscriptSegment {
    pub text: String,
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

    pub fn transcribe(&self, samples: &[f32]) -> Result<Vec<TranscriptSegment>> {
        let rms = crate::audio::resample::rms(samples);
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

            let text = raw.trim().to_string();
            if text.is_empty() {
                continue;
            }

            log::debug!("[whisper] segment {i}: {text:?}");
            segments.push(TranscriptSegment { text });
        }

        Ok(segments)
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use std::sync::Arc;
    use std::time::Instant;

    fn model_path() -> Option<std::path::PathBuf> {
        let home = std::env::var("HOME").ok()?;
        for name in [
            "ggml-large-v3-turbo.bin",
            "ggml-small.en-tdrz.bin",
            "ggml-base.en.bin",
        ] {
            let p = std::path::PathBuf::from(&home)
                .join("Library/Application Support/darric")
                .join(name);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    /// Eight seconds of a quiet tone — enough work to be representative without
    /// depending on any audio fixture.
    fn segment() -> Vec<f32> {
        let mut phase = 0.0_f32;
        (0..128_000)
            .map(|_| {
                phase += 0.02;
                phase.sin() * 0.05
            })
            .collect()
    }

    /// Answers the proposal's open question: do concurrent `state.full()` calls
    /// against one `WhisperContext` actually parallelise on a single Metal GPU?
    ///
    /// Ignored by default — it needs a downloaded model and takes tens of
    /// seconds. Run with:
    ///   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture pool_sizing
    #[test]
    #[ignore = "requires a downloaded whisper model; measures GPU behaviour"]
    fn pool_sizing_measurement() {
        const N: usize = 4;

        let Some(path) = model_path() else {
            println!("no model present — skipping");
            return;
        };
        println!("model: {}", path.display());
        let t = Arc::new(
            Transcriber::new(path.to_str().expect("model path is valid UTF-8"))
                .expect("load whisper model"),
        );
        let seg = segment();

        // Warm up so model load and first-run Metal setup are not counted.
        t.transcribe(&seg).expect("warm-up transcription");

        let start = Instant::now();
        for _ in 0..N {
            t.transcribe(&seg).expect("serial transcription");
        }
        let serial = start.elapsed();

        let start = Instant::now();
        let handles: Vec<_> = (0..N)
            .map(|_| {
                let t = Arc::clone(&t);
                let seg = seg.clone();
                std::thread::spawn(move || t.transcribe(&seg).expect("parallel transcription"))
            })
            .collect();
        for h in handles {
            h.join().expect("worker thread");
        }
        let parallel = start.elapsed();

        let speedup = serial.as_secs_f64() / parallel.as_secs_f64();
        println!("serial   {N} segments: {serial:?}");
        println!("parallel {N} segments: {parallel:?}");
        println!("speedup: {speedup:.2}x  (1.0 == fully serialised)");
    }
}
