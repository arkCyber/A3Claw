//! `image` — vision analysis tool.
//!
//! Mirrors the official OpenClaw `image` built-in tool.
//! Encodes a local file or fetches a URL as base64 and submits it to the
//! configured vision model via the Gateway `/skills/image` endpoint.
//!
//! Parameters:
//! - `image` (required) — local file path or URL
//! - `prompt` — analysis prompt (default: "Describe the image.")
//! - `model` — override the default vision model
//! - `max_bytes_mb` — size cap in MB (default: 10)

use std::path::Path;

pub const DEFAULT_PROMPT: &str = "Describe the image.";
pub const DEFAULT_MAX_BYTES_MB: u64 = 10;

pub struct ImageArgs<'a> {
    pub source: &'a str,
    pub prompt: &'a str,
    pub model: Option<&'a str>,
    pub max_bytes_mb: u64,
}

impl<'a> ImageArgs<'a> {
    pub fn from_json(args: &'a serde_json::Value) -> Result<Self, String> {
        let source = args["image"].as_str().ok_or("missing 'image' argument")?;
        let prompt = args["prompt"].as_str().unwrap_or(DEFAULT_PROMPT);
        let model = args["model"].as_str();
        let max_bytes_mb = args["max_bytes_mb"].as_u64().unwrap_or(DEFAULT_MAX_BYTES_MB);
        Ok(ImageArgs { source, prompt, model, max_bytes_mb })
    }
}

/// Analyse an image by delegating to the Gateway vision endpoint.
/// Falls back to a descriptive stub if the gateway is unreachable or
/// vision model is not configured.
pub async fn analyze(
    client: &reqwest::Client,
    gateway_url: &str,
    args: &ImageArgs<'_>,
) -> Result<String, String> {
    let max_bytes = args.max_bytes_mb * 1024 * 1024;

    let (data_b64, mime) = load_image(client, args.source, max_bytes).await?;

    let payload = serde_json::json!({
        "image": data_b64,
        "mimeType": mime,
        "prompt": args.prompt,
        "model": args.model,
    });

    let url = format!("{}/skills/image", gateway_url);
    match client.post(&url).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {
            resp.text().await.map_err(|e| format!("image response error: {}", e))
        }
        Ok(resp) => Ok(format!(
            "(image tool: gateway returned HTTP {} — ensure imageModel is configured in OpenClaw+)",
            resp.status()
        )),
        Err(e) => Ok(format!(
            "(image tool: gateway unreachable — {})\nImage source: {}\nPrompt: {}",
            e, args.source, args.prompt
        )),
    }
}

async fn load_image(
    client: &reqwest::Client,
    source: &str,
    max_bytes: u64,
) -> Result<(String, String), String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let resp = client
            .get(source)
            .send()
            .await
            .map_err(|e| format!("fetch image: {}", e))?;

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        let bytes = resp.bytes().await.map_err(|e| format!("read image bytes: {}", e))?;
        if bytes.len() as u64 > max_bytes {
            return Err(format!(
                "Image too large: {} bytes (limit {} MB)",
                bytes.len(),
                max_bytes / 1024 / 1024
            ));
        }
        Ok((base64_encode(&bytes), content_type))
    } else {
        let path = Path::new(source);
        let bytes = std::fs::read(path)
            .map_err(|e| format!("read image file {}: {}", source, e))?;
        if bytes.len() as u64 > max_bytes {
            return Err(format!(
                "Image too large: {} bytes (limit {} MB)",
                bytes.len(),
                max_bytes / 1024 / 1024
            ));
        }
        let mime = guess_mime(path);
        Ok((base64_encode(&bytes), mime.to_string()))
    }
}

fn guess_mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png")  => "image/png",
        Some("gif")  => "image/gif",
        Some("webp") => "image/webp",
        Some("svg")  => "image/svg+xml",
        _            => "image/jpeg",
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        let _ = write!(
            out,
            "{}{}{}{}",
            TABLE[((n >> 18) & 63) as usize] as char,
            TABLE[((n >> 12) & 63) as usize] as char,
            if chunk.len() > 1 { TABLE[((n >> 6) & 63) as usize] as char } else { '=' },
            if chunk.len() > 2 { TABLE[(n & 63) as usize] as char } else { '=' },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_args_missing_source_errors() {
        let v = serde_json::json!({});
        assert!(ImageArgs::from_json(&v).is_err());
    }

    #[test]
    fn image_args_defaults() {
        let v = serde_json::json!({"image": "/tmp/test.png"});
        let args = ImageArgs::from_json(&v).unwrap();
        assert_eq!(args.prompt, DEFAULT_PROMPT);
        assert_eq!(args.max_bytes_mb, DEFAULT_MAX_BYTES_MB);
        assert!(args.model.is_none());
    }

    #[test]
    fn guess_mime_types() {
        assert_eq!(guess_mime(Path::new("a.jpg")),  "image/jpeg");
        assert_eq!(guess_mime(Path::new("a.png")),  "image/png");
        assert_eq!(guess_mime(Path::new("a.webp")), "image/webp");
        assert_eq!(guess_mime(Path::new("a.xyz")),  "image/jpeg");
    }

    #[test]
    fn base64_encode_hello() {
        let encoded = base64_encode(b"Hello");
        assert_eq!(encoded, "SGVsbG8=");
    }

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_one_byte() {
        // 'M' = 0x4D → binary 01001101 → Q = 010011, Q = 010000+pad, == pad, == pad
        let encoded = base64_encode(b"M");
        assert_eq!(encoded, "TQ==");
    }

    #[test]
    fn base64_encode_two_bytes() {
        // "Ma" = 0x4D 0x61 → "TWE="
        let encoded = base64_encode(b"Ma");
        assert_eq!(encoded, "TWE=");
    }

    #[test]
    fn base64_encode_three_bytes_roundtrip() {
        // Three bytes produce exactly 4 chars with no padding
        let encoded = base64_encode(b"Man");
        assert_eq!(encoded, "TWFu");
    }

    #[test]
    fn base64_encode_longer_string() {
        // "Hello, World!" known base64
        let encoded = base64_encode(b"Hello, World!");
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn guess_mime_svg() {
        assert_eq!(guess_mime(std::path::Path::new("icon.svg")), "image/svg+xml");
    }

    #[test]
    fn guess_mime_gif() {
        assert_eq!(guess_mime(std::path::Path::new("anim.gif")), "image/gif");
    }

    #[test]
    fn image_args_custom_fields() {
        let v = serde_json::json!({
            "image": "/tmp/photo.jpg",
            "prompt": "What is in this image?",
            "model": "llava",
            "max_bytes_mb": 5
        });
        let args = ImageArgs::from_json(&v).unwrap();
        assert_eq!(args.prompt, "What is in this image?");
        assert_eq!(args.model, Some("llava"));
        assert_eq!(args.max_bytes_mb, 5);
    }
}
