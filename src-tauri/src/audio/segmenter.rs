//! Cuts a capture source's samples into segments at pauses in speech.
//!
//! The previous segmenter emitted a fixed eight seconds regardless of what was
//! being said, so utterances were split mid-phrase ("Even with the big ox, I
//! won't let me get" / "all the way back.") and each half was decoded with no
//! context. Whisper is trained on 30-second windows and does best on whole
//! utterances, so segments now end where the speaker paused.
//!
//! The pause detector is deliberately cheap: it runs on the audio callback
//! thread, so it is a per-frame RMS against an adaptive noise floor and nothing
//! more. It only decides *where to cut*. Whether a segment contains speech at
//! all is the Silero gate's call, in `transcription::vad`, so a noisy room
//! degrades to segments cut at the length cap, never to silence transcribed.
//!
//! Every segment carries the wall-clock time its first sample arrived, which
//! becomes the transcript line's `recorded_at`.

use super::resample::{rms, TARGET_RATE};
use chrono::{DateTime, Duration as ChronoDuration, Utc};

/// The detector's frame: 20 ms.
pub const FRAME_SAMPLES: usize = TARGET_RATE as usize / 50;

/// A segment is not cut at a pause before it holds this much audio, so a run
/// of short remarks is not decoded one word at a time.
const MIN_SEGMENT_SAMPLES: usize = 2 * TARGET_RATE as usize;

/// A segment is cut here regardless, comfortably inside whisper's window.
const MAX_SEGMENT_SAMPLES: usize = 25 * TARGET_RATE as usize;

/// Trailing non-speech frames that count as a pause: 400 ms.
const PAUSE_FRAMES: usize = 20;

/// A frame is speech when its RMS exceeds this multiple of the noise floor …
const SPEECH_RATIO: f32 = 4.0;
/// … and this absolute level, whichever is higher.
const SPEECH_MIN: f32 = 0.004;

/// Where the noise floor starts, and the lowest it can drop to. A floor of
/// exactly zero could never rise again by a percentage.
const INITIAL_FLOOR: f32 = 0.001;
const FLOOR_MIN: f32 = 0.000_1;

/// How fast the floor may rise per non-speech frame that is louder than it —
/// a room that gets noisier is tracked within a few seconds.
const FLOOR_RISE: f32 = 1.02;
/// How fast it may rise per *speech* frame: slow enough that a long utterance
/// barely moves it, fast enough that steady noise mistaken for speech is
/// reclassified within about half a minute.
const FLOOR_CREEP: f32 = 1.002;

/// A chunk arriving later than the buffered audio accounts for, by more than
/// this, is a delivery gap — a stream rebuilt after a failure, callbacks
/// dropped under load — and the clock is re-anchored to it rather than left
/// extrapolating from the first callback of the session.
const GAP_TOLERANCE: ChronoDuration = ChronoDuration::milliseconds(100);

/// One segment: its 16 kHz samples and when its first sample arrived.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub samples: Vec<f32>,
    pub captured_at: DateTime<Utc>,
}

/// Buffers samples and emits a segment at each pause in speech.
pub struct Segmenter {
    buf: Vec<f32>,
    /// Samples of `buf` already classified into frames.
    classified: usize,
    /// Consecutive non-speech frames at the end of the classified part.
    trailing_silence: usize,
    /// Whether any frame in `buf` was speech.
    has_speech: bool,
    floor: f32,
    /// When `buf[0]` arrived; `None` while the buffer is empty.
    started_at: Option<DateTime<Utc>>,
    min_samples: usize,
    max_samples: usize,
    pause_frames: usize,
}

impl Segmenter {
    pub fn new() -> Self {
        Self::with_limits(MIN_SEGMENT_SAMPLES, MAX_SEGMENT_SAMPLES, PAUSE_FRAMES)
    }

    /// Test seam — limits other than the production ones.
    pub fn with_limits(min_samples: usize, max_samples: usize, pause_frames: usize) -> Self {
        Self {
            buf: Vec::with_capacity(max_samples),
            classified: 0,
            trailing_silence: 0,
            has_speech: false,
            floor: INITIAL_FLOOR,
            started_at: None,
            min_samples: min_samples.max(FRAME_SAMPLES),
            max_samples: max_samples.max(FRAME_SAMPLES),
            pause_frames: pause_frames.max(1),
        }
    }

    /// Append `chunk`, which arrived just now, returning every segment it completed.
    pub fn push(&mut self, chunk: &[f32]) -> Vec<Segment> {
        self.push_at(chunk, Utc::now())
    }

