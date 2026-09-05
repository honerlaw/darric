//! Thin, checked wrappers over the Core Audio property API.
//!
//! Everything below is `unsafe` at the boundary and safe above it. The pattern
//! throughout is: ask for the property's size, allocate exactly that, ask again
//! for the data, and only then interpret it. Reading into a fixed-size buffer
//! and trusting the size is how this API is usually misused.
//!
//! Bindings come from `objc2-core-audio`, which generates them from the SDK
//! headers. Nothing here re-declares a C signature by hand — a transcription
//! error in a calling convention or struct layout is exactly the class of
//! defect that is hardest to see in review and worst at runtime.

use crate::error::{AppError, Result};
use objc2_core_audio::{
    kAudioDevicePropertyDeviceUID, kAudioDevicePropertyStreamConfiguration,
    kAudioDevicePropertyStreamFormat, kAudioHardwarePropertyDevices,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyName, kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyScopeInput, kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject,
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
    AudioObjectPropertyAddress,
};
use objc2_core_audio_types::{AudioBufferList, AudioStreamBasicDescription};
use objc2_core_foundation::{CFRetained, CFString};
use std::ffi::c_void;
use std::ptr::NonNull;

/// An `OSStatus` that was not `noErr`.
fn status(code: i32, what: &str) -> AppError {
    // OSStatus codes are often four-character codes; show both readings so a
    // log line is greppable against Apple's headers either way.
    let bytes = code.to_be_bytes();
    let fourcc: String = bytes
        .iter()
        .map(|b| {
            let c = char::from(*b);
            if c.is_ascii_graphic() {
                c
            } else {
                '.'
            }
        })
        .collect();
    AppError::Audio(format!("{what} failed: OSStatus {code} ('{fourcc}')"))
}

pub const fn address(selector: u32, scope: u32, element: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: element,
    }
}

/// Byte size Core Audio reports for a property.
fn property_size(id: AudioObjectID, addr: &AudioObjectPropertyAddress) -> Result<u32> {
    let mut size: u32 = 0;
    let mut addr = *addr;
    // SAFETY: both pointers are to live locals held for the duration of the
    // call, and the qualifier is null with a zero length, which this API
    // accepts for every property read here.
    let st = unsafe {
        AudioObjectGetPropertyDataSize(
            id,
            NonNull::from(&mut addr),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
        )
    };
    if st != 0 {
        return Err(status(st, "AudioObjectGetPropertyDataSize"));
    }
    Ok(size)
}

/// Read a property whose value is a homogeneous array of `T`.
fn read_vec<T: Copy + Default>(
    id: AudioObjectID,
    addr: &AudioObjectPropertyAddress,
) -> Result<Vec<T>> {
    let size = property_size(id, addr)?;
    let elem = u32::try_from(size_of::<T>()).unwrap_or(1).max(1);
    let count = (size / elem) as usize;
    if count == 0 {
        return Ok(Vec::new());
    }

    let mut buf: Vec<T> = vec![T::default(); count];
    let mut io_size = size;
    let mut addr = *addr;
    // SAFETY: `buf` holds exactly `size` bytes (count * size_of::<T>()), which
    // is the size Core Audio just reported for this property, and it stays
    // alive and uniquely borrowed across the call.
    let st = unsafe {
        AudioObjectGetPropertyData(
            id,
            NonNull::from(&mut addr),
            0,
            std::ptr::null(),
            NonNull::from(&mut io_size),
            NonNull::new(buf.as_mut_ptr().cast::<c_void>())
                .ok_or_else(|| AppError::Audio("null property buffer".into()))?,
        )
    };
    if st != 0 {
        return Err(status(st, "AudioObjectGetPropertyData"));
    }
    // Core Audio may return fewer bytes than it advertised.
    let actual = (io_size / elem) as usize;
    buf.truncate(actual.min(count));
    Ok(buf)
}

/// Read a property whose value is a `CFStringRef`.
fn read_cfstring(id: AudioObjectID, addr: &AudioObjectPropertyAddress) -> Result<String> {
    let mut raw: *const CFString = std::ptr::null();
    let mut io_size = u32::try_from(size_of::<*const CFString>()).unwrap_or(8);
    let mut addr = *addr;
    // SAFETY: `raw` is a live local of exactly the size declared in `io_size`.
    // Core Audio writes an owned +1 CFStringRef into it, which is adopted below.
    let st = unsafe {
        AudioObjectGetPropertyData(
            id,
            NonNull::from(&mut addr),
            0,
            std::ptr::null(),
            NonNull::from(&mut io_size),
            NonNull::from(&mut raw).cast::<c_void>(),
        )
    };
    if st != 0 {
        return Err(status(st, "AudioObjectGetPropertyData(CFString)"));
    }
    let ptr = NonNull::new(raw.cast_mut())
        .ok_or_else(|| AppError::Audio("property returned a null CFString".into()))?;
    // SAFETY: this property is documented to return a +1 (owned) reference, so
    // adopting it here balances the retain; `CFRetained` releases on drop.
    let s = unsafe { CFRetained::from_raw(ptr) };
    Ok(s.to_string())
}

