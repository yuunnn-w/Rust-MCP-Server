use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiffParams {
    /// Operation: compare_text, compare_files, directory_diff, git_diff_file
    pub operation: String,
    /// For compare_text: old text content
    pub old_text: Option<String>,
    /// For compare_text: new text content
    pub new_text: Option<String>,
    /// For compare_files / directory_diff / git_diff_file: old file or directory path
    pub old_path: Option<String>,
    /// For compare_files / directory_diff: new file or directory path
    pub new_path: Option<String>,
    /// For git_diff_file: file path (compares working copy vs HEAD)
    pub file_path: Option<String>,
    /// Output format: unified (default), side_by_side, summary, inline
    pub output_format: Option<String>,
    /// Context lines for unified diff (default: 3, max: 20)
    pub context_lines: Option<usize>,
    /// Ignore whitespace differences (default: false)
    pub ignore_whitespace: Option<bool>,
    /// Ignore case differences (default: false)
    pub ignore_case: Option<bool>,
    /// Maximum output lines (default: 500)
    pub max_output_lines: Option<usize>,
    /// Enable word-level inline diff for inline format (default: true)
    pub word_level: Option<bool>,
}

#[derive(Debug, Serialize)]
struct DirectoryDiffResult {
    only_in_left: Vec<String>,
    only_in_right: Vec<String>,
    modified: Vec<String>,
    identical: Vec<String>,
}

pub async fn diff(params: Parameters<DiffParams>, working_dir: &Path) -> Result<CallToolResult, String> {
    let p = params.0;
    let op = p.operation.to_lowercase();
    let format = p.output_format.as_deref().unwrap_or("unified").to_lowercase();
    let context_lines = p.context_lines.unwrap_or(3).clamp(1, 20);
    let ignore_ws = p.ignore_whitespace.unwrap_or(false);
    let ignore_case = p.ignore_case.unwrap_or(false);
    let max_lines = p.max_output_lines.unwrap_or(500);
    let word_level = p.word_level.unwrap_or(true);

    match op.as_str() {
        "compare_text" => {
            let old = p.old_text.ok_or("Missing 'old_text' for compare_text")?;
            let new = p.new_text.ok_or("Missing 'new_text' for compare_text")?;
            let result = diff_text(&old, &new, &format, context_lines, ignore_ws, ignore_case, max_lines, word_level)?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(result)]))
        }
        "compare_files" => {
            let old_p = p.old_path.ok_or("Missing 'old_path' for compare_files")?;
            let new_p = p.new_path.ok_or("Missing 'new_path' for compare_files")?;
            let old_path = crate::utils::file_utils::ensure_path_within_working_dir(Path::new(&old_p), working_dir)
                .map_err(|e| e.to_string())?;
            let new_path = crate::utils::file_utils::ensure_path_within_working_dir(Path::new(&new_p), working_dir)
                .map_err(|e| e.to_string())?;
            let old = tokio::fs::read_to_string(&old_path).await
                .map_err(|e| format!("Failed to read '{}': {}", old_p, e))?;
            let new = tokio::fs::read_to_string(&new_path).await
                .map_err(|e| format!("Failed to read '{}': {}", new_p, e))?;
            let result = diff_text(&old, &new, &format, context_lines, ignore_ws, ignore_case, max_lines, word_level)?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(result)]))
        }
        "directory_diff" => {
            let old_p = p.old_path.ok_or("Missing 'old_path' for directory_diff")?;
            let new_p = p.new_path.ok_or("Missing 'new_path' for directory_diff")?;
            let old_dir = crate::utils::file_utils::ensure_path_within_working_dir(Path::new(&old_p), working_dir)
                .map_err(|e| e.to_string())?;
            let new_dir = crate::utils::file_utils::ensure_path_within_working_dir(Path::new(&new_p), working_dir)
                .map_err(|e| e.to_string())?;
            let result = diff_directories(&old_dir, &new_dir).await?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(result)]))
        }
        "git_diff_file" => {
            let file_p = p.file_path.ok_or("Missing 'file_path' for git_diff_file")?;
            let file_path = crate::utils::file_utils::ensure_path_within_working_dir(Path::new(&file_p), working_dir)
                .map_err(|e| e.to_string())?;
            let parent = file_path.parent().unwrap_or(working_dir);
            let file_name = file_path.file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid file path")?;

            // Get HEAD version via git show
            let output = tokio::process::Command::new("git")
                .args(["show", &format!("HEAD:{}", file_name)])
                .current_dir(parent)
                .output()
                .await
                .map_err(|e| format!("Failed to run git show: {}", e))?;

            let old = if output.status.success() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                return Err(format!("Failed to get HEAD version of '{}': {}", file_p, String::from_utf8_lossy(&output.stderr)));
            };

            let new = tokio::fs::read_to_string(&file_path).await
                .map_err(|e| format!("Failed to read '{}': {}", file_p, e))?;
            let result = diff_text(&old, &new, &format, context_lines, ignore_ws, ignore_case, max_lines, word_level)?;
            Ok(CallToolResult::success(vec![rmcp::model::Content::text(result)]))
        }
        _ => Err(format!("Unknown diff operation: '{}'. Supported: compare_text, compare_files, directory_diff, git_diff_file", p.operation)),
    }
}

