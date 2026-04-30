use crate::utils::file_utils::{
    format_datetime, format_file_size, get_text_file_info_sync,
    glob_match, resolve_path, should_skip_dir, strip_unc_prefix,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirListParams {
    /// Directory path to list
    #[schemars(description = "The directory path to list")]
    pub path: String,
    /// Maximum depth to traverse (default: 2, max: 5)
    #[schemars(description = "Maximum depth to traverse (default: 2, max: 5)")]
    pub max_depth: Option<usize>,
    /// Include hidden files (default: false)
    #[schemars(description = "Include hidden files (default: false)")]
    pub include_hidden: Option<bool>,
    /// Glob pattern to filter entries, e.g. "*.rs" (default: no filter)
    #[schemars(description = "Glob pattern to filter entries, e.g. '*.rs'")]
    pub pattern: Option<String>,
    /// Brief mode: only return name, path, is_dir (default: true)
    #[schemars(description = "Brief mode: only return name, path, is_dir (default: true)")]
    pub brief: Option<bool>,
    /// Sort by: "name" (default), "type", "size", "modified"
    #[schemars(description = "Sort by: name (default), type, size, modified")]
    pub sort_by: Option<String>,
    /// Flatten output: return a flat list instead of nested tree (default: false)
    /// When true, all entries across all depths are returned as a single flat array.
    #[schemars(description = "Flatten output: return a flat list instead of nested tree (default: false)")]
    pub flatten: Option<bool>,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    depth: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    char_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<FileEntry>>,
}

#[derive(Debug, Serialize)]
struct DirListResponse {
    name: String,
    path: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<FileEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entries: Option<Vec<FileEntry>>,
    summary: Summary,
}

#[derive(Debug, Serialize)]
struct Summary {
    total_files: usize,
    total_dirs: usize,
    truncated: bool,
    max_depth_reached: bool,
}

pub async fn dir_list(
    params: Parameters<DirListParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let max_depth = params.max_depth.unwrap_or(2).min(5);
    let include_hidden = params.include_hidden.unwrap_or(false);
    let brief = params.brief.unwrap_or(true);
    let sort_by = params.sort_by.as_deref().unwrap_or("name").to_lowercase();
    let pattern = params.pattern.as_deref();
    let flatten = params.flatten.unwrap_or(false);

    let path = Path::new(&params.path);
    let canonical_path = resolve_path(path, working_dir)?;

    if !canonical_path.exists() {
        return Err(format!("Path '{}' does not exist", params.path));
    }
    if !canonical_path.is_dir() {
        return Err(format!("Path '{}' is not a directory", params.path));
    }

    let mut total_files = 0usize;
    let mut total_dirs = 0usize;

    let response = if flatten {
        let mut entries = Vec::new();
        let (truncated, max_depth_reached) = list_directory_flat(
            &canonical_path,
            &canonical_path,
            max_depth,
            0,
            include_hidden,
            brief,
            &sort_by,
            pattern,
            &mut total_files,
            &mut total_dirs,
            &mut entries,
        )
        .map_err(|e| format!("Failed to list directory: {}", e))?;

        // Sort flat entries
        sort_entries(&mut entries, &sort_by, Some(&canonical_path));

        let metadata = std::fs::metadata(&canonical_path).ok();
        DirListResponse {
            name: canonical_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| canonical_path.to_string_lossy().to_string()),
            path: strip_unc_prefix(&canonical_path.to_string_lossy()),
            is_dir: true,
            size: None,
            modified: metadata.and_then(|m| m.modified().ok()).map(format_datetime),
            children: None,
            entries: Some(entries),
            summary: Summary {
                total_files,
                total_dirs,
                truncated,
                max_depth_reached,
            },
        }
    } else {
        let (entry, truncated, max_depth_reached) = list_directory_recursive(
            &canonical_path,
            max_depth,
            0,
            include_hidden,
            brief,
            &sort_by,
            pattern,
            &mut total_files,
            &mut total_dirs,
        )
        .map_err(|e| format!("Failed to list directory: {}", e))?;

        DirListResponse {
            name: entry.name,
            path: entry.path,
            is_dir: entry.is_dir,
            size: entry.size,
            modified: entry.modified,
            children: entry.children,
            entries: None,
            summary: Summary {
                total_files,
                total_dirs,
                truncated,
                max_depth_reached,
            },
        }
    };

    let json = serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

fn list_directory_flat(
    root_path: &Path,
    path: &Path,
    max_depth: usize,
    current_depth: usize,
    include_hidden: bool,
    brief: bool,
    sort_by: &str,
    pattern: Option<&str>,
    total_files: &mut usize,
    total_dirs: &mut usize,
    entries: &mut Vec<FileEntry>,
) -> Result<(bool, bool), std::io::Error> {
    if current_depth > max_depth {
        return Ok((false, true));
    }

    let dir_entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => return Err(e),
    };

    let mut truncated = false;

    for item in dir_entries {
        let item = item?;
        let name = item.file_name();
        let name_str = name.to_string_lossy();

        if !include_hidden && name_str.starts_with('.') {
            continue;
        }

        let item_is_dir = item.file_type()?.is_dir();
        if item_is_dir && should_skip_dir(&name_str) {
            continue;
        }

        if let Some(pat) = pattern {
            if !glob_match(pat, &name_str) {
                continue;
            }
        }

        if entries.len() >= 500 {
            truncated = true;
            break;
        }

        let metadata = item.metadata()?;
        let is_dir = metadata.is_dir();
        let entry_path = item.path();
        let rel_path = entry_path.strip_prefix(root_path).unwrap_or(&entry_path);

        let size = if is_dir || brief {
            None
        } else {
            Some(format_file_size(metadata.len()))
        };

        let modified = if brief {
            None
        } else {
            metadata.modified().ok().map(format_datetime)
        };

        let (char_count, line_count) = if !is_dir {
            get_text_file_info_sync(&entry_path)
                .map(|info| (Some(info.char_count), Some(info.line_count)))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        entries.push(FileEntry {
            name: name_str.to_string(),
            path: strip_unc_prefix(&rel_path.to_string_lossy()),
            is_dir,
            depth: Some(current_depth),
            size,
            modified,
            char_count,
            line_count,
            children: None,
        });

        if is_dir {
            *total_dirs += 1;
            let (child_truncated, child_depth) = list_directory_flat(
                root_path,
                &entry_path,
                max_depth,
                current_depth + 1,
                include_hidden,
                brief,
                sort_by,
                pattern,
                total_files,
                total_dirs,
                entries,
            )?;
            if child_truncated {
                truncated = true;
            }
            if child_depth {
                return Ok((truncated, true));
            }
        } else {
            *total_files += 1;
        }
    }

    Ok((truncated, false))
}

fn list_directory_recursive(
    path: &Path,
    max_depth: usize,
    current_depth: usize,
    include_hidden: bool,
    brief: bool,
    sort_by: &str,
    pattern: Option<&str>,
    total_files: &mut usize,
    total_dirs: &mut usize,
) -> Result<(FileEntry, bool, bool), std::io::Error> {
    let metadata = std::fs::metadata(path)?;
    let is_dir = metadata.is_dir();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let size = if is_dir || brief {
        None
    } else {
        Some(format_file_size(metadata.len()))
    };

    let modified = if brief {
        None
    } else {
        metadata.modified().ok().map(format_datetime)
    };

    let (char_count, line_count) = if !is_dir {
        get_text_file_info_sync(path)
            .map(|info| (Some(info.char_count), Some(info.line_count)))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    let mut entry = FileEntry {
        name,
        path: strip_unc_prefix(&path.to_string_lossy()),
        is_dir,
        depth: None,
        size,
        modified,
        char_count,
        line_count,
        children: None,
    };

    let mut truncated = false;
    let mut max_depth_reached = false;

    if is_dir && current_depth < max_depth {
        let mut children = Vec::new();

        for item in std::fs::read_dir(path)? {
            let item = item?;
            let name = item.file_name();
            let name_str = name.to_string_lossy();

            if !include_hidden && name_str.starts_with('.') {
                continue;
            }

            // Skip known noise directories
            let item_is_dir = item.file_type()?.is_dir();
            if item_is_dir && should_skip_dir(&name_str) {
                continue;
            }

            // Pattern filter
            if let Some(pat) = pattern {
                if !glob_match(pat, &name_str) {
                    continue;
                }
            }

            // Limit children count
            if children.len() >= 100 {
                truncated = true;
                break;
            }

            match list_directory_recursive(
                &item.path(),
                max_depth,
                current_depth + 1,
                include_hidden,
                brief,
                sort_by,
                pattern,
                total_files,
                total_dirs,
            ) {
                Ok((child, child_truncated, child_depth)) => {
                    if child_truncated {
                        truncated = true;
                    }
                    if child_depth {
                        max_depth_reached = true;
                    }
                    if child.is_dir {
                        *total_dirs += 1;
                    } else {
                        *total_files += 1;
                    }
                    children.push(child);
                }
                Err(_) => continue,
            }
        }

        // Sort
        sort_entries(&mut children, sort_by, None);

        entry.children = Some(children);
    } else if is_dir && current_depth >= max_depth {
        max_depth_reached = true;
        entry.children = Some(vec![]);
    }

    Ok((entry, truncated, max_depth_reached))
}

fn sort_entries(children: &mut Vec<FileEntry>, sort_by: &str, base_path: Option<&Path>) {
    children.sort_by(|a, b| match sort_by {
        "type" => match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        },
        "size" => {
            let path_a = base_path
                .map(|bp| bp.join(&a.path))
                .unwrap_or_else(|| PathBuf::from(&a.path));
            let path_b = base_path
                .map(|bp| bp.join(&b.path))
                .unwrap_or_else(|| PathBuf::from(&b.path));
            let size_a = std::fs::metadata(&path_a).map(|m| m.len()).unwrap_or(0);
            let size_b = std::fs::metadata(&path_b).map(|m| m.len()).unwrap_or(0);
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => size_b.cmp(&size_a).then_with(|| a.name.cmp(&b.name)),
            }
        }
        "modified" => {
            let path_a = base_path
                .map(|bp| bp.join(&a.path))
                .unwrap_or_else(|| PathBuf::from(&a.path));
            let path_b = base_path
                .map(|bp| bp.join(&b.path))
                .unwrap_or_else(|| PathBuf::from(&b.path));
            let time_a = std::fs::metadata(&path_a).and_then(|m| m.modified()).ok();
            let time_b = std::fs::metadata(&path_b).and_then(|m| m.modified()).ok();
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (time_b, time_a) {
                    (Some(tb), Some(ta)) => tb.cmp(&ta).then_with(|| a.name.cmp(&b.name)),
                    _ => a.name.cmp(&b.name),
                },
            }
        }
        _ => {
            // Default "name": dirs first, then files by name
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_dir_list() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        fs::create_dir(dir_path.join("subdir")).unwrap();
        fs::write(dir_path.join("file1.txt"), "content1").unwrap();
        fs::write(dir_path.join("subdir/file2.txt"), "content2\nline2").unwrap();

        let params = DirListParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(2),
            include_hidden: Some(false),
            pattern: None,
            brief: Some(true),
            sort_by: None,
            flatten: Some(false),
        };

        let result = dir_list(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                // Check UNC prefix is stripped
                assert!(!text.text.contains("\\\\?\\"));
                // Check text file info is present
                assert!(text.text.contains("char_count") || text.text.contains("line_count"));
            }
        }
    }

    #[tokio::test]
    async fn test_dir_list_flatten() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        fs::create_dir(dir_path.join("subdir")).unwrap();
        fs::write(dir_path.join("file1.txt"), "content1").unwrap();
        fs::write(dir_path.join("subdir/file2.txt"), "content2").unwrap();

        let params = DirListParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(2),
            include_hidden: Some(false),
            pattern: None,
            brief: Some(true),
            sort_by: None,
            flatten: Some(true),
        };

        let result = dir_list(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("entries"));
                assert!(!text.text.contains("children"));
            }
        }
    }

    #[tokio::test]
    async fn test_dir_list_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        fs::write(dir_path.join("a.rs"), "").unwrap();
        fs::write(dir_path.join("b.txt"), "").unwrap();
        fs::write(dir_path.join("c.rs"), "").unwrap();

        let params = DirListParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(1),
            include_hidden: Some(false),
            pattern: Some("*.rs".to_string()),
            brief: Some(true),
            sort_by: None,
            flatten: Some(false),
        };

        let result = dir_list(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_glob_match() {
        use crate::utils::file_utils::glob_match;
        assert!(glob_match("*.rs", "test.rs"));
        assert!(!glob_match("*.rs", "test.txt"));
        assert!(glob_match("file?.txt", "file1.txt"));
        assert!(glob_match("*", "anything"));
    }
}