/// Sample rate and channel count of a device's input stream.
///
/// The tap's aggregate device delivers audio at the tapped device's native
/// format, so this has to be read rather than assumed — resampling from a
/// guessed rate produces audio that is subtly the wrong speed and pitch, which
/// transcribes into plausible nonsense rather than failing visibly.
pub fn input_stream_format(id: AudioObjectID) -> Result<(u32, u16)> {
    let addr = address(
        kAudioDevicePropertyStreamFormat,
        kAudioObjectPropertyScopeInput,
        kAudioObjectPropertyElementMain,
    );
    // `AudioStreamBasicDescription` has no `Default`, so it is read as bytes
    // and copied out once the length is known to cover a whole struct.
    let bytes = read_vec::<u8>(id, &addr)?;
    if bytes.len() < size_of::<AudioStreamBasicDescription>() {
        return Err(AppError::Audio(
            "device returned a short stream-format record".into(),
        ));
    }
    // SAFETY: `bytes` is at least one whole `AudioStreamBasicDescription`, and
    // `read_unaligned` makes no alignment assumption about the Vec's buffer.
    let f =
        unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<AudioStreamBasicDescription>()) };

    // `mSampleRate` is an f64 in Hz; rates are small positive integers in
    // practice, and a non-finite or absurd value means something is wrong.
    if !f.mSampleRate.is_finite() || f.mSampleRate < 1.0 || f.mSampleRate > 1_000_000.0 {
        return Err(AppError::Audio(format!(
            "device reported an implausible sample rate: {}",
            f.mSampleRate
        )));
    }
    let rate = exact_u32_from_f64(f.mSampleRate.round());
    let channels = u16::try_from(f.mChannelsPerFrame)
        .map_err(|_| AppError::Audio("channel count out of range".into()))?;
    if channels == 0 {
        return Err(AppError::Audio("device reported zero channels".into()));
    }
    Ok((rate, channels))
}

/// Convert a non-negative, in-range `f64` to `u32` without a lossy cast.
///
/// `as` would do this in one instruction, but it is `cast_possible_truncation`
/// and this crate does not suppress lints. Binary search over the `u32` range
/// uses only `u32 -> f64`, which is exact, and settles in 32 iterations — once
/// per tap, so the cost is irrelevant. The caller has already rejected
/// non-finite and out-of-range values.
///
/// Anything not finite and positive returns 0, deliberately: 0 is rejected by
/// every caller, whereas saturating a garbage value to `u32::MAX` would hand
/// the resampler a 4-billion-hertz rate and produce convincing noise.
fn exact_u32_from_f64(v: f64) -> u32 {
    if !v.is_finite() || v <= 0.0 {
        return 0;
    }
    if v >= f64::from(u32::MAX) {
        return u32::MAX;
    }
    let mut acc: u32 = 0;
    let mut step: u32 = 1 << 31;
    while step > 0 {
        let candidate = acc.saturating_add(step);
        if f64::from(candidate) <= v {
            acc = candidate;
        }
        step /= 2;
    }
    acc
}

/// One audio device as Core Audio sees it.
#[derive(Debug, Clone)]
pub struct HardwareDevice {
    pub uid: String,
    pub name: String,
}

/// Every device the system reports.
fn all_device_ids() -> Result<Vec<AudioObjectID>> {
    let addr = address(
        kAudioHardwarePropertyDevices,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    // `kAudioObjectSystemObject` is declared as `c_int`; the object id type is
    // `u32`. The constant is 1, so the conversion is exact.
    let system = AudioObjectID::try_from(kAudioObjectSystemObject)
        .map_err(|_| AppError::Audio("system object id out of range".into()))?;
    read_vec::<AudioObjectID>(system, &addr)
}

/// Whether a device has any channels on the given scope.
///
/// `kAudioDevicePropertyStreamConfiguration` returns an `AudioBufferList`, and
/// the buffer count is what distinguishes an output device from an input one.
/// The list is variable-length, so it is read as raw bytes and only the leading
/// `mNumberBuffers` is interpreted — reading the trailing buffer array would
/// mean indexing a flexible array member, which is not worth doing for a
/// yes/no question.
fn has_channels(id: AudioObjectID, scope: u32) -> bool {
    let addr = address(
        kAudioDevicePropertyStreamConfiguration,
        scope,
        kAudioObjectPropertyElementMain,
    );
    let Ok(size) = property_size(id, &addr) else {
        return false;
    };
    if (size as usize) < size_of::<u32>() {
        return false;
    }
    let Ok(bytes) = read_vec::<u8>(id, &addr) else {
        return false;
    };
    if bytes.len() < size_of::<AudioBufferList>() {
        return false;
    }
    // SAFETY: `bytes` holds at least a whole `AudioBufferList` header, which
    // begins with `mNumberBuffers`. `read_unaligned` is required rather than a
    // plain deref: a `Vec<u8>`'s buffer carries only 1-byte alignment, and
    // dereferencing an under-aligned `*const u32` is undefined behaviour.
    let number_buffers = unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<u32>()) };
    number_buffers > 0
}

