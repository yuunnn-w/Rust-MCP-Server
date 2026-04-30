use crate::utils::file_utils::{ensure_path_within_working_dir, strip_unc_prefix};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileEditOperation {
    /// File path to edit
    #[schemars(description = "The file path to edit")]
    pub path: String,
    /// Edit mode: string_replace (default), line_replace, insert, delete, patch
    #[schemars(description = "Edit mode: string_replace (default), line_replace, insert, delete, patch. string_replace, line_replace, and insert can create new files if the file does not exist.")]
    pub mode: Option<String>,

    // === string_replace mode ===
    /// String to find (exact match, can span multiple lines). Required for string_replace mode when editing existing file.
    #[schemars(description = "String to find (exact match). Required for string_replace mode when editing existing file.")]
    pub old_string: Option<String>,
    /// Replacement string. Required for string_replace and line_replace modes.
    #[schemars(description = "Replacement or insertion string. Required for string_replace/insert/line_replace modes.")]
    pub new_string: Option<String>,
    /// Which occurrence to replace: 1=first (default), 2=second, 0=replace all. Only for string_replace.
    #[schemars(description = "Which occurrence to replace: 1=first (default), 2=second, 0=replace all. Only for string_replace.")]
    pub occurrence: Option<usize>,

    // === line_replace / insert / delete mode ===
    /// Start line number (1-based, inclusive). Required for line_replace, insert, delete.
    #[schemars(description = "Start line number (1-based, inclusive). Required for line_replace, insert, delete.")]
    pub start_line: Option<usize>,
    /// End line number (1-based, inclusive). Required for line_replace and delete.
    #[schemars(description = "End line number (1-based, inclusive). Required for line_replace and delete.")]
    pub end_line: Option<usize>,

    // === patch mode ===
    /// Unified diff patch string. Required for patch mode.
    #[schemars(description = "Unified diff patch string. Required for patch mode.")]
    pub patch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileEditParams {
    /// List of file edit operations to perform concurrently
    #[schemars(description = "List of file edit operations to perform concurrently. string_replace, line_replace, and insert modes can create new files if they do not exist (provide new_string and optionally old_string for string_replace on new files).")]
    pub operations: Vec<FileEditOperation>,
}

#[derive(Debug, Serialize)]
struct FileEditResult {
    file: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacements: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inserted_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<bool>,
}

pub async fn file_edit(
    params: Parameters<FileEditParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;

    let mut futures = Vec::new();
    for op in params.operations {
        futures.push(edit_single_file(op, working_dir));
    }

    let results = futures::future::join_all(futures).await;

    let json = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

async fn edit_single_file(op: FileEditOperation, working_dir: &Path) -> FileEditResult {
    let mode = op.mode.as_deref().unwrap_or("string_replace");
    let path = Path::new(&op.path);

    let canonical_path = match ensure_path_within_working_dir(path, working_dir) {
        Ok(p) => p,
        Err(e) => {
            return FileEditResult {
                file: op.path,
                success: false,
                error: Some(e),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            }
        }
    };

    let file_exists = canonical_path.exists() && canonical_path.is_file();

    // Support creating new files with string_replace, line_replace, or insert mode
    let can_create_new = matches!(mode, "string_replace" | "line_replace" | "insert");

    if !file_exists {
        if !can_create_new {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!("File '{}' does not exist", op.path)),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            };
        }

        // Try to create new file
        let new_content = match op.new_string.as_deref() {
            Some(s) => s,
            None => {
                return FileEditResult {
                    file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "new_string is required to create new file '{}'",
                        op.path
                    )),
                    mode: Some(mode.to_string()),
                    replacements: None,
                    lines: None,
                    inserted_lines: None,
                    deleted_lines: None,
                    preview: None,
                    total_lines: None,
                    created: None,
                }
            }
        };

        // Create parent directories if needed
        if let Some(parent) = canonical_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return FileEditResult {
                    file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                    success: false,
                    error: Some(format!(
                        "Failed to create parent directories for '{}': {}",
                        op.path, e
                    )),
                    mode: Some(mode.to_string()),
                    replacements: None,
                    lines: None,
                    inserted_lines: None,
                    deleted_lines: None,
                    preview: None,
                    total_lines: None,
                    created: None,
                };
            }
        }

        if let Err(e) = tokio::fs::write(&canonical_path, new_content).await {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!("Failed to write file '{}': {}", op.path, e)),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            };
        }

        let total_lines = new_content.lines().count();
        let preview = build_preview(new_content, None);

        return FileEditResult {
            file: strip_unc_prefix(&canonical_path.to_string_lossy()),
            success: true,
            error: None,
            mode: Some(mode.to_string()),
            replacements: Some(0),
            lines: None,
            inserted_lines: Some(total_lines),
            deleted_lines: Some(0),
            preview: Some(preview),
            total_lines: Some(total_lines),
            created: Some(true),
        };
    }

    // File exists: read and edit
    let content = match tokio::fs::read_to_string(&canonical_path).await {
        Ok(c) => c,
        Err(e) => {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!(
                    "Failed to read file '{}': {}",
                    canonical_path.display(),
                    e
                )),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            }
        }
    };

    let result = match mode {
        "string_replace" => string_replace_mode(&content, &op, &canonical_path).await,
        "line_replace" => line_replace_mode(&content, &op, &canonical_path).await,
        "insert" => insert_mode(&content, &op, &canonical_path).await,
        "delete" => delete_mode(&content, &op, &canonical_path).await,
        "patch" => patch_mode(&content, &op, &canonical_path).await,
        _ => {
            return FileEditResult {
                file: strip_unc_prefix(&canonical_path.to_string_lossy()),
                success: false,
                error: Some(format!(
                    "Invalid edit mode '{}'. Use string_replace, line_replace, insert, delete, or patch.",
                    mode
                )),
                mode: Some(mode.to_string()),
                replacements: None,
                lines: None,
                inserted_lines: None,
                deleted_lines: None,
                preview: None,
                total_lines: None,
                created: None,
            }
        }
    };

    match result {
        Ok(mut r) => {
            r.file = strip_unc_prefix(&canonical_path.to_string_lossy());
            r
        }
        Err(e) => FileEditResult {
            file: strip_unc_prefix(&canonical_path.to_string_lossy()),
            success: false,
            error: Some(e),
            mode: Some(mode.to_string()),
            replacements: None,
            lines: None,
            inserted_lines: None,
            deleted_lines: None,
            preview: None,
            total_lines: None,
            created: None,
        },
    }
}

