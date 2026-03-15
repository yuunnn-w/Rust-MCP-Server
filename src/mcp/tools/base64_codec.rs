use base64::Engine;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Base64EncodeParams {
    /// String to encode
    #[schemars(description = "The string to encode to base64")]
    pub input: String,
}

pub async fn base64_encode(
    params: Parameters<Base64EncodeParams>,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let encoded = base64::engine::general_purpose::STANDARD.encode(params.input.as_bytes());

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        encoded,
    )]))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Base64DecodeParams {
    /// Base64 string to decode
    #[schemars(description = "The base64 string to decode")]
    pub input: String,
}

pub async fn base64_decode(
    params: Parameters<Base64DecodeParams>,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(params.input)
        .map_err(|e| format!("Invalid base64 string: {}", e))?;

    let decoded_str = String::from_utf8(decoded)
        .map_err(|_| "Decoded data is not valid UTF-8".to_string())?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        decoded_str,
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base64_encode() {
        let params = Base64EncodeParams {
            input: "Hello, World!".to_string(),
        };

        let result = base64_encode(Parameters(params)).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert_eq!(text.text, "SGVsbG8sIFdvcmxkIQ==");
            }
        }
    }

    #[tokio::test]
    async fn test_base64_decode() {
        let params = Base64DecodeParams {
            input: "SGVsbG8sIFdvcmxkIQ==".to_string(),
        };

        let result = base64_decode(Parameters(params)).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert_eq!(text.text, "Hello, World!");
            }
        }
    }

    #[tokio::test]
    async fn test_base64_decode_invalid() {
        let params = Base64DecodeParams {
            input: "not-valid-base64!!!".to_string(),
        };

        let result = base64_decode(Parameters(params)).await;
        assert!(result.is_err());
    }
}
