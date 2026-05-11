use base64::Engine;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClipboardParams {
    #[schemars(description = "Operation: read_text, write_text, read_image, or clear")]
    pub operation: String,
    #[schemars(description = "Text to write (required for write_text operation)")]
    pub text: Option<String>,
    #[schemars(description = "Clipboard format: text (default), html, or rtf")]
    pub format: Option<String>,
}

pub async fn clipboard(params: Parameters<ClipboardParams>) -> Result<CallToolResult, String> {
    let p = params.0;
    let op = p.operation.to_lowercase();
    let format = p.format.as_deref().unwrap_or("text").to_lowercase();
    if !["text", "html", "rtf"].contains(&format.as_str()) {
        return Err(format!("Unsupported format: '{}'. Supported: text, html, rtf", format));
    }
    match op.as_str() {
        "read_text" => {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| format!("Failed to access clipboard: {}", e))?;
            let text = clipboard.get_text()
                .map_err(|e| format!("Failed to read clipboard text: {}", e))?;
            let output = if format == "html" {
                format!("<pre>{}</pre>", text)
            } else {
                text
            };
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text(output),
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
            let base64_data = base64::engine::general_purpose::STANDARD.encode(bytes);
            Ok(CallToolResult::success(vec![
                rmcp::model::Content::text(format!(
                    "Clipboard image: {}x{} pixels, {} bytes (RGBA raw pixel data). Base64 encoded RGBA below:\n{}",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_write_and_read_text() {
        // Write
        let write_params = Parameters(ClipboardParams {
            operation: "write_text".to_string(),
            text: Some("Hello from Rust MCP clipboard test".to_string()),
            format: None,
        });
        let result = clipboard(write_params).await;
        assert!(result.is_ok(), "Write failed: {:?}", result);

        // Read
        let read_params = Parameters(ClipboardParams {
            operation: "read_text".to_string(),
            text: None,
            format: None,
        });
        let result = clipboard(read_params).await;
        assert!(result.is_ok());
        let content = result.unwrap().content;
        assert!(!content.is_empty());
    }
}
