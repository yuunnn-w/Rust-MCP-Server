use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, elicit_safe};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "User response to a question")]
pub struct UserResponse {
    #[schemars(description = "The user's response")]
    pub response: String,
}

elicit_safe!(UserResponse);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AskUserParams {
    #[schemars(description = "Question to ask the user (required, max 1000 chars)")]
    pub question: String,
    #[schemars(description = "Optional list of choices (max 10)")]
    pub options: Option<Vec<String>>,
    #[schemars(description = "Timeout in seconds (default: 120, max: 600)")]
    pub timeout_sec: Option<u64>,
    #[schemars(description = "Timeout in seconds (alias for timeout_sec, default: 120)")]
    pub timeout: Option<u64>,
    #[schemars(description = "Default value returned when user does not respond")]
    pub default_value: Option<String>,
}

#[derive(Debug, Serialize)]
struct AskUserOutput {
    question: String,
    response: String,
    selected_option: Option<String>,
}

pub async fn ask_user(
    params: Parameters<AskUserParams>,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    let p = params.0;
    let question = p.question.trim().to_string();
    if question.is_empty() {
        return Ok(CallToolResult::error(vec![rmcp::model::Content::text(
            "Question must not be empty".to_string(),
        )]));
    }
    if question.len() > 1000 {
        return Ok(CallToolResult::error(vec![rmcp::model::Content::text(
            "Question must be 1000 characters or fewer".to_string(),
        )]));
    }

    let options = p.options.map(|opts| {
        opts
            .into_iter()
            .filter(|o| !o.trim().is_empty())
            .take(10)
            .collect::<Vec<_>>()
    });

    let timeout_sec = p.timeout
        .or(p.timeout_sec)
        .unwrap_or(120)
        .min(600);
    let default_value = p.default_value.clone();

    let prompt = if let Some(ref options) = options {
        format!(
            "{}\n\nAvailable options:\n{}",
            question,
            options
                .iter()
                .enumerate()
                .map(|(i, opt)| format!("  {}. {}", i + 1, opt))
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        question.clone()
    };

    let timeout_dur = std::time::Duration::from_secs(timeout_sec);
    match tokio::time::timeout(timeout_dur, context.peer.elicit::<UserResponse>(prompt)).await {
        Ok(Ok(Some(user_response))) => {
            let selected_option = if let Some(ref opts) = options {
                user_response
                    .response
                    .parse::<usize>()
                    .ok()
                    .filter(|&n| n >= 1 && n <= opts.len())
                    .map(|n| opts[n - 1].clone())
            } else {
                None
            };

            let output = AskUserOutput {
                question,
                response: user_response.response,
                selected_option,
            };
            let json =
                serde_json::to_string_pretty(&output).map_err(|e| McpError::internal_error(format!("Failed to serialize response: {}", e), None))?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                json,
            )]))
        }
        Ok(Ok(None)) => {
            if let Some(ref dv) = default_value {
                let output = AskUserOutput {
                    question,
                    response: dv.clone(),
                    selected_option: None,
                };
                let json = serde_json::to_string_pretty(&output)
                    .map_err(|e| McpError::internal_error(format!("Failed to serialize response: {}", e), None))?;
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
            } else {
                Ok(CallToolResult::error(vec![rmcp::model::Content::text(
                    "User did not provide a response".to_string(),
                )]))
            }
        }
        Ok(Err(e)) => Ok(CallToolResult::error(vec![rmcp::model::Content::text(
            format!("Elicitation failed: {}", e),
        )])),
        Err(_elapsed) => {
            if let Some(ref dv) = default_value {
                let output = AskUserOutput {
                    question,
                    response: dv.clone(),
                    selected_option: None,
                };
                let json = serde_json::to_string_pretty(&output)
                    .map_err(|e| McpError::internal_error(format!("Failed to serialize response: {}", e), None))?;
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
            } else {
                Ok(CallToolResult::error(vec![rmcp::model::Content::text(
                    format!("Request timed out after {} seconds", timeout_sec),
                )]))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ask_user_params_deserialization() {
        let json = r#"{"question": "What is your name?", "options": ["Alice", "Bob"], "timeout_sec": 60}"#;
        let params: AskUserParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.question, "What is your name?");
        assert_eq!(params.options, Some(vec!["Alice".to_string(), "Bob".to_string()]));
        assert_eq!(params.timeout_sec, Some(60));
        assert!(params.timeout.is_none());
        assert!(params.default_value.is_none());
    }

    #[test]
    fn test_ask_user_params_with_default() {
        let json = r#"{"question": "OK?", "timeout": 30, "default_value": "yes"}"#;
        let params: AskUserParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.timeout, Some(30));
        assert_eq!(params.default_value, Some("yes".to_string()));
    }

    #[test]
    fn test_ask_user_params_minimal() {
        let json = r#"{"question": "What is your name?"}"#;
        let params: AskUserParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.question, "What is your name?");
        assert!(params.options.is_none());
        assert!(params.timeout_sec.is_none());
    }
}
