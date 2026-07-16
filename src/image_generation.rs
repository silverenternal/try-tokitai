use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const WAN_IMAGE_MODEL: &str = "wan2.7-image-pro";
const DEFAULT_WAN_ENDPOINT: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";

#[derive(Debug, Clone)]
pub struct WanImageRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub size: String,
    pub output_path: PathBuf,
    pub watermark: bool,
}

#[derive(Debug, Clone)]
pub struct WanImageResult {
    pub output_path: PathBuf,
    pub model: String,
    pub request_id: Option<String>,
    pub source_url: Option<String>,
}

pub async fn generate_wan_image(api_key: &str, request: WanImageRequest) -> Result<WanImageResult> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(anyhow!(
            "DashScope API key is required for image generation"
        ));
    }
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Err(anyhow!("image prompt is empty"));
    }
    let endpoint = std::env::var("DASHSCOPE_IMAGE_API_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_WAN_ENDPOINT.to_string());
    let mut parameters = json!({
        "size": normalize_size(&request.size),
        "n": 1,
        "prompt_extend": true,
        "watermark": request.watermark,
    });
    if let Some(negative_prompt) = request
        .negative_prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parameters["negative_prompt"] = Value::String(negative_prompt.to_string());
    }
    let payload = json!({
        "model": WAN_IMAGE_MODEL,
        "input": {
            "messages": [{
                "role": "user",
                "content": [{ "text": prompt }]
            }]
        },
        "parameters": parameters,
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;
    let response = client
        .post(endpoint)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .header(CONTENT_TYPE, "application/json")
        .json(&payload)
        .send()
        .await?;
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!(
            "Wan image generation failed with HTTP {}: {}",
            status,
            compact_error(&body)
        ));
    }
    let value: Value =
        serde_json::from_str(&body).map_err(|err| anyhow!("Wan returned invalid JSON: {}", err))?;
    let source = find_generated_image(&value)
        .ok_or_else(|| anyhow!("Wan response did not contain a generated image"))?;
    let bytes = if source.starts_with("data:") {
        decode_data_url(&source)?
    } else {
        let download = client.get(&source).send().await?;
        if !download.status().is_success() {
            return Err(anyhow!(
                "failed to download generated image: HTTP {}",
                download.status()
            ));
        }
        download.bytes().await?.to_vec()
    };
    if bytes.is_empty() {
        return Err(anyhow!("Wan returned an empty image"));
    }
    if let Some(parent) = request.output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&request.output_path, bytes)?;
    Ok(WanImageResult {
        output_path: request.output_path,
        model: WAN_IMAGE_MODEL.to_string(),
        request_id: request_id.or_else(|| {
            value
                .get("request_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        source_url: (!source.starts_with("data:")).then_some(source),
    })
}

pub fn image_api_key(fallback: Option<&str>) -> Option<String> {
    std::env::var("DASHSCOPE_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            fallback
                .map(str::to_string)
                .filter(|value| !value.trim().is_empty())
        })
}

fn normalize_size(size: &str) -> String {
    let trimmed = size.trim().replace('x', "*");
    if trimmed.split_once('*').is_some_and(|(width, height)| {
        width.parse::<u32>().is_ok() && height.parse::<u32>().is_ok()
    }) {
        trimmed
    } else {
        "2048*2048".to_string()
    }
}

fn find_generated_image(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in ["image", "image_url", "url"] {
                if let Some(source) = map.get(key).and_then(Value::as_str) {
                    if source.starts_with("http://")
                        || source.starts_with("https://")
                        || source.starts_with("data:image/")
                    {
                        return Some(source.to_string());
                    }
                }
            }
            map.values().find_map(find_generated_image)
        }
        Value::Array(items) => items.iter().find_map(find_generated_image),
        _ => None,
    }
}

fn decode_data_url(value: &str) -> Result<Vec<u8>> {
    let (_, encoded) = value
        .split_once(',')
        .ok_or_else(|| anyhow!("invalid generated image data URL"))?;
    BASE64_STANDARD
        .decode(encoded.trim())
        .map_err(|err| anyhow!("invalid generated image base64: {}", err))
}

fn compact_error(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(400).collect()
}

pub fn ensure_png_path(path: &Path) -> PathBuf {
    if path.extension().is_some() {
        path.to_path_buf()
    } else {
        path.with_extension("png")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_nested_wan_image_url() {
        let value = json!({
            "output": {
                "choices": [{
                    "message": {
                        "content": [{ "image": "https://example.com/generated.png" }]
                    }
                }]
            }
        });
        assert_eq!(
            find_generated_image(&value).as_deref(),
            Some("https://example.com/generated.png")
        );
    }

    #[test]
    fn normalizes_missing_extension_to_png() {
        assert_eq!(
            ensure_png_path(Path::new("assets/figure")),
            PathBuf::from("assets/figure.png")
        );
    }
}
