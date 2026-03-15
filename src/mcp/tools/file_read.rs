use crate::utils::file_utils::is_text_file;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

const MAX_CHARS: usize = 10 * 1024; // 10KB limit
const DEFAULT_END_LINE: usize = 100;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileReadParams {
    /// File path to read
    #[schemars(description = "The file path to read")]
    pub path: String,
    /// Start line number (0-indexed, default: 0)
    #[schemars(description = "Start line number (0-indexed, default: 0)")]
    pub start_line: Option<usize>,
    /// End line number (exclusive, default: 100)
    #[schemars(description = "End line number (exclusive, default: 100)")]
    pub end_line: Option<usize>,
}

#[derive(Debug)]
struct ReadResult {
    content: String,
    total_lines: usize,
    lines_read: usize,
    truncated: bool,
}

async fn read_file_lines(path: &Path, start_line: usize, end_line: usize) -> Result<ReadResult, String> {
    // Read file content
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;
    
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    
    // Ensure start_line is within bounds
    let start = start_line.min(total_lines);
    let end = end_line.min(total_lines);
    
    // Extract requested lines
    let selected_lines: Vec<&str> = if start < total_lines && start < end {
        lines[start..end].to_vec()
    } else {
        vec![]
    };
    
    let lines_read = selected_lines.len();
    
    // Join lines and check character limit
    let mut result = selected_lines.join("\n");
    let mut truncated = false;
    
    if result.len() > MAX_CHARS {
        // Find the last newline within the limit
        let trunc_point = result[..MAX_CHARS].rfind('\n').unwrap_or(MAX_CHARS);
        result = result[..trunc_point].to_string();
        truncated = true;
    }
    
    Ok(ReadResult {
        content: result,
        total_lines,
        lines_read,
        truncated,
    })
}

pub async fn file_read(params: Parameters<FileReadParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);
    let start_line = params.start_line.unwrap_or(0);
    let end_line = params.end_line.unwrap_or(DEFAULT_END_LINE);

    // Check if file exists
    if !path.exists() {
        return Err(format!("File '{}' does not exist", params.path));
    }

    if !path.is_file() {
        return Err(format!("Path '{}' is not a file", params.path));
    }

    // Check if it's a text file (warning for binary files)
    let is_text = is_text_file(path);

    // Read file content with line range
    let read_result = match read_file_lines(path, start_line, end_line).await {
        Ok(result) => result,
        Err(e) if !is_text => {
            return Err(format!(
                "File '{}' appears to be binary and cannot be read as text: {}",
                params.path, e
            ));
        }
        Err(e) => return Err(e),
    };

    // Build response
    let mut response = String::new();
    
    // Add content
    response.push_str(&read_result.content);
    
    // Add truncation warning if needed
    if read_result.truncated {
        response.push_str("\n\n[... Content truncated: character count exceeds 10KB limit in current line range, last line truncated ...]");
    }
    
    // Add line info
    let has_more = end_line < read_result.total_lines || read_result.truncated;
    let is_partial = read_result.lines_read < read_result.total_lines;
    
    if is_partial || has_more {
        response.push_str(&format!(
            "\n\n[File info: total_lines={}, lines_displayed={} (line {} to line {})",
            read_result.total_lines,
            read_result.lines_read,
            start_line,
            start_line + read_result.lines_read
        ));
        
        if has_more {
            response.push_str(&format!(
                ", {} lines not shown",
                read_result.total_lines.saturating_sub(end_line)
            ));
        }
        
        response.push_str("]");
        
        // Add suggestion for reading more
        if has_more {
            response.push_str(&format!(
                "\n[Hint: To read more content, use parameters start_line={} end_line={}]",
                end_line,
                end_line + DEFAULT_END_LINE
            ));
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
        };

        let result = file_read(Parameters(params)).await;
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
    async fn test_file_read_not_found() {
        let params = FileReadParams {
            path: "/nonexistent/file.txt".to_string(),
            start_line: None,
            end_line: None,
        };

        let result = file_read(Parameters(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_read_truncation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        // Create content that exceeds 10KB within 100 lines
        let line = "a".repeat(200);
        let content: String = (0..60).map(|i| format!("Line {}: {}\n", i, line)).collect();
        fs::write(&file_path, &content).unwrap();

        let params = FileReadParams {
            path: file_path.to_string_lossy().to_string(),
            start_line: Some(0),
            end_line: Some(100),
        };

        let result = file_read(Parameters(params)).await;
        assert!(result.is_ok());
        
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("truncated"));
                assert!(text.text.contains("total_lines="));
            }
        }
    }
}
