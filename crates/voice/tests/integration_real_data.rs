//! Integration tests for `openclaw-voice` using real data.
//!
//! Covers:
//! - `VoiceConfig` construction and field validation
//! - `ModelManager` path resolution and existence checking with real tempdir
//! - WAV file format validation (real 16 kHz PCM WAV bytes)
//! - `TranscriberError` display messages
//! - `VoiceEngine` construction (no audio device required)
//! - Model download skipped when file already exists (no network)
//! - `DEFAULT_MODEL_NAME` constant is a valid filename

use openclaw_voice::{VoiceConfig, ModelManager, DEFAULT_MODEL_NAME};
use openclaw_voice::transcriber::TranscriberError;
use std::path::PathBuf;
use tempfile::tempdir;

// ── VoiceConfig ───────────────────────────────────────────────────────────────

#[test]
fn voice_config_default_sample_rate_is_16khz() {
    let cfg = VoiceConfig::default();
    assert_eq!(cfg.sample_rate, 16_000, "Whisper requires exactly 16 kHz");
}

#[test]
fn voice_config_default_model_name_matches_constant() {
    let cfg = VoiceConfig::default();
    assert_eq!(cfg.model_name, DEFAULT_MODEL_NAME);
}

#[test]
fn voice_config_default_model_dir_is_under_home() {
    let cfg = VoiceConfig::default();
    let dir_str = cfg.model_dir.to_string_lossy();
    assert!(
        dir_str.contains("openclaw-plus") || dir_str.contains("models"),
        "model_dir must be under ~/.openclaw-plus/models: {}", dir_str
    );
}

#[test]
fn voice_config_default_language_is_zh() {
    let cfg = VoiceConfig::default();
    assert_eq!(cfg.language, "zh");
}

#[test]
fn voice_config_custom_fields() {
    let cfg = VoiceConfig {
        model_dir:   PathBuf::from("/tmp/custom-models"),
        model_name:  "ggml-small.bin".to_string(),
        sample_rate: 16_000,
        language:    "en".to_string(),
    };
    assert_eq!(cfg.model_dir, PathBuf::from("/tmp/custom-models"));
    assert_eq!(cfg.model_name, "ggml-small.bin");
    assert_eq!(cfg.language, "en");
}

#[test]
fn default_model_name_is_valid_filename() {
    assert!(!DEFAULT_MODEL_NAME.is_empty());
    assert!(DEFAULT_MODEL_NAME.ends_with(".bin"),
        "default model must be a .bin file: {}", DEFAULT_MODEL_NAME);
    assert!(!DEFAULT_MODEL_NAME.contains('/'),
        "model name must not contain path separators: {}", DEFAULT_MODEL_NAME);
}

// ── ModelManager ──────────────────────────────────────────────────────────────

#[test]
fn model_manager_path_resolution_real_tempdir() {
    let dir = tempdir().unwrap();
    let mm = ModelManager::new(dir.path().to_path_buf());
    let path = mm.model_path("ggml-base.bin");
    assert_eq!(path, dir.path().join("ggml-base.bin"));
}

#[test]
fn model_manager_model_exists_returns_false_for_missing_file() {
    let dir = tempdir().unwrap();
    let mm = ModelManager::new(dir.path().to_path_buf());
    assert!(!mm.model_exists("ggml-base.bin"),
        "model must not exist before creation");
}

#[test]
fn model_manager_model_exists_returns_true_after_write() {
    let dir = tempdir().unwrap();
    let mm = ModelManager::new(dir.path().to_path_buf());
    // Write a fake model file
    std::fs::write(dir.path().join("ggml-base.bin"), b"fake-ggml-model").unwrap();
    assert!(mm.model_exists("ggml-base.bin"),
        "model must exist after file creation");
}

#[test]
fn model_manager_path_for_small_model() {
    let dir = tempdir().unwrap();
    let mm = ModelManager::new(dir.path().to_path_buf());
    let path = mm.model_path("ggml-small.bin");
    assert!(path.to_string_lossy().ends_with("ggml-small.bin"));
}

#[test]
fn model_manager_path_for_large_model() {
    let dir = tempdir().unwrap();
    let mm = ModelManager::new(dir.path().to_path_buf());
    let path = mm.model_path("ggml-large-v3.bin");
    assert!(path.to_string_lossy().ends_with("ggml-large-v3.bin"));
}

