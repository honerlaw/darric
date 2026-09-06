//! One capture source: a device, its stream, and the thread that keeps it alive.
//!
//! Each source runs its own supervisor thread which builds the stream, watches
//! it, and rebuilds it with backoff if it fails. A device that disappears —
//! a USB mic unplugged mid-recording — takes down only its own source; the
//! recording and every other device continue.
//!
//! The rebuilding is bounded. A Continuity iPhone microphone that was
//! enumerable when the recording started and gone by the time its stream was
//! built used to be retried, by name, for the whole session: the UI pulsed
//! "retrying" for an hour on a device that was never coming back. After
//! [`GIVE_UP_AFTER`] of continuous failure the supervisor marks the source
//! failed and exits.
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
use std::time::{Duration, Instant};

/// Backoff schedule for rebuilding a failed stream. The last value repeats.
const BACKOFF: [Duration; 4] = [
    Duration::from_millis(250),
    Duration::from_millis(1_000),
    Duration::from_secs(3),
    Duration::from_secs(10),
];

/// How often the supervisor checks whether its stream is still healthy.
const HEALTH_POLL: Duration = Duration::from_millis(200);

/// How long a source may keep failing to rebuild before the supervisor gives up.
///
/// Long enough to ride out a USB device being unplugged and plugged back in;
/// short enough that a device that has really gone is reported as such within
/// the first minute rather than never.
const GIVE_UP_AFTER: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    Starting,
    Active,
    /// The stream failed and is being rebuilt.
    Retrying,
    /// The device is gone or refuses to open, or the recording stopped; no
    /// further attempts.
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
    // When the current run of failures began; `None` while the stream is up.
    let mut failing_since: Option<Instant> = None;

    while !shutdown.load(Ordering::Relaxed) {
        // `stream_failed` is set by cpal's error callback; the supervisor polls
        // it because the callback itself must stay cheap and non-blocking.
        let stream_failed = Arc::new(AtomicBool::new(false));

        match build_stream(device, status, &stream_failed, Arc::clone(&on_samples)) {
            Ok(stream) => {
                attempt = 0;
                failing_since = None;
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

        let since = *failing_since.get_or_insert_with(Instant::now);
        if should_give_up(since, Instant::now()) {
            log::warn!(
                "[source] {} still unavailable after {}s — giving up",
                device.name,
                GIVE_UP_AFTER.as_secs()
            );
            break;
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

/// Whether a failure streak that began at `failing_since` has run long enough
/// to stop rebuilding. Pure, so the rule is testable without hardware.
fn should_give_up(failing_since: Instant, now: Instant) -> bool {
    now.saturating_duration_since(failing_since) >= GIVE_UP_AFTER
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
    fn a_failure_streak_gives_up_only_after_the_full_minute() {
        let began = Instant::now();
        assert!(!should_give_up(began, began), "a first failure is retried");
        let just_short = GIVE_UP_AFTER
            .checked_sub(Duration::from_millis(1))
            .expect("the limit is longer than a millisecond");
        assert!(
            !should_give_up(began, began + just_short),
            "just short of the limit is still retried"
        );
        assert!(
            should_give_up(began, began + GIVE_UP_AFTER),
            "the limit itself gives up"
        );
        assert!(
            !should_give_up(began + Duration::from_secs(5), began),
            "a clock that went backwards is not a reason to give up"
        );
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

    /// The give-up rule on the wall clock: a device that never appears is
    /// abandoned on its own, with no shutdown signal, a little after a minute.
    /// Ignored because it takes that minute and enumerates real hardware.
    #[test]
    #[ignore = "waits out the full give-up window against real audio hardware"]
    fn an_absent_device_is_abandoned_after_a_minute() {
        let status: SharedStatus = Arc::new(Mutex::new(SourceStatus::new(device())));
        let shutdown = Arc::new(AtomicBool::new(false));

        let s2 = Arc::clone(&status);
        let sd = Arc::clone(&shutdown);
        let began = Instant::now();
        let handle = std::thread::spawn(move || run_source(&device(), &s2, &sd, |_| {}));

        // Well past the window plus the longest backoff step.
        std::thread::sleep(GIVE_UP_AFTER + Duration::from_secs(15));
        assert!(
            handle.is_finished(),
            "the supervisor must have exited on its own"
        );
        handle.join().expect("supervisor thread");
        assert_eq!(status.lock().unwrap().state, SourceState::Failed);
        println!("gave up after {:?}", began.elapsed());
        assert!(
            !shutdown.load(Ordering::Relaxed),
            "nothing asked it to stop"
        );
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
