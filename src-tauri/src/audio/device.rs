//! Enumeration of the machine's capture devices.
//!
//! Inputs are enumerated through `cpal`; outputs through Core Audio, since
//! `cpal` has no notion of capturing them.
//!
//! [`ExclusionRegistry`] is what stops the app recording itself. Tapping an
//! output device requires an aggregate device, aggregates are ordinary
//! `AudioObjectID`s, and `cpal`'s macOS backend walks exactly that list. The
//! aggregates are created *private*, which hides them from other processes —
//! but not from this one, which is the process doing the enumerating. So each
//! tap registers its aggregate's UID here and input enumeration filters it out.

use super::coreaudio;
use crate::transcription::pool::Direction;
use cpal::traits::{DeviceTrait, HostTrait};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// A device the recorder can draw audio from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureDevice {
    /// Stable within a run. `cpal` exposes no UID for inputs, so the name is the
    /// identity; phase 3's outputs will use their Core Audio UID.
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
        let Ok(name) = device.name() else {
            log::warn!("[devices] skipping an input device with an unreadable name");
            continue;
        };
        if excluded.contains(&name) {
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

    #[test]
    #[ignore = "enumerates real audio hardware"]
    fn enumeration_yields_unique_stable_ids() {
        let devices = list_input_devices(&ExclusionRegistry::new());
        let ids: HashSet<&str> = devices.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids.len(), devices.len(), "ids must be unique");
        assert!(devices.iter().all(|d| d.direction == Direction::Input));
    }
}
