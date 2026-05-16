use rustfft::num_complex::Complex;
use std::f32::consts::PI;

const FRAME_LEN: usize = 400; // 25 ms at 16 kHz
const HOP_LEN: usize = 160;   // 10 ms at 16 kHz
const N_MELS: usize = 26;
const N_MFCC: usize = 13;
const SAMPLE_RATE: f32 = 16_000.0;

pub struct SpeakerTracker {
    known: Vec<(u32, Vec<f32>)>,
    next_id: u32,
    threshold: f32,
    hamming: Vec<f32>,
    filterbank: Vec<Vec<f32>>,
}

unsafe impl Send for SpeakerTracker {}
unsafe impl Sync for SpeakerTracker {}

impl SpeakerTracker {
    pub fn new() -> Self {
        let hamming = build_hamming(FRAME_LEN);
        let filterbank = build_mel_filterbank(N_MELS, FRAME_LEN, SAMPLE_RATE);
        Self {
            known: Vec::new(),
            next_id: 0,
            threshold: 0.82,
            hamming,
            filterbank,
        }
    }

    pub fn reset(&mut self) {
        self.known.clear();
        self.next_id = 0;
    }

    /// Returns the 1-based speaker number ("Speaker 1", "Speaker 2", …).
    pub fn identify_or_register(&mut self, samples: &[f32]) -> u32 {
        let fp = compute_fingerprint(samples, &self.hamming, &self.filterbank);

        if fp.iter().all(|&x| x == 0.0) {
            return self.next_id.saturating_sub(1);
        }

        let best = self
            .known
            .iter()
            .enumerate()
            .map(|(i, (id, stored))| (*id, i, cosine_similarity(&fp, stored)))
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((id, idx, sim)) = best {
            if sim >= self.threshold {
                // Exponential moving average update so the profile drifts with the speaker
                let stored = &mut self.known[idx].1;
                for (s, n) in stored.iter_mut().zip(fp.iter()) {
                    *s = 0.9_f32.mul_add(*s, 0.1 * n);
                }
                return id;
            }
        }

        let new_id = self.next_id;
        self.next_id += 1;
        self.known.push((new_id, fp));
        new_id
    }
}

#[allow(clippy::cast_precision_loss)]
fn compute_fingerprint(samples: &[f32], hamming: &[f32], filterbank: &[Vec<f32>]) -> Vec<f32> {
    if samples.len() < FRAME_LEN {
        return vec![0.0; N_MFCC];
    }

    let mut planner = rustfft::FftPlanner::new();
    let fft = planner.plan_fft_forward(FRAME_LEN);

    let n_fft_bins = FRAME_LEN / 2 + 1;
    let n_frames = (samples.len() - FRAME_LEN) / HOP_LEN + 1;
    let mut mel_sum = vec![0.0f32; N_MELS];
    let mut buf = vec![Complex::new(0.0f32, 0.0f32); FRAME_LEN];

    for frame_idx in 0..n_frames {
        let start = frame_idx * HOP_LEN;
        for (j, b) in buf.iter_mut().enumerate() {
            b.re = samples[start + j] * hamming[j];
            b.im = 0.0;
        }
        fft.process(&mut buf);

        for (m, filter) in filterbank.iter().enumerate() {
            let energy: f32 = buf[..n_fft_bins]
                .iter()
                .zip(filter.iter())
                .map(|(c, w)| c.norm_sqr() * w)
                .sum();
            mel_sum[m] += energy.max(1e-10_f32).ln();
        }
    }

    let n_f = n_frames as f32;
    for v in &mut mel_sum {
        *v /= n_f;
    }

    // DCT-II
    let mut mfcc = vec![0.0f32; N_MFCC];
    for (k, coeff) in mfcc.iter_mut().enumerate() {
        for (n, &m) in mel_sum.iter().enumerate() {
            *coeff += m * (PI * k as f32 * (2 * n + 1) as f32 / (2 * N_MELS) as f32).cos();
        }
    }

    mfcc
}

