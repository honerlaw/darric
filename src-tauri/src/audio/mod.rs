pub mod microphone;

use crate::{
    error::Result,
    state::{AudioHandle, DbConn},
    transcription::{speaker_tracker::SpeakerTracker, Transcriber},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use tauri::{AppHandle, Emitter};

// cpal::Stream has PhantomData<*mut ()> making it !Send.
// It is safe to move to another thread for the purpose of keeping it alive;
// the internal callbacks run on Core Audio's own threads.
#[allow(clippy::non_send_fields_in_send_ty)]
pub struct SendableStream(#[allow(dead_code)] cpal::Stream);
unsafe impl Send for SendableStream {}

// 8s balances responsiveness vs. Whisper accuracy (trained on 30s; < 5s causes hallucinations)
const SEGMENT_SECONDS: u64 = 8;
const SAMPLE_RATE: u32 = 16_000;
#[allow(clippy::cast_lossless, clippy::cast_possible_truncation)]
const SEGMENT_SAMPLES: usize = (SAMPLE_RATE as u64 * SEGMENT_SECONDS) as usize;

#[derive(Clone, Copy)]
pub enum AudioSource {
    Mic,
}

pub struct AudioChunk {
    pub source: AudioSource,
    pub samples: Vec<f32>,
}

pub fn start_capture(
    session_id: String,
    app: AppHandle,
    db: Arc<DbConn>,
    transcriber: Option<Arc<Transcriber>>,
    speaker_tracker: Arc<Mutex<SpeakerTracker>>,
) -> Result<AudioHandle> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::sync_channel::<AudioChunk>(256);

    let mic_stream = microphone::start_mic_capture(tx, shutdown.clone())?;

    if transcriber.is_none() {
        log::warn!("[audio] no transcriber — audio will be captured but not transcribed");
    }

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let shutdown_flag = shutdown.clone();

    tokio::spawn(async move {
        shutdown_rx.await.ok();
        shutdown_flag.store(true, Ordering::SeqCst);
    });

    let rt_handle = tokio::runtime::Handle::current();
    let session_id_drain = session_id.clone();
    let shutdown_drain = shutdown;

    log::info!(
        "[audio] drain thread starting (segment={SEGMENT_SECONDS}s, {SEGMENT_SAMPLES} samples)"
    );

    std::thread::spawn(move || {
        let _mic = mic_stream;
        let mut mic_buf: Vec<f32> = Vec::new();
        let mut chunks_received: u64 = 0;

        loop {
            if shutdown_drain.load(Ordering::Relaxed) {
                log::info!(
                    "[audio] shutdown — flushing {} buffered samples",
                    mic_buf.len()
                );
                flush_segment(
                    &mic_buf,
                    AudioSource::Mic,
                    &session_id_drain,
                    transcriber.as_ref(),
                    speaker_tracker.clone(),
                    &app,
                    &db,
                    &rt_handle,
                );
                log::info!("[audio] drain thread exiting ({chunks_received} chunks total)");
                break;
            }

            match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(chunk) => {
                    chunks_received += 1;
                    mic_buf.extend_from_slice(&chunk.samples);

                    #[allow(clippy::cast_precision_loss)]
                    if chunks_received % 50 == 1 {
                        log::debug!(
                            "[audio] buf={}/{} samples ({:.1}s/{:.1}s)",
                            mic_buf.len(),
                            SEGMENT_SAMPLES,
                            mic_buf.len() as f32 / SAMPLE_RATE as f32,
                            SEGMENT_SECONDS as f32
                        );
                    }

                    while mic_buf.len() >= SEGMENT_SAMPLES {
                        let segment: Vec<f32> = mic_buf.drain(..SEGMENT_SAMPLES).collect();
                        #[allow(clippy::cast_precision_loss)]
                        {
                            log::info!(
                                "[audio] segment ready ({} samples, {:.1}s) — sending to whisper",
                                segment.len(),
                                segment.len() as f32 / SAMPLE_RATE as f32
                            );
                        }
                        transcribe_and_emit(
                            segment,
                            chunk.source,
                            session_id_drain.clone(),
                            transcriber.clone(),
                            speaker_tracker.clone(),
                            app.clone(),
                            db.clone(),
                            &rt_handle,
                        );
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    log::warn!("[audio] channel disconnected");
                    break;
                }
            }
        }
    });

    Ok(AudioHandle {
        session_id,
        shutdown_tx: Some(shutdown_tx),
    })
}

fn flush_segment(
    samples: &[f32],
    source: AudioSource,
    session_id: &str,
    transcriber: Option<&Arc<Transcriber>>,
    speaker_tracker: Arc<Mutex<SpeakerTracker>>,
    app: &AppHandle,
    db: &Arc<DbConn>,
    rt: &tokio::runtime::Handle,
) {
    if samples.is_empty() {
        return;
    }
    transcribe_and_emit(
        samples.to_vec(),
        source,
        session_id.to_string(),
        transcriber.cloned(),
        speaker_tracker,
        app.clone(),
        db.clone(),
        rt,
    );
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn transcribe_and_emit(
    segment: Vec<f32>,
    source: AudioSource,
    session_id: String,
    transcriber: Option<Arc<Transcriber>>,
    speaker_tracker: Arc<Mutex<SpeakerTracker>>,
    app: AppHandle,
    db: Arc<DbConn>,
    rt: &tokio::runtime::Handle,
) {
    let Some(t) = transcriber else {
        log::debug!("[whisper] no transcriber available — skipping segment");
        return;
    };
    let src_label = match source {
        AudioSource::Mic => "mic",
    }
    .to_string();
    let n_samples = segment.len();

    rt.spawn(async move {
        log::info!(
            "[whisper] transcribing {} samples ({:.1}s)…",
            n_samples,
            n_samples as f32 / 16000.0
        );
        let start = std::time::Instant::now();

        match tokio::task::spawn_blocking(move || t.transcribe(&segment).map(|segs| (segs, segment))).await {
            Ok(Ok((whisper_segments, audio))) => {
                log::info!(
                    "[whisper] done in {:.2}s → {} segment(s)",
                    start.elapsed().as_secs_f32(),
                    whisper_segments.len()
                );

                for ws in whisper_segments {
                    if ws.text.is_empty() {
                        continue;
                    }

                    // Extract audio for this segment to compute speaker fingerprint
                    let start_sample = (ws.start_ms * SAMPLE_RATE as i64 / 1000) as usize;
                    let end_sample = ((ws.end_ms * SAMPLE_RATE as i64 / 1000) as usize)
                        .min(audio.len());
                    let seg_audio = if end_sample > start_sample {
                        &audio[start_sample..end_sample]
                    } else {
                        &audio[..]
                    };

                    let speaker_id = {
                        let mut tracker = speaker_tracker.lock().unwrap();
                        tracker.identify_or_register(seg_audio)
                    };
                    let speaker_label = format!("Speaker {}", speaker_id + 1);

                    log::info!(
                        "[whisper] {speaker_label}: {:?}",
                        ws.text
                    );

                    let now = chrono::Utc::now().to_rfc3339();
                    let id = uuid::Uuid::new_v4().to_string();
                    {
                        let conn = db.0.lock().unwrap();
                        conn.execute(
                            "INSERT INTO transcript_lines(id, session_id, source, content, recorded_at, speaker_label)
                             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                            rusqlite::params![id, session_id, src_label, ws.text, now, speaker_label],
                        )
                        .ok();
                    }
                    app.emit(
                        "transcript_chunk",
                        serde_json::json!({
                            "source": src_label,
                            "speaker_label": speaker_label,
                            "content": ws.text,
                            "recorded_at": now,
                        }),
                    )
                    .ok();
                }
            }
            Ok(Err(e)) => log::error!("[whisper] transcription error: {e}"),
            Err(e) => log::error!("[whisper] spawn_blocking error: {e}"),
        }
    });
}
