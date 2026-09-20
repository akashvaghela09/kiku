//! Microphone discovery and stream configuration.
//!
//! Most of what an audio host calls an "input device" is not one. On this development
//! machine ALSA reports twenty-one, of which exactly one is a microphone: eight are
//! plugin pseudo-devices with no usable configuration (rate converters, JACK, OSS,
//! channel up/downmix), one is the null device that discards everything, nine are the
//! same physical microphone reached through different ALSA paths, and the rest are
//! routes to the same sound server. Presenting that list to someone choosing a
//! microphone is useless, so it is filtered down to what can actually record.
//!
//! Devices are identified by `cpal::DeviceId`, which round-trips through a string and
//! survives replugging far better than a display name does. The name is carried
//! alongside purely for the Settings list.
//!
//! Where a device can give us 16 kHz mono directly we take it: that hands rate
//! conversion to the OS audio server, which does it well, and skips our resampler
//! entirely.

use std::str::FromStr;

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, DeviceId, SampleFormat, SupportedStreamConfig};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};

pub const TARGET_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneInfo {
    /// Persistable handle, from `DeviceId`'s `Display` implementation.
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Devices that exist to swallow audio rather than record it.
///
/// ALSA's `null` device reports itself as a perfectly good capture device with
/// hundreds of supported configurations, and records silence forever.
const DISCARD_MARKERS: [&str; 2] = ["discard all samples", "null device"];

/// Whether a device is worth offering as a microphone.
///
/// Two rules, both cheap. A device that supports no input configuration cannot record,
/// whatever it calls itself. And a device whose whole purpose is to discard audio is
/// not a microphone even though it will happily accept a stream.
fn is_usable_microphone(name: &str, input_config_count: usize) -> bool {
    if input_config_count == 0 {
        return false;
    }
    let lowered = name.to_lowercase();
    !DISCARD_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn describe(device: &Device) -> Option<(String, String)> {
    let id = device.id().ok()?.to_string();
    let name = device
        .description()
        .map(|description| description.name().to_owned())
        .unwrap_or_else(|_| "Unknown microphone".to_owned());
    Some((id, name))
}

/// Every input device the host exposes, default first.
pub fn list() -> Result<Vec<MicrophoneInfo>> {
    // On a PipeWire or PulseAudio desktop, ask the sound server. It knows about the
    // Bluetooth headset that never appears as an ALSA card, and it names things the
    // way the system settings do.
    #[cfg(target_os = "linux")]
    if let Some(sources) = super::sources::list() {
        if !sources.is_empty() {
            let default = default_source_name();
            return Ok(sources
                .into_iter()
                .map(|source| MicrophoneInfo {
                    is_default: Some(&source.name) == default.as_ref(),
                    id: source.name,
                    name: source.description,
                })
                .collect());
        }
    }

    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|device| device.id().ok())
        .map(|id| id.to_string());

    let devices = host
        .input_devices()
        .map_err(|e| Error::Microphone(e.to_string()))?;

    let mut found: Vec<MicrophoneInfo> = devices
        .filter_map(|device| {
            let (id, name) = describe(&device)?;
            let configs = device
                .supported_input_configs()
                .map(Iterator::count)
                .unwrap_or(0);
            is_usable_microphone(&name, configs).then_some(MicrophoneInfo {
                is_default: Some(&id) == default_id.as_ref(),
                id,
                name,
            })
        })
        .collect();

    // Default first, then alphabetical, so de-duplication keeps the default rather
    // than an arbitrary sibling.
    found.sort_by(|a, b| b.is_default.cmp(&a.is_default).then(a.name.cmp(&b.name)));

    // One entry per name. The same microphone is reachable through several device
    // paths, and nobody choosing from a list can tell those apart.
    let mut seen = std::collections::HashSet::new();
    found.retain(|microphone| seen.insert(microphone.name.clone()));

    Ok(found)
}

/// The sound server's current default source, if it has one.
#[cfg(target_os = "linux")]
fn default_source_name() -> Option<String> {
    let output = std::process::Command::new("pactl")
        .arg("get-default-source")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|name| !name.is_empty())
}

