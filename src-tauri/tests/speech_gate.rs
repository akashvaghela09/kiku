//! Replays the development machine's real microphone readings through the gate.
use kiku_lib::audio::SpeechLevel;

#[test]
fn the_measured_room_tone_of_this_machine_reads_as_silence() {
    // 0.169 RMS is what an empty room measured here - a very hot microphone.
    let mut gate = SpeechLevel::new(30.0);
    let mut last = 1.0;
    for _ in 0..90 {
        last = gate.observe(0.169);
    }
    println!(
        "ambient 0.169 -> level {last:.3}  (floor {:.1} dBFS)",
        gate.floor_db()
    );
    assert_eq!(last, 0.0);

    // Now speak over it.
    let speech = gate.observe(0.169 * 6.0);
    println!("speech  1.014 -> level {speech:.3}");
    assert!(speech > 0.2, "speech should register: {speech}");
}
