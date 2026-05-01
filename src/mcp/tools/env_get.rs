use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnvGetParams {
    /// Environment variable name
    #[schemars(description = "Environment variable name")]
    pub name: String,
}

#[derive(Debug, Serialize)]
struct EnvGetResult {
    name: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_set: Option<bool>,
}

pub async fn env_get(params: Parameters<EnvGetParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let name = params.name;

    // Sensitive environment variable blacklist
    let sensitive_patterns = ["SECRET", "PASSWORD", "TOKEN", "KEY", "CREDENTIAL", "AUTH"];
    let upper_name = name.to_uppercase();
    for pattern in &sensitive_patterns {
        if upper_name.contains(pattern) {
            return Err(format!(
                "Access to environment variable '{}' is restricted for security reasons.",
                name
            ));
        }
    }

    let env_result = std::env::var(&name);
    let is_set = env_result.is_ok();

    let result = EnvGetResult {
        name: name.clone(),
        value: if is_set { env_result.unwrap() } else { format!("Environment variable '{}' is not set", name) },
        is_set: Some(is_set),
    };

    let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_env_get_existing() {
        // PATH should exist on all platforms
        let params = EnvGetParams {
            name: "PATH".to_string(),
        };

        let result = env_get(Parameters(params)).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"is_set\": true"));
            }
        }
    }

    #[tokio::test]
    async fn test_env_get_not_set() {
        let params = EnvGetParams {
            name: "RUST_MCP_TEST_VAR_THAT_DOES_NOT_EXIST".to_string(),
        };

        let result = env_get(Parameters(params)).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"is_set\": false"));
            }
        }
    }
}
