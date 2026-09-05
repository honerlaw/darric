//! Mono mixdown and resampling to whisper's 16 kHz input rate.
//!
//! Every conversion here stays in the integer domain where it can. The obvious
//! implementation walks a `f64` cursor and casts it back to an index per sample,
//! which trips `cast_possible_truncation` and `cast_sign_loss` — and this crate
//! forbids `#[allow]`, so the position is carried as a fixed-point integer
//! instead. That is not lint appeasement: an accumulating float cursor also
//! drifts over a long buffer, and the integer form cannot.

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

/// Linearly resample mono `input` from `src_rate` to [`TARGET_RATE`].
///
/// The source position for output sample `i` is `i * src_rate / TARGET_RATE`.
/// Holding that as a numerator over `TARGET_RATE` keeps the whole index
/// calculation in `u64`, and the remainder is always `< TARGET_RATE`, so it
/// fits a `u16` and converts to `f32` exactly.
pub fn resample_mono(input: &[f32], src_rate: u32) -> Vec<f32> {
    if src_rate == TARGET_RATE || input.is_empty() {
        return input.to_vec();
    }

    let src_len = u64::try_from(input.len()).unwrap_or(u64::MAX);
    let out_len = src_len * u64::from(TARGET_RATE) / u64::from(src_rate);
    let mut out = Vec::with_capacity(usize::try_from(out_len).unwrap_or(0));

    for i in 0..out_len {
        let numerator = i * u64::from(src_rate);
        let idx = usize::try_from(numerator / u64::from(TARGET_RATE)).unwrap_or(usize::MAX);
        let remainder = numerator % u64::from(TARGET_RATE);
        // `remainder < TARGET_RATE` (16000), which always fits a u16.
        let frac = u16::try_from(remainder).map_or(0.0, f32::from) / f32::from(16_000_u16);

        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push((b - a).mul_add(frac, a));
    }
    out
}

/// Mix to mono and resample in one step — what a capture callback wants.
pub fn to_16k_mono(input: &[f32], channels: u16, src_rate: u32) -> Vec<f32> {
    resample_mono(&mix_to_mono(input, channels), src_rate)
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

    /// 0.0, 1.0, 2.0, … with no integer-to-float cast.
    fn ramp(n: usize) -> Vec<f32> {
        std::iter::successors(Some(0.0_f32), |x| Some(x + 1.0))
            .take(n)
            .collect()
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
    fn resample_at_target_rate_is_identity() {
        let input = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_mono(&input, TARGET_RATE), input);
    }

    #[test]
    fn downsampling_halves_length() {
        let input: Vec<f32> = (0..320)
            .map(|i| if i % 2 == 0 { 0.0 } else { 1.0 })
            .collect();
        let out = resample_mono(&input, 32_000);
        assert_eq!(out.len(), 160);
    }

    #[test]
    fn upsampling_doubles_length() {
        let input: Vec<f32> = vec![0.0; 80];
        let out = resample_mono(&input, 8_000);
        assert_eq!(out.len(), 160);
    }

    #[test]
    fn resample_interpolates_linearly() {
        // A ramp resampled 2x down should sample every other point exactly.
        let input: Vec<f32> = ramp(16);
        let out = resample_mono(&input, 32_000);
        assert_eq!(out.len(), 8);
        assert!((out[0] - 0.0).abs() < 1e-6);
        assert!((out[1] - 2.0).abs() < 1e-6);
        assert!((out[7] - 14.0).abs() < 1e-6);
    }

    #[test]
    fn resample_of_empty_input_is_empty() {
        assert!(resample_mono(&[], 48_000).is_empty());
    }

    #[test]
    fn last_sample_does_not_interpolate_into_silence() {
        // Reading past the end must hold the final value, not fall to 0.0 —
        // otherwise every segment boundary gets a downward click.
        let input = vec![1.0, 1.0, 1.0, 1.0];
        let out = resample_mono(&input, 20_000);
        assert!(out.iter().all(|s| (*s - 1.0).abs() < 1e-6), "{out:?}");
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

    #[test]
    fn to_16k_mono_combines_both_stages() {
        // 4 stereo frames at 32 kHz -> 2 mono frames -> 1 frame at 16 kHz.
        let input = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let out = to_16k_mono(&input, 2, 32_000);
        assert_eq!(out.len(), 2);
    }
}
