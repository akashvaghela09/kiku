//! Keeps one recogniser loaded for the lifetime of the process.
//!
//! Loading the model costs about four seconds. Dictation has a sub-second budget, so
//! the model is loaded once in the background at startup and never unloaded. Everything
//! here exists to make that single warm instance safe to share and honest about which
//! state it is in - the UI must be able to say "still getting ready" rather than
//! appearing to ignore a hotkey.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::engine::{Engine, Transcript};
use super::parakeet::{ModelFiles, ParakeetEngine};
use crate::error::{Error, Result};

/// What the UI needs to know about the recogniser.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "state", content = "detail")]
pub enum EngineStatus {
    /// No model installed yet - onboarding has not finished.
    Unloaded,
    /// Loading into memory; a hotkey pressed now should say "getting ready".
    Loading,
    /// Ready to transcribe. Carries the model identifier.
    Ready(String),
    /// Loading failed. Carries a sentence fit to show the user.
    Failed(String),
}

enum State {
    Unloaded,
    Loading,
    Ready(Box<dyn Engine>),
    Failed(String),
}

impl State {
    fn status(&self) -> EngineStatus {
        match self {
            Self::Unloaded => EngineStatus::Unloaded,
            Self::Loading => EngineStatus::Loading,
            Self::Ready(engine) => EngineStatus::Ready(engine.model_id().to_owned()),
            Self::Failed(reason) => EngineStatus::Failed(reason.clone()),
        }
    }
}

pub struct AsrService {
    state: Mutex<State>,
}

impl Default for AsrService {
    fn default() -> Self {
        Self::new()
    }
}

impl AsrService {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::Unloaded),
        }
    }

    pub fn status(&self) -> EngineStatus {
        match self.state.lock() {
            Ok(state) => state.status(),
            // A poisoned lock means a decode panicked. Report it rather than panicking
            // again, so the user gets an error instead of a dead application.
            Err(_) => EngineStatus::Failed("The recogniser stopped unexpectedly.".into()),
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.status(), EngineStatus::Ready(_))
    }

    /// Load a model, replacing whatever is loaded now.
    ///
    /// Blocks for the duration of the load, so callers run it off the UI thread.
    pub fn load(&self, files: &ModelFiles, model_id: &str) -> Result<()> {
        // Dropping the previous engine frees several hundred megabytes and is not
        // instant, so it is timed separately from loading the new one.
        let releasing = std::time::Instant::now();
        self.set(State::Loading);
        tracing::debug!(
            released_ms = releasing.elapsed().as_millis() as u64,
            "released the previous model"
        );

        match ParakeetEngine::load(files, model_id) {
            Ok(engine) => {
                self.set(State::Ready(Box::new(engine)));
                Ok(())
            }
            Err(error) => {
                self.set(State::Failed(error.to_string()));
                Err(error)
            }
        }
    }

    /// Drop the loaded model, freeing its memory.
    ///
    /// Used when the model it came from is deleted: continuing to transcribe with an
    /// engine whose files no longer exist would work, but would report a model the
    /// user has just removed.
    pub fn unload(&self) {
        self.set(State::Unloaded);
    }

    /// Transcribe 16 kHz mono audio with the warm engine.
    pub fn transcribe(&self, samples: &[f32]) -> Result<Transcript> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| Error::Internal("The recogniser stopped unexpectedly.".into()))?;

        match &mut *state {
            State::Ready(engine) => engine.transcribe(samples),
            State::Loading => Err(Error::ModelLoad(
                "The speech model is still loading.".into(),
            )),
            State::Failed(reason) => Err(Error::ModelLoad(reason.clone())),
            State::Unloaded => Err(Error::ModelMissing),
        }
    }

    fn set(&self, next: State) {
        match self.state.lock() {
            Ok(mut state) => *state = next,
            Err(poisoned) => *poisoned.into_inner() = next,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_service_reports_unloaded() {
        let service = AsrService::new();
        assert!(matches!(service.status(), EngineStatus::Unloaded));
        assert!(!service.is_ready());
    }

    #[test]
    fn transcribing_without_a_model_asks_for_one_rather_than_failing_obscurely() {
        let service = AsrService::new();
        let error = service.transcribe(&[0.0; 16_000]).unwrap_err();
        assert!(matches!(error, Error::ModelMissing));
    }

    #[test]
    fn unloading_returns_the_service_to_asking_for_a_model() {
        let service = AsrService::new();
        service.set(State::Failed("something".into()));
        service.unload();
        assert!(matches!(service.status(), EngineStatus::Unloaded));
        assert!(matches!(
            service.transcribe(&[0.0; 16]).unwrap_err(),
            Error::ModelMissing
        ));
    }

    #[test]
    fn a_failed_load_keeps_reporting_its_reason() {
        let service = AsrService::new();
        service.set(State::Failed("encoder.onnx is corrupt".into()));

        match service.status() {
            EngineStatus::Failed(reason) => assert!(reason.contains("corrupt")),
            other => panic!("expected Failed, got {other:?}"),
        }
        assert!(service.transcribe(&[0.0; 16]).is_err());
    }
}
