//! Audio going **out**: the short cues marking the start and end of a dictation.
//!
//! All three live in `assets/sounds/` and are embedded in the binary. Audio coming
//! **in** — the microphone — is `crate::audio`, which shares nothing with this but the
//! cpal dependency: opposite stream direction, a different sample rate, and a
//! different resampler, because aliasing matters for speech and not for a blip.

mod cue;
mod player;
mod tone;

pub use cue::Cue;
pub use tone::SAMPLE_RATE;

/// Exposed so the one-off generator test can re-render `assets/sounds/error.wav`.
pub use tone::render_error as render_error_cue;

/// Play a cue, unless the user has turned sounds off.
///
/// Returns immediately; a cue that cannot be played is logged and never surfaced,
/// because silent feedback is a far smaller problem than an error mid-sentence.
pub fn play(cue: Cue, enabled: bool) {
    if enabled {
        player::play(cue);
    }
}
