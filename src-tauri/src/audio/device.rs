//! Enumeration of the machine's capture devices.
//!
//! Inputs are enumerated through `cpal`; outputs through Core Audio directly,
//! because each one is captured by this app's own process tap ([`super::tap`]).
//!
//! [`ExclusionRegistry`] is what stops the app recording itself. Tapping an
//! output device requires an aggregate device, aggregates are ordinary
//! `AudioObjectID`s, and `cpal`'s macOS backend walks exactly that list. The
//! aggregates are created *private*, which hides them from other processes —
//! but not from this one, which is the process doing the enumerating. So each
//! tap registers its aggregate's UID here, and input enumeration filters on
//! each device's UID (cpal's `id()`) — not its name, which for an aggregate is
//! "Darric Tap — …" and never matches.

use super::coreaudio;
use crate::transcription::pool::Direction;
use cpal::traits::{DeviceTrait, HostTrait};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// A device the recorder can draw audio from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureDevice {
    /// For inputs, the device's name (see [`device_name`] for why not its UID);
    /// for outputs, its Core Audio UID.
    pub id: String,
    pub name: String,
    pub direction: Direction,
}

/// Devices this process created, which must never be enumerated as inputs.
///
/// Cloneable and shared: the engine hands a clone to each tap so it can register
/// its aggregate, and to the enumerator so it can filter them out. Both must see
/// the same set.
#[derive(Debug, Clone, Default)]
pub struct ExclusionRegistry {
    ids: Arc<Mutex<HashSet<String>>>,
}

impl ExclusionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, id: impl Into<String>) {
        self.ids
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id.into());
    }

    pub fn unregister(&self, id: &str) {
        self.ids
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(id);
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(id)
    }
}

/// Every output device that can be tapped.
///
/// The id is the Core Audio UID, which is what a tap is scoped by — unlike
/// inputs, where `cpal` exposes only a name.
pub fn list_output_devices(excluded: &ExclusionRegistry) -> Vec<CaptureDevice> {
    coreaudio::list_output_devices()
        .into_iter()
        .filter(|d| !excluded.contains(&d.uid))
        .map(|d| CaptureDevice {
            id: d.uid,
            name: d.name,
            direction: Direction::Output,
        })
        .collect()
}

/// The name a cpal device is known by: its id in this app, persisted in
/// `capture.disabled_devices` and on every transcript line.
///
/// This is cpal's human-readable description name, deliberately not its
/// stable `id()` (the Core Audio UID). The description name reads
/// `kAudioDevicePropertyDeviceNameCFString`, exactly as the pre-0.17 `name()`
/// did, so the strings are byte-identical across the cpal upgrade; switching
/// the identity to `id()` would silently orphan every device a user has
/// already disabled and every transcript line's `device_id`.
pub fn device_name(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|d| d.name().to_string())
}

/// Every input device the host reports, minus anything this process created.
///
/// A device whose name cannot be read is skipped rather than given a
/// placeholder: an unnameable device cannot be shown in the UI or matched back
/// to a transcript line, so a synthetic id would only create a phantom.
pub fn list_input_devices(excluded: &ExclusionRegistry) -> Vec<CaptureDevice> {
    let host = cpal::default_host();
    let Ok(devices) = host.input_devices() else {
        log::error!("[devices] host returned no input device iterator");
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for device in devices {
        let Some(name) = device_name(&device) else {
            log::warn!("[devices] skipping an input device with an unreadable name");
            continue;
        };
        if device.id().is_ok_and(|uid| excluded.contains(uid.id())) {
            log::debug!("[devices] skipping our own device {name}");
            continue;
        }
        // The host can list the same device twice (e.g. via two APIs); the first
        // wins so the id stays stable across refreshes.
        if !seen.insert(name.clone()) {
            continue;
        }
        out.push(CaptureDevice {
            id: name.clone(),
            name,
            direction: Direction::Input,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Touches real Core Audio. Ignored by default: it depends on the machine's
    // hardware and on microphone permission, and concurrent enumeration from
    // several test threads can block. Run explicitly with `-- --ignored`.
    #[test]
    fn registry_starts_empty_and_records_what_it_is_told() {
        let r = ExclusionRegistry::new();
        assert!(!r.contains("darric-tap-1"));
        r.register("darric-tap-1");
        assert!(r.contains("darric-tap-1"));
        r.unregister("darric-tap-1");
        assert!(!r.contains("darric-tap-1"));
    }

    #[test]
    fn registry_clones_share_state() {
        // The engine hands one clone to each tap and another to the enumerator.
        // If they did not share, the filter would silently stop working and the
        // app would record its own output.
        let a = ExclusionRegistry::new();
        let b = a.clone();
        a.register("darric-tap-2");
        assert!(b.contains("darric-tap-2"));
    }

    /// A tap's private aggregate is visible to this process's own enumeration,
    /// so the registry is the only thing keeping it off the microphone list.
    ///
    /// Ignored by default: it creates a real Core Audio tap, which needs the
    /// audio-capture permission on the terminal running it.
    #[test]
    #[ignore = "creates a real Core Audio tap; needs the audio-capture permission"]
    fn a_registered_tap_is_not_listed_as_a_microphone() {
        let registry = ExclusionRegistry::new();
        let Some(output) = list_output_devices(&registry).into_iter().next() else {
            panic!("no output device to tap on this machine");
        };
        let tap = super::super::tap::OutputTap::start(&output.id, &output.name, |_| {})
            .expect("start a tap (is audio capture permitted for this terminal?)");
        registry.register(tap.aggregate_uid());

        let inputs = list_input_devices(&registry);
        for d in &inputs {
            println!("input device while tapping: {:?}", d.name);
        }
        drop(tap);
        assert!(
            inputs.iter().all(|d| !d.name.starts_with("Darric Tap")),
            "the tap's own aggregate was listed as a microphone"
        );
    }

    #[test]
    #[ignore = "enumerates real audio hardware"]
    fn enumeration_yields_unique_stable_ids() {
        let devices = list_input_devices(&ExclusionRegistry::new());
        // Printed so a cpal upgrade can be checked for renamed devices: the
        // name is the id persisted in `capture.disabled_devices` and on every
        // transcript line, so a changed string silently orphans both.
        for d in &devices {
            println!("input device: {:?}", d.name);
        }
        let ids: HashSet<&str> = devices.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids.len(), devices.len(), "ids must be unique");
        assert!(devices.iter().all(|d| d.direction == Direction::Input));
    }
}
