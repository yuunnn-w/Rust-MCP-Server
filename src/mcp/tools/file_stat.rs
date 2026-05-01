use crate::utils::file_utils::{
    format_datetime, format_file_size, get_text_file_info, resolve_path, strip_unc_prefix,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileStatParams {
    /// Paths to files or directories (supports multiple paths)
    #[schemars(description = "Paths to files or directories (supports multiple paths)")]
    pub paths: Vec<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    is_text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    char_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding: Option<String>,
}

pub async fn file_stat(
    params: Parameters<FileStatParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let mut results = Vec::new();

    for path_str in &params.paths {
        let result = stat_single_path(path_str, working_dir).await;
        results.push(result);
    }

    let json = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

async fn stat_single_path(path_str: &str, working_dir: &Path) -> FileStatResult {
    let path = Path::new(path_str);

    let canonical_path = match resolve_path(path, working_dir) {
        Ok(p) => p,
        Err(_e) => {
            return FileStatResult {
                path: path_str.to_string(),
                name: Path::new(path_str)
                    .file_name()
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
                is_text: None,
                char_count: None,
                line_count: None,
                encoding: None,
            }
        }
    };

    // Check for broken symlinks: symlink_metadata succeeds even if target is missing
    let symlink_meta = tokio::fs::symlink_metadata(&canonical_path).await.ok();
    let is_broken_symlink = symlink_meta.as_ref().map(|m| m.is_symlink()).unwrap_or(false);

    if !canonical_path.exists() && !is_broken_symlink {
        return FileStatResult {
            path: strip_unc_prefix(&canonical_path.to_string_lossy()),
            name: canonical_path
                .file_name()
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
            is_text: None,
            char_count: None,
            line_count: None,
            encoding: None,
        };
    }

    let metadata = match tokio::fs::symlink_metadata(&canonical_path).await {
        Ok(m) => m,
        Err(_) => {
            return FileStatResult {
                path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                name: canonical_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                exists: true,
                file_type: Some("unknown".to_string()),
                size: None,
                size_human: None,
                readable: None,
                writable: None,
                executable: None,
                modified: None,
                created: None,
                accessed: None,
                is_symlink: None,
                is_text: None,
                char_count: None,
                line_count: None,
                encoding: None,
            }
        }
    };

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
    let size_human = size.map(|s| format_file_size(s));

    let permissions = final_metadata.as_ref().map(|m| m.permissions());
    #[cfg(unix)]
    let (readable, writable, executable) = {
        use std::os::unix::fs::PermissionsExt;
        permissions
            .map(|p| {
                let mode = p.mode();
                (
                    Some(mode & 0o444 != 0),
                    Some(mode & 0o222 != 0),
                    Some(mode & 0o111 != 0),
                )
            })
            .unwrap_or((None, None, None))
    };
    #[cfg(not(unix))]
    let (readable, writable, executable) = {
        permissions
            .map(|p| {
                (
                    Some(true), // Windows files are generally readable
                    Some(!p.readonly()),
                    None, // No simple executable bit on Windows
                )
            })
            .unwrap_or((None, None, None))
    };

    let modified = final_metadata
        .as_ref()
        .and_then(|m| m.modified().ok())
        .map(|t| format_datetime(t));
    let created = final_metadata
        .as_ref()
        .and_then(|m| m.created().ok())
        .map(|t| format_datetime(t));
    let accessed = final_metadata
        .as_ref()
        .and_then(|m| m.accessed().ok())
        .map(|t| format_datetime(t));

    // Text file info
    let (is_text, char_count, line_count, encoding) =
        if file_type.as_deref() == Some("file") {
            match get_text_file_info(&canonical_path).await {
                Some(info) => (Some(true), Some(info.char_count), Some(info.line_count), Some("utf-8".to_string())),
                None => (Some(false), None, None, None),
            }
        } else {
            (None, None, None, None)
        };

    FileStatResult {
        path: strip_unc_prefix(&canonical_path.to_string_lossy()),
        name: canonical_path
            .file_name()
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
        is_text,
        char_count,
        line_count,
        encoding,
    }
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
        fs::write(&file_path, "Hello, World!\nLine 2").unwrap();

        let params = FileStatParams {
            paths: vec![file_path.to_string_lossy().to_string()],
        };

        let result = file_stat(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("exists"));
                assert!(text.text.contains("file"));
                assert!(text.text.contains("size"));
                assert!(text.text.contains("is_text"));
                assert!(text.text.contains("char_count"));
                assert!(text.text.contains("line_count"));
                assert!(!text.text.contains("\\\\?\\"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_stat_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();

        let params = FileStatParams {
            paths: vec![dir_path.to_string_lossy().to_string()],
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
            paths: vec!["nonexistent_file.txt".to_string()],
        };

        let result = file_stat(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"exists\": false"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_stat_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("a.txt");
        let file2 = temp_dir.path().join("b.txt");
        fs::write(&file1, "Hello").unwrap();
        fs::write(&file2, "World\n!").unwrap();

        let params = FileStatParams {
            paths: vec![
                file1.to_string_lossy().to_string(),
                file2.to_string_lossy().to_string(),
            ],
        };

        let result = file_stat(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                // Should be an array with 2 items
                let arr: Vec<serde_json::Value> = serde_json::from_str(&text.text).unwrap();
                assert_eq!(arr.len(), 2);
            }
        }
    }
}
