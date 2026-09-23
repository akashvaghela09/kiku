//! The press-and-hold state machine for a single-key shortcut.
//!
//! A bare modifier such as Right Ctrl cannot be registered as a global shortcut - no
//! operating system accepts a modifier as a hotkey's main key. It has to be watched
//! instead, and watching a key that *also* still works as a modifier needs rules:
//!
//! * **Recording starts on key-down, not after a hold threshold.** Waiting 250 ms to
//!   decide whether a press is a hold would put 250 ms of latency in front of every
//!   dictation. Instead recording begins immediately and a press that turns out to be
//!   a tap is discarded, which costs nothing because the audio was never used.
//! * **Any other key cancels.** This is what makes a bare modifier safe: without it,
//!   Right Ctrl + C would copy *and* start dictating. Holding the key and pressing
//!   anything else means the user is using it as a modifier, so the recording is
//!   thrown away.
//! * **Two quick taps latch.** Hands-free recording without holding anything, ended by
//!   a third tap. Once latched, other keys no longer cancel - the whole point is to
//!   keep working while dictating.
//!
//! The machine is pure: it takes events and a clock and returns outcomes, so every
//! rule above is covered by tests rather than by hope.

use std::time::{Duration, Instant};

/// A press shorter than this was a tap, not a hold.
///
/// Measured against real taps on this machine, which ran 50-120ms: 250ms is ample,
/// and raising it would only reclassify a genuine short hold as a tap.
pub const DEFAULT_HOLD_THRESHOLD: Duration = Duration::from_millis(250);

/// A second tap within this long of the first latches hands-free recording.
///
/// Measured from the release of the first tap. Real double taps on this machine left
/// gaps of up to 350ms, which the previous 350ms window rejected on a boundary
/// comparison. 500ms is also what Windows has long used for a double click, and a
/// shortcut stricter than the desktop it runs on feels broken rather than precise.
pub const DEFAULT_DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(500);

/// Which gestures a watched key answers to.
///
/// One key can carry both, which is what a fresh install does. Binding them to
/// different keys is what makes each one answer to its own gesture and ignore the
/// other: holding the hands-free key should not record, and tapping the hold key twice
/// should not latch, or the two bindings would each quietly do the other's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gestures {
    /// Hold to record. Tapping does nothing.
    Hold,
    /// Double-tap to record hands-free. Holding does nothing.
    HandsFree,
    /// Both, on the one key.
    Both,
}

impl Gestures {
    fn holds(self) -> bool {
        matches!(self, Self::Hold | Self::Both)
    }

    fn latches(self) -> bool {
        matches!(self, Self::HandsFree | Self::Both)
    }
}

/// What the watcher observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// The watched key went down.
    Down,
    /// The watched key came up.
    Up,
    /// Some other key went down while the watched key was held.
    OtherKey,
    /// Time passed with nothing happening.
    Tick,
}

/// What should happen to the recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Begin recording.
    Start,
    /// Stop recording and transcribe it.
    Finish,
    /// Stop recording and throw the audio away.
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    /// Recording while the key is held.
    Holding {
        since: Instant,
    },
    /// A tap just ended; a second one within the window would latch.
    AwaitingSecondTap {
        since: Instant,
    },
    /// Recording hands-free until the next tap.
    Latched,
}

#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub hold: Duration,
    pub double_tap: Duration,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            hold: DEFAULT_HOLD_THRESHOLD,
            double_tap: DEFAULT_DOUBLE_TAP_WINDOW,
        }
    }
}

#[derive(Debug)]
pub struct TapMachine {
    state: State,
    thresholds: Thresholds,
    gestures: Gestures,
}

impl Default for TapMachine {
    fn default() -> Self {
        Self::new(Thresholds::default())
    }
}

impl TapMachine {
    pub fn new(thresholds: Thresholds) -> Self {
        Self::for_gestures(thresholds, Gestures::Both)
    }

    pub fn for_gestures(thresholds: Thresholds, gestures: Gestures) -> Self {
        Self {
            state: State::Idle,
            thresholds,
            gestures,
        }
    }

