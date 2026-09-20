//! Plays each cue in turn so the audio path can be checked by listening — or, in CI,
//! by recording the output device. Ignored by default.
use kiku_lib::sound::{play, Cue};
use std::{thread::sleep, time::Duration};

#[test]
#[ignore = "produces sound"]
fn play_every_cue() {
    for cue in [Cue::Listening, Cue::Pasted, Cue::Error] {
        println!("playing {cue:?}");
        play(cue, true);
        sleep(Duration::from_millis(900));
    }
    sleep(Duration::from_millis(400));
}