// ============================================================================
// string_replace mode (original behavior)
// ============================================================================
async fn string_replace_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let old = op
        .old_string
        .as_deref()
        .ok_or("old_string is required for string_replace mode")?;
    let new = op
        .new_string
        .as_deref()
        .ok_or("new_string is required for string_replace mode")?;
    let occurrence = op.occurrence.unwrap_or(1);

    if old.is_empty() {
        return Err("old_string cannot be empty".to_string());
    }

    // Find all occurrences with line numbers
    let mut occurrences: Vec<usize> = Vec::new();
    let mut search_start = 0;
    while let Some(pos) = content[search_start..].find(old) {
        let absolute_pos = search_start + pos;
        let line_num = content[..absolute_pos].lines().count() + 1;
        occurrences.push(line_num);
        search_start = absolute_pos + old.len();
        if search_start >= content.len() {
            break;
        }
    }

    if occurrences.is_empty() {
        return Err(format!(
            "Could not find the specified old_string in '{}'. Please verify the exact text you want to replace.",
            canonical_path.display()
        ));
    }

    let mut replaced_lines: Vec<usize> = Vec::new();

    let replaced_content = if occurrence == 0 {
        replaced_lines = occurrences.clone();
        content.replace(old, new)
    } else {
        if occurrence > occurrences.len() {
            return Err(format!(
                "Requested occurrence {} but only {} occurrence(s) found at line(s): {:?}",
                occurrence,
                occurrences.len(),
                occurrences
            ));
        }
        let target_line = occurrences[occurrence - 1];
        let mut count = 0;
        let mut result = String::new();
        let mut search_start = 0;
        while let Some(pos) = content[search_start..].find(old) {
            let absolute_pos = search_start + pos;
            count += 1;
            result.push_str(&content[search_start..absolute_pos]);
            if count == occurrence {
                result.push_str(new);
                replaced_lines.push(target_line);
            } else {
                result.push_str(old);
            }
            search_start = absolute_pos + old.len();
        }
        result.push_str(&content[search_start..]);
        result
    };

    // Write back
    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview = build_preview(&replaced_content, replaced_lines.first().copied());
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("string_replace".to_string()),
        replacements: Some(replaced_lines.len()),
        lines: Some(replaced_lines),
        inserted_lines: None,
        deleted_lines: None,
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

