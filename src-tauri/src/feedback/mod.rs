//! Audio going **out**: two short cues marking when Kiku starts and stops listening.
//!
//! Both live in `assets/sounds/` and are embedded in the binary. Audio coming **in** —
//! the microphone — is `crate::audio`, which shares nothing with this but the cpal
//! dependency: opposite stream direction, a different sample rate, and a different
//! resampler, because aliasing matters for speech and not for a blip.
//!
//! There is deliberately no cue for a dictation that produced nothing. The overlay
//! already says so, and a failure chime is one more noise in a tool used dozens of
//! times a day.

mod cue;
mod player;

pub use cue::Cue;

/// Play a cue, unless the user has turned sounds off.
///
/// Returns immediately; a cue that cannot be played is logged and never surfaced,
/// because silent feedback is a far smaller problem than an error mid-sentence.
pub fn play(cue: Cue, enabled: bool) {
    if enabled {
        player::play(cue);
    }
}
