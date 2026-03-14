//! openclaw-voice — pure-Rust audio recording + local Whisper transcription.
//!
//! # Usage
//! ```no_run
//! use openclaw_voice::{VoiceEngine, VoiceConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let engine = VoiceEngine::new(VoiceConfig::default()).await.unwrap();
//!     engine.start_recording().unwrap();
//!     // … wait …
//!     let text = engine.stop_and_transcribe().await.unwrap();
//!     println!("{text}");
//! }
//! ```

pub mod recorder;
pub mod transcriber;
pub mod model;

pub use recorder::{Recorder, RecorderError};
pub use transcriber::{Transcriber, TranscriberError};
pub use model::{ModelManager, ModelError, DEFAULT_MODEL_NAME};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Top-level configuration for the voice engine.
#[derive(Debug, Clone)]
pub struct VoiceConfig {
    /// Directory where Whisper GGML model files are stored.
    /// Defaults to `~/.openclaw-plus/models/`.
    pub model_dir: PathBuf,
    /// Which Whisper model to use (e.g. "ggml-base.bin").
    pub model_name: String,
    /// Target sample rate (Whisper requires 16 kHz).
    pub sample_rate: u32,
    /// Recording language hint (e.g. "zh", "en", or "" for auto-detect).
    pub language: String,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        let model_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".openclaw-plus")
            .join("models");
        Self {
            model_dir,
            model_name: DEFAULT_MODEL_NAME.to_string(),
            sample_rate: 16_000,
            language: "zh".to_string(),
        }
    }
}

/// Combined voice engine: manages recording state and transcription.
pub struct VoiceEngine {
    config: VoiceConfig,
    recorder: Arc<Mutex<Option<Recorder>>>,
}

impl VoiceEngine {
    /// Create a new engine. Does NOT download the model — call
    /// [`ModelManager::ensure_model`] first if needed.
    pub async fn new(config: VoiceConfig) -> Result<Self, VoiceEngineError> {
        std::fs::create_dir_all(&config.model_dir)
            .map_err(|e| VoiceEngineError::Io(e.to_string()))?;
        Ok(Self {
            config,
            recorder: Arc::new(Mutex::new(None)),
        })
    }

    /// Start capturing audio from the default input device.
    pub fn start_recording(&self) -> Result<(), VoiceEngineError> {
        let mut guard = self.recorder.lock().unwrap();
        if guard.is_some() {
            return Err(VoiceEngineError::AlreadyRecording);
        }
        let rec = Recorder::start(self.config.sample_rate)
            .map_err(|e| VoiceEngineError::Recorder(e.to_string()))?;
        *guard = Some(rec);
        Ok(())
    }

    /// Stop recording, write a temporary WAV file, transcribe it, and return the text.
    pub async fn stop_and_transcribe(&self) -> Result<String, VoiceEngineError> {
        let samples = {
            let mut guard = self.recorder.lock().unwrap();
            let rec = guard.take().ok_or(VoiceEngineError::NotRecording)?;
            rec.stop()
                .map_err(|e| VoiceEngineError::Recorder(e.to_string()))?
        };

        if samples.is_empty() {
            return Err(VoiceEngineError::NoAudio);
        }

        // Write WAV to a temp file
        let wav_path = std::env::temp_dir().join("openclaw_voice.wav");
        recorder::write_wav(&wav_path, &samples, self.config.sample_rate)
            .map_err(|e| VoiceEngineError::Recorder(e.to_string()))?;

        // Transcribe
        let model_path = self.config.model_dir.join(&self.config.model_name);
        if !model_path.exists() {
            return Err(VoiceEngineError::ModelNotFound(model_path.display().to_string()));
        }

        let lang = self.config.language.clone();
        let text = tokio::task::spawn_blocking(move || {
            Transcriber::transcribe_wav(&wav_path, &model_path, &lang)
        })
        .await
        .map_err(|e| VoiceEngineError::Transcriber(e.to_string()))?
        .map_err(|e| VoiceEngineError::Transcriber(e.to_string()))?;

        Ok(text)
    }

    /// Returns `true` if recording is in progress.
    pub fn is_recording(&self) -> bool {
        self.recorder.lock().unwrap().is_some()
    }

    /// Get a reference to the model manager for this engine's config.
    pub fn model_manager(&self) -> ModelManager {
        ModelManager::new(self.config.model_dir.clone())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceEngineError {
    #[error("already recording")]
    AlreadyRecording,
    #[error("not recording")]
    NotRecording,
    #[error("no audio captured")]
    NoAudio,
    #[error("model not found at {0} — run ModelManager::ensure_model() first")]
    ModelNotFound(String),
    #[error("recorder error: {0}")]
    Recorder(String),
    #[error("transcriber error: {0}")]
    Transcriber(String),
    #[error("io error: {0}")]
    Io(String),
}
