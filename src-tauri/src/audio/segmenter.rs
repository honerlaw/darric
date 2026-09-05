//! Accumulates a capture source's samples into fixed-length segments.
//!
//! Whisper is trained on 30-second windows and hallucinates badly on very short
//! input, so audio is batched rather than transcribed as it arrives. Eight
//! seconds is the existing balance between that and responsiveness.

use super::resample::TARGET_RATE;

/// Seconds of audio per transcription segment.
pub const SEGMENT_SECONDS: usize = 8;

/// Samples per segment at [`TARGET_RATE`].
pub const SEGMENT_SAMPLES: usize = SEGMENT_SECONDS * TARGET_RATE as usize;

/// Buffers samples and emits whole segments as they complete.
pub struct Segmenter {
    buf: Vec<f32>,
    segment_samples: usize,
}

impl Segmenter {
    pub fn new() -> Self {
        Self::with_segment_samples(SEGMENT_SAMPLES)
    }

    /// Test seam — a segment size other than the eight-second default.
    pub fn with_segment_samples(segment_samples: usize) -> Self {
        Self {
            buf: Vec::with_capacity(segment_samples),
            segment_samples: segment_samples.max(1),
        }
    }

    /// Append `chunk`, returning every segment it completed.
    ///
    /// A chunk larger than one segment yields several, which is why this returns
    /// a `Vec` rather than an `Option`.
    pub fn push(&mut self, chunk: &[f32]) -> Vec<Vec<f32>> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();
        while self.buf.len() >= self.segment_samples {
            out.push(self.buf.drain(..self.segment_samples).collect());
        }
        out
    }

    /// Take whatever is buffered, however short — used when a recording stops so
    /// the trailing partial segment is still transcribed.
    pub fn flush(&mut self) -> Option<Vec<f32>> {
        if self.buf.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.buf))
        }
    }

    /// Samples held back, waiting for the segment to fill.
    #[cfg(test)]
    pub const fn buffered(&self) -> usize {
        self.buf.len()
    }
}

impl Default for Segmenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_nothing_until_a_segment_is_full() {
        let mut s = Segmenter::with_segment_samples(4);
        assert!(s.push(&[0.0, 0.0, 0.0]).is_empty());
        assert_eq!(s.buffered(), 3);
    }

    #[test]
    fn emits_one_segment_at_the_boundary() {
        let mut s = Segmenter::with_segment_samples(4);
        let out = s.push(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(out, vec![vec![1.0, 2.0, 3.0, 4.0]]);
        assert_eq!(s.buffered(), 0);
    }

    #[test]
    fn a_large_chunk_yields_several_segments() {
        let mut s = Segmenter::with_segment_samples(2);
        let out = s.push(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(out, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(s.buffered(), 1, "the odd sample stays buffered");
    }

    #[test]
    fn flush_returns_a_partial_segment() {
        let mut s = Segmenter::with_segment_samples(4);
        s.push(&[1.0, 2.0]);
        assert_eq!(s.flush(), Some(vec![1.0, 2.0]));
        assert_eq!(s.buffered(), 0);
    }

    #[test]
    fn flush_of_an_empty_buffer_is_none() {
        let mut s = Segmenter::with_segment_samples(4);
        assert_eq!(s.flush(), None);
    }

    #[test]
    fn samples_are_not_reordered_across_pushes() {
        let mut s = Segmenter::with_segment_samples(4);
        s.push(&[1.0, 2.0]);
        let out = s.push(&[3.0, 4.0]);
        assert_eq!(out, vec![vec![1.0, 2.0, 3.0, 4.0]]);
    }

    #[test]
    fn default_segment_is_eight_seconds_at_16k() {
        assert_eq!(SEGMENT_SAMPLES, 128_000);
        assert_eq!(Segmenter::new().buffered(), 0);
    }
}
