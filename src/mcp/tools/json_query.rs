use crate::utils::file_utils::{ensure_path_within_working_dir, is_text_file};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JsonQueryParams {
    /// Path to JSON file
    #[schemars(description = "Path to JSON file")]
    pub path: String,
    /// JSON Pointer query path, e.g. "/data/0/name" or "/users"
    #[schemars(description = "JSON Pointer query path, e.g. '/data/0/name'")]
    pub query: String,
    /// Maximum characters to return (default: 15000)
    #[schemars(description = "Maximum characters to return (default: 15000)")]
    pub max_chars: Option<usize>,
}

#[derive(Debug, Serialize)]
struct JsonQueryResult {
    path: String,
    query: String,
    found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result_type: Option<String>,
}

pub async fn json_query(
    params: Parameters<JsonQueryParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);
    let max_chars = params.max_chars.unwrap_or(15000);

    let canonical_path = ensure_path_within_working_dir(path, working_dir)?;

    if !canonical_path.exists() {
        return Err(format!("File '{}' does not exist", params.path));
    }
    if !canonical_path.is_file() {
        return Err(format!("Path '{}' is not a file", params.path));
    }

    if !is_text_file(&canonical_path) {
        return Err(format!("File '{}' does not appear to be a text file", params.path));
    }

    let content = tokio::fs::read_to_string(&canonical_path)
        .await
        .map_err(|e| format!("Failed to read file '{}': {}", canonical_path.display(), e))?;

    let json_value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON in '{}': {}", canonical_path.display(), e))?;

    let query = if params.query.starts_with('/') {
        params.query.clone()
    } else {
        format!("/ {}", params.query)
    };

    let result = json_value.pointer(&query);

    let response = if let Some(value) = result {
        let result_str = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
        let truncated = result_str.len() > max_chars;
        let result_text = if truncated {
            format!(
                "{}\n\n[... Result truncated at {} characters, total {} ...]",
                &result_str[..max_chars],
                max_chars,
                result_str.len()
            )
        } else {
            result_str
        };

        JsonQueryResult {
            path: canonical_path.to_string_lossy().to_string(),
            query: params.query,
            found: true,
            result_type: Some(json_type_name(value)),
            result: Some(serde_json::Value::String(result_text)),
        }
    } else {
        JsonQueryResult {
            path: canonical_path.to_string_lossy().to_string(),
            query: params.query,
            found: false,
            result_type: None,
            result: None,
        }
    };

    let json = serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

fn json_type_name(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Array(arr) => format!("array[{}]", arr.len()),
        serde_json::Value::Object(obj) => format!("object{{{}}}", obj.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_json_query_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.json");
        fs::write(&file_path, r#"{"data": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]}"#).unwrap();

        let params = JsonQueryParams {
            path: file_path.to_string_lossy().to_string(),
            query: "/data/0/name".to_string(),
            max_chars: None,
        };

        let result = json_query(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"found\": true"));
                assert!(text.text.contains("Alice"));
            }
        }
    }

    #[tokio::test]
    async fn test_json_query_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.json");
        fs::write(&file_path, r#"{"data": []}"#).unwrap();

        let params = JsonQueryParams {
            path: file_path.to_string_lossy().to_string(),
            query: "/missing".to_string(),
            max_chars: None,
        };

        let result = json_query(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"found\": false"));
            }
        }
    }

    #[tokio::test]
    async fn test_json_query_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.json");
        fs::write(&file_path, "not json").unwrap();

        let params = JsonQueryParams {
            path: file_path.to_string_lossy().to_string(),
            query: "/".to_string(),
            max_chars: None,
        };

        let result = json_query(Parameters(params), temp_dir.path()).await;
        assert!(result.is_err());
    }
}
