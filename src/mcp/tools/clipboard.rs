use base64::Engine;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClipboardReadTextParams {
    /// Operation: read_text, write_text, read_image, or clear
    pub operation: String,
    /// Text to write (required for write_text operation)
    pub text: Option<String>,
}

pub async fn clipboard(params: Parameters<ClipboardReadTextParams>) -> Result<CallToolResult, String> {
    let p = params.0;
    let op = p.operation.to_lowercase();
    match op.as_str() {
        "read_text" => {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| format!("Failed to access clipboard: {}", e))?;
            let text = clipboard.get_text()
                .map_err(|e| format!("Failed to read clipboard text: {}", e))?;
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text(text),
            ]))
        }
        "write_text" => {
            let text = p.text.ok_or("Missing 'text' parameter for write_text operation")?;
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| format!("Failed to access clipboard: {}", e))?;
            clipboard.set_text(text)
                .map_err(|e| format!("Failed to write clipboard text: {}", e))?;
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text("Text written to clipboard successfully".to_string()),
            ]))
        }
        "read_image" => {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| format!("Failed to access clipboard: {}", e))?;
            let image = clipboard.get_image()
                .map_err(|e| format!("Failed to read clipboard image: {}. Note: clipboard may not contain an image.", e))?;
            let width = image.width;
            let height = image.height;
            let bytes: &[u8] = &image.bytes;
            // Convert RGBA bytes to base64
            let base64_data = base64::engine::general_purpose::STANDARD.encode(bytes);
            let _mime_type = "image/png";
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text(format!(
                    "Clipboard image: {}x{} pixels, {} bytes (RGBA raw data). Base64 encoded below:\n{}",
                    width, height, bytes.len(), base64_data
                )),
            ]))
        }
        "clear" => {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| format!("Failed to access clipboard: {}", e))?;
            clipboard.clear()
                .map_err(|e| format!("Failed to clear clipboard: {}", e))?;
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text("Clipboard cleared successfully".to_string()),
            ]))
        }
        _ => Err(format!("Unknown clipboard operation: '{}'. Supported: read_text, write_text, read_image, clear", p.operation)),
    }
}

use rmcp::handler::server::wrapper::Parameters;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_write_and_read_text() {
        // Write
        let write_params = Parameters(ClipboardReadTextParams {
            operation: "write_text".to_string(),
            text: Some("Hello from Rust MCP clipboard test".to_string()),
        });
        let result = clipboard(write_params).await;
        assert!(result.is_ok(), "Write failed: {:?}", result);

        // Read
        let read_params = Parameters(ClipboardReadTextParams {
            operation: "read_text".to_string(),
            text: None,
        });
        let result = clipboard(read_params).await;
        assert!(result.is_ok());
        let content = result.unwrap().content;
        assert!(!content.is_empty());
    }
}