fn normalize_text(text: &str, ignore_ws: bool, ignore_case: bool) -> String {
    let mut result = text.to_string();
    if ignore_ws {
        // Normalize line endings and trailing whitespace
        result = result.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n");
        // Remove blank lines at end
        while result.ends_with('\n') {
            result.pop();
        }
    }
    if ignore_case {
        result = result.to_lowercase();
    }
    result
}

fn diff_text(
    old: &str,
    new: &str,
    format: &str,
    context_lines: usize,
    ignore_ws: bool,
    ignore_case: bool,
    max_lines: usize,
    word_level: bool,
) -> Result<String, String> {
    let old_norm = normalize_text(old, ignore_ws, ignore_case);
    let new_norm = normalize_text(new, ignore_ws, ignore_case);

    let diff = TextDiff::from_lines(&old_norm, &new_norm);

    match format {
        "summary" => {
            let mut added = 0usize;
            let mut removed = 0usize;
            let mut modified = 0usize;
            for change in diff.iter_all_changes() {
                match change.tag() {
                    ChangeTag::Insert => added += 1,
                    ChangeTag::Delete => removed += 1,
                    ChangeTag::Equal => {},
                }
            }
            if added > 0 && removed > 0 {
                modified = added.min(removed);
                added -= modified;
                removed -= modified;
            }
            Ok(format!("Diff Summary:\n  Added lines: {}\n  Removed lines: {}\n  Modified lines: {}\n  Total changes: {}",
                added + modified, removed + modified, modified, added + removed + modified))
        }
        "side_by_side" => {
            let mut output = String::from("Side-by-Side Diff:\n");
            let mut line_count = 0;
            for change in diff.iter_all_changes() {
                if line_count >= max_lines { break; }
                let prefix = match change.tag() {
                    ChangeTag::Delete => "[-] ",
                    ChangeTag::Insert => "[+] ",
                    ChangeTag::Equal => "    ",
                };
                output.push_str(prefix);
                output.push_str(change.value());
                if !change.value().ends_with('\n') {
                    output.push('\n');
                }
                line_count += 1;
            }
            if line_count >= max_lines {
                output.push_str(&format!("\n...[output truncated at {} lines]", max_lines));
            }
            Ok(output)
        }
        "inline" => {
            let mut output = String::from("Inline Diff:\n");
            let mut line_count = 0;
            for group in diff.grouped_ops(context_lines) {
                for op in group {
                    for change in diff.iter_inline_changes(&op) {
                        if line_count >= max_lines { break; }
                        let sign = match change.tag() {
                            ChangeTag::Delete => "-",
                            ChangeTag::Insert => "+",
                            ChangeTag::Equal => " ",
                        };
                        output.push_str(sign);
                        if word_level {
                            for (emphasized, value) in change.iter_strings_lossy() {
                                if emphasized {
                                    output.push_str(&format!("[[{}]]", value));
                                } else {
                                    output.push_str(&value);
                                }
                            }
                        } else {
                            let text = change.to_string();
                            output.push_str(&text);
                        }
                        let text = change.to_string();
                        if !text.ends_with('\n') {
                            output.push('\n');
                        }
                        line_count += 1;
                    }
                }
            }
            if line_count >= max_lines {
                output.push_str(&format!("\n...[output truncated at {} lines]", max_lines));
            }
            Ok(output)
        }
        _ => {
            // unified diff (default)
            let mut output = String::new();
            let mut line_count = 0;
            for group in diff.grouped_ops(context_lines) {
                for op in group {
                    for change in diff.iter_inline_changes(&op) {
                        if line_count >= max_lines { break; }
                        let sign = match change.tag() {
                            ChangeTag::Delete => "-",
                            ChangeTag::Insert => "+",
                            ChangeTag::Equal => " ",
                        };
                        output.push_str(sign);
                        let text = change.to_string();
                        output.push_str(&text);
                        if !text.ends_with('\n') {
                            output.push('\n');
                        }
                        line_count += 1;
                    }
                }
            }
            if line_count >= max_lines {
                output.push_str(&format!("\n...[output truncated at {} lines]", max_lines));
            }
            if output.is_empty() {
                output.push_str("No differences found.");
            }
            Ok(output)
        }
    }
}