    /// Append `chunk`, whose last sample arrived at `now`.
    ///
    /// A chunk larger than one segment yields several, which is why this returns
    /// a `Vec` rather than an `Option`.
    pub fn push_at(&mut self, chunk: &[f32], now: DateTime<Utc>) -> Vec<Segment> {
        if chunk.is_empty() {
            return Vec::new();
        }
        self.anchor(chunk.len(), now);
        self.buf.extend_from_slice(chunk);
        self.classify_new_frames(true);

        let mut out = Vec::new();
        loop {
            if self.buf.len() >= self.max_samples {
                out.push(self.take(self.max_samples));
                continue;
            }
            if !self.has_speech {
                self.drop_leading_silence();
                break;
            }
            if self.buf.len() >= self.min_samples && self.trailing_silence >= self.pause_frames {
                // Cut so that exactly one pause stays behind: the next segment
                // opens on the silence that ended this one rather than on a
                // clipped syllable. Silence that accumulated while the buffer
                // was still short of the minimum goes with this segment.
                // `has_speech` guarantees a speech frame precedes the trailing
                // pause, so the cut is never zero.
                let cut = self.classified - self.pause_frames * FRAME_SAMPLES;
                out.push(self.take(cut));
                continue;
            }
            break;
        }
        out
    }

    /// Take whatever is buffered, however short — used when a recording stops so
    /// the trailing partial segment is still transcribed.
    pub fn flush(&mut self) -> Option<Segment> {
        if self.buf.is_empty() {
            return None;
        }
        let len = self.buf.len();
        Some(self.take(len))
    }

    /// Samples held back, waiting for a pause or the cap.
    #[cfg(test)]
    pub const fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// Set or re-anchor the buffer's start time for a chunk of `len` samples
    /// whose last sample arrived at `now`.
    fn anchor(&mut self, len: usize, now: DateTime<Utc>) {
        let chunk_start = now - duration_of(len);
        match self.started_at {
            None => self.started_at = Some(chunk_start),
            Some(started) => {
                let expected = started + duration_of(self.buf.len());
                if chunk_start - expected > GAP_TOLERANCE {
                    // Everything buffered is older than the gap; keep its
                    // duration and slide it up to end where this chunk begins.
                    self.started_at = Some(chunk_start - duration_of(self.buf.len()));
                }
            }
        }
    }

    /// Classify every whole frame not yet looked at. The floor is updated only
    /// on the first pass over a frame; re-deriving state after a cut passes
    /// `false` so the retained pause does not move it twice.
    fn classify_new_frames(&mut self, update_floor: bool) {
        while self.classified + FRAME_SAMPLES <= self.buf.len() {
            let frame = &self.buf[self.classified..self.classified + FRAME_SAMPLES];
            let level = rms(frame);
            let threshold = (self.floor * SPEECH_RATIO).max(SPEECH_MIN);
            if level > threshold {
                self.has_speech = true;
                self.trailing_silence = 0;
                if update_floor {
                    self.floor = level.min(self.floor * FLOOR_CREEP);
                }
            } else {
                self.trailing_silence += 1;
                if update_floor {
                    self.floor = if level < self.floor {
                        level.max(FLOOR_MIN)
                    } else {
                        level.min(self.floor * FLOOR_RISE)
                    };
                }
            }
            self.classified += FRAME_SAMPLES;
        }
    }

    /// Remove the first `n` samples as a segment and re-derive the state of
    /// what remains (at most a pause plus a partial chunk, so re-classifying it
    /// is cheap).
    fn take(&mut self, n: usize) -> Segment {
        let n = n.min(self.buf.len());
        let samples: Vec<f32> = self.buf.drain(..n).collect();
        let captured_at = self.started_at.unwrap_or_else(Utc::now);
        self.started_at = if self.buf.is_empty() {
            None
        } else {
            Some(captured_at + duration_of(n))
        };
        self.classified = 0;
        self.trailing_silence = 0;
        self.has_speech = false;
        self.classify_new_frames(false);
        Segment {
            samples,
            captured_at,
        }
    }

    /// With no speech heard yet, keep only the last pause's worth of audio so a
    /// long silence neither fills the buffer nor delays the cut once someone
    /// speaks. A segment that begins after silence therefore starts with at
    /// most one pause of it.
    fn drop_leading_silence(&mut self) {
        let keep = self.pause_frames * FRAME_SAMPLES;
        if self.buf.len() <= keep {
            return;
        }
        // Whole frames only, so the frame grid stays aligned with the buffer.
        let drop = (self.buf.len() - keep) / FRAME_SAMPLES * FRAME_SAMPLES;
        if drop == 0 {
            return;
        }
        self.buf.drain(..drop);
        self.classified = self.classified.saturating_sub(drop);
        if let Some(t) = self.started_at {
            self.started_at = Some(t + duration_of(drop));
        }
    }
}

