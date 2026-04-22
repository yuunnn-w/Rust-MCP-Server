use crate::utils::file_utils::ensure_path_within_working_dir;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileStatParams {
    /// Path to file or directory
    #[schemars(description = "Path to file or directory")]
    pub path: String,
}

#[derive(Debug, Serialize)]
struct FileStatResult {
    path: String,
    name: String,
    exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_human: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    readable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    writable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    executable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accessed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "is_symlink")]
    is_symlink: Option<bool>,
}

pub async fn file_stat(
    params: Parameters<FileStatParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);

    let canonical_path = ensure_path_within_working_dir(path, working_dir)?;

    if !canonical_path.exists() {
        return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&FileStatResult {
                path: canonical_path.to_string_lossy().to_string(),
                name: canonical_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                exists: false,
                file_type: None,
                size: None,
                size_human: None,
                readable: None,
                writable: None,
                executable: None,
                modified: None,
                created: None,
                accessed: None,
                is_symlink: None,
            }).map_err(|e| e.to_string())?
        )]));
    }

    let metadata = tokio::fs::symlink_metadata(&canonical_path)
        .await
        .map_err(|e| format!("Failed to stat '{}': {}", canonical_path.display(), e))?;

    let is_symlink = metadata.is_symlink();

    // For symlinks, also get the target metadata if possible
    let final_metadata = if is_symlink {
        tokio::fs::metadata(&canonical_path).await.ok()
    } else {
        Some(metadata)
    };

    let file_type = if is_symlink {
        Some("symlink".to_string())
    } else if final_metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
        Some("directory".to_string())
    } else if final_metadata.as_ref().map(|m| m.is_file()).unwrap_or(false) {
        Some("file".to_string())
    } else {
        Some("unknown".to_string())
    };

    let size = final_metadata.as_ref().map(|m| m.len());
    let size_human = size.map(|s| crate::utils::file_utils::format_file_size(s));

    let permissions = final_metadata.as_ref().map(|m| m.permissions());
    #[cfg(unix)]
    let (readable, writable, executable) = {
        use std::os::unix::fs::PermissionsExt;
        permissions.map(|p| {
            let mode = p.mode();
            (
                Some(mode & 0o444 != 0),
                Some(mode & 0o222 != 0),
                Some(mode & 0o111 != 0),
            )
        }).unwrap_or((None, None, None))
    };
    #[cfg(not(unix))]
    let (readable, writable, executable) = {
        permissions.map(|p| {
            (
                Some(true), // Windows files are generally readable
                Some(!p.readonly()),
                None, // No simple executable bit on Windows
            )
        }).unwrap_or((None, None, None))
    };

    let modified = final_metadata.as_ref()
        .and_then(|m| m.modified().ok())
        .map(|t| crate::utils::file_utils::format_datetime(t));
    let created = final_metadata.as_ref()
        .and_then(|m| m.created().ok())
        .map(|t| crate::utils::file_utils::format_datetime(t));
    let accessed = final_metadata.as_ref()
        .and_then(|m| m.accessed().ok())
        .map(|t| crate::utils::file_utils::format_datetime(t));

    let result = FileStatResult {
        path: canonical_path.to_string_lossy().to_string(),
        name: canonical_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        exists: true,
        file_type,
        size,
        size_human,
        readable,
        writable,
        executable,
        modified,
        created,
        accessed,
        is_symlink: Some(is_symlink),
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
    async fn test_file_stat_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let params = FileStatParams {
            path: file_path.to_string_lossy().to_string(),
        };

        let result = file_stat(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("exists"));
                assert!(text.text.contains("file"));
                assert!(text.text.contains("size"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_stat_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();

        let params = FileStatParams {
            path: dir_path.to_string_lossy().to_string(),
        };

        let result = file_stat(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("directory"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_stat_not_exists() {
        let temp_dir = TempDir::new().unwrap();

        let params = FileStatParams {
            path: "nonexistent_file.txt".to_string(),
        };

        let result = file_stat(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"exists\": false"));
            }
        }
    }
}