// ============================================================================
// line_replace mode
// ============================================================================
async fn line_replace_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let start_line = op
        .start_line
        .ok_or("start_line is required for line_replace mode")?;
    let end_line = op
        .end_line
        .ok_or("end_line is required for line_replace mode")?;
    let new_content = op
        .new_string
        .as_deref()
        .ok_or("new_string is required for line_replace mode")?;

    if start_line == 0 || end_line == 0 {
        return Err("Line numbers are 1-based and must be >= 1".to_string());
    }
    if start_line > end_line {
        return Err("start_line must be <= end_line".to_string());
    }

    let lines: Vec<&str> = content.lines().collect();
    let total_lines_before = lines.len();

    if start_line > total_lines_before {
        return Err(format!(
            "start_line {} is beyond file length ({} lines)",
            start_line, total_lines_before
        ));
    }

    let end_line = end_line.min(total_lines_before);
    let start_idx = start_line - 1;
    let end_idx = end_line; // exclusive

    let mut result_lines: Vec<&str> = Vec::new();
    result_lines.extend_from_slice(&lines[..start_idx]);

    let new_lines: Vec<&str> = new_content.lines().collect();
    for nl in &new_lines {
        result_lines.push(nl);
    }

    result_lines.extend_from_slice(&lines[end_idx..]);

    let replaced_content = result_lines.join("\n");
    let replaced_content = if content.ends_with('\n') && !replaced_content.is_empty() {
        replaced_content + "\n"
    } else {
        replaced_content
    };

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview = build_preview(&replaced_content, Some(start_line));
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("line_replace".to_string()),
        replacements: Some(1),
        lines: Some((start_line..=end_line).collect()),
        inserted_lines: Some(new_lines.len()),
        deleted_lines: Some(end_line - start_line + 1),
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

// ============================================================================
// insert mode
// ============================================================================
async fn insert_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let start_line = op
        .start_line
        .ok_or("start_line is required for insert mode")?;
    let new_content = op
        .new_string
        .as_deref()
        .ok_or("new_string is required for insert mode")?;

    if start_line == 0 {
        return Err("start_line must be >= 1 (1-based)".to_string());
    }

    let lines: Vec<&str> = content.lines().collect();
    let total_lines_before = lines.len();

    let insert_idx = if start_line > total_lines_before {
        total_lines_before
    } else {
        start_line - 1
    };

    let mut result_lines: Vec<&str> = Vec::new();
    result_lines.extend_from_slice(&lines[..insert_idx]);

    let new_lines: Vec<&str> = new_content.lines().collect();
    for nl in &new_lines {
        result_lines.push(nl);
    }

    result_lines.extend_from_slice(&lines[insert_idx..]);

    let replaced_content = result_lines.join("\n");
    let replaced_content = if content.ends_with('\n') && !replaced_content.is_empty() {
        replaced_content + "\n"
    } else {
        replaced_content
    };

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview = build_preview(&replaced_content, Some(start_line));
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("insert".to_string()),
        replacements: Some(0),
        lines: Some(vec![start_line]),
        inserted_lines: Some(new_lines.len()),
        deleted_lines: Some(0),
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

