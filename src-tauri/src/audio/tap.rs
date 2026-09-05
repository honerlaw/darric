//! Capturing what the machine plays, via a Core Audio process tap.
//!
//! For each output device we build three objects and must tear all three down:
//!
//! 1. a **process tap** scoped to that device's UID, excluding no processes —
//!    i.e. everything routed to that device;
//! 2. a private **aggregate device** whose tap list contains that tap, which is
//!    what turns the tap into something readable as an input;
//! 3. an **IOProc** on the aggregate, which delivers the audio.
//!
//! [`OutputTap`] owns all three and unwinds them in reverse on drop, so a
//! partially-constructed tap cannot leak a system-wide audio object.
//!
//! # Panic safety
//!
//! The IOProc block is invoked by Core Audio on a realtime thread through a C
//! function pointer. A Rust panic crossing that boundary aborts the process, so
//! the block body is wrapped in `catch_unwind` — without it, one bad index in
//! the sample conversion takes down the whole app rather than one device, which
//! would quietly falsify the per-device isolation the engine is built on.

use super::coreaudio;
use super::resample;
use crate::error::{AppError, Result};
use block2::RcBlock;
use objc2::AnyThread;
use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceIsStackedKey,
    kAudioAggregateDeviceNameKey, kAudioAggregateDeviceTapListKey, kAudioAggregateDeviceUIDKey,
    kAudioSubTapUIDKey, AudioDeviceCreateIOProcIDWithBlock, AudioDeviceDestroyIOProcID,
    AudioDeviceIOProcID, AudioDeviceStart, AudioDeviceStop, AudioHardwareCreateAggregateDevice,
    AudioHardwareCreateProcessTap, AudioHardwareDestroyAggregateDevice,
    AudioHardwareDestroyProcessTap, AudioObjectID, CATapDescription,
};
use objc2_core_audio_types::{AudioBufferList, AudioTimeStamp};
use objc2_core_foundation::CFDictionary;
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;

/// Convert one of Core Audio's `&CStr` dictionary keys into an `NSString`.
fn key(raw: &std::ffi::CStr) -> objc2::rc::Retained<NSString> {
    NSString::from_str(&raw.to_string_lossy())
}

/// A running tap on one output device.
///
/// Dropping this stops and destroys everything it created.
pub struct OutputTap {
    tap_id: AudioObjectID,
    aggregate_id: AudioObjectID,
    proc_id: AudioDeviceIOProcID,
    /// The aggregate's UID, so the engine can exclude it from input enumeration.
    aggregate_uid: String,
    device_name: String,
}

impl OutputTap {
    /// The UID of the private aggregate device this tap created.
    ///
    /// The aggregate is created private, which hides it from *other* processes —
    /// but not from this one, and `cpal`'s macOS backend enumerates within this
    /// process. So this UID still has to be excluded explicitly, or the app
    /// would list its own tap as a microphone and record itself.
    pub fn aggregate_uid(&self) -> &str {
        &self.aggregate_uid
    }

    /// Build and start a tap on `device_uid`.
    ///
    /// `on_samples` is called from a Core Audio realtime thread with 16 kHz mono
    /// audio. It must not block.
    pub fn start<F>(device_uid: &str, device_name: &str, on_samples: F) -> Result<Self>
    where
        F: Fn(&[f32]) + Send + Sync + 'static,
    {
        let (tap_id, uuid) = create_tap(device_uid, device_name)?;

        let own_uid = format!("darric-tap-{uuid}");
        let aggregate_id = match create_aggregate(&own_uid, device_name, &uuid) {
            Ok(id) => id,
            Err(e) => {
                destroy_tap(tap_id);
                return Err(e);
            }
        };

        // The aggregate delivers at the tapped device's native format, so it is
        // read rather than assumed — resampling from a guessed rate yields audio
        // at the wrong speed, which transcribes into plausible nonsense.
        let (rate, channels) = match coreaudio::input_stream_format(aggregate_id) {
            Ok(f) => f,
            Err(e) => {
                destroy_aggregate(aggregate_id);
                destroy_tap(tap_id);
                return Err(e);
            }
        };
        log::info!("[tap] {device_name}: {rate} Hz, {channels} ch");

        match start_io_proc(aggregate_id, rate, channels, on_samples) {
            Ok(proc_id) => Ok(Self {
                tap_id,
                aggregate_id,
                proc_id,
                aggregate_uid: own_uid,
                device_name: device_name.to_string(),
            }),
            Err(e) => {
                destroy_aggregate(aggregate_id);
                destroy_tap(tap_id);
                Err(e)
            }
        }
    }
}

