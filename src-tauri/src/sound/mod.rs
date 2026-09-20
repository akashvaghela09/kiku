//! Short cues marking the start and end of a dictation.
//!
//! Two are recorded assets embedded in the binary; the rare failure cue is generated.
//! See `cue` for why.

mod cue;
mod player;
mod tone;

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