// ============================================================================
// delete mode
// ============================================================================
async fn delete_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let start_line = op
        .start_line
        .ok_or("start_line is required for delete mode")?;
    let end_line = op
        .end_line
        .ok_or("end_line is required for delete mode")?;

    if start_line == 0 || end_line == 0 {
        return Err("Line numbers are 1-based and must be >= 1".to_string());
    }
    if start_line > end_line {
        return Err("start_line must be <= end_line".to_string());
    }

    let lines: Vec<&str> = content.lines().collect();
    let total_lines_before = lines.len();

    if start_line > total_lines_before {
        return Err(format!(
            "start_line {} is beyond file length ({} lines)",
            start_line, total_lines_before
        ));
    }

    let end_line = end_line.min(total_lines_before);
    let start_idx = start_line - 1;
    let end_idx = end_line; // exclusive

    let mut result_lines: Vec<&str> = Vec::new();
    result_lines.extend_from_slice(&lines[..start_idx]);
    result_lines.extend_from_slice(&lines[end_idx..]);

    let replaced_content = result_lines.join("\n");
    let replaced_content = if content.ends_with('\n') && !replaced_content.is_empty() {
        replaced_content + "\n"
    } else {
        replaced_content
    };

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview_start = start_idx.saturating_sub(1);
    let preview = build_preview_at(&replaced_content, preview_start);
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("delete".to_string()),
        replacements: Some(0),
        lines: Some((start_line..=end_line).collect()),
        inserted_lines: Some(0),
        deleted_lines: Some(end_line - start_line + 1),
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

// ============================================================================
// patch mode (unified diff)
// ============================================================================
async fn patch_mode(
    content: &str,
    op: &FileEditOperation,
    canonical_path: &Path,
) -> Result<FileEditResult, String> {
    let patch_str = op
        .patch
        .as_deref()
        .ok_or("patch is required for patch mode")?;

    let replaced_content = apply_unified_diff(content, patch_str)?;

    tokio::fs::write(&canonical_path, &replaced_content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", canonical_path.display(), e))?;

    let preview = build_preview(&replaced_content, None);
    let total_lines = replaced_content.lines().count();

    Ok(FileEditResult {
        file: canonical_path.to_string_lossy().to_string(),
        success: true,
        error: None,
        mode: Some("patch".to_string()),
        replacements: Some(1),
        lines: None,
        inserted_lines: None,
        deleted_lines: None,
        preview: Some(preview),
        total_lines: Some(total_lines),
        created: Some(false),
    })
}

/// Apply a unified diff patch to content.
/// Supports multiple hunks. Applies hunks from bottom to top to avoid line offset issues.
fn apply_unified_diff(content: &str, patch: &str) -> Result<String, String> {
    let patch_lines: Vec<&str> = patch.lines().collect();
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut i = 0;

    // Parse hunks
    while i < patch_lines.len() {
        let line = patch_lines[i];
        // Skip file header lines
        if line.starts_with("---") || line.starts_with("+++") {
            i += 1;
            continue;
        }
        if line.starts_with("@@") {
            let hunk = parse_hunk_header(line)?;
            i += 1;
            let mut hunk_lines: Vec<DiffLine> = Vec::new();
            while i < patch_lines.len() {
                let l = patch_lines[i];
                if l.starts_with("@@") || l.starts_with("---") || l.starts_with("+++") {
                    break;
                }
                if l.is_empty() {
                    // Empty line in diff context: treat as context line with empty content
                    hunk_lines.push(DiffLine::Context(""));
                    i += 1;
                    continue;
                }
                let first_char = l.chars().next().unwrap();
                match first_char {
                    ' ' => hunk_lines.push(DiffLine::Context(&l[1..])),
                    '-' => hunk_lines.push(DiffLine::Delete(&l[1..])),
                    '+' => hunk_lines.push(DiffLine::Add(&l[1..])),
                    '\\' => {
                        // "\ No newline at end of file" - skip
                    }
                    _ => return Err(format!("Unexpected line in patch hunk: {}", l)),
                }
                i += 1;
            }
            hunks.push(Hunk {
                old_start: hunk.0,
                lines: hunk_lines,
            });
        } else {
            i += 1;
        }
    }

    if hunks.is_empty() {
        return Err("No valid hunks found in patch".to_string());
    }

    // Apply hunks from bottom to top
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    for hunk in hunks.iter().rev() {
        apply_hunk(&mut lines, hunk)?;
    }

    let mut result = lines.join("\n");
    if content.ends_with('\n') && !result.is_empty() {
        result.push('\n');
    }
    Ok(result)
}

#[derive(Debug)]
enum DiffLine<'a> {
    Context(&'a str),
    Delete(&'a str),
    Add(&'a str),
}

struct Hunk<'a> {
    old_start: usize,
    lines: Vec<DiffLine<'a>>,
}

fn parse_hunk_header(line: &str) -> Result<(usize, usize, usize, usize), String> {
    // Format: @@ -old_start,old_count +new_start,new_count @@
    let line = line.trim();
    if !line.starts_with("@@") || !line[2..].contains("@@") {
        return Err(format!("Invalid hunk header: {}", line));
    }
    let inner = &line[3..];
    let end = inner
        .find(" @@")
        .ok_or_else(|| format!("Invalid hunk header: {}", line))?;
    let inner = &inner[..end];

    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("Invalid hunk header format: {}", line));
    }

    let old_part = parts[0].trim_start_matches('-');
    let new_part = parts[1].trim_start_matches('+');

    let (old_start, old_count) = parse_hunk_range(old_part)?;
    let (new_start, new_count) = parse_hunk_range(new_part)?;

    Ok((old_start, old_count, new_start, new_count))
}

