use crate::utils::file_utils::ensure_path_within_working_dir;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileOpsParams {
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

pub async fn file_ops(
    params: Parameters<FileOpsParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let action = params.action.to_lowercase();
    let source = Path::new(&params.source);
    let overwrite = params.overwrite.unwrap_or(false);

    // Security check for source
    let source_path = ensure_path_within_working_dir(source, working_dir)?;

    match action.as_str() {
        "copy" => {
            let target = params
                .target
                .as_deref()
                .ok_or("'target' is required for copy operation")?;
            let target_path = ensure_path_within_working_dir(Path::new(target), working_dir)?;

            if !source_path.exists() {
                return Err(format!("Source file '{}' does not exist", params.source));
            }
            if !source_path.is_file() {
                return Err(format!("Source path '{}' is not a file", params.source));
            }
            if target_path.exists() && !overwrite {
                return Err(format!(
                    "Destination file '{}' already exists. Set overwrite=true to replace.",
                    target
                ));
            }
            if let Some(parent) = target_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("Failed to create parent directories: {}", e))?;
            }
            tokio::fs::copy(&source_path, &target_path)
                .await
                .map_err(|e| format!("Failed to copy file: {}", e))?;

            Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                format!(
                    "File '{}' copied to '{}' successfully.",
                    source_path.display(),
                    target_path.display()
                ),
            )]))
        }
        "move" => {
            let target = params
                .target
                .as_deref()
                .ok_or("'target' is required for move operation")?;
            let target_path = ensure_path_within_working_dir(Path::new(target), working_dir)?;

            if !source_path.exists() {
                return Err(format!("Source file '{}' does not exist", params.source));
            }
            if !source_path.is_file() {
                return Err(format!("Source path '{}' is not a file", params.source));
            }
            if target_path.exists() && !overwrite {
                return Err(format!(
                    "Destination file '{}' already exists. Set overwrite=true to replace.",
                    target
                ));
            }
            if let Some(parent) = target_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("Failed to create parent directories: {}", e))?;
            }
            tokio::fs::rename(&source_path, &target_path)
                .await
                .map_err(|e| format!("Failed to move file: {}", e))?;

            Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                format!(
                    "File '{}' moved to '{}' successfully.",
                    source_path.display(),
                    target_path.display()
                ),
            )]))
        }
        "delete" => {
            if !source_path.exists() {
                return Err(format!("File '{}' does not exist", params.source));
            }
            if !source_path.is_file() {
                return Err(format!("Path '{}' is not a file", params.source));
            }
            tokio::fs::remove_file(&source_path)
                .await
                .map_err(|e| format!("Failed to delete file: {}", e))?;

            let parent = source_path.parent();
            let parent_info = if let Some(parent) = parent {
                format!(" Parent directory: {}", parent.display())
            } else {
                String::new()
            };

            Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                format!("File '{}' deleted successfully.{}", source_path.display(), parent_info),
            )]))
        }
        "rename" => {
            let new_name = params
                .target
                .as_deref()
                .ok_or("'target' (new name) is required for rename operation")?;

            if !source_path.exists() {
                return Err(format!("File '{}' does not exist", params.source));
            }
            if !source_path.is_file() {
                return Err(format!("Path '{}' is not a file", params.source));
            }

            let parent = source_path
                .parent()
                .ok_or_else(|| "Cannot rename root file".to_string())?;
            let new_path = parent.join(new_name);

            if new_path.exists() {
                return Err(format!(
                    "A file with name '{}' already exists in the same directory",
                    new_name
                ));
            }

            tokio::fs::rename(&source_path, &new_path)
                .await
                .map_err(|e| format!("Failed to rename file: {}", e))?;

            Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                format!(
                    "File '{}' renamed to '{}' successfully.",
                    source_path.display(),
                    new_path.display()
                ),
            )]))
        }
        _ => Err(format!(
            "Invalid action '{}'. Use 'copy', 'move', 'delete', or 'rename'.",
            params.action
        )),
    }
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
            action: "copy".to_string(),
            source: source.to_string_lossy().to_string(),
            target: Some(dest.to_string_lossy().to_string()),
            overwrite: Some(false),
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
            action: "move".to_string(),
            source: source.to_string_lossy().to_string(),
            target: Some(dest.to_string_lossy().to_string()),
            overwrite: Some(false),
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
            action: "delete".to_string(),
            source: file.to_string_lossy().to_string(),
            target: None,
            overwrite: None,
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
            action: "rename".to_string(),
            source: file.to_string_lossy().to_string(),
            target: Some("new.txt".to_string()),
            overwrite: None,
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
            action: "invalid".to_string(),
            source: "/tmp/test.txt".to_string(),
            target: None,
            overwrite: None,
        };

        let result = file_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_err());
    }
}