impl Default for Segmenter {
    fn default() -> Self {
        Self::new()
    }
}

/// How long `samples` samples last at [`TARGET_RATE`] — 62.5 µs each.
fn duration_of(samples: usize) -> ChronoDuration {
    let n = i64::try_from(samples).unwrap_or(i64::MAX);
    ChronoDuration::nanoseconds(n.saturating_mul(62_500))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// Speech-level signal: a ±0.1 square wave, RMS 0.1.
    fn loud(samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|i| if i % 2 == 0 { 0.1 } else { -0.1 })
            .collect()
    }

    fn quiet(samples: usize) -> Vec<f32> {
        vec![0.0; samples]
    }

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 6, 15, 0, 0).unwrap()
    }

    /// Feed `stream` in 10 ms chunks with a clock that advances with the audio.
    fn feed(seg: &mut Segmenter, stream: &[f32]) -> Vec<Segment> {
        feed_from(seg, stream, t0())
    }

    /// The same, starting the clock at `start` — for a second feed that must
    /// continue the first one's timeline rather than restart it.
    fn feed_from(seg: &mut Segmenter, stream: &[f32], start: DateTime<Utc>) -> Vec<Segment> {
        let mut out = Vec::new();
        let mut now = start;
        for chunk in stream.chunks(160) {
            now += duration_of(chunk.len());
            out.extend(seg.push_at(chunk, now));
        }
        out
    }

    fn seconds(n: usize) -> usize {
        n * TARGET_RATE as usize
    }

    #[test]
    fn a_pause_ends_a_segment() {
        // 2 s speech, 0.6 s silence, 2 s speech, 0.6 s silence → two segments.
        let mut stream = loud(seconds(2));
        stream.extend(quiet(9_600));
        stream.extend(loud(seconds(2)));
        stream.extend(quiet(9_600));

        let mut seg = Segmenter::new();
        let out = feed(&mut seg, &stream);

        assert_eq!(out.len(), 2, "one segment per utterance");
        // The first ends where its pause began; the second opens on the kept
        // 400 ms of that pause.
        assert_eq!(out[0].samples.len(), seconds(2));
        assert_eq!(out[1].samples.len(), 6_400 + seconds(2));
        assert_eq!(out[0].captured_at, t0());
        // 2 s speech + 0.6 s silence − the 0.4 s handed to the next segment.
        assert_eq!(out[1].captured_at, t0() + duration_of(seconds(2) + 3_200));
        assert!(seg.buffered() <= 6_400, "only the trailing pause remains");
    }

    #[test]
    fn continuous_speech_is_cut_at_the_cap() {
        let mut seg = Segmenter::new();
        let out = feed(&mut seg, &loud(seconds(30)));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].samples.len(), seconds(25));
        assert_eq!(out[0].captured_at, t0());
        assert_eq!(seg.buffered(), seconds(5));
        assert_eq!(
            seg.flush().map(|s| s.captured_at),
            Some(t0() + duration_of(seconds(25)))
        );
    }

    #[test]
    fn a_short_remark_waits_for_the_minimum_then_leaves_on_the_pause() {
        // 0.5 s of speech and then silence: no cut at 0.9 s (too short), a cut
        // once two seconds are buffered and the tail is a pause.
        let mut stream = loud(8_000);
        stream.extend(quiet(seconds(3)));
        let mut seg = Segmenter::new();
        let out = feed(&mut seg, &stream);
        assert_eq!(out.len(), 1);
        assert!(out[0].samples.len() >= seconds(2) - 6_400);
        assert!(out[0].samples.len() < seconds(3));
        assert_eq!(out[0].captured_at, t0());
    }

    #[test]
    fn silence_alone_never_accumulates() {
        let mut seg = Segmenter::new();
        let out = feed(&mut seg, &quiet(seconds(60)));
        assert!(out.is_empty(), "nothing to say, nothing emitted");
        assert!(seg.buffered() <= 6_400, "only one pause of silence is kept");
        // Speech after a long silence starts its segment at most a pause late.
        let started = feed(&mut seg, &loud(seconds(3)));
        assert!(started.is_empty());
        let tail = seg.flush().expect("buffered speech");
        assert!(tail.samples.len() <= seconds(3) + 6_400);
    }

    #[test]
    fn speech_after_a_long_silence_is_stamped_when_it_was_heard() {
        // Sixty seconds of nothing, then three seconds of speech: the segment
        // starts at most one pause before the speech, not at the session start.
        let mut seg = Segmenter::new();
        assert!(feed(&mut seg, &quiet(seconds(60))).is_empty());
        let after_silence = t0() + duration_of(seconds(60));
        assert!(feed_from(&mut seg, &loud(seconds(3)), after_silence).is_empty());
        let tail = seg.flush().expect("buffered speech");
        assert!(tail.captured_at >= after_silence - duration_of(6_400));
        assert!(tail.captured_at <= after_silence);
    }

    #[test]
    fn a_delivery_gap_re_anchors_the_clock() {
        // Speech, a pause (cut), then nothing arrives for thirty seconds — a
        // stream being rebuilt — then speech again. The second segment must be
        // stamped when its audio arrived, not extrapolated from the first.
        let mut stream = loud(seconds(2));
        stream.extend(quiet(9_600));
        let mut seg = Segmenter::new();
        let first = feed(&mut seg, &stream);
        assert_eq!(first.len(), 1);
        let resumed = t0() + duration_of(stream.len()) + ChronoDuration::seconds(30);
        let mut more = loud(seconds(3));
        more.extend(quiet(9_600));
        let second = feed_from(&mut seg, &more, resumed);
        assert_eq!(second.len(), 1);
        // The retained 400 ms pause slides up to end where the new audio began.
        assert_eq!(second[0].captured_at, resumed - duration_of(6_400));
    }

    #[test]
    fn a_loud_utterance_does_not_drag_the_floor_up() {
        let mut seg = Segmenter::new();
        feed(&mut seg, &loud(seconds(10)));
        // Ten seconds of speech at RMS 0.1 raised the floor by creep only.
        assert!(seg.floor < 0.003, "floor {}", seg.floor);
        // A single silent frame brings it straight back down.
        feed(&mut seg, &quiet(FRAME_SAMPLES));
        assert!(seg.floor <= FLOOR_MIN);
    }

    #[test]
    fn a_room_that_got_noisier_is_reclassified_as_not_speech() {
        // Steady noise at RMS 0.02 reads as speech at first (0.02 > 4 × 0.001)
        // and as background within a minute.
        let noise: Vec<f32> = (0..seconds(60))
            .map(|i| if i % 2 == 0 { 0.02 } else { -0.02 })
            .collect();
        let mut seg = Segmenter::new();
        feed(&mut seg, &noise);
        assert!(seg.floor > 0.005, "floor {}", seg.floor);
        assert!(
            seg.trailing_silence > 0,
            "steady noise now counts as a pause"
        );
    }

    #[test]
    fn flush_returns_a_partial_segment_with_its_start_time() {
        let mut seg = Segmenter::new();
        assert!(seg.push_at(&loud(160), t0() + duration_of(160)).is_empty());
        let tail = seg.flush().expect("partial");
        assert_eq!(tail.samples.len(), 160);
        assert_eq!(tail.captured_at, t0());
        assert_eq!(seg.flush(), None);
    }

    #[test]
    fn samples_are_not_reordered_across_pushes() {
        // Cap of two frames: the second push completes the segment, in order.
        let mut seg = Segmenter::with_limits(2 * FRAME_SAMPLES, 2 * FRAME_SAMPLES, 1);
        let first = loud(FRAME_SAMPLES);
        let second: Vec<f32> = loud(FRAME_SAMPLES).iter().map(|s| s * 0.5).collect();
        assert!(seg.push_at(&first, t0()).is_empty());
        let out = seg.push_at(&second, t0());
        assert_eq!(out.len(), 1);
        let mut expected = first;
        expected.extend(&second);
        assert_eq!(out[0].samples, expected);
    }

    #[test]
    fn a_large_chunk_yields_several_segments() {
        // Cap of one frame, two and a half frames pushed at once.
        let mut seg = Segmenter::with_limits(FRAME_SAMPLES, FRAME_SAMPLES, 1);
        let out = seg.push_at(&loud(FRAME_SAMPLES * 5 / 2), t0());
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|s| s.samples.len() == FRAME_SAMPLES));
        assert_eq!(
            seg.buffered(),
            FRAME_SAMPLES / 2,
            "the half frame stays buffered"
        );
        // The push's clock is the chunk's end, so its first sample is 50 ms
        // before `t0` and the second segment starts one frame after that.
        assert_eq!(
            out[1].captured_at,
            t0() - duration_of(FRAME_SAMPLES * 5 / 2) + duration_of(FRAME_SAMPLES)
        );
    }

    #[test]
    fn frame_is_twenty_milliseconds() {
        assert_eq!(FRAME_SAMPLES, 320);
        assert_eq!(PAUSE_FRAMES * FRAME_SAMPLES, 6_400);
    }
}
