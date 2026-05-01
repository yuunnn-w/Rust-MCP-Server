use crate::utils::file_utils::{ensure_path_within_working_dir, strip_unc_prefix};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileOpsOperation {
    /// Operation to perform: copy, move, delete, rename
    #[schemars(description = "Operation: copy, move, delete, or rename")]
    pub action: String,
    /// Source file path (required for all actions)
    #[schemars(description = "Source file path")]
    pub source: String,
    /// Target path (required for copy/move) or new name (required for rename)
    #[schemars(description = "Target path or new name (for copy/move/rename)")]
    pub target: Option<String>,
    /// Overwrite if destination exists (default: false, for copy/move)
    #[schemars(description = "Overwrite if destination exists (default: false)")]
    pub overwrite: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileOpsParams {
    /// List of file operations to perform concurrently
    #[schemars(description = "List of file operations to perform concurrently")]
    pub operations: Vec<FileOpsOperation>,
}

#[derive(Debug, Serialize)]
struct FileOpsResult {
    action: String,
    source: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

pub async fn file_ops(
    params: Parameters<FileOpsParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;

    let mut futures = Vec::new();
    for op in params.operations {
        futures.push(process_single_op(op, working_dir));
    }

    let results = futures::future::join_all(futures).await;

    let json = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

async fn process_single_op(op: FileOpsOperation, working_dir: &Path) -> FileOpsResult {
    let FileOpsOperation { action, source, target, overwrite } = op;
    let action_str = action.to_lowercase();
    let source_path_raw = Path::new(&source);
    let overwrite = overwrite.unwrap_or(false);

    // Security check for source
    let source_path = match ensure_path_within_working_dir(source_path_raw, working_dir) {
        Ok(p) => p,
        Err(e) => {
            return FileOpsResult {
                action,
                source,
                success: false,
                error: Some(e),
                message: None,
            }
        }
    };

    let result = match action_str.as_str() {
        "copy" => {
            let target = match target.as_deref() {
                Some(t) => t,
                None => {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some("'target' is required for copy operation".to_string()),
                        message: None,
                    }
                }
            };
            let target_path = match ensure_path_within_working_dir(Path::new(target), working_dir) {
                Ok(p) => p,
                Err(e) => {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some(e),
                        message: None,
                    }
                }
            };

            if target_path.exists() && !overwrite {
                return FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "Destination file '{}' already exists. Set overwrite=true to replace.",
                        target
                    )),
                    message: None,
                };
            }
            if let Some(parent) = target_path.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some(format!("Failed to create parent directories: {}", e)),
                        message: None,
                    };
                }
            }
            match tokio::fs::copy(&source_path, &target_path).await {
                Ok(_) => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: true,
                    error: None,
                    message: Some(format!(
                        "File '{}' copied to '{}' successfully.",
                        strip_unc_prefix(&source_path.to_string_lossy()),
                        strip_unc_prefix(&target_path.to_string_lossy())
                    )),
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("Source file '{}' does not exist", source)),
                    message: None,
                },
                Err(e) => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("Failed to copy file: {}", e)),
                    message: None,
                },
            }
        }
        "move" => {
            let target = match target.as_deref() {
                Some(t) => t,
                None => {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some("'target' is required for move operation".to_string()),
                        message: None,
                    }
                }
            };
            let target_path = match ensure_path_within_working_dir(Path::new(target), working_dir) {
                Ok(p) => p,
                Err(e) => {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some(e),
                        message: None,
                    }
                }
            };

            if target_path.exists() && !overwrite {
                return FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "Destination file '{}' already exists. Set overwrite=true to replace.",
                        target
                    )),
                    message: None,
                };
            }
            if let Some(parent) = target_path.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some(format!("Failed to create parent directories: {}", e)),
                        message: None,
                    };
                }
            }
            // Try atomic rename first, fallback to copy+delete for cross-filesystem moves
            let rename_result = tokio::fs::rename(&source_path, &target_path).await;
            match rename_result {
                Ok(_) => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: true,
                    error: None,
                    message: Some(format!(
                        "File '{}' moved to '{}' successfully.",
                        strip_unc_prefix(&source_path.to_string_lossy()),
                        strip_unc_prefix(&target_path.to_string_lossy())
                    )),
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("Source file '{}' does not exist", source)),
                    message: None,
                },
                Err(e) => {
                    // Fallback: copy + delete for cross-filesystem moves
                    match tokio::fs::copy(&source_path, &target_path).await {
                        Ok(_) => {
                            match tokio::fs::remove_file(&source_path).await {
                                Ok(_) => FileOpsResult {
                                    action: action,
                                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                                    success: true,
                                    error: None,
                                    message: Some(format!(
                                        "File '{}' moved to '{}' successfully (cross-filesystem).",
                                        strip_unc_prefix(&source_path.to_string_lossy()),
                                        strip_unc_prefix(&target_path.to_string_lossy())
                                    )),
                                },
                                Err(remove_err) => FileOpsResult {
                                    action: action,
                                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                                    success: false,
                                    error: Some(format!(
                                        "Copied to target but failed to remove source: {}",
                                        remove_err
                                    )),
                                    message: None,
                                },
                            }
                        }
                        Err(copy_err) if copy_err.kind() == std::io::ErrorKind::NotFound => FileOpsResult {
                            action: action,
                            source: strip_unc_prefix(&source_path.to_string_lossy()),
                            success: false,
                            error: Some(format!("Source file '{}' does not exist", source)),
                            message: None,
                        },
                        Err(copy_err) => FileOpsResult {
                            action: action,
                            source: strip_unc_prefix(&source_path.to_string_lossy()),
                            success: false,
                            error: Some(format!("Failed to move file: {} (rename failed: {})", copy_err, e)),
                            message: None,
                        },
                    }
                }
            }
        }
        "delete" => {
            match tokio::fs::remove_file(&source_path).await {
                Ok(_) => {
                    let parent = source_path.parent();
                    let parent_info = if let Some(parent) = parent {
                        format!(" Parent directory: {}", strip_unc_prefix(&parent.to_string_lossy()))
                    } else {
                        String::new()
                    };
                    FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: true,
                        error: None,
                        message: Some(format!(
                            "File '{}' deleted successfully.{}",
                            strip_unc_prefix(&source_path.to_string_lossy()),
                            parent_info
                        )),
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("File '{}' does not exist", source)),
                    message: None,
                },
                Err(e) => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("Failed to delete file: {}", e)),
                    message: None,
                },
            }
        }
        "rename" => {
            let new_name = match target.as_deref() {
                Some(t) => t,
                None => {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some("'target' (new name) is required for rename operation".to_string()),
                        message: None,
                    }
                }
            };

            let parent = match source_path.parent() {
                Some(p) => p,
                None => {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some("Cannot rename root file".to_string()),
                        message: None,
                    }
                }
            };
            let new_path = parent.join(new_name);
            // Security check: ensure rename target stays within working directory
            let new_path = match ensure_path_within_working_dir(&new_path, working_dir) {
                Ok(p) => p,
                Err(e) => {
                    return FileOpsResult {
                        action: action,
                        source: strip_unc_prefix(&source_path.to_string_lossy()),
                        success: false,
                        error: Some(e),
                        message: None,
                    }
                }
            };

            if new_path.exists() {
                return FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "A file with name '{}' already exists in the same directory",
                        new_name
                    )),
                    message: None,
                };
            }

            match tokio::fs::rename(&source_path, &new_path).await {
                Ok(_) => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: true,
                    error: None,
                    message: Some(format!(
                        "File '{}' renamed to '{}' successfully.",
                        strip_unc_prefix(&source_path.to_string_lossy()),
                        strip_unc_prefix(&new_path.to_string_lossy())
                    )),
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("File '{}' does not exist", source)),
                    message: None,
                },
                Err(e) => FileOpsResult {
                    action: action,
                    source: strip_unc_prefix(&source_path.to_string_lossy()),
                    success: false,
                    error: Some(format!("Failed to rename file: {}", e)),
                    message: None,
                },
            }
        }
        _ => FileOpsResult {
            action: action,
            source: strip_unc_prefix(&source_path.to_string_lossy()),
            success: false,
            error: Some(format!(
                "Invalid action '{}'. Use 'copy', 'move', 'delete', or 'rename'.",
                action_str
            )),
            message: None,
        },
    };

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_ops_copy() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");
        fs::write(&source, "test content").unwrap();

        let params = FileOpsParams {
            operations: vec![FileOpsOperation {
                action: "copy".to_string(),
                source: source.to_string_lossy().to_string(),
                target: Some(dest.to_string_lossy().to_string()),
                overwrite: Some(false),
            }],
        };

        let result = file_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "test content");
    }

    #[tokio::test]
    async fn test_file_ops_move() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");
        fs::write(&source, "test content").unwrap();

        let params = FileOpsParams {
            operations: vec![FileOpsOperation {
                action: "move".to_string(),
                source: source.to_string_lossy().to_string(),
                target: Some(dest.to_string_lossy().to_string()),
                overwrite: Some(false),
            }],
        };

        let result = file_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(!source.exists());
        assert!(dest.exists());
    }

    #[tokio::test]
    async fn test_file_ops_delete() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.txt");
        fs::write(&file, "test content").unwrap();

        let params = FileOpsParams {
            operations: vec![FileOpsOperation {
                action: "delete".to_string(),
                source: file.to_string_lossy().to_string(),
                target: None,
                overwrite: None,
            }],
        };

        let result = file_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn test_file_ops_rename() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("old.txt");
        fs::write(&file, "test content").unwrap();

        let params = FileOpsParams {
            operations: vec![FileOpsOperation {
                action: "rename".to_string(),
                source: file.to_string_lossy().to_string(),
                target: Some("new.txt".to_string()),
                overwrite: None,
            }],
        };

        let result = file_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(!file.exists());
        assert!(temp_dir.path().join("new.txt").exists());
    }

    #[tokio::test]
    async fn test_file_ops_invalid_action() {
        let temp_dir = TempDir::new().unwrap();
        let params = FileOpsParams {
            operations: vec![FileOpsOperation {
                action: "invalid".to_string(),
                source: "/tmp/test.txt".to_string(),
                target: None,
                overwrite: None,
            }],
        };

        let result = file_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"success\": false"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_ops_concurrent() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("a.txt");
        let file2 = temp_dir.path().join("b.txt");
        fs::write(&file1, "A").unwrap();
        fs::write(&file2, "B").unwrap();

        let params = FileOpsParams {
            operations: vec![
                FileOpsOperation {
                    action: "delete".to_string(),
                    source: file1.to_string_lossy().to_string(),
                    target: None,
                    overwrite: None,
                },
                FileOpsOperation {
                    action: "delete".to_string(),
                    source: file2.to_string_lossy().to_string(),
                    target: None,
                    overwrite: None,
                },
            ],
        };

        let result = file_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(!file1.exists());
        assert!(!file2.exists());
    }
}
