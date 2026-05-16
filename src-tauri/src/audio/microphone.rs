use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

use super::{AudioChunk, SendableStream};
use crate::error::{AppError, Result};

#[allow(clippy::cast_precision_loss)]
pub fn start_mic_capture(
    tx: mpsc::SyncSender<AudioChunk>,
    shutdown: Arc<AtomicBool>,
) -> Result<SendableStream> {
    let host = cpal::default_host();
    log::info!("[mic] host: {}", host.id().name());

    let device = host
        .default_input_device()
        .ok_or_else(|| AppError::Audio("no default microphone found".to_string()))?;

    log::info!(
        "[mic] device: {}",
        device.name().unwrap_or_else(|_| "unknown".to_string())
    );

    let supported = device
        .default_input_config()
        .map_err(|e| AppError::Audio(e.to_string()))?;

    let sample_rate = supported.sample_rate().0;
    let channels = usize::from(supported.channels());
    log::info!(
        "[mic] config: {}Hz, {} ch, {:?}",
        sample_rate,
        channels,
        supported.sample_format()
    );

    let mut callback_count: u64 = 0;

    let stream = device
        .build_input_stream(
            &supported.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if shutdown.load(Ordering::Relaxed) {
                    return;
                }
                callback_count += 1;
                let samples = resample_to_16k_mono(data, channels, sample_rate);
                // Log every ~100 callbacks so we can confirm audio is flowing
                if callback_count % 100 == 1 {
                    let energy: f32 =
                        samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
                    log::debug!(
                        "[mic] callback #{}: {} raw frames → {} resampled, rms={:.5}",
                        callback_count,
                        data.len(),
                        samples.len(),
                        energy.sqrt()
                    );
                }
                if tx
                    .try_send(AudioChunk {
                        source: super::AudioSource::Mic,
                        samples,
                    })
                    .is_err()
                {
                    log::warn!("[mic] channel full — dropping audio chunk");
                }
            },
            move |err| {
                log::error!("[mic] stream error: {err}");
            },
            None,
        )
        .map_err(|e| AppError::Audio(e.to_string()))?;

    stream.play().map_err(|e| AppError::Audio(e.to_string()))?;
    log::info!("[mic] stream started");
    Ok(SendableStream(stream))
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn resample_to_16k_mono(input: &[f32], channels: usize, src_rate: u32) -> Vec<f32> {
    // Mix to mono first
    let mono: Vec<f32> = if channels == 1 {
        input.to_vec()
    } else {
        input
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    if src_rate == 16000 {
        return mono;
    }

    // Linear interpolation resample
    let ratio = f64::from(src_rate) / 16000.0;
    let out_len = (mono.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let src_pos = i as f64 * ratio;
            let idx = src_pos as usize;
            let frac = (src_pos - idx as f64) as f32;
            let a = mono.get(idx).copied().unwrap_or(0.0);
            let b = mono.get(idx + 1).copied().unwrap_or(0.0);
            (b - a).mul_add(frac, a)
        })
        .collect()
}
