//! Audio recording via `cpal` — captures from the default input device at 16 kHz mono.
//!
//! The `cpal::Stream` is NOT Send. We run the entire recording lifecycle on a
//! dedicated OS thread and communicate via `std::sync::mpsc`.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

/// An active recording session.
/// Internally, recording runs on a dedicated thread; this handle is `Send + Sync`.
pub struct Recorder {
    /// Channel sender: send () to request stop; the thread responds with samples.
    stop_tx: mpsc::SyncSender<()>,
    /// Channel receiver: receives accumulated samples after stop is requested.
    samples_rx: mpsc::Receiver<Vec<f32>>,
}

#[derive(Debug, thiserror::Error)]
pub enum RecorderError {
    #[error("no input device available")]
    NoDevice,
    #[error("cpal error: {0}")]
    Cpal(String),
    #[error("wav write error: {0}")]
    Wav(String),
}

impl Recorder {
    /// Start capturing audio on a dedicated OS thread.
    /// The `cpal::Stream` is confined to that thread (it is !Send).
    pub fn start(target_sample_rate: u32) -> Result<Self, RecorderError> {
        // Channel: main → recorder thread (stop signal)
        let (stop_tx, stop_rx) = mpsc::sync_channel::<()>(1);
        // Channel: recorder thread → main (samples result)
        let (samples_tx, samples_rx) = mpsc::sync_channel::<Vec<f32>>(1);

        // Error channel so we can propagate start errors back
        let (err_tx, err_rx) = mpsc::sync_channel::<RecorderError>(1);

        std::thread::spawn(move || {
            // Everything cpal-related lives here — never leaves this thread.
            let host = cpal::default_host();
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    let _ = err_tx.send(RecorderError::NoDevice);
                    return;
                }
            };

            let config = match find_input_config(&device, target_sample_rate) {
                Ok(c) => c,
                Err(e) => {
                    let _ = err_tx.send(e);
                    return;
                }
            };

            tracing::info!(
                "[voice] input device: {}  config: {:?} ch={} rate={}",
                device.name().unwrap_or_default(),
                config.sample_format(),
                config.channels(),
                config.sample_rate().0,
            );

            let channels   = config.channels() as usize;
            let device_rate = config.sample_rate().0;
            let fmt        = config.sample_format();
            let buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
            let buf_clone  = buf.clone();

            let stream = match build_stream(
                &device,
                &config.into(),
                fmt,
                channels,
                device_rate,
                target_sample_rate,
                buf_clone,
            ) {
                Ok(s) => s,
                Err(e) => {
                    let _ = err_tx.send(RecorderError::Cpal(e.to_string()));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                let _ = err_tx.send(RecorderError::Cpal(e.to_string()));
                return;
            }

            // Signal to caller that we started OK (by not sending to err_tx)
            // Wait for stop signal
            let _ = stop_rx.recv();

            // Stop & flush
            drop(stream);
            let samples = buf.lock().unwrap().clone();
            let _ = samples_tx.send(samples);
        });

        // Wait briefly to see if the thread reported a startup error
        std::thread::sleep(std::time::Duration::from_millis(80));
        if let Ok(e) = err_rx.try_recv() {
            return Err(e);
        }

        Ok(Self { stop_tx, samples_rx })
    }

    /// Signal the recording thread to stop and return collected samples.
    pub fn stop(self) -> Result<Vec<f32>, RecorderError> {
        let _ = self.stop_tx.send(());
        self.samples_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| RecorderError::Cpal("recording thread timed out".into()))
    }
}

/// Write f32 mono samples to a 16-bit PCM WAV file.
pub fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), RecorderError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| RecorderError::Wav(e.to_string()))?;
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer
            .write_sample(v)
            .map_err(|e| RecorderError::Wav(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| RecorderError::Wav(e.to_string()))?;
    Ok(())
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn find_input_config(
    device: &cpal::Device,
    preferred_rate: u32,
) -> Result<cpal::SupportedStreamConfig, RecorderError> {
    // Try exact match first
    let mut configs: Vec<cpal::SupportedStreamConfigRange> = device
        .supported_input_configs()
        .map_err(|e| RecorderError::Cpal(e.to_string()))?
        .collect();

    // Sort: prefer mono, then lowest sample rate ≥ preferred_rate
    configs.sort_by_key(|c| {
        let ch_penalty = if c.channels() == 1 { 0u32 } else { 100_000 };
        let rate = c.min_sample_rate().0.max(preferred_rate);
        ch_penalty + rate
    });

    if let Some(range) = configs.first() {
        let rate = if range.min_sample_rate().0 <= preferred_rate
            && preferred_rate <= range.max_sample_rate().0
        {
            cpal::SampleRate(preferred_rate)
        } else {
            range.min_sample_rate()
        };
        return Ok((*range).with_sample_rate(rate));
    }

    // Fall back to default config
    device
        .default_input_config()
        .map_err(|e| RecorderError::Cpal(e.to_string()))
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    fmt: cpal::SampleFormat,
    channels: usize,
    device_rate: u32,
    target_rate: u32,
    samples: Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>> {
    let err_fn = |e| tracing::error!("[voice] stream error: {e}");

    // Simple linear resampler ratio
    let ratio = target_rate as f64 / device_rate as f64;

    macro_rules! make_stream {
        ($T:ty) => {{
            let s = samples.clone();
            let cfg = config.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[$T], _: &cpal::InputCallbackInfo| {
                    let mono: Vec<f32> = data
                        .chunks(channels)
                        .map(|ch| {
                            let sum: f32 = ch
                                .iter()
                                .map(|&x| cpal::Sample::to_float_sample(x))
                                .sum::<f32>();
                            sum / channels as f32
                        })
                        .collect();
                    // Naive linear resample if needed
                    let resampled = if (ratio - 1.0).abs() < 0.001 {
                        mono
                    } else {
                        resample_linear(&mono, ratio)
                    };
                    s.lock().unwrap().extend_from_slice(&resampled);
                },
                err_fn,
                None,
            )?
        }};
    }

    let stream = match fmt {
        cpal::SampleFormat::F32 => make_stream!(f32),
        cpal::SampleFormat::I16 => make_stream!(i16),
        cpal::SampleFormat::U16 => make_stream!(u16),
        cpal::SampleFormat::I8  => make_stream!(i8),
        cpal::SampleFormat::U8  => make_stream!(u8),
        cpal::SampleFormat::I32 => make_stream!(i32),
        // F64: convert to f32 first
        cpal::SampleFormat::F64 => {
            let s = samples.clone();
            let cfg = config.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[f64], _: &cpal::InputCallbackInfo| {
                    let mono: Vec<f32> = data
                        .chunks(channels)
                        .map(|ch| {
                            let sum: f64 = ch.iter().copied().sum::<f64>();
                            (sum / channels as f64) as f32
                        })
                        .collect();
                    let resampled = if (ratio - 1.0).abs() < 0.001 {
                        mono
                    } else {
                        resample_linear(&mono, ratio)
                    };
                    s.lock().unwrap().extend_from_slice(&resampled);
                },
                err_fn,
                None,
            )?
        },
        _ => {
            return Err(format!("unsupported sample format: {:?}", fmt).into());
        }
    };
    Ok(stream)
}

fn resample_linear(input: &[f32], ratio: f64) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }
    let out_len = ((input.len() as f64) * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;
        let a = input[idx.min(input.len() - 1)];
        let b = input[(idx + 1).min(input.len() - 1)];
        out.push(a + (b - a) * frac as f32);
    }
    out
}