impl Drop for OutputTap {
    fn drop(&mut self) {
        // Reverse construction order: stop delivering, release the proc, then
        // the aggregate, then the tap. Every one of these is a system-wide
        // object that survives the process if it is not destroyed.
        // SAFETY: all three ids were produced by the matching create calls in
        // `start` and have not been destroyed yet — `Drop` runs once.
        unsafe {
            AudioDeviceStop(self.aggregate_id, self.proc_id);
            AudioDeviceDestroyIOProcID(self.aggregate_id, self.proc_id);
        }
        destroy_aggregate(self.aggregate_id);
        destroy_tap(self.tap_id);
        log::info!("[tap] released tap on {}", self.device_name);
    }
}

fn create_tap(device_uid: &str, device_name: &str) -> Result<(AudioObjectID, String)> {
    let excluded: objc2::rc::Retained<NSArray<NSNumber>> = NSArray::new();
    let uid = NSString::from_str(device_uid);

    // SAFETY: `CATapDescription` is an ordinary Objective-C class; `alloc` then
    // an `init…` family method is the required construction sequence, and both
    // arguments outlive the call.
    let desc = unsafe {
        let allocated = CATapDescription::alloc();
        CATapDescription::initExcludingProcesses_andDeviceUID_withStream(
            allocated, &excluded, &uid, 0,
        )
    };

    // SAFETY: plain property setters on a live object we own.
    unsafe {
        desc.setName(&NSString::from_str(&format!("Darric — {device_name}")));
        // Private: not advertised to other processes.
        desc.setPrivate(true);
    }
    // Mute behaviour is deliberately left at its default, `CATapUnmuted`: the
    // user must keep hearing their own audio while it is being recorded.

    let mut tap_id: AudioObjectID = 0;
    // SAFETY: `desc` is live for the call and `tap_id` is a live local.
    let st = unsafe { AudioHardwareCreateProcessTap(Some(&desc), std::ptr::from_mut(&mut tap_id)) };
    if st != 0 {
        return Err(AppError::Audio(format!(
            "AudioHardwareCreateProcessTap failed for {device_name}: OSStatus {st} \
             (is the audio-recording permission granted?)"
        )));
    }

    // Read the UUID from the very description that built the tap — it is what
    // the aggregate device's tap list refers to. Rebuilding a second
    // description to ask again would be asking about a different object.
    // SAFETY: reading a property of a live object we own.
    let tap_uuid_string = unsafe { desc.UUID() }.UUIDString().to_string();
    if tap_uuid_string.is_empty() {
        destroy_tap(tap_id);
        return Err(AppError::Audio(format!(
            "tap for {device_name} has no UUID"
        )));
    }
    Ok((tap_id, tap_uuid_string))
}

