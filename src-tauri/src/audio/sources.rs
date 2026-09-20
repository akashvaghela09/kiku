//! Listing microphones the way the system sound settings do, on Linux.
//!
//! cpal enumerates ALSA PCM devices, which is the wrong layer on any modern desktop.
//! It reports plugin pseudo-devices, the null sink, and the same card several times
//! over, while being unable to see a Bluetooth headset at all - because the headset is
//! not an ALSA card, it is a PipeWire source.
//!
//! What the system panel shows, and what a person means by "my microphones", is the
//! set of PipeWire/PulseAudio **sources that are not monitors**. A monitor is a
//! loopback of an output, so it records whatever the speakers are playing; useful for
//! screen recording, never what someone choosing a microphone wants.
//!
//! Selecting one works through `PULSE_SOURCE`, which libpulse clients - including the
//! ALSA `pulse` plugin that cpal ends up talking to - honour when choosing a source.
//! Verified on this machine: setting it to the Bluetooth source bound the capture
//! stream to that source rather than the built-in microphone.

use std::process::Command;

/// A microphone as the sound settings would name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// PulseAudio source name, for example `bluez_input.74_05_1D_0E_54_63.0`.
    pub name: String,
    /// Human description, for example `OnePlus Bullets Wireless Z3`.
    pub description: String,
}

/// Every non-monitor source, or `None` when this is not a PulseAudio/PipeWire system.
///
/// `None` means "ask cpal instead" rather than "no microphones": a machine running
/// bare ALSA has no sources to list, and its cpal enumeration is the best available.
pub fn list() -> Option<Vec<Source>> {
    let output = Command::new("pactl")
        .args(["list", "sources"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse(&String::from_utf8_lossy(&output.stdout)))
}

/// Set the source a subsequent capture stream should use.
///
/// Process-wide, which is correct here: Kiku records one thing at a time, and the
/// value is read when a stream opens rather than when it is set.
pub fn select(name: Option<&str>) {
    match name {
        Some(name) if !name.is_empty() => std::env::set_var("PULSE_SOURCE", name),
        // Unset rather than empty: an empty value is not the same as "use the default".
        _ => std::env::remove_var("PULSE_SOURCE"),
    }
}

/// Pull the non-monitor sources out of `pactl list sources`.
///
/// Parsed defensively: unknown lines are ignored, and a block missing either field is
/// skipped rather than half-reported.
fn parse(text: &str) -> Vec<Source> {
    let mut found = Vec::new();
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut is_monitor = false;

    let flush = |name: &mut Option<String>,
                 description: &mut Option<String>,
                 is_monitor: &mut bool,
                 found: &mut Vec<Source>| {
        if let (Some(name), Some(description)) = (name.take(), description.take()) {
            if !*is_monitor {
                found.push(Source { name, description });
            }
        }
        *is_monitor = false;
    };

    for line in text.lines() {
        let trimmed = line.trim();

        // A new block starts at "Source #N"; whatever was being collected is complete.
        if trimmed.starts_with("Source #") {
            flush(&mut name, &mut description, &mut is_monitor, &mut found);
        } else if let Some(value) = trimmed.strip_prefix("Name: ") {
            name = Some(value.trim().to_owned());
        } else if let Some(value) = trimmed.strip_prefix("Description: ") {
            description = Some(value.trim().to_owned());
        } else if let Some(value) = trimmed.strip_prefix("Monitor of Sink: ") {
            // "n/a" means this is a real capture source rather than a loopback.
            is_monitor = value.trim() != "n/a";
        }
    }

    flush(&mut name, &mut description, &mut is_monitor, &mut found);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real output from the development machine, trimmed to the fields that matter.
    const SAMPLE: &str = r#"
Source #53
	Name: alsa_output.pci-0000_04_00.6.analog-stereo.monitor
	Description: Monitor of Family 17h/19h HD Audio Controller Analog Stereo
	Monitor of Sink: alsa_output.pci-0000_04_00.6.analog-stereo
Source #54
	Name: alsa_input.pci-0000_04_00.6.analog-stereo
	Description: Family 17h/19h HD Audio Controller Analog Stereo
	Monitor of Sink: n/a
Source #3888
	Name: bluez_input.74_05_1D_0E_54_63.0
	Description: OnePlus Bullets Wireless Z3
	Monitor of Sink: n/a
Source #3889
	Name: bluez_output.74_05_1D_0E_54_63.1.monitor
	Description: Monitor of OnePlus Bullets Wireless Z3
	Monitor of Sink: bluez_output.74_05_1D_0E_54_63.1
"#;

    #[test]
    fn it_finds_exactly_the_microphones_the_sound_settings_show() {
        let found = parse(SAMPLE);
        let names: Vec<&str> = found.iter().map(|s| s.description.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Family 17h/19h HD Audio Controller Analog Stereo",
                "OnePlus Bullets Wireless Z3",
            ]
        );
    }

    #[test]
    fn monitors_are_excluded_because_they_record_the_speakers() {
        let found = parse(SAMPLE);
        assert!(
            !found.iter().any(|s| s.name.ends_with(".monitor")),
            "a monitor is a loopback of an output, not a microphone"
        );
    }

    #[test]
    fn a_bluetooth_headset_is_included() {
        // The case cpal cannot see at all: not an ALSA card, so it never appears in
        // an ALSA enumeration.
        let found = parse(SAMPLE);
        assert!(found.iter().any(|s| s.name.starts_with("bluez_input")));
    }

    #[test]
    fn empty_or_unrecognised_output_yields_nothing_rather_than_guesses() {
        assert!(parse("").is_empty());
        assert!(parse("some unrelated text\nwith no fields").is_empty());
    }

    #[test]
    fn a_block_missing_a_description_is_skipped() {
        let partial = "Source #1\n\tName: only.a.name\n\tMonitor of Sink: n/a\n";
        assert!(parse(partial).is_empty());
    }

    #[test]
    fn selecting_nothing_clears_the_variable_rather_than_blanking_it() {
        select(Some("some.source"));
        assert_eq!(std::env::var("PULSE_SOURCE").as_deref(), Ok("some.source"));
        select(None);
        assert!(std::env::var("PULSE_SOURCE").is_err());
    }
}
