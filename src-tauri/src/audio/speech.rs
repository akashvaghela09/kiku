//! Turning a raw loudness reading into "is someone speaking right now".
//!
//! A fixed threshold cannot do this. Microphone gain varies enormously between
//! machines: on the development laptop an empty room measures −15 dBFS, where a fixed
//! −58 dBFS floor would map silence to 85% of full scale and speech to 92% - bars
//! pinned near the top, twitching on room noise, saying nothing.
//!
//! So the floor is learned instead of assumed. The quietest moment in the last few
//! seconds *is* the noise floor, whatever the hardware is doing, and a level is how
//! far above it the current moment sits. Speech has to clear that floor by a margin
//! before the waveform moves at all, which is what makes silence read as silence.
//!
//! Speech is full of short gaps - between words, inside stops - so even continuous
//! talking keeps refreshing the floor. That is why a rolling minimum works here and a
//! slow decay does not: a decaying floor gets dragged upward by sustained speech and
//! then suppresses the very thing it is measuring.

use std::collections::VecDeque;

/// How much louder than the noise floor a sound must be before it registers.
///
/// Below this the waveform stays flat. Room tone, a fan, and a keyboard all sit within
/// a few decibels of the floor; a voice does not. Six rather than nine, because nine
/// asked a quiet voice to shout before the meter acknowledged it at all, while still
/// leaving twice the headroom a chair creak or a fan spinning up occupies.
const MARGIN_DB: f32 = 6.0;

/// Decibels above the floor that count as a full-height bar.
///
/// Speaking normally spans perhaps 20dB above the floor, so asking for 24 meant an
/// ordinary voice never reached even three quarters of the range and the marks barely
/// moved.
const SPAN_DB: f32 = 20.0;

/// Silence, for the purposes of a logarithm.
const SILENCE_DB: f32 = -90.0;

/// Seconds of history the floor is taken from.
///
/// Long enough to span a sentence, so the floor is not dragged up by speech; short
/// enough to follow a room that gets noisier.
const WINDOW_SECONDS: f32 = 3.0;

/// Tracks the noise floor and reports how far above it the current audio is.
/// Readings needed before a floor means anything, as a fraction of a second.
///
/// A third of a second is imperceptible at the start of a recording and is long enough
/// for the floor to describe a room rather than one buffer.
const WARMUP_SECONDS: f32 = 0.33;

/// Readings the floor needs before a tenth of them is more than one.
///
/// Below this, [`SpeechLevel::floor_db`]'s percentile degenerates to the plain minimum
/// and a single freak-quiet buffer is the floor again - briefly, but a meter that
/// slams to full height for a fifth of a second at the start of every recording is
/// exactly the artefact being removed.
const FLOOR_MINIMUM_READINGS: usize = 11;

#[derive(Debug)]
pub struct SpeechLevel {
    recent: VecDeque<f32>,
    capacity: usize,
    warmup: usize,
}

impl SpeechLevel {
    /// `updates_per_second` is how often [`observe`](Self::observe) will be called.
    pub fn new(updates_per_second: f32) -> Self {
        let capacity = ((WINDOW_SECONDS * updates_per_second) as usize).max(4);
        Self {
            recent: VecDeque::with_capacity(capacity),
            capacity,
            warmup: ((WARMUP_SECONDS * updates_per_second) as usize).max(FLOOR_MINIMUM_READINGS),
        }
    }

    /// Feed one RMS reading and get back a 0..1 level for the waveform.
    pub fn observe(&mut self, rms: f32) -> f32 {
        let db = to_db(rms);

        // Digital silence is not a noise floor. A stream that has been opened but is
        // not yet delivering reports exact zeros, and no room is ever exactly zero.
        //
        // Letting one in pins the floor at SILENCE_DB for the entire window, and every
        // sound after it then sits 60 dB above the floor - full scale, clamped. That is
        // the waveform standing at maximum height for the first few seconds of a
        // recording and then abruptly becoming sensible: the floor had aged out of the
        // window.
        if db > SILENCE_DB {
            if self.recent.len() == self.capacity {
                self.recent.pop_front();
            }
            self.recent.push_back(db);
        }

        // Report silence rather than a guess until the room has been heard.
        if self.recent.len() < self.warmup {
            return 0.0;
        }

        let floor = self.floor_db();

        ((db - floor - MARGIN_DB) / SPAN_DB).clamp(0.0, 1.0)
    }