fn create_aggregate(
    aggregate_uid: &str,
    device_name: &str,
    tap_uuid: &str,
) -> Result<AudioObjectID> {
    let sub_tap = NSDictionary::from_slices(
        &[&*key(kAudioSubTapUIDKey)],
        &[&*NSString::from_str(tap_uuid) as &objc2::runtime::AnyObject],
    );
    let tap_list = NSArray::from_slice(&[&*sub_tap as &objc2::runtime::AnyObject]);

    let description = NSDictionary::from_slices(
        &[
            &*key(kAudioAggregateDeviceNameKey),
            &*key(kAudioAggregateDeviceUIDKey),
            &*key(kAudioAggregateDeviceIsPrivateKey),
            &*key(kAudioAggregateDeviceIsStackedKey),
            &*key(kAudioAggregateDeviceTapListKey),
        ],
        &[
            &*NSString::from_str(&format!("Darric Tap — {device_name}"))
                as &objc2::runtime::AnyObject,
            &*NSString::from_str(aggregate_uid) as &objc2::runtime::AnyObject,
            &*NSNumber::new_bool(true) as &objc2::runtime::AnyObject,
            &*NSNumber::new_bool(false) as &objc2::runtime::AnyObject,
            &*tap_list as &objc2::runtime::AnyObject,
        ],
    );

    let mut created_id: AudioObjectID = 0;
    // SAFETY: `NSDictionary` and `CFDictionary` are toll-free bridged, so the
    // pointer cast is sound for the duration of the call; `description` is held
    // alive by this scope, and `aggregate_id` is a live local.
    let st = unsafe {
        let cf: &CFDictionary = &*(std::ptr::from_ref(&*description).cast::<CFDictionary>());
        AudioHardwareCreateAggregateDevice(cf, NonNull::from(&mut created_id))
    };
    if st != 0 {
        return Err(AppError::Audio(format!(
            "AudioHardwareCreateAggregateDevice failed for {device_name}: OSStatus {st}"
        )));
    }
    Ok(created_id)
}

fn start_io_proc<F>(
    aggregate_id: AudioObjectID,
    rate: u32,
    channels: u16,
    on_samples: F,
) -> Result<AudioDeviceIOProcID>
where
    F: Fn(&[f32]) + Send + Sync + 'static,
{
    let block = RcBlock::new(
        move |_now: NonNull<AudioTimeStamp>,
              input: NonNull<AudioBufferList>,
              _input_time: NonNull<AudioTimeStamp>,
              _output: NonNull<AudioBufferList>,
              _output_time: NonNull<AudioTimeStamp>| {
            // A panic here would cross a C boundary and abort the process, so
            // it is contained: one malformed buffer must cost this device's
            // audio, not the whole recording.
            let result = catch_unwind(AssertUnwindSafe(|| {
                // SAFETY: Core Audio guarantees `input` points at a valid
                // buffer list for the duration of the callback.
                let list = unsafe { input.as_ref() };
                let raw = interleaved_f32(list);
                if !raw.is_empty() {
                    let samples = resample::to_16k_mono(&raw, channels, rate);
                    if !samples.is_empty() {
                        on_samples(&samples);
                    }
                }
            }));
            if result.is_err() {
                log::error!("[tap] panic in the IO callback was contained");
            }
        },
    );

    let mut proc_id: AudioDeviceIOProcID = None;
    // SAFETY: `proc_id` is a live local; the block is retained by Core Audio
    // for the lifetime of the proc, and `RcBlock` keeps it alive here.
    let st = unsafe {
        AudioDeviceCreateIOProcIDWithBlock(
            NonNull::from(&mut proc_id),
            aggregate_id,
            None,
            (&raw const *block).cast_mut(),
        )
    };
    if st != 0 {
        return Err(AppError::Audio(format!(
            "AudioDeviceCreateIOProcIDWithBlock failed: OSStatus {st}"
        )));
    }

    // SAFETY: `proc_id` was just created for `aggregate_id`.
    let st = unsafe { AudioDeviceStart(aggregate_id, proc_id) };
    if st != 0 {
        // SAFETY: undo the proc we just created before reporting failure.
        unsafe { AudioDeviceDestroyIOProcID(aggregate_id, proc_id) };
        return Err(AppError::Audio(format!(
            "AudioDeviceStart failed: OSStatus {st}. A tap can be created \
             without the audio-recording permission but cannot be started; \
             check System Settings > Privacy & Security > Microphone for Darric."
        )));
    }
    // Deliberately leaked into Core Audio's ownership: the block must outlive
    // this function and is released when the IOProc is destroyed.
    std::mem::forget(block);
    Ok(proc_id)
}