fn parse_hunk_range(s: &str) -> Result<(usize, usize), String> {
    let comma = s.find(',');
    let start: usize = s[..comma.unwrap_or(s.len())]
        .parse()
        .map_err(|_| format!("Invalid hunk range number: {}", s))?;
    let count: usize = if let Some(c) = comma {
        s[c + 1..]
            .parse()
            .map_err(|_| format!("Invalid hunk count: {}", s))?
    } else {
        1
    };
    Ok((start, count))
}

fn apply_hunk(lines: &mut Vec<String>, hunk: &Hunk) -> Result<(), String> {
    let start_idx = hunk.old_start.saturating_sub(1);

    // Verify context matches
    let mut line_idx = start_idx;
    for diff_line in &hunk.lines {
        match diff_line {
            DiffLine::Context(expected) => {
                if line_idx >= lines.len() {
                    return Err(format!(
                        "Patch context mismatch at line {}: expected '{}' but file has only {} lines",
                        line_idx + 1,
                        expected,
                        lines.len()
                    ));
                }
                if lines[line_idx].as_str() != *expected {
                    return Err(format!(
                        "Patch context mismatch at line {}: expected '{}' but found '{}'",
                        line_idx + 1,
                        expected,
                        lines[line_idx]
                    ));
                }
                line_idx += 1;
            }
            DiffLine::Delete(expected) => {
                if line_idx >= lines.len() {
                    return Err(format!(
                        "Patch delete mismatch at line {}: expected '{}' but file has only {} lines",
                        line_idx + 1,
                        expected,
                        lines.len()
                    ));
                }
                if lines[line_idx].as_str() != *expected {
                    return Err(format!(
                        "Patch delete mismatch at line {}: expected '{}' but found '{}'",
                        line_idx + 1,
                        expected,
                        lines[line_idx]
                    ));
                }
                line_idx += 1;
            }
            DiffLine::Add(_) => {
                // Add lines don't consume original lines during verification
            }
        }
    }

    // Apply the hunk
    let mut new_lines: Vec<String> = Vec::new();
    new_lines.extend_from_slice(&lines[..start_idx]);

    let mut line_idx = start_idx;
    for diff_line in &hunk.lines {
        match diff_line {
            DiffLine::Context(_text) => {
                new_lines.push(lines[line_idx].clone());
                line_idx += 1;
            }
            DiffLine::Delete(_) => {
                line_idx += 1; // Skip (delete)
            }
            DiffLine::Add(text) => {
                new_lines.push((*text).to_string());
            }
        }
    }

    new_lines.extend_from_slice(&lines[line_idx..]);
    *lines = new_lines;

    Ok(())
}