fn build_hamming(len: usize) -> Vec<f32> {
    let n = len - 1;
    (0..len)
        .map(|i| {
            0.46_f32.mul_add(-(2.0 * PI * i as f32 / n as f32).cos(), 0.54)
        })
        .collect()
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn build_mel_filterbank(n_mels: usize, n_fft: usize, sample_rate: f32) -> Vec<Vec<f32>> {
    let n_fft_bins = n_fft / 2 + 1;
    let hz_to_mel = |f: f32| 2595.0 * (1.0 + f / 700.0).log10();
    let mel_to_hz = |m: f32| 700.0 * (10_f32.powf(m / 2595.0) - 1.0);

    let mel_low = hz_to_mel(0.0);
    let mel_high = hz_to_mel(sample_rate / 2.0);

    let hz_points: Vec<f32> = (0..=n_mels + 1)
        .map(|i| {
            mel_to_hz(mel_low + (mel_high - mel_low) * i as f32 / (n_mels + 1) as f32)
        })
        .collect();

    let bin_points: Vec<usize> = hz_points
        .iter()
        .map(|&f| {
            ((f / sample_rate * n_fft as f32).round() as usize).min(n_fft_bins - 1)
        })
        .collect();

    let mut filters = vec![vec![0.0f32; n_fft_bins]; n_mels];

    for m in 0..n_mels {
        let start = bin_points[m];
        let center = bin_points[m + 1];
        let end = bin_points[m + 2];

        if center > start {
            for (b, coeff) in filters[m][start..center].iter_mut().enumerate() {
                *coeff = b as f32 / (center - start) as f32;
            }
        }
        let clipped = end.min(n_fft_bins - 1);
        if end > center && clipped >= center {
            let slope = (end - center) as f32;
            for (b, coeff) in filters[m][center..=clipped].iter_mut().enumerate() {
                *coeff = (end - center - b) as f32 / slope;
            }
        }
    }

    filters
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-10 || norm_b < 1e-10 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod cosine_similarity_tests {
        use super::*;

        #[test]
        fn identical_non_zero_vectors_return_one() {
            let v = [1.0_f32, 2.0, 3.0];
            let result = cosine_similarity(&v, &v);
            assert!((result - 1.0_f32).abs() < 1e-5_f32, "expected ~1.0, got {result}");
        }

        #[test]
        fn orthogonal_vectors_return_zero() {
            let a = [1.0_f32, 0.0];
            let b = [0.0_f32, 1.0];
            let result = cosine_similarity(&a, &b);
            assert!(result.abs() < 1e-5_f32, "expected ~0.0, got {result}");
        }

        #[test]
        fn zero_vectors_return_zero() {
            let z = [0.0_f32, 0.0, 0.0];
            let result = cosine_similarity(&z, &z);
            assert!(result.abs() < 1e-5_f32, "expected 0.0, got {result}");
        }

        #[test]
        fn anti_parallel_vectors_return_negative_one() {
            let a = [1.0_f32, 0.0];
            let b = [-1.0_f32, 0.0];
            let result = cosine_similarity(&a, &b);
            assert!((result + 1.0_f32).abs() < 1e-5_f32, "expected ~-1.0, got {result}");
        }
    }

    mod fingerprint_tests {
        use super::*;

        #[test]
        fn short_sample_returns_zero_vector() {
            let hamming = build_hamming(FRAME_LEN);
            let filterbank = build_mel_filterbank(N_MELS, FRAME_LEN, SAMPLE_RATE);
            // Fewer samples than one frame length — must return all zeros
            let samples = vec![0.5_f32; FRAME_LEN - 1];
            let fp = compute_fingerprint(&samples, &hamming, &filterbank);
            assert_eq!(fp.len(), N_MFCC);
            assert!(fp.iter().all(|&x| x == 0.0_f32), "expected all zeros for short input");
        }

        #[test]
        fn valid_sample_returns_non_zero_fingerprint() {
            let hamming = build_hamming(FRAME_LEN);
            let filterbank = build_mel_filterbank(N_MELS, FRAME_LEN, SAMPLE_RATE);
            // DC signal — constant amplitude produces measurable log energy
            let samples = vec![0.5_f32; 16_000];
            let fp = compute_fingerprint(&samples, &hamming, &filterbank);
            assert_eq!(fp.len(), N_MFCC);
            assert!(fp.iter().any(|&x| x != 0.0_f32), "expected non-zero fingerprint");
        }
    }

    mod speaker_tracker_tests {
        use super::*;

        #[test]
        fn first_valid_sample_gets_id_zero() {
            let mut tracker = SpeakerTracker::new();
            let samples = vec![0.5_f32; 16_000];
            let id = tracker.identify_or_register(&samples);
            assert_eq!(id, 0);
        }

        #[test]
        fn short_sample_does_not_consume_a_speaker_slot() {
            let mut tracker = SpeakerTracker::new();
            // A short sample (below FRAME_LEN) produces an all-zero fingerprint and
            // must not register a new speaker. Verify by confirming that the next valid
            // sample still gets id 0 — if a slot had been consumed it would get id 1.
            tracker.identify_or_register(&[0.1_f32; 10]);
            let id_valid = tracker.identify_or_register(&vec![0.5_f32; 16_000]);
            assert_eq!(id_valid, 0, "short sample must not consume a speaker slot");
        }

        #[test]
        fn same_audio_recognized_as_same_speaker() {
            let mut tracker = SpeakerTracker::new();
            let samples = vec![0.5_f32; 16_000];
            let id_first = tracker.identify_or_register(&samples);
            let id_second = tracker.identify_or_register(&samples);
            assert_eq!(id_first, 0);
            assert_eq!(id_second, 0, "same audio must not be registered as a new speaker");
        }

        #[test]
        fn reset_clears_speakers_and_restarts_ids() {
            let mut tracker = SpeakerTracker::new();
            let samples = vec![0.5_f32; 16_000];
            tracker.identify_or_register(&samples);
            tracker.reset();
            // After reset, the same sample should get id 0 again
            let id = tracker.identify_or_register(&samples);
            assert_eq!(id, 0);
        }
    }
}
