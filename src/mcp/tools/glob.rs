use crate::utils::enhanced_glob::GlobMatcher;
use crate::utils::file_utils::{
    format_datetime, format_file_size, get_text_file_info_sync,
    glob_match, resolve_path, should_skip_dir, strip_unc_prefix,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GlobParams {
    /// Directory path to list
    #[schemars(description = "The directory path to list")]
    pub path: String,
    /// Maximum depth to traverse (default: 2, max: 10)
    #[schemars(description = "Maximum depth to traverse (default: 2, max: 10)")]
    pub max_depth: Option<usize>,
    /// Include hidden files (default: false)
    #[schemars(description = "Include hidden files (default: false)")]
    pub include_hidden: Option<bool>,
    /// Simple glob pattern to filter entries, e.g. "*.rs" (use patterns for multiple)
    #[schemars(description = "Simple glob pattern to filter entries, e.g. '*.rs' (use patterns for multiple)")]
    pub pattern: Option<String>,
    /// Brief mode: only return name, path, is_dir (default: true)
    #[schemars(description = "Brief mode: only return name, path, is_dir (default: true)")]
    pub brief: Option<bool>,
    /// Sort by: "name" (default), "type", "size", "modified"
    #[schemars(description = "Sort by: name (default), type, size, modified")]
    pub sort_by: Option<String>,
    /// Flatten output: return a flat list instead of nested tree (default: false)
    #[schemars(description = "Flatten output: return a flat list instead of nested tree (default: false)")]
    pub flatten: Option<bool>,
    /// Multiple glob patterns to match (OR logic: entry matches if it matches any pattern)
    #[schemars(description = "Multiple glob patterns to match (OR logic)")
]
    pub patterns: Option<Vec<String>>,
    /// Use regex patterns instead of glob (default: false)
    #[schemars(description = "Use regex patterns instead of glob (default: false)")]
    pub use_regex: Option<bool>,
    /// Glob patterns to exclude (entries matching any are skipped)
    #[schemars(description = "Glob patterns to exclude (entries matching any are skipped)")]
    pub exclude_patterns: Option<Vec<String>>,
    /// Case-sensitive pattern matching (default: true)
    #[schemars(description = "Case-sensitive pattern matching (default: true)")]
    pub case_sensitive: Option<bool>,
    /// Minimum depth (entries with depth < min_depth are skipped, default: 0)
    #[schemars(description = "Minimum depth (entries with depth < min_depth are skipped, default: 0)")]
    pub min_depth: Option<usize>,
    /// File types to include: "file", "dir", "symlink" (default: all)
    #[schemars(description = "File types to include: file, dir, symlink (default: all)")]
    pub file_types: Option<Vec<String>>,
    /// Minimum file size in bytes
    #[schemars(description = "Minimum file size in bytes")]
    pub min_size: Option<u64>,
    /// Maximum file size in bytes
    #[schemars(description = "Maximum file size in bytes")]
    pub max_size: Option<u64>,
    /// Only include entries modified after this time (ISO 8601, e.g. "2024-01-01T00:00:00Z")
    #[schemars(description = "Only include entries modified after this time (ISO 8601)")]
    pub modified_after: Option<String>,
    /// Only include entries modified before this time (ISO 8601, e.g. "2024-12-31T23:59:59Z")
    #[schemars(description = "Only include entries modified before this time (ISO 8601)")]
    pub modified_before: Option<String>,
    /// Follow symbolic links (default: false)
    #[schemars(description = "Follow symbolic links (default: false)")]
    pub follow_symlinks: Option<bool>,
    /// Sort order: "asc" or "desc" (default: asc)
    #[schemars(description = "Sort order: asc or desc (default: asc)")]
    pub sort_order: Option<String>,
    /// Maximum number of entries to return (default: 500)
    #[schemars(description = "Maximum number of entries to return (default: 500)")]
    pub max_entries: Option<usize>,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_symlink: Option<bool>,
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
    #[serde(skip)]
    raw_size: Option<u64>,
    #[serde(skip)]
    raw_modified: Option<std::time::SystemTime>,
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

fn parse_iso8601(s: &str) -> Option<std::time::SystemTime> {
    use chrono::DateTime;
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        Some(dt.into())
    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        use chrono::TimeZone;
        Some(chrono::Utc.from_utc_datetime(&dt).into())
    } else if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        use chrono::TimeZone;
        let dt = date.and_hms_opt(0, 0, 0)?;
        Some(chrono::Utc.from_utc_datetime(&dt).into())
    } else {
        None
    }
}

