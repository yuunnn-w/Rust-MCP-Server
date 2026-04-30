use crate::utils::file_utils::{
    is_text_file, read_file_with_options, resolve_path, strip_unc_prefix,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileReadItem {
    /// File path to read
    #[schemars(description = "The file path to read")]
    pub path: String,
    /// Start line number (0-indexed, default: 0)
    #[schemars(description = "Start line number (0-indexed, default: 0)")]
    pub start_line: Option<usize>,
    /// End line number (exclusive, default: 500)
    #[schemars(description = "End line number (exclusive, default: 500)")]
    pub end_line: Option<usize>,
    /// Character offset to start reading (alternative to start_line)
    #[schemars(description = "Character offset to start reading (alternative to start_line)")]
    pub offset_chars: Option<usize>,
    /// Maximum characters to return (default: 15000)
    #[schemars(description = "Maximum characters to return (default: 15000)")]
    pub max_chars: Option<usize>,
    /// Prefix each line with its line number (default: true)
    #[schemars(description = "Prefix each line with its line number (default: true)")]
    pub line_numbers: Option<bool>,
    /// Highlight a specific line with >>> marker (1-based). Useful for pinpointing search results.
    #[schemars(description = "Highlight a specific line with >>> marker (1-based)")]
    pub highlight_line: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileReadParams {
    /// List of files to read concurrently
    #[schemars(description = "List of files to read concurrently. Each item can have its own path and read parameters.")]
    pub files: Vec<FileReadItem>,
}

#[derive(Debug, Serialize)]
struct FileReadResult {
    path: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines_displayed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
}

pub async fn file_read(
    params: Parameters<FileReadParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;

    let mut futures = Vec::new();
    for item in params.files {
        futures.push(read_single_file(item, working_dir));
    }

    let results = futures::future::join_all(futures).await;

    let json = serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

async fn read_single_file(item: FileReadItem, working_dir: &Path) -> FileReadResult {
    let path = Path::new(&item.path);
    let start_line = item.start_line.unwrap_or(0);
    let end_line = item.end_line.unwrap_or(500);
    let max_chars = item.max_chars.unwrap_or(15000);
    let line_numbers = item.line_numbers.unwrap_or(true);
    let highlight_line = item.highlight_line;

    let canonical_path = match resolve_path(path, working_dir) {
        Ok(p) => p,
        Err(e) => {
            return FileReadResult {
                path: item.path,
                success: false,
                error: Some(e),
                content: None,
                lines_displayed: None,
                total_lines: None,
                truncated: None,
            }
        }
    };

    if !canonical_path.exists() {
        return FileReadResult {
            path: strip_unc_prefix(&canonical_path.to_string_lossy()),
            success: false,
            error: Some(format!("File '{}' does not exist", item.path)),
            content: None,
            lines_displayed: None,
            total_lines: None,
            truncated: None,
        };
    }
    if !canonical_path.is_file() {
        return FileReadResult {
            path: strip_unc_prefix(&canonical_path.to_string_lossy()),
            success: false,
            error: Some(format!("Path '{}' is not a file", item.path)),
            content: None,
            lines_displayed: None,
            total_lines: None,
            truncated: None,
        };
    }

    let path_ref: &Path = &canonical_path;
    let is_text = is_text_file(path_ref);

    let (content, lines_displayed, truncated, total_lines) =
        if let Some(offset) = item.offset_chars {
            let file_content = match tokio::fs::read_to_string(path_ref).await {
                Ok(c) => c,
                Err(e) => {
                    let msg = if !is_text {
                        format!(
                            "File '{}' appears to be binary and cannot be read as text: {}",
                            item.path, e
                        )
                    } else {
                        format!("Failed to read file '{}': {}", path_ref.display(), e)
                    };
                    return FileReadResult {
                        path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                        success: false,
                        error: Some(msg),
                        content: None,
                        lines_displayed: None,
                        total_lines: None,
                        truncated: None,
                    };
                }
            };

            let total_chars = file_content.chars().count();
            let chars: Vec<char> = file_content.chars().collect();
            let offset = offset.min(chars.len());
            let slice: String = chars[offset..].iter().collect();

            let lines: Vec<&str> = slice.lines().collect();
            let total_lines = file_content.lines().count();

            let prefix: String = chars[..offset].iter().collect();
            let computed_start_line = prefix.lines().count().saturating_sub(1);

            let mut result = String::new();
            let mut chars_count = 0;
            let mut lines_included = 0;
            let mut truncated = false;

            for (idx, line) in lines.iter().enumerate() {
                let line_num = computed_start_line + idx;
                let is_highlight = highlight_line.map(|hl| hl == line_num + 1).unwrap_or(false);
                let formatted = if line_numbers {
                    if is_highlight {
                        format!(">>>{:4} | {}\n", line_num, line)
                    } else {
                        format!("{:4} | {}\n", line_num, line)
                    }
                } else {
                    if is_highlight {
                        format!(">>> {}\n", line)
                    } else {
                        format!("{}\n", line)
                    }
                };

                if chars_count + formatted.len() > max_chars {
                    truncated = true;
                    break;
                }
                chars_count += formatted.len();
                result.push_str(&formatted);
                lines_included += 1;
            }
            if result.ends_with('\n') {
                result.pop();
            }

            let mut response = result.clone();
            if truncated {
                let next_start = offset + result.len();
                response.push_str(&format!(
                    "\n\n[... Content truncated at {} characters. To continue, use start_line={} or offset_chars={} ...]",
                    max_chars, next_start, next_start
                ));
            }

            let has_more = offset + result.len() < total_chars;
            let is_partial = lines_included < total_lines;

            if is_partial || has_more {
                response.push_str(&format!(
                    "\n\n[File info: total_lines={}, lines_displayed={}",
                    total_lines, lines_included
                ));
                if has_more {
                    let hint_start = offset + result.len();
                    let hint_end = hint_start + 500;
                    response.push_str(&format!(
                        ", hint: start_line={} end_line={} or offset_chars={}]",
                        hint_start, hint_end, hint_start
                    ));
                } else {
                    response.push(']');
                }
            }

            (response, lines_included, truncated, total_lines)
        } else {
            let (mut content, lines_read, truncated, total_lines) =
                match read_file_with_options(path_ref, start_line, end_line, max_chars, line_numbers).await {
                    Ok(r) => r,
                    Err(e) => {
                        let msg = if !is_text {
                            format!(
                                "File '{}' appears to be binary and cannot be read as text: {}",
                                item.path, e
                            )
                        } else {
                            e
                        };
                        return FileReadResult {
                            path: strip_unc_prefix(&canonical_path.to_string_lossy()),
                            success: false,
                            error: Some(msg),
                            content: None,
                            lines_displayed: None,
                            total_lines: None,
                            truncated: None,
                        };
                    }
                };

            // Apply highlight if specified
            if let Some(hl) = highlight_line {
                if hl > start_line && hl <= end_line.min(total_lines) {
                    let hl_0based = hl - start_line - 1;
                    let lines_in_content: Vec<&str> = content.lines().collect();
                    let mut new_lines = Vec::new();
                    for (idx, line) in lines_in_content.iter().enumerate() {
                        if idx == hl_0based {
                            if line_numbers {
                                // Replace "NNNN | " prefix with ">>>NNNN | "
                                if line.len() >= 7 && &line[4..7] == " | " {
                                    new_lines.push(format!(">>>{}", line));
                                } else {
                                    new_lines.push(format!(">>> {}", line));
                                }
                            } else {
                                new_lines.push(format!(">>> {}", line));
                            }
                        } else {
                            new_lines.push(line.to_string());
                        }
                    }
                    content = new_lines.join("\n");
                }
            }

            let mut response = content.clone();
            if truncated {
                let next_start = start_line + lines_read;
                response.push_str(&format!(
                    "\n\n[... Content truncated at {} characters. To continue, use start_line={} ...]",
                    max_chars, next_start
                ));
            }

            let has_more = end_line < total_lines || truncated;
            let is_partial = lines_read < total_lines;

            if is_partial || has_more {
                response.push_str(&format!(
                    "\n\n[File info: total_lines={}, lines_displayed={}",
                    total_lines, lines_read
                ));
                if has_more {
                    let hint_start = start_line + lines_read;
                    let hint_end = hint_start + 500;
                    response.push_str(&format!(
                        ", hint: start_line={} end_line={}]",
                        hint_start, hint_end
                    ));
                } else {
                    response.push(']');
                }
            }

            (response, lines_read, truncated, total_lines)
        };

    FileReadResult {
        path: strip_unc_prefix(&canonical_path.to_string_lossy()),
        success: true,
        error: None,
        content: Some(content),
        lines_displayed: Some(lines_displayed),
        total_lines: Some(total_lines),
        truncated: Some(truncated),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_read() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 2\nLine 3").unwrap();

        let params = FileReadParams {
            files: vec![FileReadItem {
                path: file_path.to_string_lossy().to_string(),
                start_line: Some(0),
                end_line: Some(2),
                offset_chars: None,
                max_chars: None,
                line_numbers: Some(true),
                highlight_line: None,
            }],
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("Line 1"));
                assert!(text.text.contains("Line 2"));
                assert!(!text.text.contains("Line 3"));
                assert!(text.text.contains("total_lines=3"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_read_highlight() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Line 1\nLine 2\nLine 3").unwrap();

        let params = FileReadParams {
            files: vec![FileReadItem {
                path: file_path.to_string_lossy().to_string(),
                start_line: Some(0),
                end_line: Some(10),
                offset_chars: None,
                max_chars: None,
                line_numbers: Some(true),
                highlight_line: Some(2),
            }],
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains(">>>"));
                assert!(text.text.contains("Line 2"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_read_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let params = FileReadParams {
            files: vec![FileReadItem {
                path: "/nonexistent/file.txt".to_string(),
                start_line: None,
                end_line: None,
                offset_chars: None,
                max_chars: None,
                line_numbers: None,
                highlight_line: None,
            }],
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"success\": false"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_read_offset_chars() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello\nWorld\nFoo").unwrap();

        let params = FileReadParams {
            files: vec![FileReadItem {
                path: file_path.to_string_lossy().to_string(),
                start_line: None,
                end_line: None,
                offset_chars: Some(6),
                max_chars: None,
                line_numbers: Some(true),
                highlight_line: None,
            }],
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("World"));
                assert!(!text.text.contains("Hello"));
            }
        }
    }

    #[tokio::test]
    async fn test_file_read_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("a.txt");
        let file2 = temp_dir.path().join("b.txt");
        fs::write(&file1, "File A").unwrap();
        fs::write(&file2, "File B\nLine 2").unwrap();

        let params = FileReadParams {
            files: vec![
                FileReadItem {
                    path: file1.to_string_lossy().to_string(),
                    start_line: Some(0),
                    end_line: Some(10),
                    offset_chars: None,
                    max_chars: None,
                    line_numbers: Some(true),
                    highlight_line: None,
                },
                FileReadItem {
                    path: file2.to_string_lossy().to_string(),
                    start_line: Some(0),
                    end_line: Some(10),
                    offset_chars: None,
                    max_chars: None,
                    line_numbers: Some(true),
                    highlight_line: None,
                },
            ],
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                let arr: Vec<serde_json::Value> = serde_json::from_str(&text.text).unwrap();
                assert_eq!(arr.len(), 2);
                assert!(arr[0]["content"].as_str().unwrap().contains("File A"));
                assert!(arr[1]["content"].as_str().unwrap().contains("File B"));
            }
        }
    }
}
