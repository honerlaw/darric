//! Mono mixdown and band-limited resampling to whisper's 16 kHz input rate.
//!
//! The previous resampler interpolated linearly with no filter, so everything
//! above 8 kHz in a 48 kHz capture folded down into the speech band before
//! whisper heard it. [`Resampler`] is a windowed-sinc interpolator: each output
//! sample is a weighted sum of the nearest [`TAPS`] input samples, the weights
//! being a sinc at the cutoff under a Blackman window. It keeps the last taps
//! of input between calls, so a callback boundary is invisible to the filter.
//!
//! Every position stays in the integer domain. The obvious implementation walks
//! a `f64` cursor and casts it back to an index per sample, which trips
//! `cast_possible_truncation` and `cast_sign_loss` — and this crate forbids
//! `#[allow]`, so the read position is a fixed-point numerator over
//! [`TARGET_RATE`], and the fractional phase converts to `f32` exactly through a
//! `u16`. Rates enter the filter as a reduced ratio of `u16`s for the same
//! reason. That is not lint appeasement: an accumulating float cursor also
//! drifts over a long stream, and the integer form cannot.

/// Whisper's required input rate.
pub const TARGET_RATE: u32 = 16_000;

/// Average interleaved frames down to a single channel.
///
/// `channels` is `cpal`'s own `u16`, so the divisor converts exactly via
/// `f32::from` — no precision-loss cast.
pub fn mix_to_mono(input: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return input.to_vec();
    }
    let divisor = f32::from(channels);
    input
        .chunks(usize::from(channels))
        .map(|frame| frame.iter().sum::<f32>() / divisor)
        .collect()
}

/// Input samples each output sample is computed from. Sixty-four gives a
/// transition band narrow enough that a 12 kHz tone in a 48 kHz capture is
/// more than 40 dB down after decimation.
pub const TAPS: usize = 64;
const HALF: usize = TAPS / 2;

/// Cutoff as a fraction of the lower Nyquist frequency. Leaves a transition
/// band below Nyquist so the stopband is reached before anything can alias.
const CUTOFF_OF_NYQUIST: f32 = 0.9;

/// Band-limited resampler from one source rate to [`TARGET_RATE`], mixing to
/// mono on the way. One instance per capture stream; it is stateful.
pub struct Resampler {
    src_rate: u32,
    channels: u16,
    /// `HALF` samples of history followed by input not yet fully consumed.
    pending: Vec<f32>,
    /// Absolute index of the input sample at `pending[HALF]`.
    consumed: u64,
    /// Output samples produced so far; the next one's position derives from it.
    produced: u64,
    /// Cutoff in cycles per input sample.
    cutoff: f32,
}

impl Resampler {
    pub fn new(src_rate: u32, channels: u16) -> Self {
        let lower = src_rate.min(TARGET_RATE);
        let cutoff = ratio(lower, src_rate) * 0.5 * CUTOFF_OF_NYQUIST;
        Self {
            src_rate,
            channels,
            pending: vec![0.0; HALF],
            consumed: 0,
            produced: 0,
            cutoff,
        }
    }

    /// Mix `input` (interleaved, `channels` wide) to mono and resample it.
    ///
    /// Returns the output samples that can be computed so far; the tail that
    /// still needs future input is held until the next call. At the target rate
    /// the input passes through untouched.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        let mono = mix_to_mono(input, self.channels);
        if self.src_rate == TARGET_RATE {
            return mono;
        }
        self.pending.extend_from_slice(&mono);

        let mut out = Vec::new();
        loop {
            let numerator = self.produced * u64::from(self.src_rate);
            let centre = numerator / u64::from(TARGET_RATE);
            let remainder = numerator % u64::from(TARGET_RATE);
            // Position of `centre` in `pending`; `>= HALF` by construction, so
            // the earliest tap (`HALF - 1` before it) is in range.
            let Some(at) = (centre + HALF as u64)
                .checked_sub(self.consumed)
                .and_then(|p| usize::try_from(p).ok())
            else {
                break;
            };
            if at + HALF >= self.pending.len() {
                break;
            }
            // `remainder < TARGET_RATE` (16000), which always fits a u16.
            let frac = u16::try_from(remainder).map_or(0.0, f32::from) / f32::from(16_000_u16);
            out.push(self.interpolate(at, frac));
            self.produced += 1;
        }

