//! Enumeration of the machine's capture devices.
//!
//! Phase 2 enumerates inputs through `cpal`.
//!
//! Phase 3 must add an own-device exclusion filter here before it creates any
//! Core Audio tap: a tap needs an aggregate device, aggregates are ordinary
//! `AudioObjectID`s, and `cpal`'s macOS backend walks exactly that list — so
//! without a filter the app would enumerate its own taps as phantom inputs and
//! record its own output back into itself. That filter is deliberately NOT
//! written here yet; it would have no caller and no way to be exercised.

use crate::transcription::pool::Direction;
use cpal::traits::{DeviceTrait, HostTrait};
use std::collections::HashSet;

/// A device the recorder can draw audio from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureDevice {
    /// Stable within a run. `cpal` exposes no UID for inputs, so the name is the
    /// identity; phase 3's outputs will use their Core Audio UID.
    pub id: String,
    pub name: String,
    pub direction: Direction,
}

/// Every input device the host reports, minus anything this process created.
///
/// A device whose name cannot be read is skipped rather than given a
/// placeholder: an unnameable device cannot be shown in the UI or matched back
/// to a transcript line, so a synthetic id would only create a phantom.
pub fn list_input_devices() -> Vec<CaptureDevice> {
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
    #[ignore = "enumerates real audio hardware"]
    fn enumeration_yields_unique_stable_ids() {
        let devices = list_input_devices();
        let ids: HashSet<&str> = devices.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids.len(), devices.len(), "ids must be unique");
        assert!(devices.iter().all(|d| d.direction == Direction::Input));
    }
}
