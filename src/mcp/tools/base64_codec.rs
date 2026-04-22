use base64::Engine;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Base64CodecParams {
    /// Operation: "encode" or "decode"
    #[schemars(description = "Operation: encode or decode")]
    pub operation: String,
    /// Input string for encode, or base64 string for decode
    #[schemars(description = "Input string for encode, or base64 string for decode")]
    pub input: String,
}

pub async fn base64_codec(
    params: Parameters<Base64CodecParams>,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let op = params.operation.to_lowercase();

    match op.as_str() {
        "encode" => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(params.input.as_bytes());
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(encoded)]))
        }
        "decode" => {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(params.input)
                .map_err(|e| format!("Invalid base64 string: {}", e))?;

            let decoded_str = String::from_utf8(decoded)
                .map_err(|_| "Decoded data is not valid UTF-8".to_string())?;

            Ok(CallToolResult::success(vec![rmcp::model::Content::text(decoded_str)]))
        }
        _ => Err(format!(
            "Invalid operation '{}'. Use 'encode' or 'decode'.",
            params.operation
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base64_encode() {
        let params = Base64CodecParams {
            operation: "encode".to_string(),
            input: "Hello, World!".to_string(),
        };

        let result = base64_codec(Parameters(params)).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert_eq!(text.text, "SGVsbG8sIFdvcmxkIQ==");
            }
        }
    }

    #[tokio::test]
    async fn test_base64_decode() {
        let params = Base64CodecParams {
            operation: "decode".to_string(),
            input: "SGVsbG8sIFdvcmxkIQ==".to_string(),
        };

        let result = base64_codec(Parameters(params)).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert_eq!(text.text, "Hello, World!");
            }
        }
    }

    #[tokio::test]
    async fn test_base64_decode_invalid() {
        let params = Base64CodecParams {
            operation: "decode".to_string(),
            input: "not-valid-base64!!!".to_string(),
        };

        let result = base64_codec(Parameters(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_base64_invalid_operation() {
        let params = Base64CodecParams {
            operation: "invalid".to_string(),
            input: "test".to_string(),
        };

        let result = base64_codec(Parameters(params)).await;
        assert!(result.is_err());
    }
}