    /// Feed an observation, and get back what should happen to the recording.
    pub fn advance(&mut self, input: Input, now: Instant) -> Option<Outcome> {
        match (self.state, input) {
            // Start recording the instant the key goes down. A press that turns out
            // to be a tap is discarded below; this is what removes the hold delay.
            (State::Idle, Input::Down) => {
                self.state = State::Holding { since: now };
                // A key that only latches does not record until the second tap. Opening
                // the microphone for the first one would mean opening it twice for a
                // gesture the user experiences as single.
                self.gestures.holds().then_some(Outcome::Start)
            }

            (State::Holding { since }, Input::Up) => {
                let held = now.duration_since(since);

                if self.gestures.holds() && held >= self.thresholds.hold {
                    self.state = State::Idle;
                    return Some(Outcome::Finish);
                }

                if self.gestures.latches() {
                    // Too brief to be speech. Keep the door open for a second tap.
                    self.state = State::AwaitingSecondTap { since: now };
                    return self.gestures.holds().then_some(Outcome::Discard);
                }

                // Holds only: a tap is a mistap, with no second half to wait for.
                self.state = State::Idle;
                Some(Outcome::Discard)
            }

            // The cancel rule: the key is being used as a modifier, not as a hotkey.
            (State::Holding { .. }, Input::OtherKey) => {
                self.state = State::Idle;
                self.gestures.holds().then_some(Outcome::Discard)
            }

            (State::AwaitingSecondTap { .. }, Input::Down) => {
                self.state = State::Latched;
                Some(Outcome::Start)
            }

            (State::AwaitingSecondTap { since }, Input::Tick | Input::Up) => {
                if now.duration_since(since) >= self.thresholds.double_tap {
                    self.state = State::Idle;
                }
                None
            }

            // A tap ends hands-free recording. Other keys deliberately do not: the
            // point of latching is to keep using the computer while dictating.
            (State::Latched, Input::Down) => {
                self.state = State::Idle;
                Some(Outcome::Finish)
            }

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64) -> Instant {
        // A fixed origin plus an offset, so the tests are deterministic.
        Instant::now() + Duration::from_millis(ms)
    }

    #[test]
    fn a_hold_records_and_transcribes() {
        let mut machine = TapMachine::default();
        let start = Instant::now();

        assert_eq!(machine.advance(Input::Down, start), Some(Outcome::Start));
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(1200)),
            Some(Outcome::Finish)
        );
    }

    #[test]
    fn recording_begins_on_key_down_with_no_hold_delay() {
        // The whole reason the machine starts eagerly: a 250 ms wait before recording
        // would clip the first word of every dictation.
        let mut machine = TapMachine::default();
        assert_eq!(
            machine.advance(Input::Down, Instant::now()),
            Some(Outcome::Start)
        );
    }

    /// A key bound only to hands-free must not record while it is held, or it would be
    /// a second hold key and the two settings would collide.
    #[test]
    fn a_hands_free_key_does_nothing_when_held() {
        let mut machine = TapMachine::for_gestures(Thresholds::default(), Gestures::HandsFree);
        let start = Instant::now();

        assert_eq!(
            machine.advance(Input::Down, start),
            None,
            "no recording yet"
        );
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_secs(2)),
            None,
            "a long hold is not a gesture this key answers to"
        );
    }

    #[test]
    fn a_hands_free_key_latches_on_the_second_tap() {
        let mut machine = TapMachine::for_gestures(Thresholds::default(), Gestures::HandsFree);
        let start = Instant::now();

        machine.advance(Input::Down, start);
        machine.advance(Input::Up, start + Duration::from_millis(40));

        assert_eq!(
            machine.advance(Input::Down, start + Duration::from_millis(120)),
            Some(Outcome::Start),
            "the second tap starts hands-free recording"
        );
        assert_eq!(
            machine.advance(Input::Down, start + Duration::from_secs(5)),
            Some(Outcome::Finish),
            "and a later tap ends it"
        );
    }

    /// The mirror image: a key bound only to hold must not latch, or double-tapping it
    /// would start a recording the user cannot see a way out of.
    #[test]
    fn a_hold_key_does_not_latch_on_a_double_tap() {
        let mut machine = TapMachine::for_gestures(Thresholds::default(), Gestures::Hold);
        let start = Instant::now();

        assert_eq!(machine.advance(Input::Down, start), Some(Outcome::Start));
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(40)),
            Some(Outcome::Discard)
        );

        // A second tap inside the window starts an ordinary hold, not a latch.
        assert_eq!(
            machine.advance(Input::Down, start + Duration::from_millis(120)),
            Some(Outcome::Start)
        );
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_secs(2)),
            Some(Outcome::Finish),
            "releasing ends it, which a latch would not"
        );
    }

    #[test]
    fn a_brief_tap_is_discarded_rather_than_transcribed() {
        let mut machine = TapMachine::default();
        let start = Instant::now();

        machine.advance(Input::Down, start);
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(80)),
            Some(Outcome::Discard),
            "a tap is not an utterance"
        );
    }

    #[test]
    fn another_key_while_holding_cancels_the_recording() {
        // Right Ctrl + C must copy without also dictating. This rule is what makes a
        // bare modifier usable as a hotkey at all.
        let mut machine = TapMachine::default();
        let start = Instant::now();

        machine.advance(Input::Down, start);
        assert_eq!(
            machine.advance(Input::OtherKey, start + Duration::from_millis(40)),
            Some(Outcome::Discard)
        );

        // And the key coming up afterwards must not start anything.
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(90)),
            None
        );
    }

    #[test]
    fn two_quick_taps_latch_hands_free_recording() {
        let mut machine = TapMachine::default();
        let start = Instant::now();

        machine.advance(Input::Down, start);
        machine.advance(Input::Up, start + Duration::from_millis(80));
        assert_eq!(
            machine.advance(Input::Down, start + Duration::from_millis(200)),
            Some(Outcome::Start),
            "the second tap should latch"
        );
        // Latched: the key coming up must not end the recording.
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(260)),
            None
        );
    }

    #[test]
    fn a_third_tap_ends_hands_free_recording() {
        let mut machine = TapMachine::default();
        let start = Instant::now();

        machine.advance(Input::Down, start);
        machine.advance(Input::Up, start + Duration::from_millis(80));
        machine.advance(Input::Down, start + Duration::from_millis(200));
        machine.advance(Input::Up, start + Duration::from_millis(260));

        assert_eq!(
            machine.advance(Input::Down, start + Duration::from_millis(4000)),
            Some(Outcome::Finish)
        );
    }

    /// The timings this exists to fix: a deliberate double tap, not a fast one.
    #[test]
    fn an_unhurried_double_tap_still_latches() {
        let mut machine = TapMachine::default();
        let start = Instant::now();

        // 110ms on the key and 350ms between the presses: measured from real taps,
        // and a gap the previous 350ms window rejected on its boundary comparison.
        machine.advance(Input::Down, start);
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(110)),
            Some(Outcome::Discard)
        );
        assert_eq!(
            machine.advance(Input::Down, start + Duration::from_millis(460)),
            Some(Outcome::Start),
            "a second tap 350ms after release should still latch"
        );
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(560)),
            None,
            "releasing must not end a latched recording"
        );
    }

    #[test]
    fn a_second_tap_that_arrives_too_late_does_not_latch() {
        let mut machine = TapMachine::default();
        let start = Instant::now();

        machine.advance(Input::Down, start);
        machine.advance(Input::Up, start + Duration::from_millis(80));
        // Past the double-tap window: this is a fresh press, not a latch.
        machine.advance(Input::Tick, start + Duration::from_millis(900));

        assert_eq!(
            machine.advance(Input::Down, start + Duration::from_millis(1000)),
            Some(Outcome::Start)
        );
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(2400)),
            Some(Outcome::Finish),
            "it should behave as an ordinary hold, not as a latch"
        );
    }

    #[test]
    fn typing_while_latched_does_not_cancel() {
        // Hands-free is for dictating *while* doing something else.
        let mut machine = TapMachine::default();
        let start = Instant::now();

        machine.advance(Input::Down, start);
        machine.advance(Input::Up, start + Duration::from_millis(80));
        machine.advance(Input::Down, start + Duration::from_millis(200));

        assert_eq!(
            machine.advance(Input::OtherKey, at(300)),
            None,
            "other keys must not stop hands-free mode"
        );
        // Still recording: only a tap ends it.
        assert_eq!(
            machine.advance(Input::Down, at(4000)),
            Some(Outcome::Finish)
        );
    }

    #[test]
    fn key_repeat_while_held_changes_nothing() {
        let mut machine = TapMachine::default();
        let start = Instant::now();

        machine.advance(Input::Down, start);
        for offset in [10, 40, 80, 120] {
            assert_eq!(
                machine.advance(Input::Down, start + Duration::from_millis(offset)),
                None,
                "a repeat must not restart the recording"
            );
        }
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(900)),
            Some(Outcome::Finish)
        );
    }

    #[test]
    fn ticks_while_idle_do_nothing() {
        let mut machine = TapMachine::default();
        assert_eq!(machine.advance(Input::Tick, Instant::now()), None);
    }

    #[test]
    fn thresholds_are_configurable() {
        let mut machine = TapMachine::new(Thresholds {
            hold: Duration::from_millis(50),
            double_tap: Duration::from_millis(100),
        });
        let start = Instant::now();

        machine.advance(Input::Down, start);
        assert_eq!(
            machine.advance(Input::Up, start + Duration::from_millis(80)),
            Some(Outcome::Finish),
            "80ms should count as a hold once the threshold is 50ms"
        );
    }
}