// ============================================================================
// Preview helpers
// ============================================================================
fn build_preview(content: &str, around_line: Option<usize>) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let preview_start = around_line
        .map(|l| l.saturating_sub(2))
        .unwrap_or(0)
        .min(total_lines);
    let preview_end = (preview_start + 5).min(total_lines);

    lines[preview_start..preview_end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", preview_start + i + 1, line))
        .collect()
}

fn build_preview_at(content: &str, start_idx: usize) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let preview_start = start_idx.min(total_lines);
    let preview_end = (preview_start + 5).min(total_lines);

    lines[preview_start..preview_end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", preview_start + i + 1, line))
        .collect()
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_string_replace_single() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello World\nFoo Bar\nHello World")
            .await
            .unwrap();

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: Some("string_replace".to_string()),
                old_string: Some("Hello World".to_string()),
                new_string: Some("Hi Universe".to_string()),
                occurrence: Some(1),
                start_line: None,
                end_line: None,
                patch: None,
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hi Universe\nFoo Bar\nHello World");
    }

    #[tokio::test]
    async fn test_line_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5\n")
            .await
            .unwrap();

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: Some("line_replace".to_string()),
                old_string: None,
                new_string: Some("replaced2\nreplaced3".to_string()),
                occurrence: None,
                start_line: Some(2),
                end_line: Some(3),
                patch: None,
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "line1\nreplaced2\nreplaced3\nline4\nline5\n");
    }

    #[tokio::test]
    async fn test_insert() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\n").await.unwrap();

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: Some("insert".to_string()),
                old_string: None,
                new_string: Some("inserted\n".to_string()),
                occurrence: None,
                start_line: Some(2),
                end_line: None,
                patch: None,
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "line1\ninserted\nline2\n");
    }

    #[tokio::test]
    async fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\nline3\nline4\n")
            .await
            .unwrap();

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: Some("delete".to_string()),
                old_string: None,
                new_string: None,
                occurrence: None,
                start_line: Some(2),
                end_line: Some(3),
                patch: None,
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "line1\nline4\n");
    }

    #[tokio::test]
    async fn test_patch_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\nline3\nline4\n")
            .await
            .unwrap();

        let patch = r#"--- a/test.txt
+++ b/test.txt
@@ -1,4 +1,4 @@
 line1
-line2
+line2_modified
 line3
-line4
+line4_modified
"#;

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: Some("patch".to_string()),
                old_string: None,
                new_string: None,
                occurrence: None,
                start_line: None,
                end_line: None,
                patch: Some(patch.to_string()),
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("line2_modified"));
        assert!(content.contains("line4_modified"));
        assert!(!content.contains("\nline2\n"));
        assert!(!content.contains("\nline4\n"));
    }

    #[tokio::test]
    async fn test_patch_mode_multi_hunk() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "a\nb\nc\nd\ne\nf\n")
            .await
            .unwrap();

        let patch = r#"--- a/test.txt
+++ b/test.txt
@@ -1,2 +1,2 @@
 a
-b
+B
@@ -5,2 +5,2 @@
 e
