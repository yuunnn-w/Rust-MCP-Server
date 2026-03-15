use crate::utils::file_utils::ensure_path_within_working_dir;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

// File copy operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileCopyParams {
    /// Source file path
    #[schemars(description = "The source file path")]
    pub source: String,
    /// Destination file path
    #[schemars(description = "The destination file path")]
    pub destination: String,
    /// Overwrite if destination exists (default: false)
    #[schemars(description = "Overwrite if destination exists (default: false)")]
    pub overwrite: Option<bool>,
}

pub async fn file_copy(
    params: Parameters<FileCopyParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let source = Path::new(&params.source);
    let destination = Path::new(&params.destination);
    let overwrite = params.overwrite.unwrap_or(false);

    // Security checks
    let source_path = ensure_path_within_working_dir(source, working_dir)?;
    let dest_path = ensure_path_within_working_dir(destination, working_dir)?;

    // Check source exists
    if !source_path.exists() {
        return Err(format!("Source file '{}' does not exist", params.source));
    }

    if !source_path.is_file() {
        return Err(format!("Source path '{}' is not a file", params.source));
    }

    // Check destination
    if dest_path.exists() && !overwrite {
        return Err(format!(
            "Destination file '{}' already exists. Set overwrite=true to replace.",
            params.destination
        ));
    }

    // Create parent directories
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create parent directories: {}", e))?;
    }

    // Copy file
    tokio::fs::copy(&source_path, &dest_path)
        .await
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!(
            "File '{}' copied to '{}' successfully.",
            source_path.display(),
            dest_path.display()
        ),
    )]))
}

// File move operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileMoveParams {
    /// Source file path
    #[schemars(description = "The source file path")]
    pub source: String,
    /// Destination file path
    #[schemars(description = "The destination file path")]
    pub destination: String,
    /// Overwrite if destination exists (default: false)
    #[schemars(description = "Overwrite if destination exists (default: false)")]
    pub overwrite: Option<bool>,
}

pub async fn file_move(
    params: Parameters<FileMoveParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let source = Path::new(&params.source);
    let destination = Path::new(&params.destination);
    let overwrite = params.overwrite.unwrap_or(false);

    // Security checks
    let source_path = ensure_path_within_working_dir(source, working_dir)?;
    let dest_path = ensure_path_within_working_dir(destination, working_dir)?;

    // Check source exists
    if !source_path.exists() {
        return Err(format!("Source file '{}' does not exist", params.source));
    }

    if !source_path.is_file() {
        return Err(format!("Source path '{}' is not a file", params.source));
    }

    // Check destination
    if dest_path.exists() && !overwrite {
        return Err(format!(
            "Destination file '{}' already exists. Set overwrite=true to replace.",
            params.destination
        ));
    }

    // Create parent directories
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create parent directories: {}", e))?;
    }

    // Move file
    tokio::fs::rename(&source_path, &dest_path)
        .await
        .map_err(|e| format!("Failed to move file: {}", e))?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!(
            "File '{}' moved to '{}' successfully.",
            source_path.display(),
            dest_path.display()
        ),
    )]))
}

// File delete operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileDeleteParams {
    /// File path to delete
    #[schemars(description = "The file path to delete")]
    pub path: String,
}

pub async fn file_delete(
    params: Parameters<FileDeleteParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);

    // Security check
    let file_path = ensure_path_within_working_dir(path, working_dir)?;

    // Check file exists
    if !file_path.exists() {
        return Err(format!("File '{}' does not exist", params.path));
    }

    if !file_path.is_file() {
        return Err(format!("Path '{}' is not a file", params.path));
    }

    // Delete file
    tokio::fs::remove_file(&file_path)
        .await
        .map_err(|e| format!("Failed to delete file: {}", e))?;

    // Return parent directory structure
    let parent = file_path.parent();
    let parent_info = if let Some(parent) = parent {
        format!(" Parent directory: {}", parent.display())
    } else {
        String::new()
    };

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("File '{}' deleted successfully. {}", file_path.display(), parent_info),
    )]))
}

// File rename operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileRenameParams {
    /// Current file path
    #[schemars(description = "The current file path")]
    pub path: String,
    /// New file name (not full path, just the new name)
    #[schemars(description = "The new file name (not full path)")]
    pub new_name: String,
}

pub async fn file_rename(
    params: Parameters<FileRenameParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);

    // Security check
    let file_path = ensure_path_within_working_dir(path, working_dir)?;

    // Check file exists
    if !file_path.exists() {
        return Err(format!("File '{}' does not exist", params.path));
    }

    if !file_path.is_file() {
        return Err(format!("Path '{}' is not a file", params.path));
    }

    // Get parent directory
    let parent = file_path
        .parent()
        .ok_or_else(|| "Cannot rename root file".to_string())?;

    // Create new path
    let new_path = parent.join(&params.new_name);

    // Check if new path already exists
    if new_path.exists() {
        return Err(format!(
            "A file with name '{}' already exists in the same directory",
            params.new_name
        ));
    }

    // Rename file
    tokio::fs::rename(&file_path, &new_path)
        .await
        .map_err(|e| format!("Failed to rename file: {}", e))?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!(
            "File '{}' renamed to '{}' successfully.",
            file_path.display(),
            new_path.display()
        ),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_copy() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");
        fs::write(&source, "test content").unwrap();

        let params = FileCopyParams {
            source: source.to_string_lossy().to_string(),
            destination: dest.to_string_lossy().to_string(),
            overwrite: Some(false),
        };

        let result = file_copy(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "test content");
    }

    #[tokio::test]
    async fn test_file_move() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");
        fs::write(&source, "test content").unwrap();

        let params = FileMoveParams {
            source: source.to_string_lossy().to_string(),
            destination: dest.to_string_lossy().to_string(),
            overwrite: Some(false),
        };

        let result = file_move(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(!source.exists());
        assert!(dest.exists());
    }

    #[tokio::test]
    async fn test_file_delete() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("test.txt");
        fs::write(&file, "test content").unwrap();

        let params = FileDeleteParams {
            path: file.to_string_lossy().to_string(),
        };

        let result = file_delete(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn test_file_rename() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("old.txt");
        fs::write(&file, "test content").unwrap();

        let params = FileRenameParams {
            path: file.to_string_lossy().to_string(),
            new_name: "new.txt".to_string(),
        };

        let result = file_rename(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        assert!(!file.exists());
        assert!(temp_dir.path().join("new.txt").exists());
    }
}