async fn diff_directories(old_dir: &Path, new_dir: &Path) -> Result<String, String> {
    let mut only_in_left = Vec::new();
    let mut only_in_right = Vec::new();
    let mut modified = Vec::new();
    let mut identical = Vec::new();

    fn collect_files(dir: &Path, base: &Path, files: &mut Vec<String>) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if path.is_dir() {
                collect_files(&path, base, files)?;
            } else {
                files.push(rel_str);
            }
        }
        Ok(())
    }

    let mut old_files = Vec::new();
    let mut new_files = Vec::new();
    if old_dir.exists() {
        collect_files(old_dir, old_dir, &mut old_files)?;
    }
    if new_dir.exists() {
        collect_files(new_dir, new_dir, &mut new_files)?;
    }

    let old_set: std::collections::HashSet<String> = old_files.iter().cloned().collect();
    let new_set: std::collections::HashSet<String> = new_files.iter().cloned().collect();

    for f in &old_files {
        if !new_set.contains(f) {
            only_in_left.push(f.clone());
        }
    }
    for f in &new_files {
        if !old_set.contains(f) {
            only_in_right.push(f.clone());
        }
    }
    for f in old_files.iter().filter(|f| new_set.contains(*f)) {
        let old_content = std::fs::read(old_dir.join(f)).unwrap_or_default();
        let new_content = std::fs::read(new_dir.join(f)).unwrap_or_default();
        if old_content == new_content {
            identical.push(f.clone());
        } else {
            modified.push(f.clone());
        }
    }

    let result = DirectoryDiffResult {
        only_in_left,
        only_in_right,
        modified,
        identical,
    };
    serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
}

use rmcp::handler::server::wrapper::Parameters;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_diff_compare_text_summary() {
        let params = Parameters(DiffParams {
            operation: "compare_text".to_string(),
            old_text: Some("line1\nline2\nline3".to_string()),
            new_text: Some("line1\nmodified\nline3".to_string()),
            old_path: None,
            new_path: None,
            file_path: None,
            output_format: Some("summary".to_string()),
            context_lines: None,
            ignore_whitespace: None,
            ignore_case: None,
            max_output_lines: None,
            word_level: None,
        });
        let result = diff(params, Path::new(".")).await;
        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Added") || text.contains("Removed") || text.contains("Modified"));
    }

    #[tokio::test]
    async fn test_diff_compare_text_unified() {
        let params = Parameters(DiffParams {
            operation: "compare_text".to_string(),
            old_text: Some("a\nb\nc".to_string()),
            new_text: Some("a\nB\nc".to_string()),
            old_path: None,
            new_path: None,
            file_path: None,
            output_format: None,
            context_lines: None,
            ignore_whitespace: None,
            ignore_case: Some(true),
            max_output_lines: None,
            word_level: None,
        });
        let result = diff(params, Path::new(".")).await;
        assert!(result.is_ok());
        let text = result.unwrap().content[0].as_text().unwrap().text.clone();
        // With ignore_case=true, b and B should match
        assert!(text.contains("No differences") || text.is_empty() || text.contains("No differences found"));
    }
}
