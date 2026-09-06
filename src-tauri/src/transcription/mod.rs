//! Whisper, gated by a voice activity detector.
//!
//! A segment goes through [`vad::Gate`] first and reaches the model only if the
//! detector found speech in it; see that module for why. Whisper then decodes
//! with beam search — on large-v3-turbo the decoder is four layers, so the
//! wider search costs little next to the encoder, and accuracy is the priority.
//! Every whisper sub-segment of one audio segment is joined into a single
//! transcript line: beam search likes to break mid-sentence, and one line per
//! segment reads as the one utterance it usually is.

pub mod loader;
pub mod pool;
pub mod vad;

use crate::audio::resample::TARGET_RATE;
use crate::error::{AppError, Result};
use std::ffi::c_int;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Inputs shorter than this are dropped unheard. The stop-time flush can hand
/// over a few hundred milliseconds of tail, which is not enough to say anything
/// in and exactly enough for whisper to invent something.
const MIN_INPUT_SAMPLES: usize = TARGET_RATE as usize / 2;

/// Beam width. whisper.cpp's own default for its CLI.
const BEAM_SIZE: c_int = 5;

pub struct Transcriber {
    ctx: WhisperContext,
    /// Silero runs on the CPU in ~20 ms for 8 s of audio, so the lock is
    /// uncontended in practice even with several workers.
    vad: Mutex<vad::Gate>,
}

impl Transcriber {
    /// Load the whisper model at `model_path` and the bundled VAD model.
    ///
    /// Writes the VAD model into the model directory if it is not already
    /// there byte-for-byte; `vad::ensure_model` serialises that write, so this
    /// is safe from the loader's single flight and from tests alike.
    pub fn new(model_path: &str) -> Result<Self> {
        let vad_path = vad::ensure_model(&crate::model::model_dir())?;
        let gate = vad::Gate::new(&vad_path)?;
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(model_path, params)
            .map_err(|e| AppError::Transcription(e.to_string()))?;
        Ok(Self {
            ctx,
            vad: Mutex::new(gate),
        })
    }

    /// One line of text for `samples`, or `None` when there is nothing to say:
    /// the input is too short, the detector heard no speech, or whisper produced
    /// only whitespace.
    pub fn transcribe(&self, samples: &[f32]) -> Result<Option<String>> {
        if too_short(samples.len()) {
            log::debug!(
                "[whisper] {} samples is too short to transcribe",
                samples.len()
            );
            return Ok(None);
        }

        let speech = self
            .vad
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .speech(samples)?;
        let Some(speech) = speech else {
            log::debug!("[whisper] no speech in {} samples", samples.len());
            return Ok(None);
        };
        log::debug!(
            "[whisper] {} of {} samples are speech",
            speech.len(),
            samples.len()
        );

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| AppError::Transcription(e.to_string()))?;

        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: BEAM_SIZE,
            patience: -1.0,
        });
        params.set_language(Some("en"));
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, &speech)
            .map_err(|e| AppError::Transcription(e.to_string()))?;

        let mut pieces = Vec::new();
        for segment in state.as_iter() {
            pieces.push(
                segment
                    .to_str_lossy()
                    .map_err(|e| AppError::Transcription(e.to_string()))?
                    .into_owned(),
            );
        }
        let line = join_pieces(pieces.iter().map(String::as_str));
        log::debug!("[whisper] {line:?}");
        Ok(line)
    }
}

/// Whether `len` samples is too little audio to hold a word.
const fn too_short(len: usize) -> bool {
    len < MIN_INPUT_SAMPLES
}

/// Whisper's sub-segments as one line: trimmed, blanks dropped, joined with
/// single spaces; `None` when nothing is left.
fn join_pieces<'a>(pieces: impl Iterator<Item = &'a str>) -> Option<String> {
    let line = pieces
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_segments_become_one_line() {
        assert_eq!(
            join_pieces([" the quarterly numbers", "came in above forecast. ", ""].into_iter()),
            Some("the quarterly numbers came in above forecast.".to_string())
        );
    }

    #[test]
    fn only_whitespace_is_no_line() {
        assert_eq!(join_pieces(["", "   ", "\n"].into_iter()), None);
        assert_eq!(join_pieces(std::iter::empty()), None);
    }

    #[test]
    fn half_a_second_is_the_floor() {
        assert!(too_short(0));
        assert!(too_short(7_999));
        assert!(!too_short(8_000), "0.5 s at 16 kHz is enough to try");
    }
}

/// Shared by the two ignored, model-dependent tests below.
#[cfg(test)]
pub mod fixture {
    use std::path::PathBuf;

    /// The downloaded whisper model, if this machine has one.
    pub fn model_path() -> Option<PathBuf> {
        let p = crate::model::default_model_path();
        p.exists().then_some(p)
    }

