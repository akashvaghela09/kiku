//! Short synthesised tones marking the start and end of listening.
//!
//! Generated rather than shipped as files: no licence to carry, no bytes in the
//! bundle, and the cues stay tunable in code.

mod player;
mod tone;

pub use tone::Cue;

/// Play a cue, unless the user has turned sounds off.
///
/// Returns immediately; a cue that cannot be played is logged, never surfaced, because
/// silent feedback is a far smaller problem than an error dialog mid-sentence.
pub fn play(cue: Cue, enabled: bool) {
    if enabled {
        player::play(cue);
    }
}
