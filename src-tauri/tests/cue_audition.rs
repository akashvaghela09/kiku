//! Plays both cues so the audio path can be checked by listening. Ignored by default.
use kiku_lib::feedback::{play, Cue};
use std::{thread::sleep, time::Duration};

#[test]
#[ignore = "produces sound"]
fn play_every_cue() {
    for cue in [Cue::Start, Cue::Stop] {
        println!("playing {cue:?}");
        play(cue, true);
        sleep(Duration::from_millis(900));
    }
    sleep(Duration::from_millis(400));
}