    /// The sentence the spoken fixture says, and the phrase a transcript of it
    /// must contain.
    pub const SPOKEN: &str = "The quarterly numbers came in above forecast, so we are moving \
                              the launch to the second week of October. Please send the revised \
                              budget to finance by Friday.";
    pub const EXPECTED_PHRASE: &str = "second week of october";

    /// About nine seconds of real speech at 16 kHz, synthesized with macOS
    /// `say` and converted with `afconvert` — both ship with the only platform
    /// darric builds for, so no audio file is committed. Synthesized once per
    /// test process and shared.
    pub fn spoken() -> Vec<f32> {
        static SAMPLES: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
        SAMPLES
            .get_or_init(|| {
                let (_dir, wav) = spoken_wav();
                wav_data_f32(&std::fs::read(&wav).expect("read wav"))
            })
            .clone()
    }

    /// The same speech as a 16 kHz float WAV on disk, for playing through a
    /// real output device. The directory is returned so the file outlives the
    /// call.
    pub fn spoken_wav() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let aiff = dir.path().join("speech.aiff");
        let wav = dir.path().join("speech.wav");
        run_bounded("say", |c| c.arg("-o").arg(&aiff).arg(SPOKEN));
        run_bounded("afconvert", |c| {
            c.args(["-f", "WAVE", "-d", "LEF32@16000", "-c", "1"])
                .arg(&aiff)
                .arg(&wav)
        });
        (dir, wav)
    }

    /// Run a macOS audio tool with stdin closed and a hard time limit.
    ///
    /// `say` has been seen to sit forever under the test harness — once with
    /// inherited stdin, once with it closed — while it returns in a second from
    /// a shell. A hung fixture must fail the test, not stall the run, so the
    /// child is polled and killed after [`TOOL_TIMEOUT`], and retried once.
    fn run_bounded(
        tool: &str,
        configure: impl Fn(&mut std::process::Command) -> &mut std::process::Command,
    ) {
        const TOOL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        for attempt in 1..=2 {
            let mut cmd = std::process::Command::new(tool);
            configure(&mut cmd);
            let mut child = cmd
                .stdin(std::process::Stdio::null())
                .spawn()
                .unwrap_or_else(|e| panic!("spawn `{tool}`: {e}"));
            let started = std::time::Instant::now();
            loop {
                match child.try_wait().expect("poll child") {
                    Some(status) => {
                        assert!(status.success(), "`{tool}` failed: {status}");
                        return;
                    }
                    None if started.elapsed() > TOOL_TIMEOUT => {
                        child.kill().ok();
                        child.wait().ok();
                        eprintln!("`{tool}` hung for {TOOL_TIMEOUT:?} (attempt {attempt}); killed");
                        break;
                    }
                    None => std::thread::sleep(std::time::Duration::from_millis(50)),
                }
            }
        }
        panic!("`{tool}` hung twice");
    }

    /// The `data` chunk of a 32-bit float WAV as samples.
    fn wav_data_f32(bytes: &[u8]) -> Vec<f32> {
        let mut i = 12;
        while i + 8 <= bytes.len() {
            let id = &bytes[i..i + 4];
            let size = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]);
            let size = usize::try_from(size).expect("chunk fits");
            let body = &bytes[i + 8..(i + 8 + size).min(bytes.len())];
            if id == b"data" {
                return body
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| f32::from_le_bytes(*c))
                    .collect();
            }
            i += 8 + size + (size & 1);
        }
        panic!("no data chunk in WAV");
    }

    /// The fixture generator alone, so a hang in `say` is diagnosable apart
    /// from the model.
    #[test]
    #[ignore = "runs macOS speech synthesis"]
    fn fixture_speaks() {
        let samples = spoken();
        println!("fixture: {} samples", samples.len());
        assert!(
            samples.len() > 16_000 * 5,
            "at least five seconds of speech"
        );
    }

    /// The inputs that used to be transcribed as "Thank you." or ".": digital
    /// silence, low noise at three levels, and two short tails. Shared by the
    /// VAD gate test (CI) and the model-level accuracy test (ignored).
    pub fn silent_cases() -> Vec<(&'static str, Vec<f32>)> {
        vec![
            ("digital zero, 8 s", vec![0.0; 128_000]),
            ("noise rms 0.0005, 8 s", noise(128_000, 0.0005)),
            ("noise rms 0.003, 8 s", noise(128_000, 0.003)),
            ("noise rms 0.01, 8 s", noise(128_000, 0.01)),
            ("noise rms 0.003, 0.4 s tail", noise(6_400, 0.003)),
            ("noise rms 0.003, 1.5 s tail", noise(24_000, 0.003)),
        ]
    }

    /// Pseudo-random noise at RMS `rms`, built without a single float cast:
    /// the generator's top sixteen bits convert to `f32` exactly through `u16`.
    pub fn noise(len: usize, rms: f32) -> Vec<f32> {
        let mut x: u32 = 0x9E37_79B9;
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                let top = u16::try_from(x >> 16).expect("16 bits fit");
                let unit = (f32::from(top) / f32::from(u16::MAX)).mul_add(2.0, -1.0);
                unit * rms * 1.732
            })
            .collect()
    }
}