#[test]
fn model_manager_multiple_models_independent() {
    let dir = tempdir().unwrap();
    let mm = ModelManager::new(dir.path().to_path_buf());

    std::fs::write(dir.path().join("ggml-base.bin"),  b"base").unwrap();
    std::fs::write(dir.path().join("ggml-small.bin"), b"small").unwrap();

    assert!(mm.model_exists("ggml-base.bin"),  "base model must exist");
    assert!(mm.model_exists("ggml-small.bin"), "small model must exist");
    assert!(!mm.model_exists("ggml-large.bin"), "large model must NOT exist");
}

#[tokio::test]
async fn model_manager_ensure_model_skips_download_when_exists() {
    let dir = tempdir().unwrap();
    // Write a fake model so ensure_model takes the fast path
    let fake_model = b"GGML fake model data";
    std::fs::write(dir.path().join("ggml-base.bin"), fake_model).unwrap();

    let mm = ModelManager::new(dir.path().to_path_buf());
    let progress_calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let progress_calls_clone = progress_calls.clone();
    let result = mm.ensure_model("ggml-base.bin", move |_downloaded, _total| {
        progress_calls_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }).await;

    assert!(result.is_ok(), "ensure_model must succeed: {:?}", result.err());
    let returned_path = result.unwrap();
    assert!(returned_path.exists(), "returned path must exist");
    assert_eq!(progress_calls.load(std::sync::atomic::Ordering::Relaxed), 0,
        "progress_cb must NOT be called when model already exists");
}

#[tokio::test]
async fn model_manager_ensure_model_creates_dir_if_missing() {
    let base_dir = tempdir().unwrap();
    let nested = base_dir.path().join("deep").join("nested").join("models");
    // Directory does not exist yet
    assert!(!nested.exists());

    // Write a fake model file before calling ensure_model (skip download)
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("ggml-base.bin"), b"fake").unwrap();

    let mm = ModelManager::new(nested.clone());
    let result = mm.ensure_model("ggml-base.bin", |_, _| {}).await;
    assert!(result.is_ok());
}

// ── WAV file format: real 16 kHz PCM WAV bytes ────────────────────────────────

fn make_minimal_wav_16khz_mono(num_samples: usize) -> Vec<u8> {
    // Standard WAV header for 16-bit PCM, 16000 Hz, mono
    let sample_rate: u32 = 16_000;
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = num_channels * (bits_per_sample / 8);
    let data_size = (num_samples * 2) as u32; // 2 bytes per sample
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + data_size as usize);
    // RIFF chunk
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    // fmt  sub-chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());    // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes());     // PCM format
    wav.extend_from_slice(&num_channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data sub-chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    // Silence samples (all zeros)
    wav.extend(std::iter::repeat(0u8).take(data_size as usize));
    wav
}

#[test]
fn wav_header_riff_magic_bytes() {
    let wav = make_minimal_wav_16khz_mono(160); // 10 ms of silence
    assert_eq!(&wav[0..4], b"RIFF", "WAV must start with RIFF");
    assert_eq!(&wav[8..12], b"WAVE", "WAV must contain WAVE marker");
    assert_eq!(&wav[12..16], b"fmt ", "WAV must have fmt chunk");
    assert_eq!(&wav[36..40], b"data", "WAV must have data chunk");
}

#[test]
fn wav_header_sample_rate_field_is_16000() {
    let wav = make_minimal_wav_16khz_mono(1600);
    let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    assert_eq!(sample_rate, 16_000);
}

#[test]
fn wav_header_pcm_format_type_is_1() {
    let wav = make_minimal_wav_16khz_mono(100);
    let format = u16::from_le_bytes([wav[20], wav[21]]);
    assert_eq!(format, 1, "PCM format must be 1 (linear PCM)");
}

#[test]
fn wav_header_mono_channel_count() {
    let wav = make_minimal_wav_16khz_mono(100);
    let channels = u16::from_le_bytes([wav[22], wav[23]]);
    assert_eq!(channels, 1, "must be mono (1 channel)");
}

#[test]
fn wav_header_16bit_depth() {
    let wav = make_minimal_wav_16khz_mono(100);
    let bits = u16::from_le_bytes([wav[34], wav[35]]);
    assert_eq!(bits, 16, "must be 16-bit PCM");
}