fn passes_file_type_filter(is_dir: bool, is_symlink: bool, file_types: &[String]) -> bool {
    if file_types.is_empty() {
        return true;
    }
    let lower: Vec<String> = file_types.iter().map(|t| t.to_lowercase()).collect();
    if is_symlink && lower.contains(&"symlink".to_string()) {
        return true;
    }
    if is_dir && lower.contains(&"dir".to_string()) {
        return true;
    }
    if !is_dir && !is_symlink && lower.contains(&"file".to_string()) {
        return true;
    }
    false
}

fn passes_size_filter(size: u64, min_size: Option<u64>, max_size: Option<u64>) -> bool {
    if let Some(min) = min_size {
        if size < min {
            return false;
        }
    }
    if let Some(max) = max_size {
        if size > max {
            return false;
        }
    }
    true
}

fn passes_time_filter(
    modified: Option<std::time::SystemTime>,
    after: Option<std::time::SystemTime>,
    before: Option<std::time::SystemTime>,
) -> bool {
    if let Some(m) = modified {
        if let Some(a) = after {
            if m <= a {
                return false;
            }
        }
        if let Some(b) = before {
            if m >= b {
                return false;
            }
        }
    }
    true
}

pub async fn dir_list(
    params: Parameters<GlobParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let max_depth = params.max_depth.unwrap_or(2).min(10);
    let include_hidden = params.include_hidden.unwrap_or(false);
    let brief = params.brief.unwrap_or(true);
    let sort_by = params.sort_by.as_deref().unwrap_or("name").to_lowercase();
    let pattern = params.pattern.clone();
    let flatten = params.flatten.unwrap_or(false);
    let path_str = params.path.clone();
    let working_dir = working_dir.to_path_buf();

    let patterns = params.patterns.unwrap_or_default();
    let exclude_patterns = params.exclude_patterns.unwrap_or_default();
    let use_regex = params.use_regex.unwrap_or(false);
    let case_sensitive = params.case_sensitive.unwrap_or(true);
    let min_depth = params.min_depth.unwrap_or(0);
    let file_types = params.file_types.unwrap_or_default();
    let min_size = params.min_size;
    let max_size = params.max_size;
    let modified_after = params
        .modified_after
        .as_deref()
        .and_then(parse_iso8601);
    let modified_before = params
        .modified_before
        .as_deref()
        .and_then(parse_iso8601);
    let follow_symlinks = params.follow_symlinks.unwrap_or(false);
    let sort_order = params.sort_order.as_deref().unwrap_or("asc").to_lowercase();
    let max_entries = params.max_entries.unwrap_or(500);

    let glob_matcher = if !patterns.is_empty() || !exclude_patterns.is_empty() {
        Some(
            GlobMatcher::new(&patterns, &exclude_patterns, use_regex, case_sensitive)
                .map_err(|e| format!("Invalid glob pattern: {}", e))?,
        )
    } else {
        None
    };

    let path = Path::new(&path_str);
    let canonical_path = resolve_path(path, &working_dir)?;

    if !canonical_path.exists() {
        return Err(format!("Path '{}' does not exist", path_str));
    }
    if !canonical_path.is_dir() {
        return Err(format!("Path '{}' is not a directory", path_str));
    }

    let canonical_path_str = canonical_path.to_string_lossy().to_string();
    let response = tokio::task::spawn_blocking(move || {
        let canonical_path = Path::new(&canonical_path_str);
        let mut total_files = 0usize;
        let mut total_dirs = 0usize;

        let response = if flatten {
            let mut entries = Vec::new();
            let (truncated, max_depth_reached) = list_directory_flat(
                canonical_path,
                canonical_path,
                max_depth,
                min_depth,
                0,
                include_hidden,
                brief,
                &sort_by,
                pattern.as_deref(),
                glob_matcher.as_ref(),
                file_types.as_slice(),
                min_size,
                max_size,
                modified_after,
                modified_before,
                follow_symlinks,
                max_entries,
                &mut total_files,
                &mut total_dirs,
                &mut entries,
            )?;

            sort_entries(&mut entries, &sort_by, &sort_order, Some(canonical_path));

            let metadata = std::fs::metadata(canonical_path).ok();
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
            let (entry, truncated, max_depth_reached) = match list_directory_recursive(
                canonical_path,
                max_depth,
                min_depth,
                0,
                include_hidden,
                brief,
                &sort_by,
                pattern.as_deref(),
                glob_matcher.as_ref(),
                file_types.as_slice(),
                min_size,
                max_size,
                modified_after,
                modified_before,
                follow_symlinks,
                max_entries,
                &mut total_files,
                &mut total_dirs,
            )? {
                Some(result) => result,
                None => {
                    // Root directory filtered out by patterns - still return it with empty children
                    let metadata = std::fs::metadata(canonical_path).ok();
                    (FileEntry {
                        name: canonical_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| canonical_path.to_string_lossy().to_string()),
                        path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                        is_dir: true,
                        is_symlink: None,
                        depth: None,
                        size: None,
                        modified: metadata.and_then(|m| m.modified().ok()).map(format_datetime),
                        char_count: None,
                        line_count: None,
                        children: Some(vec![]),
                        raw_size: None,
                        raw_modified: None,
                    }, false, false)
                }
            };

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

        Ok::<DirListResponse, std::io::Error>(response)
    })
    .await
    .map_err(|e| format!("Directory listing task failed: {}", e))?
    .map_err(|e| format!("Failed to list directory: {}", e))?;

    let json = serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

#[allow(clippy::too_many_arguments)]
fn entry_matches_filters(
    name: &str,
    is_dir: bool,
    is_symlink: bool,
    size: u64,
    modified: Option<std::time::SystemTime>,
    current_depth: usize,
    pattern: Option<&str>,
    glob_matcher: Option<&GlobMatcher>,
    file_types: &[String],
    min_size: Option<u64>,
    max_size: Option<u64>,
    modified_after: Option<std::time::SystemTime>,
    modified_before: Option<std::time::SystemTime>,
    min_depth: usize,
) -> bool {
    if current_depth < min_depth {
        return false;
    }

    if !passes_file_type_filter(is_dir, is_symlink, file_types) {
        return false;
    }

    if !passes_size_filter(size, min_size, max_size) {
        return false;
    }

    if !passes_time_filter(modified, modified_after, modified_before) {
        return false;
    }

    if let Some(matcher) = glob_matcher {
        if !matcher.matches(name) {
            return false;
        }
    } else if let Some(pat) = pattern {
        if !glob_match(pat, name) {
            return false;
        }
    }

    true
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::only_used_in_recursion)]
fn list_directory_flat(
    root_path: &Path,
    path: &Path,
    max_depth: usize,
    min_depth: usize,
    current_depth: usize,
    include_hidden: bool,
    brief: bool,
    sort_by: &str,
    pattern: Option<&str>,
    glob_matcher: Option<&GlobMatcher>,
    file_types: &[String],
    min_size: Option<u64>,
    max_size: Option<u64>,
    modified_after: Option<std::time::SystemTime>,
    modified_before: Option<std::time::SystemTime>,
    follow_symlinks: bool,
    max_entries: usize,
    total_files: &mut usize,
    total_dirs: &mut usize,
    entries: &mut Vec<FileEntry>,
) -> Result<(bool, bool), std::io::Error> {
    if current_depth > max_depth {
        return Ok((false, true));
    }

    let dir_entries = std::fs::read_dir(path)?;

    let mut truncated = false;

    for item in dir_entries {
        let item = item?;
        let name = item.file_name();
        let name_str = name.to_string_lossy();

        if !include_hidden && name_str.starts_with('.') {
            continue;
        }

        let metadata = if follow_symlinks {
            item.metadata()?
        } else {
            std::fs::symlink_metadata(item.path())?
        };

        let is_dir = metadata.is_dir();
        let is_symlink = metadata.file_type().is_symlink();

        if is_dir && should_skip_dir(&name_str) {
            continue;
        }

        if entries.len() >= max_entries {
            truncated = true;
            break;
        }

        let entry_path = item.path();
        let rel_path = entry_path.strip_prefix(root_path).unwrap_or_else(|_| {
            Path::new(entry_path.file_name().unwrap_or_default())
        });

        let entry_matched = entry_matches_filters(
            &name_str,
            is_dir,
            is_symlink,
            metadata.len(),
            metadata.modified().ok(),
            current_depth,
            pattern,
            glob_matcher,
            file_types,
            min_size,
            max_size,
            modified_after,
            modified_before,
            min_depth,
        );

        if entry_matched {
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
                is_symlink: if is_symlink { Some(true) } else { None },
                depth: Some(current_depth),
                size,
                modified,
                char_count,
                line_count,
                children: None,
                raw_size: Some(metadata.len()),
                raw_modified: metadata.modified().ok(),
            });
        }

        if is_dir {
            let (child_truncated, child_depth) = list_directory_flat(
                root_path,
                &entry_path,
                max_depth,
                min_depth,
                current_depth + 1,
                include_hidden,
                brief,
                sort_by,
                pattern,
                glob_matcher,
                file_types,
                min_size,
                max_size,
                modified_after,
                modified_before,
                follow_symlinks,
                max_entries,
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
        } else if entry_matched {
            *total_files += 1;
        }

        if is_dir && entry_matched {
            *total_dirs += 1;
        }
    }

    Ok((truncated, false))
}