/// The accuracy contract, against the real model. Ignored by default because
/// it needs the 1.6 GB whisper download; run before shipping with
///   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture accuracy
#[cfg(test)]
mod accuracy {
    use super::fixture::{model_path, noise, silent_cases, spoken, EXPECTED_PHRASE};
    use super::*;

    #[test]
    #[ignore = "requires the downloaded whisper model and macOS speech synthesis"]
    fn silence_yields_nothing_and_speech_yields_the_words() {
        let Some(path) = model_path() else {
            println!("no model present — skipping");
            return;
        };
        let t = Transcriber::new(path.to_str().expect("utf-8 path")).expect("load models");

        for (label, samples) in &silent_cases() {
            let line = t.transcribe(samples).expect("transcribe");
            println!("{label}: {line:?}");
            assert_eq!(line, None, "{label} must produce no line");
        }

        let speech = spoken();
        let mut padded = vec![0.0; 48_000];
        padded.extend_from_slice(&speech);
        padded.resize(padded.len() + 48_000, 0.0);
        let noisy: Vec<f32> = speech
            .iter()
            .zip(noise(speech.len(), 0.01))
            .map(|(a, b)| a + b)
            .collect();
        let spoken_cases: Vec<(&str, Vec<f32>)> = vec![
            ("speech", speech),
            ("speech padded with 3 s silence each side", padded),
            ("speech plus noise rms 0.01", noisy),
        ];
        for (label, samples) in &spoken_cases {
            let line = t.transcribe(samples).expect("transcribe");
            println!("{label}: {line:?}");
            let line = line.unwrap_or_default().to_lowercase();
            assert!(
                line.contains(EXPECTED_PHRASE),
                "{label} must contain {EXPECTED_PHRASE:?}, got {line:?}"
            );
        }
    }
}

/// The whole pipeline below the capture callback on real speech: the pause
/// segmenter, then the VAD gate, then whisper. Two spoken sentences separated by
/// silence must come out as two lines, each with its own words, stamped in
/// order.
#[cfg(test)]
mod pipeline {
    use super::fixture::{model_path, spoken, EXPECTED_PHRASE};
    use super::*;
    use crate::audio::segmenter::Segmenter;

    #[test]
    #[ignore = "requires the downloaded whisper model and macOS speech synthesis"]
    fn two_utterances_become_two_lines_in_order() {
        let Some(path) = model_path() else {
            println!("no model present — skipping");
            return;
        };
        let t = Transcriber::new(path.to_str().expect("utf-8 path")).expect("load models");
        let speech = spoken();

        // 2 s silence, the sentence, 1 s silence, the sentence again, 2 s silence.
        let mut stream = vec![0.0_f32; 32_000];
        stream.extend_from_slice(&speech);
        stream.extend(std::iter::repeat_n(0.0_f32, 16_000));
        stream.extend_from_slice(&speech);
        stream.extend(std::iter::repeat_n(0.0_f32, 32_000));

        let mut seg = Segmenter::new();
        let mut segments = Vec::new();
        for chunk in stream.chunks(160) {
            segments.extend(seg.push(chunk));
        }
        segments.extend(seg.flush());

        let mut lines = Vec::new();
        for s in &segments {
            let line = t.transcribe(&s.samples).expect("transcribe");
            println!(
                "{} ms at {} -> {line:?}",
                s.samples.len() * 1_000 / 16_000,
                s.captured_at.format("%H:%M:%S%.3f")
            );
            if let Some(text) = line {
                lines.push((s.captured_at, text));
            }
        }
        // The fixture has a natural pause between its two sentences, so each
        // spoken pass may come out as one line or two; either way the phrase
        // from its first sentence appears once per pass and no line is a
        // fragment.
        let with_phrase = lines
            .iter()
            .filter(|(_, text)| text.to_lowercase().contains(EXPECTED_PHRASE))
            .count();
        assert_eq!(with_phrase, 2, "the sentence was spoken twice: {lines:?}");
        assert!(lines
            .iter()
            .all(|(_, text)| text.split_whitespace().count() >= 4));
        assert!(
            lines.windows(2).all(|w| w[0].0 < w[1].0),
            "stamped in order"
        );
    }
}

#[cfg(test)]
mod bench {
    use super::fixture::{model_path, spoken};
    use super::*;
    use std::sync::Arc;
    use std::time::Instant;

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
        // Real speech, or the VAD gate skips the model and nothing is measured.
        let seg = spoken();

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
