//! One capture source: a device, its stream, and the thread that keeps it alive.
//!
//! Each source runs its own supervisor thread which builds the stream, watches
//! it, and rebuilds it with backoff if it fails. A device that disappears —
//! a USB mic unplugged mid-recording — takes down only its own source; the
//! recording and every other device continue.
//!
//! The `cpal::Stream` is created and dropped **on the supervisor thread that
//! owns it**. The previous implementation built it elsewhere and moved it across
//! a thread boundary behind an `unsafe impl Send` wrapper; keeping it on one
//! thread removes that unsafety rather than asserting past it.

use super::device::CaptureDevice;
use super::resample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Backoff schedule for rebuilding a failed stream. The last value repeats.
const BACKOFF: [Duration; 4] = [
    Duration::from_millis(250),
    Duration::from_millis(1_000),
    Duration::from_secs(3),
    Duration::from_secs(10),
];

/// How often the supervisor checks whether its stream is still healthy.
const HEALTH_POLL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    Starting,
    Active,
    /// The stream failed and is being rebuilt.
    Retrying,
    /// The device is gone or refuses to open; no further attempts.
    Failed,
}

impl SourceState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Active => "active",
            Self::Retrying => "retrying",
            Self::Failed => "failed",
        }
    }
}

/// What the UI shows for one device.
#[derive(Debug, Clone)]
pub struct SourceStatus {
    pub device: CaptureDevice,
    pub state: SourceState,
    /// Most recent RMS level, for the meter.
    pub level: f32,
}

impl SourceStatus {
    pub const fn new(device: CaptureDevice) -> Self {
        Self {
            device,
            state: SourceState::Starting,
            level: 0.0,
        }
    }
}

/// Shared, mutable view of one source, read by the UI and written by its thread.
pub type SharedStatus = Arc<Mutex<SourceStatus>>;

/// Run one device's capture loop until `shutdown` is set.
///
/// `on_samples` receives 16 kHz mono audio. It is called from the audio callback
/// thread and must not block.
pub fn run_source<F>(
    device: &CaptureDevice,
    status: &SharedStatus,
    shutdown: &AtomicBool,
    on_samples: F,
) where
    F: Fn(&[f32]) + Send + Sync + 'static,
{
    let on_samples = Arc::new(on_samples);
    let mut attempt = 0_usize;

    while !shutdown.load(Ordering::Relaxed) {
        // `stream_failed` is set by cpal's error callback; the supervisor polls
        // it because the callback itself must stay cheap and non-blocking.
        let stream_failed = Arc::new(AtomicBool::new(false));

        match build_stream(device, status, &stream_failed, Arc::clone(&on_samples)) {
            Ok(stream) => {
                attempt = 0;
                set_state(status, SourceState::Active);
                log::info!("[source] {} active", device.name);

                while !shutdown.load(Ordering::Relaxed) && !stream_failed.load(Ordering::Relaxed) {
                    std::thread::sleep(HEALTH_POLL);
                }
                // Dropped here, on the thread that created it.
                drop(stream);

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
                log::warn!("[source] {} stream failed — will rebuild", device.name);
                set_state(status, SourceState::Retrying);
            }
            Err(e) => {
                set_state(status, SourceState::Retrying);
                log::warn!(
                    "[source] {} unavailable (attempt {}): {e}",
                    device.name,
                    attempt + 1
                );
            }
        }

        let wait = BACKOFF
            .get(attempt)
            .copied()
            .unwrap_or(BACKOFF[BACKOFF.len() - 1]);
        attempt = attempt.saturating_add(1);

        // Sleep in slices so shutdown is responsive during a long backoff.
        let mut slept = Duration::ZERO;
        while slept < wait && !shutdown.load(Ordering::Relaxed) {
            std::thread::sleep(HEALTH_POLL.min(wait.saturating_sub(slept)));
            slept += HEALTH_POLL;
        }
    }

    set_state(status, SourceState::Failed);
    log::info!("[source] {} stopped", device.name);
}

fn set_state(status: &SharedStatus, state: SourceState) {
    let mut s = status
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    s.state = state;
    if state != SourceState::Active {
        s.level = 0.0;
    }
}

fn build_stream<F>(
    device: &CaptureDevice,
    status: &SharedStatus,
    stream_failed: &Arc<AtomicBool>,
    on_samples: Arc<F>,
) -> Result<cpal::Stream, String>
where
    F: Fn(&[f32]) + Send + Sync + 'static,
{
    let host = cpal::default_host();
    let target = host
        .input_devices()
        .map_err(|e| e.to_string())?
        .find(|d| d.name().is_ok_and(|n| n == device.name))
        .ok_or_else(|| format!("device {} not present", device.name))?;

    let supported = target.default_input_config().map_err(|e| e.to_string())?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels();

    log::info!(
        "[source] {}: {} Hz, {} ch, {:?}",
        device.name,
        sample_rate,
        channels,
        supported.sample_format()
    );

    let status_for_cb = Arc::clone(status);
    let failed_for_cb = Arc::clone(stream_failed);

    let stream = target
        .build_input_stream(
            &supported.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let samples = resample::to_16k_mono(data, channels, sample_rate);
                if let Ok(mut s) = status_for_cb.try_lock() {
                    s.level = resample::rms(&samples);
                }
                on_samples(&samples);
            },
            move |err| {
                log::error!("[source] stream error: {err}");
                failed_for_cb.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::pool::Direction;

    fn device() -> CaptureDevice {
        CaptureDevice {
            id: "test".into(),
            name: "Definitely Not A Real Device".into(),
            direction: Direction::Input,
        }
    }

    #[test]
    fn state_renders_for_the_ui() {
        assert_eq!(SourceState::Starting.as_str(), "starting");
        assert_eq!(SourceState::Active.as_str(), "active");
        assert_eq!(SourceState::Retrying.as_str(), "retrying");
        assert_eq!(SourceState::Failed.as_str(), "failed");
    }

    #[test]
    fn a_new_status_starts_silent_and_starting() {
        let s = SourceStatus::new(device());
        assert_eq!(s.state, SourceState::Starting);
        assert!((s.level - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn setting_a_non_active_state_zeroes_the_meter() {
        // A retrying or failed device must not leave a frozen meter reading —
        // that reads as live audio from a device that is not capturing.
        let status: SharedStatus = Arc::new(Mutex::new(SourceStatus::new(device())));
        status.lock().unwrap().level = 0.75;
        set_state(&status, SourceState::Retrying);
        assert!((status.lock().unwrap().level - 0.0).abs() < f32::EPSILON);
    }

    // Drives the real supervisor loop, which enumerates Core Audio devices on
    // every rebuild attempt. Ignored for the same reason as the enumeration
    // test above; the pure state transitions are covered without hardware.
    #[test]
    #[ignore = "drives the supervisor against real audio hardware"]
    fn an_absent_device_gives_up_without_hanging_the_caller() {
        // The supervisor must exit promptly on shutdown even while backing off
        // against a device that will never appear.
        let status: SharedStatus = Arc::new(Mutex::new(SourceStatus::new(device())));
        let shutdown = Arc::new(AtomicBool::new(false));

        let s2 = Arc::clone(&status);
        let sd = Arc::clone(&shutdown);
        let handle = std::thread::spawn(move || run_source(&device(), &s2, &sd, |_| {}));

        std::thread::sleep(Duration::from_millis(400));
        shutdown.store(true, Ordering::Relaxed);
        handle.join().expect("supervisor thread should exit");

        assert_eq!(status.lock().unwrap().state, SourceState::Failed);
    }
}