/// Resolve a saved device id, falling back to the system default.
///
/// A microphone that has since been unplugged must not stop dictation working, so an
/// unknown id degrades to the default rather than erroring.
pub fn resolve(preferred: Option<&str>) -> Result<Device> {
    let host = cpal::default_host();

    // A PipeWire source is selected by telling the sound server, not by asking cpal
    // for a device it cannot name. The chosen source applies to the next stream the
    // ALSA plugin opens, which is the one we are about to create.
    #[cfg(target_os = "linux")]
    if super::sources::list().is_some_and(|sources| !sources.is_empty()) {
        super::sources::select(preferred);
        return host.default_input_device().ok_or(Error::NoMicrophone);
    }

    if let Some(saved) = preferred {
        match DeviceId::from_str(saved) {
            Ok(id) => {
                if let Some(device) = host.device_by_id(&id) {
                    return Ok(device);
                }
                tracing::warn!(
                    device = saved,
                    "saved microphone not connected; using the default"
                );
            }
            Err(_) => tracing::warn!(device = saved, "saved microphone id is unreadable"),
        }
    }

    host.default_input_device().ok_or(Error::NoMicrophone)
}

/// Human-readable name for a device, for logs and history entries.
pub fn name_of(device: &Device) -> String {
    device
        .description()
        .map(|description| description.name().to_owned())
        .unwrap_or_else(|_| "Unknown microphone".to_owned())
}

/// Pick the stream configuration to open.
///
/// Preference order: 16 kHz at the fewest channels, then the device default. The first
/// avoids our resampler altogether.
pub fn best_config(device: &Device) -> Result<SupportedStreamConfig> {
    let supported: Vec<_> = device
        .supported_input_configs()
        .map_err(|e| Error::Microphone(e.to_string()))?
        .collect();

    let native_16k = supported
        .iter()
        .filter(|range| {
            range.min_sample_rate() <= TARGET_SAMPLE_RATE
                && range.max_sample_rate() >= TARGET_SAMPLE_RATE
                && matches!(
                    range.sample_format(),
                    SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16
                )
        })
        .min_by_key(|range| range.channels())
        .map(|range| range.with_sample_rate(TARGET_SAMPLE_RATE));

    if let Some(config) = native_16k {
        tracing::debug!(
            channels = config.channels(),
            "opening the microphone at 16 kHz natively"
        );
        return Ok(config);
    }

    device
        .default_input_config()
        .map_err(|e| Error::Microphone(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_with_no_input_configuration_is_not_a_microphone() {
        // ALSA's rate converters, JACK and OSS all report themselves as input devices
        // and support nothing at all.
        for name in [
            "Rate Converter Plugin Using Speex Resampler",
            "JACK Audio Connection Kit",
            "Open Sound System",
            "Plugin for channel upmix (4,6,8)",
        ] {
            assert!(!is_usable_microphone(name, 0), "{name} should be filtered");
        }
    }

    #[test]
    fn the_null_device_is_rejected_despite_looking_capable() {
        // It advertises 832 configurations and records silence forever.
        assert!(!is_usable_microphone(
            "Discard all samples (playback) or generate zero samples (capture)",
            832
        ));
    }

    #[test]
    fn a_real_microphone_is_kept() {
        assert!(is_usable_microphone("HD-Audio Generic, ALC294 Analog", 640));
        assert!(is_usable_microphone("PipeWire Sound Server", 320));
        assert!(is_usable_microphone("MacBook Pro Microphone", 4));
    }

    #[test]
    fn the_check_is_case_insensitive() {
        assert!(!is_usable_microphone("DISCARD ALL SAMPLES (capture)", 100));
    }
}