-f
+F
"#;

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: Some("patch".to_string()),
                old_string: None,
                new_string: None,
                occurrence: None,
                start_line: None,
                end_line: None,
                patch: Some(patch.to_string()),
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok(), "{:?}", result);

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "a\nB\nc\nd\ne\nF\n");
    }

    #[tokio::test]
    async fn test_default_mode_is_string_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello World").await.unwrap();

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: None,
                old_string: Some("Hello".to_string()),
                new_string: Some("Hi".to_string()),
                occurrence: None,
                start_line: None,
                end_line: None,
                patch: None,
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hi World");
    }

    #[tokio::test]
    async fn test_create_new_file_string_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new_file.txt");

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: Some("string_replace".to_string()),
                old_string: None,
                new_string: Some("Hello, new file!\nLine 2".to_string()),
                occurrence: None,
                start_line: None,
                end_line: None,
                patch: None,
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        assert!(file_path.exists());
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, new file!\nLine 2");
    }

    #[tokio::test]
    async fn test_create_new_file_insert() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("insert_new.txt");

        let params = FileEditParams {
            operations: vec![FileEditOperation {
                path: file_path.to_string_lossy().to_string(),
                mode: Some("insert".to_string()),
                old_string: None,
                new_string: Some("Inserted content".to_string()),
                occurrence: None,
                start_line: Some(1),
                end_line: None,
                patch: None,
            }],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        assert!(file_path.exists());
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Inserted content");
    }

    #[tokio::test]
    async fn test_concurrent_edits() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("a.txt");
        let file2 = temp_dir.path().join("b.txt");
        tokio::fs::write(&file1, "Hello A").await.unwrap();
        tokio::fs::write(&file2, "Hello B").await.unwrap();

        let params = FileEditParams {
            operations: vec![
                FileEditOperation {
                    path: file1.to_string_lossy().to_string(),
                    mode: Some("string_replace".to_string()),
                    old_string: Some("Hello A".to_string()),
                    new_string: Some("Hi A".to_string()),
                    occurrence: None,
                    start_line: None,
                    end_line: None,
                    patch: None,
                },
                FileEditOperation {
                    path: file2.to_string_lossy().to_string(),
                    mode: Some("string_replace".to_string()),
                    old_string: Some("Hello B".to_string()),
                    new_string: Some("Hi B".to_string()),
                    occurrence: None,
                    start_line: None,
                    end_line: None,
                    patch: None,
                },
            ],
        };

        let result = file_edit(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                let arr: Vec<serde_json::Value> = serde_json::from_str(&text.text).unwrap();
                assert_eq!(arr.len(), 2);
                assert!(arr[0]["success"].as_bool().unwrap());
                assert!(arr[1]["success"].as_bool().unwrap());
            }
        }

        assert_eq!(tokio::fs::read_to_string(&file1).await.unwrap(), "Hi A");
        assert_eq!(tokio::fs::read_to_string(&file2).await.unwrap(), "Hi B");
    }

    #[test]
    fn test_apply_unified_diff_basic() {
        let content = "line1\nline2\nline3\n";
        let patch = r#"@@ -1,3 +1,3 @@
 line1
-line2
+line2_modified
 line3
"#;
        let result = apply_unified_diff(content, patch).unwrap();
        assert_eq!(result, "line1\nline2_modified\nline3\n");
    }

    #[test]
    fn test_apply_unified_diff_add_line() {
        let content = "line1\nline2\n";
        let patch = r#"@@ -1,2 +1,3 @@
 line1
+line1_5
 line2
"#;
        let result = apply_unified_diff(content, patch).unwrap();
        assert_eq!(result, "line1\nline1_5\nline2\n");
    }

    #[test]
    fn test_apply_unified_diff_delete_line() {
        let content = "line1\nline2\nline3\n";
        let patch = r#"@@ -1,3 +1,2 @@
 line1
-line2
 line3
"#;
        let result = apply_unified_diff(content, patch).unwrap();
        assert_eq!(result, "line1\nline3\n");
    }

    #[test]
    fn test_apply_unified_diff_context_mismatch() {
        let content = "line1\nline2\nline3\n";
        let patch = r#"@@ -1,3 +1,3 @@
 line1
-wrong_line
+replaced
 line3
"#;
        let result = apply_unified_diff(content, patch);
        assert!(result.is_err());
    }
}