        // Keep only the history the next output can still reach.
        let next_centre = (self.produced * u64::from(self.src_rate)) / u64::from(TARGET_RATE);
        let keep_from = (next_centre + 1).saturating_sub(HALF as u64);
        if let Some(drop) = keep_from.checked_sub(self.consumed) {
            let drop = usize::try_from(drop).unwrap_or(0).min(self.pending.len());
            self.pending.drain(..drop);
            self.consumed += drop as u64;
        }
        out
    }

    /// The windowed-sinc sum around `pending[at] + frac`, normalised so the
    /// filter passes DC at exactly unity whatever the phase.
    fn interpolate(&self, at: usize, frac: f32) -> f32 {
        let mut acc = 0.0_f32;
        let mut weight = 0.0_f32;
        for tap in 0..TAPS {
            // Offsets -(HALF-1) ..= HALF around the centre sample.
            let offset = f32::from(u16::try_from(tap).unwrap_or(0))
                - f32::from(u16::try_from(HALF - 1).unwrap_or(0));
            let t = offset - frac;
            let w = self.cutoff
                * sinc(self.cutoff * t)
                * blackman(t / f32::from(u16::try_from(HALF).unwrap_or(1)));
            let sample = self.pending[at + 1 + tap - HALF];
            acc = w.mul_add(sample, acc);
            weight += w;
        }
        if weight.abs() < f32::EPSILON {
            0.0
        } else {
            acc / weight
        }
    }
}

/// `a / b` as an `f32`, with both rates reduced by their gcd first so each side
/// fits a `u16` and converts exactly — every common audio rate over 16 kHz does.
fn ratio(a: u32, b: u32) -> f32 {
    let g = gcd(a, b).max(1);
    let num = u16::try_from(a / g).unwrap_or(u16::MAX);
    let den = u16::try_from(b / g).unwrap_or(u16::MAX).max(1);
    f32::from(num) / f32::from(den)
}

const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// Normalised sinc: `sin(πx) / (πx)`, 1 at zero.
fn sinc(x: f32) -> f32 {
    if x.abs() < 1e-6 {
        1.0
    } else {
        let px = std::f32::consts::PI * x;
        px.sin() / px
    }
}

/// Blackman window over `x` in [-1, 1], zero outside.
fn blackman(x: f32) -> f32 {
    if x.abs() >= 1.0 {
        return 0.0;
    }
    let p = std::f32::consts::PI * x;
    0.08_f32.mul_add((2.0 * p).cos(), 0.5_f32.mul_add(p.cos(), 0.42))
}