fn device_uid(id: AudioObjectID) -> Result<String> {
    read_cfstring(
        id,
        &address(
            kAudioDevicePropertyDeviceUID,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        ),
    )
}

fn device_name(id: AudioObjectID) -> Result<String> {
    read_cfstring(
        id,
        &address(
            kAudioObjectPropertyName,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        ),
    )
}

/// Every device with output channels — the things this app can tap.
///
/// A device whose UID or name cannot be read is skipped rather than guessed at:
/// the UID is what the tap is scoped by, so a device without one cannot be
/// tapped at all, and a device without a name cannot be shown or attributed.
pub fn list_output_devices() -> Vec<HardwareDevice> {
    let ids = match all_device_ids() {
        Ok(ids) => ids,
        Err(e) => {
            log::error!("[coreaudio] enumerating devices failed: {e}");
            return Vec::new();
        }
    };

    let mut out = Vec::new();
    for id in ids {
        if !has_channels(id, kAudioObjectPropertyScopeOutput) {
            continue;
        }
        let (Ok(uid), Ok(name)) = (device_uid(id), device_name(id)) else {
            log::warn!("[coreaudio] skipping output device {id} with unreadable uid or name");
            continue;
        };
        out.push(HardwareDevice { uid, name });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_property_address_carries_all_three_fields() {
        let a = address(1, 2, 3);
        assert_eq!(a.mSelector, 1);
        assert_eq!(a.mScope, 2);
        assert_eq!(a.mElement, 3);
    }

    #[test]
    fn status_renders_the_code_and_its_four_character_reading() {
        // 'nope' as a big-endian four-character code.
        let code = i32::from_be_bytes(*b"nope");
        let e = status(code, "SomeCall");
        let text = e.to_string();
        assert!(text.contains("SomeCall failed"), "{text}");
        assert!(text.contains("nope"), "{text}");
    }

    #[test]
    fn status_renders_non_printable_codes_without_panicking() {
        let e = status(-10875, "SomeCall");
        assert!(e.to_string().contains("-10875"));
    }

    // Touches real Core Audio; ignored for the same reason as the cpal
    // enumeration test — it depends on the machine and can contend under
    // parallel test execution.
    #[test]
    fn exact_u32_conversion_matches_expected_rates() {
        for rate in [8_000_u32, 16_000, 22_050, 44_100, 48_000, 96_000, 192_000] {
            assert_eq!(exact_u32_from_f64(f64::from(rate)), rate);
        }
    }

    #[test]
    fn exact_u32_conversion_clamps_and_floors() {
        assert_eq!(exact_u32_from_f64(-1.0), 0);
        assert_eq!(exact_u32_from_f64(0.0), 0);
        assert_eq!(exact_u32_from_f64(f64::NAN), 0);
        // Non-finite is rejected rather than saturated — see the doc comment.
        assert_eq!(exact_u32_from_f64(f64::INFINITY), 0);
        assert_eq!(exact_u32_from_f64(f64::NEG_INFINITY), 0);
        // A finite value above the range still saturates.
        assert_eq!(exact_u32_from_f64(f64::from(u32::MAX) * 2.0), u32::MAX);
        assert_eq!(
            exact_u32_from_f64(44_099.6),
            44_099,
            "truncates, does not round"
        );
    }

    #[test]
    #[ignore = "enumerates real audio hardware"]
    fn output_devices_have_uids_and_names() {
        let devices = list_output_devices();
        println!("{} output device(s):", devices.len());
        for d in &devices {
            println!("  uid={:?} name={:?}", d.uid, d.name);
        }
        for d in devices {
            assert!(!d.uid.is_empty(), "every tappable device needs a UID");
            assert!(!d.name.is_empty());
        }
    }
}
