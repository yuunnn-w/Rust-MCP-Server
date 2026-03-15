use crate::utils::file_utils::ensure_path_within_working_dir;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileWriteParams {
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

pub async fn file_write(
    params: Parameters<FileWriteParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);
    let mode = params.mode.as_deref().unwrap_or("new");

    // Security check: ensure path is within working directory
    let canonical_path = ensure_path_within_working_dir(path, working_dir)?;

    // Check mode-specific conditions
    match mode {
        "new" => {
            if canonical_path.exists() {
                return Err(format!(
                    "File '{}' already exists. Use 'overwrite' or 'append' mode.",
                    params.path
                ));
            }
        }
        "append" | "overwrite" => {
            // These modes are fine with existing files
        }
        _ => return Err(format!("Invalid mode '{}'. Use 'new', 'append', or 'overwrite'.", mode)),
    }

    // Create parent directories if needed
    if let Some(parent) = canonical_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create parent directories: {}", e))?;
    }

    // Write file based on mode
    match mode {
        "new" | "overwrite" => {
            tokio::fs::write(&canonical_path, &params.content)
                .await
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }
        "append" => {
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&canonical_path)
                .await
                .map_err(|e| format!("Failed to open file for append: {}", e))?;

            file.write_all(params.content.as_bytes())
                .await
                .map_err(|e| format!("Failed to append to file: {}", e))?;
        }
        _ => unreachable!(),
    }

    let action = match mode {
        "new" => "created",
        "append" => "appended to",
        "overwrite" => "overwritten",
        _ => "written",
    };

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!(
            "File '{}' {} successfully ({} bytes).",
            canonical_path.display(),
            action,
            params.content.len()
        ),
    )]))
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
            path: file_path.to_string_lossy().to_string(),
            content: "Hello, World!".to_string(),
            mode: Some("new".to_string()),
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
            path: file_path.to_string_lossy().to_string(),
            content: "Appended".to_string(),
            mode: Some("append".to_string()),
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
            path: "/etc/test.txt".to_string(),
            content: "test".to_string(),
            mode: Some("new".to_string()),
        };

        let result = file_write(Parameters(params), temp_dir.path()).await;
        assert!(result.is_err());
    }
}
