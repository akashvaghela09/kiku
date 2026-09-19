//! Microphone discovery and stream configuration.
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
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|device| device.id().ok())
        .map(|id| id.to_string());

    let devices = host
        .input_devices()
        .map_err(|e| Error::Microphone(e.to_string()))?;

    let mut found: Vec<MicrophoneInfo> = devices
        .filter_map(|device| describe(&device))
        .map(|(id, name)| {
            let is_default = Some(&id) == default_id.as_ref();
            MicrophoneInfo {
                id,
                name,
                is_default,
            }
        })
        .collect();

    found.sort_by(|a, b| b.is_default.cmp(&a.is_default).then(a.name.cmp(&b.name)));
    found.dedup_by(|a, b| a.id == b.id);
    Ok(found)
}

/// Resolve a saved device id, falling back to the system default.
///
/// A microphone that has since been unplugged must not stop dictation working, so an
/// unknown id degrades to the default rather than erroring.
pub fn resolve(preferred: Option<&str>) -> Result<Device> {
    let host = cpal::default_host();

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
