//! Whisper GGML model management: check existence and download from Hugging Face.

use std::path::{Path, PathBuf};

/// Default model name — "base" is small (142 MB) and works well for Chinese + English.
pub const DEFAULT_MODEL_NAME: &str = "ggml-base.bin";

/// HuggingFace URL template for ggerganov/whisper.cpp models.
const MODEL_BASE_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("download failed: {0}")]
    Download(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("model not found and download skipped")]
    NotFound,
}

pub struct ModelManager {
    model_dir: PathBuf,
}

impl ModelManager {
    pub fn new(model_dir: PathBuf) -> Self {
        Self { model_dir }
    }

    /// Returns the full path to the requested model file.
    pub fn model_path(&self, name: &str) -> PathBuf {
        self.model_dir.join(name)
    }

    /// Returns `true` if the model file exists on disk.
    pub fn model_exists(&self, name: &str) -> bool {
        self.model_path(name).exists()
    }

    /// Ensure the model is present, downloading it from HuggingFace if necessary.
    /// `progress_cb` receives bytes downloaded so far and total bytes (0 if unknown).
    pub async fn ensure_model(
        &self,
        name: &str,
        mut progress_cb: impl FnMut(u64, u64) + Send + 'static,
    ) -> Result<PathBuf, ModelError> {
        let path = self.model_path(name);
        if path.exists() {
            tracing::info!("[voice/model] {} already present at {:?}", name, path);
            return Ok(path);
        }

        std::fs::create_dir_all(&self.model_dir)
            .map_err(|e| ModelError::Io(e.to_string()))?;

        let url = format!("{}/{}", MODEL_BASE_URL, name);
        tracing::info!("[voice/model] downloading {} from {}", name, url);

        let client = reqwest::Client::builder()
            .user_agent("openclaw-plus/0.1")
            .build()
            .map_err(|e| ModelError::Download(e.to_string()))?;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| ModelError::Download(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ModelError::Download(format!(
                "HTTP {} for {}",
                resp.status(),
                url
            )));
        }

        let total = resp.content_length().unwrap_or(0);
        let tmp_path = path.with_extension("bin.part");

        {
            use tokio::io::AsyncWriteExt;
            use futures_util::StreamExt;

            let mut file = tokio::fs::File::create(&tmp_path)
                .await
                .map_err(|e| ModelError::Io(e.to_string()))?;

            let mut downloaded: u64 = 0;
            let mut stream = resp.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| ModelError::Download(e.to_string()))?;
                file.write_all(&chunk)
                    .await
                    .map_err(|e| ModelError::Io(e.to_string()))?;
                downloaded += chunk.len() as u64;
                progress_cb(downloaded, total);
            }
            file.flush()
                .await
                .map_err(|e| ModelError::Io(e.to_string()))?;
        }

        // Atomic rename
        tokio::fs::rename(&tmp_path, &path)
            .await
            .map_err(|e| ModelError::Io(e.to_string()))?;

        tracing::info!("[voice/model] download complete: {:?}", path);
        Ok(path)
    }
}

/// Human-readable size string.
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_path_construction() {
        let mgr = ModelManager::new(PathBuf::from("/tmp/models"));
        assert_eq!(
            mgr.model_path("ggml-base.bin"),
            PathBuf::from("/tmp/models/ggml-base.bin")
        );
    }

    #[test]
    fn format_size_units() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(1_500_000), "1.4 MB");
    }
}
