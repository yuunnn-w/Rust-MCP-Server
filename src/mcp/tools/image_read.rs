use crate::utils::image_utils::{get_image_mime_type, is_image_file};
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
}

#[derive(Debug, Serialize)]
struct ImageData {
    mime_type: String,
    base64_data: String,
    size_bytes: usize,
    filename: String,
}

pub async fn image_read(params: Parameters<ImageReadParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);

    // Check if file exists
    if !path.exists() {
        return Err(format!("Image file '{}' does not exist", params.path));
    }

    if !path.is_file() {
        return Err(format!("Path '{}' is not a file", params.path));
    }

    // Check if it's an image file (warning for non-image files)
    let is_image = is_image_file(path);

    // Read file
    let data = tokio::fs::read(path)
        .await
        .map_err(|e| format!("Failed to read image file: {}", e))?;

    let mime_type = get_image_mime_type(path);
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&data);

    let image_data = ImageData {
        mime_type: mime_type.to_string(),
        base64_data,
        size_bytes: data.len(),
        filename: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
    };

    let warning = if !is_image {
        "\n\nWarning: This file may not be a valid image format."
    } else {
        ""
    };

    let json_data =
        serde_json::to_string_pretty(&image_data).map_err(|e| e.to_string())?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("{}{}", json_data, warning),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_image_read() {
        // Create a simple test file (not a real image, just for testing)
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.png");
        fs::write(&file_path, b"fake image data").unwrap();

        let params = ImageReadParams {
            path: file_path.to_string_lossy().to_string(),
        };

        let result = image_read(Parameters(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_image_read_not_found() {
        let params = ImageReadParams {
            path: "/nonexistent/image.png".to_string(),
        };

        let result = image_read(Parameters(params)).await;
        assert!(result.is_err());
    }
}
