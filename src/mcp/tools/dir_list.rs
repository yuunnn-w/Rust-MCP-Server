use crate::utils::file_utils::{format_datetime, format_file_size};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirListParams {
    /// Directory path to list
    #[schemars(description = "The directory path to list")]
    pub path: String,
    /// Maximum depth to traverse (default: 3, max: 5)
    #[schemars(description = "Maximum depth to traverse (default: 3)")]
    pub max_depth: Option<usize>,
    /// Include hidden files (default: false)
    #[schemars(description = "Include hidden files (default: false)")]
    pub include_hidden: Option<bool>,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: Option<String>,
    modified: Option<String>,
    children: Option<Vec<FileEntry>>,
}

pub async fn dir_list(params: Parameters<DirListParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let max_depth = params.max_depth.unwrap_or(3).min(5);
    let include_hidden = params.include_hidden.unwrap_or(false);
    let path = Path::new(&params.path);

    if !path.exists() {
        return Err(format!("Path '{}' does not exist", params.path));
    }

    if !path.is_dir() {
        return Err(format!("Path '{}' is not a directory", params.path));
    }

    let entry = list_directory_recursive(path, max_depth, 0, include_hidden)
        .map_err(|e| format!("Failed to list directory: {}", e))?;

    let json = serde_json::to_string_pretty(&entry).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

fn list_directory_recursive(
    path: &Path,
    max_depth: usize,
    current_depth: usize,
    include_hidden: bool,
) -> Result<FileEntry, std::io::Error> {
    let metadata = std::fs::metadata(path)?;
    let is_dir = metadata.is_dir();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let size = if is_dir {
        None
    } else {
        Some(format_file_size(metadata.len()))
    };

    let modified = metadata
        .modified()
        .ok()
        .map(format_datetime);

    let mut entry = FileEntry {
        name,
        path: path.to_string_lossy().to_string(),
        is_dir,
        size,
        modified,
        children: None,
    };

    if is_dir && current_depth < max_depth {
        let mut children = Vec::new();
        let truncated = Arc::new(std::sync::atomic::AtomicBool::new(false));
        
        for item in std::fs::read_dir(path)? {
            let item = item?;
            let name = item.file_name();
            let name_str = name.to_string_lossy();
            
            // Skip hidden files unless include_hidden is true
            if !include_hidden && name_str.starts_with('.') {
                continue;
            }
            
            // Limit number of children to prevent overwhelming output
            if children.len() >= 100 {
                truncated.store(true, std::sync::atomic::Ordering::SeqCst);
                break;
            }
            
            match list_directory_recursive(&item.path(), max_depth, current_depth + 1, include_hidden) {
                Ok(child) => children.push(child),
                Err(_) => continue, // Skip entries we can't read
            }
        }
        
        children.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        entry.children = Some(children);
    } else if is_dir && current_depth >= max_depth {
        // Mark as truncated
        entry.children = Some(vec![]);
    }

    Ok(entry)
}

use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_dir_list() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create some test files and directories
        fs::create_dir(dir_path.join("subdir")).unwrap();
        fs::write(dir_path.join("file1.txt"), "content1").unwrap();
        fs::write(dir_path.join("subdir/file2.txt"), "content2").unwrap();

        let params = DirListParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(2),
            include_hidden: Some(false),
        };

        let result = dir_list(Parameters(params)).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(1024), "1.00 KB");
    }
}