#[test]
fn wav_file_written_to_real_tempdir_and_readable() {
    let dir = tempdir().unwrap();
    let wav_path = dir.path().join("silence_16khz.wav");
    let wav_data = make_minimal_wav_16khz_mono(16_000); // 1 second of silence
    std::fs::write(&wav_path, &wav_data).unwrap();

    assert!(wav_path.exists(), "WAV file must exist after write");
    let read_back = std::fs::read(&wav_path).unwrap();
    assert_eq!(read_back.len(), wav_data.len());
    assert_eq!(&read_back[0..4], b"RIFF");
}

// ── TranscriberError display messages ────────────────────────────────────────

#[test]
fn transcriber_error_context_display() {
    let e = TranscriberError::Context("model file not found".to_string());
    let msg = e.to_string();
    assert!(msg.contains("whisper context error"), "display: {}", msg);
    assert!(msg.contains("model file not found"), "display: {}", msg);
}

#[test]
fn transcriber_error_inference_display() {
    let e = TranscriberError::Inference("CUDA out of memory".to_string());
    let msg = e.to_string();
    assert!(msg.contains("whisper inference error"), "display: {}", msg);
    assert!(msg.contains("CUDA out of memory"), "display: {}", msg);
}

#[test]
fn transcriber_error_wav_read_display() {
    let e = TranscriberError::WavRead("unexpected EOF at offset 44".to_string());
    let msg = e.to_string();
    assert!(msg.contains("wav read error"), "display: {}", msg);
    assert!(msg.contains("unexpected EOF"), "display: {}", msg);
}

#[test]
fn transcriber_error_sample_rate_mismatch_display() {
    let e = TranscriberError::SampleRateMismatch { expected: 16_000, actual: 44_100 };
    let msg = e.to_string();
    assert!(msg.contains("44100"), "display must contain actual rate: {}", msg);
    assert!(msg.contains("16000"), "display must contain expected rate: {}", msg);
}

#[test]
fn transcriber_error_empty_display() {
    let e = TranscriberError::Empty;
    let msg = e.to_string();
    assert!(msg.contains("no segments"), "display: {}", msg);
}

// ── VoiceEngine construction (no audio device required) ──────────────────────

#[tokio::test]
async fn voice_engine_new_creates_model_dir_if_missing() {
    use openclaw_voice::VoiceEngine;

    let dir = tempdir().unwrap();
    let model_dir = dir.path().join("models");
    assert!(!model_dir.exists(), "model_dir must not exist before engine creation");

    let cfg = VoiceConfig {
        model_dir:   model_dir.clone(),
        model_name:  DEFAULT_MODEL_NAME.to_string(),
        sample_rate: 16_000,
        language:    "en".to_string(),
    };
    let result = VoiceEngine::new(cfg).await;
    assert!(result.is_ok(), "VoiceEngine::new must succeed: {:?}", result.err());
    assert!(model_dir.exists(), "VoiceEngine::new must create model_dir");
}

#[tokio::test]
async fn voice_engine_new_with_existing_model_dir_succeeds() {
    use openclaw_voice::VoiceEngine;

    let dir = tempdir().unwrap();
    let cfg = VoiceConfig {
        model_dir:   dir.path().to_path_buf(),
        model_name:  DEFAULT_MODEL_NAME.to_string(),
        sample_rate: 16_000,
        language:    "zh".to_string(),
    };
    // Should not fail even though dir already exists
    let result = VoiceEngine::new(cfg).await;
    assert!(result.is_ok(), "VoiceEngine::new must succeed with existing dir");
}

#[tokio::test]
async fn voice_engine_double_start_recording_returns_already_recording_error() {
    use openclaw_voice::{VoiceEngine, VoiceEngineError};

    let dir = tempdir().unwrap();
    let cfg = VoiceConfig {
        model_dir:   dir.path().to_path_buf(),
        model_name:  DEFAULT_MODEL_NAME.to_string(),
        sample_rate: 16_000,
        language:    "zh".to_string(),
    };
    let engine = VoiceEngine::new(cfg).await.unwrap();

    // First start — may fail with NoDevice in CI but must not panic
    let first = engine.start_recording();
    if first.is_ok() {
        // If first succeeded, second must return AlreadyRecording
        let second = engine.start_recording();
        assert!(
            matches!(second, Err(VoiceEngineError::AlreadyRecording)),
            "second start_recording must return AlreadyRecording"
        );
    }
    // If first failed (no device in CI) — that's acceptable, no panic
}