/// Root-mean-square level of a buffer, for the UI's per-device meter.
///
/// The sample count is accumulated as an `f32` rather than converted from
/// `usize`, so there is no lossy cast anywhere in the calculation. Meter
/// precision does not need more than this.
pub fn rms(samples: &[f32]) -> f32 {
    let mut sum_squares = 0.0_f32;
    let mut count = 0.0_f32;
    for s in samples {
        sum_squares = s.mul_add(*s, sum_squares);
        count += 1.0;
    }
    if count == 0.0 {
        return 0.0;
    }
    (sum_squares / count).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tone at `hz` sampled at `rate`, amplitude 1, built without casts.
    fn tone(hz: u16, rate: u16, samples: usize) -> Vec<f32> {
        let step = std::f32::consts::TAU * f32::from(hz) / f32::from(rate);
        let mut phase = 0.0_f32;
        (0..samples)
            .map(|_| {
                let s = phase.sin();
                phase += step;
                if phase > std::f32::consts::TAU {
                    phase -= std::f32::consts::TAU;
                }
                s
            })
            .collect()
    }

    /// RMS of the output with the filter's warm-up and tail trimmed.
    fn settled_rms(out: &[f32]) -> f32 {
        let trim = TAPS.min(out.len() / 4);
        rms(&out[trim..out.len() - trim])
    }

    fn db(ratio: f32) -> f32 {
        20.0 * ratio.log10()
    }

    #[test]
    fn mono_passthrough_is_identity() {
        let input = vec![0.1, -0.2, 0.3];
        assert_eq!(mix_to_mono(&input, 1), input);
    }

    #[test]
    fn stereo_averages_pairs() {
        // L/R interleaved: (0,1) -> 0.5, (1,0) -> 0.5
        let input = vec![0.0, 1.0, 1.0, 0.0];
        assert_eq!(mix_to_mono(&input, 2), vec![0.5, 0.5]);
    }

    #[test]
    fn a_speech_band_tone_passes_at_unity() {
        let input = tone(1_000, 48_000, 48_000);
        let out = Resampler::new(48_000, 1).process(&input);
        let level = settled_rms(&out) / std::f32::consts::FRAC_1_SQRT_2;
        assert!(db(level).abs() < 1.0, "1 kHz gain {} dB", db(level));
    }

    #[test]
    fn a_tone_above_the_new_nyquist_is_removed_not_aliased() {
        // 12 kHz at 48 kHz would fold to 4 kHz through a linear resampler.
        let input = tone(12_000, 48_000, 48_000);
        let out = Resampler::new(48_000, 1).process(&input);
        let level = settled_rms(&out) / std::f32::consts::FRAC_1_SQRT_2;
        assert!(db(level) <= -40.0, "12 kHz residue {} dB", db(level));
    }

    #[test]
    fn the_target_rate_passes_through_unchanged() {
        let input = tone(1_000, 16_000, 1_600);
        assert_eq!(Resampler::new(16_000, 1).process(&input), input);
    }

    #[test]
    fn output_does_not_depend_on_push_size() {
        let input = tone(700, 44_100, 44_100);
        let whole = Resampler::new(44_100, 1).process(&input);
        let mut piecewise = Resampler::new(44_100, 1);
        let mut pieces = Vec::new();
        for chunk in input.chunks(441) {
            pieces.extend(piecewise.process(chunk));
        }
        assert_eq!(whole.len(), pieces.len());
        let worst = whole
            .iter()
            .zip(&pieces)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);
        assert!(worst < 1e-5, "worst difference {worst}");
    }

    #[test]
    fn one_second_in_is_one_second_out_less_the_filter_delay() {
        let out = Resampler::new(48_000, 1).process(&vec![0.0; 48_000]);
        assert!(
            out.len() > 16_000 - TAPS && out.len() <= 16_000,
            "{}",
            out.len()
        );
        let out = Resampler::new(8_000, 1).process(&vec![0.0; 8_000]);
        assert!(
            out.len() > 16_000 - 2 * TAPS && out.len() <= 16_000,
            "{}",
            out.len()
        );
    }

    #[test]
    fn stereo_input_is_mixed_before_resampling() {
        // Two identical channels resample to the same thing as one.
        let mono = tone(1_000, 48_000, 4_800);
        let stereo: Vec<f32> = mono.iter().flat_map(|s| [*s, *s]).collect();
        let a = Resampler::new(48_000, 1).process(&mono);
        let b = Resampler::new(48_000, 2).process(&stereo);
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-6));
    }

    #[test]
    fn ratios_reduce_exactly() {
        assert!((ratio(16_000, 48_000) - 1.0 / 3.0).abs() < 1e-7);
        assert!((ratio(16_000, 44_100) - 160.0 / 441.0).abs() < 1e-7);
        assert!((ratio(16_000, 16_000) - 1.0).abs() < f32::EPSILON);
        assert_eq!(gcd(48_000, 16_000), 16_000);
    }

    #[test]
    fn rms_of_empty_is_zero() {
        assert!((rms(&[]) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rms_of_silence_is_zero() {
        assert!((rms(&[0.0; 64]) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rms_of_full_scale_square_is_one() {
        let signal: Vec<f32> = (0..64)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        assert!((rms(&signal) - 1.0).abs() < 1e-4);
    }
}