#[allow(clippy::too_many_arguments)]
fn list_directory_recursive(
    path: &Path,
    max_depth: usize,
    min_depth: usize,
    current_depth: usize,
    include_hidden: bool,
    brief: bool,
    sort_by: &str,
    pattern: Option<&str>,
    glob_matcher: Option<&GlobMatcher>,
    file_types: &[String],
    min_size: Option<u64>,
    max_size: Option<u64>,
    modified_after: Option<std::time::SystemTime>,
    modified_before: Option<std::time::SystemTime>,
    follow_symlinks: bool,
    max_entries: usize,
    total_files: &mut usize,
    total_dirs: &mut usize,
) -> Result<Option<(FileEntry, bool, bool)>, std::io::Error> {
    let metadata = if follow_symlinks {
        std::fs::metadata(path)?
    } else {
        std::fs::symlink_metadata(path)?
    };

    let is_dir = metadata.is_dir();
    let is_symlink = metadata.file_type().is_symlink();
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
        is_symlink: if is_symlink { Some(true) } else { None },
        depth: None,
        size,
        modified,
        char_count,
        line_count,
        children: None,
        raw_size: Some(metadata.len()),
        raw_modified: metadata.modified().ok(),
    };

    let mut truncated = false;
    let mut max_depth_reached = false;
    let mut added_self = false;

    if is_dir && current_depth < max_depth {
        let mut children = Vec::new();

        for item in std::fs::read_dir(path)? {
            let item = item?;
            let name = item.file_name();
            let name_str = name.to_string_lossy();

            if !include_hidden && name_str.starts_with('.') {
                continue;
            }

            let item_metadata = if follow_symlinks {
                item.metadata()?
            } else {
                std::fs::symlink_metadata(item.path())?
            };
            let item_is_dir = item_metadata.is_dir();
            let item_is_symlink = item_metadata.file_type().is_symlink();

            if item_is_dir && should_skip_dir(&name_str) {
                continue;
            }

            let child_matched = entry_matches_filters(
                &name_str,
                item_is_dir,
                item_is_symlink,
                item_metadata.len(),
                item_metadata.modified().ok(),
                current_depth + 1,
                pattern,
                glob_matcher,
                file_types,
                min_size,
                max_size,
                modified_after,
                modified_before,
                min_depth,
            );

            if children.len() >= max_entries {
                truncated = true;
                break;
            }

            match list_directory_recursive(
                &item.path(),
                max_depth,
                min_depth,
                current_depth + 1,
                include_hidden,
                brief,
                sort_by,
                pattern,
                glob_matcher,
                file_types,
                min_size,
                max_size,
                modified_after,
                modified_before,
                follow_symlinks,
                max_entries,
                total_files,
                total_dirs,
            ) {
                Ok(Some((child, child_truncated, child_depth))) => {
                    if child_truncated {
                        truncated = true;
                    }
                    if child_depth {
                        max_depth_reached = true;
                    }
                    let has_children = !child.children.as_ref().is_none_or(|c| c.is_empty());
                    if child_matched || has_children {
                        if child_matched {
                            if child.is_dir {
                                *total_dirs += 1;
                            } else {
                                *total_files += 1;
                            }
                        }
                        children.push(child);
                    }
                }
                Ok(None) => {}
                Err(_) => {} // I/O error while reading child item - skip this entry
            }
        }

        sort_entries(&mut children, sort_by, "asc", None);

        let self_matched = entry_matches_filters(
            &entry.name,
            entry.is_dir,
            is_symlink,
            entry.raw_size.unwrap_or(0),
            entry.raw_modified,
            current_depth,
            pattern,
            glob_matcher,
            file_types,
            min_size,
            max_size,
            modified_after,
            modified_before,
            min_depth,
        );

        if self_matched {
            added_self = true;
        }
        if !children.is_empty() {
            entry.children = Some(children);
        }
    } else if is_dir && current_depth >= max_depth {
        max_depth_reached = true;
        let self_matched = entry_matches_filters(
            &entry.name,
            entry.is_dir,
            is_symlink,
            entry.raw_size.unwrap_or(0),
            entry.raw_modified,
            current_depth,
            pattern,
            glob_matcher,
            file_types,
            min_size,
            max_size,
            modified_after,
            modified_before,
            min_depth,
        );
        if self_matched {
            entry.children = Some(vec![]);
            added_self = true;
        }
    } else {
        let self_matched = entry_matches_filters(
            &entry.name,
            entry.is_dir,
            is_symlink,
            entry.raw_size.unwrap_or(0),
            entry.raw_modified,
            current_depth,
            pattern,
            glob_matcher,
            file_types,
            min_size,
            max_size,
            modified_after,
            modified_before,
            min_depth,
        );
        if !self_matched {
            return Ok(None);
        }
        added_self = true;
    }

    if added_self {
        *total_files += if is_dir { 0 } else { 1 };
        *total_dirs += if is_dir { 1 } else { 0 };
    }

    Ok(Some((entry, truncated, max_depth_reached)))
}

