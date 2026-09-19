//! The dictation session — what actually happens between pressing the key and getting
//! text back.
//!
//! One session at a time, deliberately. Two overlapping recordings would compete for
//! the microphone and produce text in an order the user never intended, so a start
//! while already listening is ignored rather than queued.
//!
//! The state machine is small on purpose:
//!
//! ```text
//!   Idle ──start──▶ Listening ──stop──▶ Processing ──▶ Idle
//!     ▲                  │                               │
//!     └───────── cancel ─┘                               │
//!     └───────────────────────────────────────────────── ┘
//! ```

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::asr::{AsrService, Transcript};
use crate::audio::{Capture, Level, Recording};
use crate::error::{Error, Result};

/// How often audio levels are published to the overlay.
///
/// The audio callback fires far more often than this — every few milliseconds — and
/// forwarding each one would flood the IPC channel for a waveform that redraws at
/// screen rate anyway. Thirty a second is smooth to the eye and cheap; sixty doubles
/// the traffic during transcription for no visible gain.
pub const LEVEL_INTERVAL: Duration = Duration::from_millis(33);

/// Recordings shorter than this are treated as a mistap rather than speech.
const MIN_UTTERANCE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum DictationState {
    Idle,
    Listening,
    Processing,
}

/// Why a session produced no text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Discarded {
    /// Released the key almost immediately — a mistap, not an utterance.
    TooShort,
    /// The microphone captured effectively nothing.
    Silent,
    /// The recogniser heard audio but produced no words.
    NoSpeech,
}

/// What a finished session yielded.
///
/// `Debug` is hand-written so a transcript never ends up in a log line — dictated text
/// is the most private thing this application handles.
pub enum Outcome {
    Transcribed(Box<Transcript>),
    Discarded(Discarded),
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transcribed(transcript) => f
                .debug_struct("Transcribed")
                .field("chars", &transcript.text.len())
                .finish(),
            Self::Discarded(reason) => f.debug_tuple("Discarded").field(reason).finish(),
        }
    }
}

enum Session {
    Idle,
    Listening { capture: Capture, started: Instant },
    Processing,
}

pub struct Dictation {
    session: Mutex<Session>,
}

impl Default for Dictation {
    fn default() -> Self {
        Self::new()
    }
}

impl Dictation {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(Session::Idle),
        }
    }

    pub fn state(&self) -> DictationState {
        match self.session.lock().as_deref() {
            Ok(Session::Listening { .. }) => DictationState::Listening,
            Ok(Session::Processing) => DictationState::Processing,
            _ => DictationState::Idle,
        }
    }

    pub fn is_listening(&self) -> bool {
        self.state() == DictationState::Listening
    }

    /// Begin recording.
    ///
    /// `on_level` is called at most every [`LEVEL_INTERVAL`], already throttled, so
    /// callers can forward straight to the overlay without rate-limiting again.
    pub fn start(
        &self,
        microphone: Option<String>,
        on_level: impl Fn(Level) + Send + 'static,
    ) -> Result<()> {
        let mut session = self.lock()?;

        if !matches!(*session, Session::Idle) {
            // Not an error: a second press while already listening is a user pressing
            // twice, and the right response is to carry on recording.
            tracing::debug!("start ignored — a session is already running");
            return Ok(());
        }

        let capture = Capture::start(microphone, throttled(on_level))?;
        *session = Session::Listening {
            capture,
            started: Instant::now(),
        };
        Ok(())
    }

    /// Stop recording and transcribe what was captured.
    ///
    /// Blocking: the decode takes a few hundred milliseconds, so callers run this off
    /// the UI thread.
    pub fn stop(&self, asr: &AsrService) -> Result<Outcome> {
        let (capture, started) = {
            let mut session = self.lock()?;
            match std::mem::replace(&mut *session, Session::Processing) {
                Session::Listening { capture, started } => (capture, started),
                other => {
                    // Put back whatever we found; stopping when not listening is a
                    // no-op, not a failure.
                    *session = other;
                    return Ok(Outcome::Discarded(Discarded::NoSpeech));
                }
            }
        };

        let outcome = self.finish(capture, started, asr);
        *self.lock()? = Session::Idle;
        outcome
    }

    /// Abandon a recording without transcribing it.
    pub fn cancel(&self) -> Result<()> {
        let mut session = self.lock()?;
        if let Session::Listening { capture, .. } = std::mem::replace(&mut *session, Session::Idle)
        {
            // Dropping the capture stops the stream; the samples go nowhere.
            let _ = capture.stop();
            tracing::debug!("dictation cancelled");
        }
        Ok(())
    }

    fn finish(&self, capture: Capture, started: Instant, asr: &AsrService) -> Result<Outcome> {
        let held = started.elapsed();
        let recording = capture.stop()?;

        if held < MIN_UTTERANCE {
            tracing::debug!(?held, "discarded — too short to be speech");
            return Ok(Outcome::Discarded(Discarded::TooShort));
        }

        if recording.is_silent() {
            tracing::debug!("discarded — the microphone captured silence");
            return Ok(Outcome::Discarded(Discarded::Silent));
        }

        if recording.clipped {
            tracing::warn!("input clipped; the microphone gain is too high");
        }
        if recording.truncated {
            tracing::warn!("recording hit the maximum length and was cut short");
        }

        Ok(transcribe(&recording, asr))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Session>> {
        self.session
            .lock()
            .map_err(|_| Error::Internal("The dictation session stopped unexpectedly.".into()))
    }
}