    /// The noise floor currently in use, in dBFS. Exposed for diagnostics.
    ///
    /// A low percentile rather than the outright minimum. The minimum is one sample,
    /// and the opening moments of a stream supply bad ones: a half-filled first buffer
    /// reads far quieter than the room ever is, becomes the floor, and puts ordinary
    /// room tone tens of decibels above it - full scale, clamped - for as long as it
    /// takes to age out of the window. That is the meter standing at full height for
    /// the first seconds of a recording.
    ///
    /// Taking the tenth percentile costs nothing in the steady state, where the quietest
    /// tenth of a window of room tone is room tone, and ignores the handful of freak
    /// samples that a minimum cannot survive.
    pub fn floor_db(&self) -> f32 {
        if self.recent.is_empty() {
            return SILENCE_DB;
        }

        let mut sorted: Vec<f32> = self.recent.iter().copied().collect();
        sorted.sort_by(f32::total_cmp);

        let index = (sorted.len() - 1) / 10;
        sorted[index].max(SILENCE_DB)
    }
}

fn to_db(rms: f32) -> f32 {
    if rms <= 0.0 {
        return SILENCE_DB;
    }
    (20.0 * rms.log10()).max(SILENCE_DB)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thirty updates a second is what the capture layer emits.
    fn mapper() -> SpeechLevel {
        SpeechLevel::new(30.0)
    }

    /// Feed a steady level for `seconds` and return the last reading.
    fn settle(mapper: &mut SpeechLevel, rms: f32, seconds: f32) -> f32 {
        let mut last = 0.0;
        for _ in 0..((seconds * 30.0) as usize) {
            last = mapper.observe(rms);
        }
        last
    }

    /// The waveform stood at full height for the first seconds of every recording.
    ///
    /// The microphone stream delivers a buffer of exact zeros before it is really
    /// running. That became the noise floor, and a floor of -90 dBFS puts ordinary room
    /// tone 60 dB above it - clamped to full scale - until it aged out of the window.
    #[test]
    fn a_silent_first_buffer_does_not_pin_the_meter_at_full_scale() {
        let mut mapper = mapper();

        for _ in 0..5 {
            mapper.observe(0.0);
        }

        let level = settle(&mut mapper, 0.05, 1.0);
        assert_eq!(
            level, 0.0,
            "room tone after a silent start must still read as silence"
        );
    }

    /// The half-filled opening buffer, which is not silent enough to be excluded as
    /// digital silence but is far quieter than the room.
    #[test]
    fn one_freak_quiet_buffer_does_not_become_the_floor() {
        let mut mapper = mapper();

        mapper.observe(0.00002); // about -94 dBFS, then clamped to the silence floor
        mapper.observe(0.0002); // about -74 dBFS: quiet, but a real reading

        // A room thirty decibels louder than that must read as silence throughout -
        // not eventually, once the bad sample has aged out.
        let mut highest: f32 = 0.0;
        for _ in 0..45 {
            highest = highest.max(mapper.observe(0.006));
        }

        assert_eq!(
            highest, 0.0,
            "a single freak-quiet buffer must not pin the meter open, even briefly"
        );
    }

    #[test]
    fn nothing_is_reported_until_the_room_has_been_heard() {
        let mut mapper = mapper();
        assert_eq!(mapper.observe(0.5), 0.0, "the first reading is not a floor");
    }

    #[test]
    fn steady_room_noise_reads_as_silence_however_loud_the_microphone_is() {
        // The bug this exists to fix: a hot mic whose "silence" is -15 dBFS must still
        // produce flat bars.
        for ambient in [0.169, 0.05, 0.005, 0.0005] {
            let mut mapper = mapper();
            let level = settle(&mut mapper, ambient, 2.0);
            assert_eq!(level, 0.0, "ambient {ambient} should read as silence");
        }
    }

    #[test]
    fn speech_over_a_hot_microphone_still_moves_the_bars() {
        let mut mapper = mapper();
        settle(&mut mapper, 0.169, 2.0); // the development laptop's room tone
        let level = mapper.observe(0.169 * 8.0); // ~18 dB above it
        assert!(level > 0.3, "speech should register, got {level}");
    }

    #[test]
    fn speech_over_a_quiet_microphone_moves_the_bars_the_same_amount() {
        // The whole point of a learned floor: the same *relative* loudness produces
        // the same bars regardless of gain.
        let mut hot = mapper();
        settle(&mut hot, 0.169, 2.0);
        let hot_level = hot.observe(0.169 * 8.0);

        let mut quiet = mapper();
        settle(&mut quiet, 0.002, 2.0);
        let quiet_level = quiet.observe(0.002 * 8.0);

        assert!(
            (hot_level - quiet_level).abs() < 0.05,
            "gain should not change the reading: {hot_level} vs {quiet_level}"
        );
    }

    #[test]
    fn louder_speech_gives_taller_bars() {
        let mut mapper = mapper();
        settle(&mut mapper, 0.005, 2.0);

        let quiet = mapper.observe(0.005 * 4.0);
        let loud = mapper.observe(0.005 * 40.0);
        assert!(loud > quiet, "{loud} should exceed {quiet}");
    }

    #[test]
    fn a_sound_barely_above_the_floor_is_ignored() {
        // A fan spinning up, a chair creaking: a few decibels, not a voice.
        let mut mapper = mapper();
        settle(&mut mapper, 0.005, 2.0);
        let level = mapper.observe(0.005 * 1.5); // ~3.5 dB
        assert_eq!(level, 0.0, "a small rise should not register as speech");
    }

    #[test]
    fn the_level_never_leaves_its_range() {
        let mut mapper = mapper();
        for rms in [0.0, 1e-9, 0.001, 0.5, 1.0, 40.0] {
            let level = mapper.observe(rms);
            assert!((0.0..=1.0).contains(&level), "{rms} produced {level}");
        }
    }

    #[test]
    fn digital_silence_does_not_produce_a_nan() {
        let mut mapper = mapper();
        let level = settle(&mut mapper, 0.0, 1.0);
        assert!(level.is_finite() && level == 0.0);
    }

    #[test]
    fn the_floor_follows_a_room_that_gets_quieter() {
        let mut mapper = mapper();
        settle(&mut mapper, 0.05, 4.0);
        let noisy_floor = mapper.floor_db();

        settle(&mut mapper, 0.001, 4.0);
        assert!(
            mapper.floor_db() < noisy_floor - 10.0,
            "the floor should drop with the room"
        );
    }

    /// A perfectly unvarying sound *is* a noise floor, and is meant to read as one.
    ///
    /// This is the central trade of the module, pinned so that nobody removes it by
    /// accident. Speech modulates by 15-25dB over syllable timescales and so keeps
    /// refreshing the floor through its own gaps; a held tone, a hum or a fan does not,
    /// so once the window fills with it the level falls away. The tempting "fix" -
    /// lengthening `WINDOW_SECONDS` - would regress the hot-microphone bug this module
    /// exists to solve. A rise-rate limit on the floor is the right change if one is
    /// ever needed.
    #[test]
    fn an_unvarying_tone_becomes_the_floor_once_it_fills_the_window() {
        let mut mapper = mapper();
        settle(&mut mapper, 0.005, 2.0);

        assert!(
            mapper.observe(0.005 * 20.0) > 0.5,
            "a new sound should register at first"
        );

        let after = settle(&mut mapper, 0.005 * 20.0, WINDOW_SECONDS + 0.5);
        assert_eq!(
            after, 0.0,
            "and become the floor once the window is full of it"
        );
    }

    /// The counterpart: a real voice keeps the meter alive for as long as it talks.
    #[test]
    fn modulated_speech_keeps_reading_for_as_long_as_it_lasts() {
        let mut mapper = mapper();
        settle(&mut mapper, 0.005, 2.0);

        // Eight seconds of syllables, with the short gaps that every voice has.
        let ticks = (8.0 * 30.0) as usize;
        let last_second = ticks - 30;
        let mut peak = 0.0f32;

        for tick in 0..ticks {
            let syllable = if tick % 7 < 2 {
                1.0
            } else {
                6.0 + 20.0 * ((tick % 5) as f32 / 4.0)
            };
            let level = mapper.observe(0.005 * syllable);
            if tick >= last_second {
                peak = peak.max(level);
            }
        }

        assert!(peak > 0.3, "the meter died during speech: {peak}");
    }

    #[test]
    fn sustained_speech_does_not_drag_the_floor_up_with_it() {
        // If the floor chased the signal, a long sentence would fade its own bars out.
        let mut mapper = mapper();
        settle(&mut mapper, 0.005, 2.0);

        let first = mapper.observe(0.005 * 20.0);
        let sustained = settle(&mut mapper, 0.005 * 20.0, 2.0);
        assert!(
            sustained > first * 0.6,
            "bars collapsed during sustained speech: {first} then {sustained}"
        );
    }
}