fn sort_entries(
    children: &mut [FileEntry],
    sort_by: &str,
    sort_order: &str,
    _base_path: Option<&Path>,
) {
    let desc = sort_order == "desc";
    children.sort_by(|a, b| {
        let order = match sort_by {
            "type" => match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            },
            "size" => {
                let size_a = a.raw_size.unwrap_or(0);
                let size_b = b.raw_size.unwrap_or(0);
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => size_b.cmp(&size_a).then_with(|| a.name.cmp(&b.name)),
                }
            }
            "modified" => {
                let time_a = a.raw_modified;
                let time_b = b.raw_modified;
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => match (time_b, time_a) {
                        (Some(tb), Some(ta)) => tb.cmp(&ta).then_with(|| a.name.cmp(&b.name)),
                        _ => a.name.cmp(&b.name),
                    },
                }
            }
            _ => match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            },
        };
        if desc {
            order.reverse()
        } else {
            order
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

        let params = GlobParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(2),
            include_hidden: Some(false),
            pattern: None,
            brief: Some(true),
            sort_by: None,
            flatten: Some(false),
            patterns: None,
            use_regex: None,
            exclude_patterns: None,
            case_sensitive: None,
            min_depth: None,
            file_types: None,
            min_size: None,
            max_size: None,
            modified_after: None,
            modified_before: None,
            follow_symlinks: None,
            sort_order: None,
            max_entries: None,
        };

        let result = dir_list(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(!text.text.contains("\\\\?\\"));
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

        let params = GlobParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(2),
            include_hidden: Some(false),
            pattern: None,
            brief: Some(true),
            sort_by: None,
            flatten: Some(true),
            patterns: None,
            use_regex: None,
            exclude_patterns: None,
            case_sensitive: None,
            min_depth: None,
            file_types: None,
            min_size: None,
            max_size: None,
            modified_after: None,
            modified_before: None,
            follow_symlinks: None,
            sort_order: None,
            max_entries: None,
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

        let params = GlobParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(1),
            include_hidden: Some(false),
            pattern: Some("*.rs".to_string()),
            brief: Some(true),
            sort_by: None,
            flatten: Some(false),
            patterns: None,
            use_regex: None,
            exclude_patterns: None,
            case_sensitive: None,
            min_depth: None,
            file_types: None,
            min_size: None,
            max_size: None,
            modified_after: None,
            modified_before: None,
            follow_symlinks: None,
            sort_order: None,
            max_entries: None,
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

    #[tokio::test]
    async fn test_dir_list_file_types_filter() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        fs::create_dir(dir_path.join("subdir")).unwrap();
        fs::write(dir_path.join("file1.txt"), "content1").unwrap();

        let params = GlobParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(1),
            include_hidden: Some(false),
            pattern: None,
            brief: Some(true),
            sort_by: None,
            flatten: Some(true),
            patterns: None,
            use_regex: None,
            exclude_patterns: None,
            case_sensitive: None,
            min_depth: None,
            file_types: Some(vec!["file".to_string()]),
            min_size: None,
            max_size: None,
            modified_after: None,
            modified_before: None,
            follow_symlinks: None,
            sort_order: None,
            max_entries: None,
        };

        let result = dir_list(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("file1.txt"));
                assert!(!text.text.contains("subdir"));
            }
        }
    }

    #[tokio::test]
    async fn test_dir_list_multi_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        fs::write(dir_path.join("a.rs"), "").unwrap();
        fs::write(dir_path.join("b.toml"), "").unwrap();
        fs::write(dir_path.join("c.txt"), "").unwrap();

        let params = GlobParams {
            path: dir_path.to_string_lossy().to_string(),
            max_depth: Some(1),
            include_hidden: Some(false),
            pattern: None,
            brief: Some(true),
            sort_by: None,
            flatten: Some(true),
            patterns: Some(vec!["*.rs".to_string(), "*.toml".to_string()]),
            use_regex: None,
            exclude_patterns: None,
            case_sensitive: None,
            min_depth: None,
            file_types: None,
            min_size: None,
            max_size: None,
            modified_after: None,
            modified_before: None,
            follow_symlinks: None,
            sort_order: None,
            max_entries: None,
        };

        let result = dir_list(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("a.rs"));
                assert!(text.text.contains("b.toml"));
                assert!(!text.text.contains("c.txt"));
            }
        }
    }

    #[test]
    fn test_parse_iso8601() {
        assert!(parse_iso8601("2024-01-01T00:00:00Z").is_some());
        assert!(parse_iso8601("2024-01-01T00:00:00+08:00").is_some());
        assert!(parse_iso8601("2024-01-01T00:00:00").is_some());
        assert!(parse_iso8601("2024-01-01").is_some());
        assert!(parse_iso8601("invalid").is_none());
    }
}
