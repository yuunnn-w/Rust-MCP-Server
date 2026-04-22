use crate::utils::file_utils::ensure_path_within_working_dir;
use crate::utils::image_utils::{get_image_dimensions, get_image_mime_type, is_image_file};
use base64::Engine;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImageReadParams {
    /// Image file path
    #[schemars(description = "The image file path")]
    pub path: String,
    /// Read mode: "full" (default) returns base64 data; "metadata" returns only dimensions and type
    #[schemars(description = "Read mode: full (default) returns base64 data; metadata returns only dimensions and type")]
    pub mode: Option<String>,
}

#[derive(Debug, Serialize)]
struct ImageMetadata {
    mime_type: String,
    size_bytes: usize,
    filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
}

pub async fn image_read(
    params: Parameters<ImageReadParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);
    let mode = params.mode.as_deref().unwrap_or("full");

    let canonical_path = ensure_path_within_working_dir(path, working_dir)?;

    if !canonical_path.exists() {
        return Err(format!("Image file '{}' does not exist", params.path));
    }
    if !canonical_path.is_file() {
        return Err(format!("Path '{}' is not a file", params.path));
    }

    let path = &canonical_path;
    let is_image = is_image_file(path);
    let mime_type = get_image_mime_type(path).to_string();
    let dimensions = get_image_dimensions(path);

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let size_bytes = std::fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0);

    if mode == "metadata" {
        let meta = ImageMetadata {
            mime_type,
            size_bytes,
            filename,
            width: dimensions.map(|d| d.0),
            height: dimensions.map(|d| d.1),
        };
        let json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
        return Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]));
    }

    // full mode: return ImageContent + TextContent metadata
    let data = tokio::fs::read(path)
        .await
        .map_err(|e| format!("Failed to read image file: {}", e))?;

    let base64_data = base64::engine::general_purpose::STANDARD.encode(&data);

    let mut contents = vec![
        // Primary: image content for vision model encoding
        rmcp::model::Content::image(base64_data.clone(), mime_type.clone()),
    ];

    // Secondary: text metadata for human/model reference
    let meta_text = format!(
        "Image metadata: filename={}, mime_type={}, size_bytes={}, dimensions={}x{}{}",
        filename,
        mime_type,
        size_bytes,
        dimensions.map(|d| d.0.to_string()).unwrap_or_else(|| "unknown".to_string()),
        dimensions.map(|d| d.1.to_string()).unwrap_or_else(|| "unknown".to_string()),
        if !is_image { "\nWarning: This file may not be a valid image format." } else { "" }
    );
    contents.push(rmcp::model::Content::text(meta_text));

    Ok(CallToolResult::success(contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_image_read_full() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.png");
        fs::write(&file_path, b"fake image data").unwrap();

        let params = ImageReadParams {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("full".to_string()),
        };

        let result = image_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        let call_result = result.unwrap();
        // Should return 2 contents: image + text metadata
        assert_eq!(call_result.content.len(), 2);
        // First content should be image
        assert!(call_result.content[0].as_image().is_some());
        // Second content should be text
        assert!(call_result.content[1].as_text().is_some());
    }

    #[tokio::test]
    async fn test_image_read_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.png");
        fs::write(&file_path, b"fake image data").unwrap();

        let params = ImageReadParams {
            path: file_path.to_string_lossy().to_string(),
            mode: Some("metadata".to_string()),
        };

        let result = image_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(!text.text.contains("base64_data"));
                assert!(text.text.contains("mime_type"));
            }
        }
    }

    #[tokio::test]
    async fn test_image_read_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let params = ImageReadParams {
            path: "/nonexistent/image.png".to_string(),
            mode: None,
        };

        let result = image_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_err());
    }
}
