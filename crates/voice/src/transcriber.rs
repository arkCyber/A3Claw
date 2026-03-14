//! Local Whisper transcription using `whisper-rs` (whisper.cpp GGML bindings).

use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Debug, thiserror::Error)]
pub enum TranscriberError {
    #[error("whisper context error: {0}")]
    Context(String),
    #[error("whisper inference error: {0}")]
    Inference(String),
    #[error("wav read error: {0}")]
    WavRead(String),
    #[error("no segments produced")]
    Empty,
}

pub struct Transcriber;

impl Transcriber {
    /// Transcribe a 16 kHz mono WAV file using a local GGML model.
    ///
    /// * `wav_path`   — path to the 16-bit PCM WAV file (16 kHz mono).
    /// * `model_path` — path to the GGML model file (e.g. `ggml-base.bin`).
    /// * `language`   — ISO-639-1 language code ("zh", "en", …) or "" for auto.
    pub fn transcribe_wav(
        wav_path: &Path,
        model_path: &Path,
        language: &str,
    ) -> Result<String, TranscriberError> {
        // Read WAV → f32 samples
        let samples = read_wav_f32(wav_path)?;

        // Load whisper model
        let ctx_params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().unwrap_or(""),
            ctx_params,
        )
        .map_err(|e| TranscriberError::Context(e.to_string()))?;

        let mut state = ctx
            .create_state()
            .map_err(|e| TranscriberError::Context(e.to_string()))?;

        // Build inference params
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(num_cpus());
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        if !language.is_empty() {
            params.set_language(Some(language));
        }

        // Run inference
        state
            .full(params, &samples)
            .map_err(|e| TranscriberError::Inference(e.to_string()))?;

        // Collect segment text
        let n = state
            .full_n_segments()
            .map_err(|e| TranscriberError::Inference(e.to_string()))?;

        if n == 0 {
            return Err(TranscriberError::Empty);
        }

        let mut result = String::new();
        for i in 0..n {
            let seg = state
                .full_get_segment_text(i)
                .map_err(|e| TranscriberError::Inference(e.to_string()))?;
            result.push_str(seg.trim());
            result.push(' ');
        }

        let text = result.trim().to_string();
        if text.is_empty() {
            return Err(TranscriberError::Empty);
        }

        tracing::info!("[voice] transcription: {}", text);
        Ok(text)
    }
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn read_wav_f32(path: &Path) -> Result<Vec<f32>, TranscriberError> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| TranscriberError::WavRead(e.to_string()))?;

    let spec = reader.spec();
    let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .map(|s| s.map_err(|e| TranscriberError::WavRead(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?,
        (hound::SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32)
                      .map_err(|e| TranscriberError::WavRead(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?,
        (hound::SampleFormat::Int, 32) => reader
            .samples::<i32>()
            .map(|s| s.map(|v| v as f32 / i32::MAX as f32)
                      .map_err(|e| TranscriberError::WavRead(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(TranscriberError::WavRead(format!(
                "unsupported WAV format: {:?} {}bit",
                spec.sample_format, spec.bits_per_sample
            )))
        }
    };

    Ok(samples)
}

fn num_cpus() -> i32 {
    // Use at most 4 threads to avoid starving the UI thread
    (std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2) as i32)
        .min(4)
}