fn transcribe(recording: &Recording, asr: &AsrService) -> Outcome {
    match asr.transcribe(&recording.samples) {
        Ok(transcript) if transcript.is_empty() => Outcome::Discarded(Discarded::NoSpeech),
        Ok(transcript) => {
            tracing::info!(
                audio_ms = transcript.audio_ms,
                decode_ms = transcript.decode_ms,
                "dictation transcribed"
            );
            Outcome::Transcribed(Box::new(transcript))
        }
        Err(error) => {
            tracing::error!(%error, "transcription failed");
            Outcome::Discarded(Discarded::NoSpeech)
        }
    }
}

/// Wrap a level callback so it fires at most once per [`LEVEL_INTERVAL`].
///
/// The first level always passes through, so the waveform starts moving the moment
/// recording begins rather than up to 70 ms later.
fn throttled(inner: impl Fn(Level) + Send + 'static) -> impl Fn(Level) + Send + 'static {
    let last = Mutex::new(None::<Instant>);

    move |level| {
        let mut last = match last.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let now = Instant::now();
        let due = last.is_none_or(|previous| now.duration_since(previous) >= LEVEL_INTERVAL);
        if due {
            *last = Some(now);
            drop(last);
            inner(level);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn a_new_session_is_idle() {
        let dictation = Dictation::new();
        assert_eq!(dictation.state(), DictationState::Idle);
        assert!(!dictation.is_listening());
    }

    #[test]
    fn stopping_when_idle_is_harmless() {
        let dictation = Dictation::new();
        let asr = AsrService::new();
        match dictation.stop(&asr).unwrap() {
            Outcome::Discarded(Discarded::NoSpeech) => {}
            other => panic!("expected a discard, got {other:?}"),
        }
        assert_eq!(dictation.state(), DictationState::Idle);
    }

    #[test]
    fn cancelling_when_idle_is_harmless() {
        let dictation = Dictation::new();
        assert!(dictation.cancel().is_ok());
    }

    #[test]
    fn the_first_level_is_published_immediately() {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        let publish = throttled(move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
        });

        publish(Level::default());
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "the waveform must start at once"
        );
    }

    #[test]
    fn a_burst_of_levels_is_collapsed_to_one() {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        let publish = throttled(move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
        });

        // Stands in for the audio callback firing every few milliseconds.
        for _ in 0..500 {
            publish(Level::default());
        }

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "a burst inside one interval must publish once, not 500 times"
        );
    }

    #[test]
    fn levels_resume_after_the_interval_elapses() {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        let publish = throttled(move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
        });

        publish(Level::default());
        std::thread::sleep(LEVEL_INTERVAL + Duration::from_millis(15));
        publish(Level::default());

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
