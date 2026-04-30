use crate::utils::file_utils::{ensure_path_within_working_dir, strip_unc_prefix};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileWriteItem {
    /// File path to write
    #[schemars(description = "The file path to write")]
    pub path: String,
    /// Content to write
    #[schemars(description = "The content to write")]
    pub content: String,
    /// Write mode: new (default), append, overwrite
    #[schemars(description = "Write mode: new, append, or overwrite (default: new)")]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileWriteParams {
    /// List of files to write concurrently
    #[schemars(description = "List of files to write concurrently")]
    pub files: Vec<FileWriteItem>,
}

#[derive(Debug, Serialize)]
struct FileWriteResult {
    path: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes_written: Option<usize>,
}

pub async fn file_write(
    params: Parameters<FileWriteParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;

    let mut futures = Vec::new();
    for item in params.files {
        futures.push(write_single_file(item, working_dir));
    }

    let results = futures::future::join_all(futures).await;

    let json = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

async fn write_single_file(item: FileWriteItem, working_dir: &Path) -> FileWriteResult {
    let path = Path::new(&item.path);
    let mode = item.mode.as_deref().unwrap_or("new");

    // Security check: ensure path is within working directory
    let canonical_path = match ensure_path_within_working_dir(path, working_dir) {
        Ok(p) => p,
        Err(e) => {
            return FileWriteResult {
                path: item.path,
                success: false,
                error: Some(e),
                message: None,
                bytes_written: None,
            }
        }
    };

    // Check mode-specific conditions
    match mode {
        "new" => {
            if canonical_path.exists() {
                return FileWriteResult {
                    path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "File '{}' already exists. Use 'overwrite' or 'append' mode.",
                        item.path
                    )),
                    message: None,
                    bytes_written: None,
                };
            }
        }
        "append" | "overwrite" => {
            // These modes are fine with existing files
        }
        _ => {
            return FileWriteResult {
                path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!(
                    "Invalid mode '{}'. Use 'new', 'append', or 'overwrite'.",
                    mode
                )),
                message: None,
                bytes_written: None,
            }
        }
    }

    // Create parent directories if needed
    if let Some(parent) = canonical_path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return FileWriteResult {
                path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!("Failed to create parent directories: {}", e)),
                message: None,
                bytes_written: None,
            };
        }
    }

    // Write file based on mode
    let write_result = match mode {
        "new" | "overwrite" => {
            tokio::fs::write(&canonical_path, &item.content).await
        }
        "append" => {
            use tokio::io::AsyncWriteExt;
            let mut file = match tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&canonical_path)
                .await
            {
                Ok(f) => f,
                Err(e) => {
                    return FileWriteResult {
                        path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                        success: false,
                        error: Some(format!("Failed to open file for append: {}", e)),
                        message: None,
                        bytes_written: None,
                    }
                }
            };

            file.write_all(item.content.as_bytes()).await
        }
        _ => unreachable!(),
    };

    if let Err(e) = write_result {
        return FileWriteResult {
            path: strip_unc_prefix(&canonical_path.to_string_lossy()),
            success: false,
            error: Some(format!("Failed to write file: {}", e)),
            message: None,
            bytes_written: None,
        };
    }

    let action = match mode {
        "new" => "created",
        "append" => "appended to",
        "overwrite" => "overwritten",
        _ => "written",
    };

    FileWriteResult {
        path: strip_unc_prefix(&canonical_path.to_string_lossy()),
        success: true,
        error: None,
        message: Some(format!(
            "File '{}' {} successfully.",
            strip_unc_prefix(&canonical_path.to_string_lossy()),
            action
        )),
        bytes_written: Some(item.content.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_write_new() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let params = FileWriteParams {
            files: vec![FileWriteItem {
                path: file_path.to_string_lossy().to_string(),
                content: "Hello, World!".to_string(),
                mode: Some("new".to_string()),
            }],
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_file_write_append() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Initial ").await.unwrap();

        let params = FileWriteParams {
            files: vec![FileWriteItem {
                path: file_path.to_string_lossy().to_string(),
                content: "Appended".to_string(),
                mode: Some("append".to_string()),
            }],
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Initial Appended");
    }

    #[tokio::test]
    async fn test_file_write_outside_working_dir() {
        let temp_dir = TempDir::new().unwrap();

        let params = FileWriteParams {
            files: vec![FileWriteItem {
                path: "/etc/test.txt".to_string(),
                content: "test".to_string(),
                mode: Some("new".to_string()),
            }],
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"success\": false"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_write_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("a.txt");
        let file2 = temp_dir.path().join("b.txt");

        let params = FileWriteParams {
            files: vec![
                FileWriteItem {
                    path: file1.to_string_lossy().to_string(),
                    content: "File A".to_string(),
                    mode: Some("new".to_string()),
                },
                FileWriteItem {
                    path: file2.to_string_lossy().to_string(),
                    content: "File B".to_string(),
                    mode: Some("new".to_string()),
                },
            ],
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(file1.exists());
        assert!(file2.exists());
    }
}
