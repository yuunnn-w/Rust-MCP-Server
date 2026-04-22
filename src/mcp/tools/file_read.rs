use crate::utils::file_utils::{ensure_path_within_working_dir, is_text_file, read_file_with_options};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileReadParams {
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

pub async fn file_read(
    params: Parameters<FileReadParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);
    let start_line = params.start_line.unwrap_or(0);
    let end_line = params.end_line.unwrap_or(500);
    let max_chars = params.max_chars.unwrap_or(15000);
    let line_numbers = params.line_numbers.unwrap_or(true);
    let highlight_line = params.highlight_line;

    let canonical_path = ensure_path_within_working_dir(path, working_dir)?;

    if !canonical_path.exists() {
        return Err(format!("File '{}' does not exist", params.path));
    }
    if !canonical_path.is_file() {
        return Err(format!("Path '{}' is not a file", params.path));
    }

    let path: &Path = &canonical_path;
    let is_text = is_text_file(path);

    let (content, lines_read, truncated, total_lines, total_chars) = if let Some(offset) = params.offset_chars {
        let file_content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| {
                if !is_text {
                    format!(
                        "File '{}' appears to be binary and cannot be read as text: {}",
                        params.path, e
                    )
                } else {
                    format!("Failed to read file '{}': {}", path.display(), e)
                }
            })?;

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

        (result, lines_included, truncated, total_lines, total_chars)
    } else {
        let (mut content, lines_read, truncated, total_lines) = read_file_with_options(path, start_line, end_line, max_chars, line_numbers)
            .await
            .map_err(|e| {
                if !is_text {
                    format!(
                        "File '{}' appears to be binary and cannot be read as text: {}",
                        params.path, e
                    )
                } else {
                    e
                }
            })?;

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

        (content, lines_read, truncated, total_lines, 0usize)
    };

    let mut response = String::new();
    response.push_str(&content);

    if truncated {
        let next_start = if params.offset_chars.is_some() {
            params.offset_chars.unwrap_or(0) + content.len()
        } else {
            start_line + lines_read
        };
        response.push_str(&format!(
            "\n\n[... Content truncated at {} characters. To continue, use start_line={} or offset_chars={} ...]",
            max_chars, next_start, next_start
        ));
    }

    let has_more = if params.offset_chars.is_some() {
        let offset = params.offset_chars.unwrap_or(0);
        offset + content.len() < total_chars
    } else {
        end_line < total_lines || truncated
    };

    let is_partial = lines_read < total_lines;

    if is_partial || has_more {
        response.push_str(&format!(
            "\n\n[File info: total_lines={}, lines_displayed={}",
            total_lines, lines_read
        ));

        if has_more {
            let hint_start = if params.offset_chars.is_some() {
                params.offset_chars.unwrap_or(0) + content.len()
            } else {
                start_line + lines_read
            };
            let hint_end = hint_start + 500;
            response.push_str(&format!(
                ", hint: start_line={} end_line={} or offset_chars={}]",
                hint_start, hint_end, hint_start
            ));
        } else {
            response.push(']');
        }
    }

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(response)]))
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
            path: file_path.to_string_lossy().to_string(),
            start_line: Some(0),
            end_line: Some(2),
            offset_chars: None,
            max_chars: None,
            line_numbers: Some(true),
            highlight_line: None,
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
            path: file_path.to_string_lossy().to_string(),
            start_line: Some(0),
            end_line: Some(10),
            offset_chars: None,
            max_chars: None,
            line_numbers: Some(true),
            highlight_line: Some(2),
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
            path: "/nonexistent/file.txt".to_string(),
            start_line: None,
            end_line: None,
            offset_chars: None,
            max_chars: None,
            line_numbers: None,
            highlight_line: None,
        };

        let result = file_read(Parameters(params), temp_dir.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_read_offset_chars() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello\nWorld\nFoo").unwrap();

        let params = FileReadParams {
            path: file_path.to_string_lossy().to_string(),
            start_line: None,
            end_line: None,
            offset_chars: Some(6),
            max_chars: None,
            line_numbers: Some(true),
            highlight_line: None,
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
}