/// Flatten a buffer list of 32-bit float samples into one interleaved slice.
///
/// A tap delivers one buffer per stream. Anything that is not 32-bit float is
/// ignored rather than reinterpreted — guessing at a format would produce
/// convincing noise rather than an obvious failure.
fn interleaved_f32(list: &AudioBufferList) -> Vec<f32> {
    let count = list.mNumberBuffers as usize;
    if count == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    // SAFETY: `mBuffers` is a flexible array member of length `mNumberBuffers`;
    // Core Audio guarantees that many entries are present.
    let buffers = unsafe { std::slice::from_raw_parts(list.mBuffers.as_ptr(), count) };
    for b in buffers {
        let bytes = b.mDataByteSize as usize;
        let frames = bytes / size_of::<f32>();
        if frames == 0 || b.mData.is_null() {
            continue;
        }
        // SAFETY: `mData` points at `mDataByteSize` bytes of sample data owned
        // by Core Audio for the duration of the callback; `frames` is derived
        // from that same byte count, so the slice cannot overrun it.
        let slice = unsafe { std::slice::from_raw_parts(b.mData.cast::<f32>(), frames) };
        out.extend_from_slice(slice);
    }
    out
}

fn destroy_tap(tap_id: AudioObjectID) {
    // SAFETY: `tap_id` came from `AudioHardwareCreateProcessTap`.
    let st = unsafe { AudioHardwareDestroyProcessTap(tap_id) };
    if st != 0 {
        log::warn!("[tap] AudioHardwareDestroyProcessTap failed: OSStatus {st}");
    }
}

fn destroy_aggregate(aggregate_id: AudioObjectID) {
    // SAFETY: `aggregate_id` came from `AudioHardwareCreateAggregateDevice`.
    let st = unsafe { AudioHardwareDestroyAggregateDevice(aggregate_id) };
    if st != 0 {
        log::warn!("[tap] AudioHardwareDestroyAggregateDevice failed: OSStatus {st}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Builds a real tap on a real output device and waits briefly for audio.
    ///
    /// Ignored by default: it needs the audio-recording permission, real
    /// hardware, and something actually playing for the sample count to be
    /// non-zero. Run with:
    ///   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture tap_
    #[test]
    #[ignore = "creates a real Core Audio tap; needs the audio-recording permission"]
    fn tap_starts_and_tears_down_cleanly() {
        let Some(device) = coreaudio::list_output_devices().into_iter().next() else {
            println!("no output devices — skipping");
            return;
        };
        println!("tapping {:?} ({})", device.name, device.uid);

        let calls = Arc::new(AtomicUsize::new(0));
        let frames = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let f = Arc::clone(&frames);

        let tap = match OutputTap::start(&device.uid, &device.name, move |samples| {
            c.fetch_add(1, Ordering::Relaxed);
            f.fetch_add(samples.len(), Ordering::Relaxed);
        }) {
            Ok(t) => t,
            Err(e) => {
                // NOT a pass. A bare `cargo test` binary has no Info.plist and
                // no bundle id, so macOS TCC cannot associate it with an
                // audio-recording grant or prompt for one — the
                // NSAudioCaptureUsageDescription this needs lives in the app
                // bundle. Expect this path outside the built .app; the tap
                // itself is verified by launching the app.
                println!("!! TAP DID NOT START (expected outside the app bundle): {e}");
                return;
            }
        };
        println!("aggregate uid: {}", tap.aggregate_uid());
        assert!(tap.aggregate_uid().starts_with("darric-tap-"));

        std::thread::sleep(std::time::Duration::from_millis(1500));
        println!(
            "callbacks: {}, samples: {}",
            calls.load(Ordering::Relaxed),
            frames.load(Ordering::Relaxed)
        );

        drop(tap);
        println!("tap released without panicking");
    }
}
