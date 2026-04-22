use crate::utils::file_utils::ensure_path_within_working_dir;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PathExistsParams {
    /// Path to check
    #[schemars(description = "Path to check")]
    pub path: String,
}

#[derive(Debug, Serialize)]
struct PathExistsResult {
    path: String,
    exists: bool,
    path_type: String, // "file", "dir", "symlink", "none"
}

pub async fn path_exists(
    params: Parameters<PathExistsParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);

    let canonical_path = ensure_path_within_working_dir(path, working_dir)?;

    let (exists, path_type) = if !canonical_path.exists() {
        (false, "none".to_string())
    } else {
        let metadata = tokio::fs::symlink_metadata(&canonical_path)
            .await
            .map_err(|e| format!("Failed to stat '{}': {}", canonical_path.display(), e))?;

        if metadata.is_symlink() {
            (true, "symlink".to_string())
        } else if canonical_path.is_dir() {
            (true, "dir".to_string())
        } else if canonical_path.is_file() {
            (true, "file".to_string())
        } else {
            (true, "unknown".to_string())
        }
    };

    let result = PathExistsResult {
        path: canonical_path.to_string_lossy().to_string(),
        exists,
        path_type,
    };

    let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_path_exists_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let params = PathExistsParams {
            path: file_path.to_string_lossy().to_string(),
        };

        let result = path_exists(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"exists\": true"));
                assert!(text.text.contains("\"path_type\": \"file\""));
            }
        }
    }

    #[tokio::test]
    async fn test_path_exists_dir() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();

        let params = PathExistsParams {
            path: dir_path.to_string_lossy().to_string(),
        };

        let result = path_exists(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"path_type\": \"dir\""));
            }
        }
    }

    #[tokio::test]
    async fn test_path_exists_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let params = PathExistsParams {
            path: "nonexistent.txt".to_string(),
        };

        let result = path_exists(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"exists\": false"));
                assert!(text.text.contains("\"path_type\": \"none\""));
            }
        }
    }
}
