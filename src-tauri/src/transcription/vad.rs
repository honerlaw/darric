//! Voice activity detection in front of whisper.
//!
//! Whisper hallucinates on silence: eight seconds of digital zeros through
//! large-v3-turbo decode to "Thank you." with high token confidence, and
//! whisper.cpp's own `no_speech_thold` / `logprob_thold` / `entropy_thold` do
//! not suppress it (measured 2026-09-06). An output tap with nothing routed to
//! it therefore produced that line every segment, and the stop-time flush
//! produced it once more on every microphone.
//!
//! whisper.cpp ships a Silero VAD and applies it inside `whisper_full` — but
//! only there. `whisper_full_with_state`, which is what whisper-rs's
//! `WhisperState::full` calls, ignores `params.vad` entirely, so enabling the
//! integrated VAD through `FullParams` changes nothing. This module runs the
//! same detector itself and hands whisper only the speech.
//!
//! The model is bundled rather than downloaded: it is 885 KB, huggingface.co is
//! blocked on some networks the app runs on, and a file that small has no
//! business needing a network round trip.

use crate::audio::coreaudio::exact_u32_from_f64;
use crate::audio::resample::TARGET_RATE;
use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use whisper_rs::{WhisperVadContext, WhisperVadContextParams, WhisperVadParams};

/// Silero VAD v5.1.2 in ggml format, from `ggml-org/whisper-vad` (MIT).
const MODEL_BYTES: &[u8] = include_bytes!("../../models/ggml-silero-v5.1.2.bin");

/// File name the bundled model is written under in the model directory.
pub const MODEL_FILENAME: &str = "ggml-silero-v5.1.2.bin";

/// The detector reports timestamps in centiseconds.
const SAMPLES_PER_CENTISECOND: u64 = (TARGET_RATE / 100) as u64;

/// Silence inserted between two speech regions when they are joined, so the
/// decoder still hears a boundary. Matches what `whisper_full` does.
const GAP_SAMPLES: usize = TARGET_RATE as usize / 10;

/// Serialises [`ensure_model`] across threads. Production reaches it once,
/// through the loader's single flight, but the ignored tests build several
/// transcribers in parallel, and two writers sharing one `.tmp` path is the
/// shape of the bug the whisper download once had.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Write the bundled model into `dir` unless an identical file is already there.
///
/// Compared byte-for-byte rather than by presence or length: a bare `exists()`
/// check is how a corrupt whisper model once got cached forever
/// (`2026-09-05-bug-concurrent-model-downloads-share-one-tmp-file`), and 885 KB
/// is cheap to read. Written through a `.tmp` and renamed so a crash mid-write
/// leaves either the old file or the new one, never a partial.
pub fn ensure_model(dir: &Path) -> Result<PathBuf> {
    let path = dir.join(MODEL_FILENAME);
    let up_to_date = || std::fs::read(&path).is_ok_and(|bytes| bytes == MODEL_BYTES);
    if up_to_date() {
        return Ok(path);
    }
    let _guard = WRITE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // A caller that waited on the lock finds the winner's file.
    if up_to_date() {
        return Ok(path);
    }
    std::fs::create_dir_all(dir)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, MODEL_BYTES)?;
    std::fs::rename(&tmp, &path)?;
    log::info!("[vad] wrote bundled model to {}", path.display());
    Ok(path)
}

/// One loaded detector. Not thread-safe across concurrent calls — the
/// transcriber keeps it behind a mutex.
pub struct Gate {
    ctx: WhisperVadContext,
}

impl Gate {
    pub fn new(model_path: &Path) -> Result<Self> {
        let path = model_path.to_string_lossy();
        let ctx = WhisperVadContext::new(&path, WhisperVadContextParams::default())
            .map_err(|e| AppError::Transcription(format!("loading the VAD model: {e}")))?;
        Ok(Self { ctx })
    }

    /// The speech in `samples`, or `None` when the detector found none.
    ///
    /// Speech regions are concatenated with [`GAP_SAMPLES`] of silence between
    /// them, exactly as `whisper_full` builds its own buffer. The timing between
    /// regions is discarded — nothing downstream reads sub-segment timestamps.
    pub fn speech(&mut self, samples: &[f32]) -> Result<Option<Vec<f32>>> {
        let segments = self
            .ctx
            .segments_from_samples(WhisperVadParams::default(), samples)
            .map_err(|e| AppError::Transcription(format!("voice activity detection: {e}")))?;

        let mut out = Vec::new();
        for segment in segments {
            let start = sample_index(segment.start, samples.len());
            let end = sample_index(segment.end, samples.len());
            if end <= start {
                continue;
            }
            if !out.is_empty() {
                out.resize(out.len() + GAP_SAMPLES, 0.0);
            }
            out.extend_from_slice(&samples[start..end]);
        }
        Ok(if out.is_empty() { None } else { Some(out) })
    }
}

/// A detector timestamp in centiseconds as an index into a buffer of `len`.
///
/// The detector derives its timestamps from integer sample positions, so the
/// value is whole centiseconds carried in an `f32`; it is rounded and converted
/// exactly rather than cast, per the lint policy, and clamped to the buffer.
fn sample_index(centiseconds: f32, len: usize) -> usize {
    let cs = exact_u32_from_f64(f64::from(centiseconds).round());
    usize::try_from(u64::from(cs) * SAMPLES_PER_CENTISECOND).map_or(len, |i| i.min(len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_index_converts_centiseconds_exactly_and_clamps() {
        assert_eq!(sample_index(0.0, 128_000), 0);
        assert_eq!(sample_index(100.0, 128_000), 16_000);
        assert_eq!(sample_index(799.0, 128_000), 127_840);
        assert_eq!(sample_index(900.0, 128_000), 128_000, "past the end clamps");
        assert_eq!(sample_index(-5.0, 128_000), 0, "negative is not an index");
    }

    #[test]
    fn ensure_model_writes_once_and_repairs_a_damaged_copy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = ensure_model(dir.path()).expect("first write");
        assert_eq!(std::fs::read(&path).expect("read").len(), MODEL_BYTES.len());

        // A same-length file with different bytes is exactly what a length
        // check would wrongly accept.
        let mut damaged = MODEL_BYTES.to_vec();
        damaged[1000] ^= 0xFF;
        std::fs::write(&path, &damaged).expect("damage");
        ensure_model(dir.path()).expect("repair");
        assert_eq!(std::fs::read(&path).expect("read"), MODEL_BYTES);
        assert!(!path.with_extension("tmp").exists(), "no tmp left behind");
    }

    /// The bundled model needs no download, so this runs in CI: the exact
    /// inputs that used to become "Thank you." must now be nothing at all.
    /// The same list feeds the model-level accuracy test, so the two cannot
    /// drift apart.
    #[test]
    fn silence_and_noise_are_not_speech() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = ensure_model(dir.path()).expect("model");
        let mut gate = Gate::new(&path).expect("gate");

        for (label, samples) in &crate::transcription::fixture::silent_cases() {
            assert!(
                gate.speech(samples).expect("vad").is_none(),
                "{label} must contain no speech"
            );
        }
    }
}
