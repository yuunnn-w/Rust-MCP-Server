use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

const MAX_DEPTH: usize = 3;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileSearchParams {
    /// Path to file or directory to search
    #[schemars(description = "Path to file or directory to search")]
    pub path: String,
    /// Keyword to search for
    #[schemars(description = "Keyword to search for")]
    pub keyword: String,
}

#[derive(Debug)]
struct SearchResult {
    file_path: String,
    line_numbers: Vec<usize>,
}

#[derive(Debug)]
struct SearchSummary {
    results: Vec<SearchResult>,
    skipped_dirs: Vec<String>,
    searched_files: usize,
    matched_files: usize,
}

/// Check if content is valid UTF-8 text by attempting decode
fn is_valid_text(content: &[u8]) -> bool {
    // Try UTF-8 first
    if std::str::from_utf8(content).is_ok() {
        return true;
    }
    // Could add more encoding checks here (GBK, etc.)
    false
}

/// Search for keyword in a single file
fn search_in_file(file_path: &Path, keyword: &str) -> Result<Option<SearchResult>, String> {
    // Read file content
    let content = match std::fs::read(file_path) {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to read '{}': {}", file_path.display(), e)),
    };
    
    // Check if it's valid text
    if !is_valid_text(&content) {
        return Ok(None); // Skip binary files
    }
    
    // Convert to string (we know it's valid UTF-8)
    let text = String::from_utf8_lossy(&content);
    
    // Search for keyword (case-insensitive)
    let keyword_lower = keyword.to_lowercase();
    let mut line_numbers = Vec::new();
    
    for (line_num, line) in text.lines().enumerate() {
        if line.to_lowercase().contains(&keyword_lower) {
            line_numbers.push(line_num);
        }
    }
    
    if line_numbers.is_empty() {
        Ok(None)
    } else {
        Ok(Some(SearchResult {
            file_path: file_path.to_string_lossy().to_string(),
            line_numbers,
        }))
    }
}

/// Recursively search directory
fn search_directory(
    dir_path: &Path,
    keyword: &str,
    current_depth: usize,
    summary: &mut SearchSummary,
) -> Result<(), String> {
    if current_depth > MAX_DEPTH {
        summary.skipped_dirs.push(dir_path.to_string_lossy().to_string());
        return Ok(());
    }
    
    let entries = match std::fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to read directory '{}': {}", dir_path.display(), e)),
    };
    
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let path = entry.path();
        
        if path.is_file() {
            summary.searched_files += 1;
            if let Some(result) = search_in_file(&path, keyword)? {
                summary.matched_files += 1;
                summary.results.push(result);
            }
        } else if path.is_dir() {
            search_directory(&path, keyword, current_depth + 1, summary)?;
        }
    }
    
    Ok(())
}

pub async fn file_search(params: Parameters<FileSearchParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);
    let keyword = &params.keyword;
    
    if keyword.is_empty() {
        return Err("Keyword cannot be empty".to_string());
    }
    
    // Check if path exists
    if !path.exists() {
        return Err(format!("Path '{}' does not exist", params.path));
    }
    
    let mut summary = SearchSummary {
        results: Vec::new(),
        skipped_dirs: Vec::new(),
        searched_files: 0,
        matched_files: 0,
    };
    
    if path.is_file() {
        // Search single file
        summary.searched_files = 1;
        if let Some(result) = search_in_file(path, keyword)? {
            summary.matched_files = 1;
            summary.results.push(result);
        }
    } else if path.is_dir() {
        // Search directory recursively
        search_directory(path, keyword, 0, &mut summary)?;
    }
    
    // Build response
    let mut response = String::new();
    
    if summary.results.is_empty() {
        response.push_str(&format!(
            "No files containing keyword '{}' found.\n\nSearch stats:\n- Files searched: {}\n- Files matched: 0",
            keyword, summary.searched_files
        ));
    } else {
        response.push_str(&format!("Found {} file(s) containing keyword '{}':\n\n", summary.results.len(), keyword));
        
        for result in &summary.results {
            response.push_str(&format!("File: {}\n", result.file_path));
            
            // Group consecutive line numbers into ranges
            if !result.line_numbers.is_empty() {
                let mut ranges = Vec::new();
                let mut start = result.line_numbers[0];
                let mut prev = result.line_numbers[0];
                
                for &line_num in &result.line_numbers[1..] {
                    if line_num == prev + 1 {
                        prev = line_num;
                    } else {
                        if start == prev {
                            ranges.push(format!("line {}", start + 1));
                        } else {
                            ranges.push(format!("line {}-{}", start + 1, prev + 1));
                        }
                        start = line_num;
                        prev = line_num;
                    }
                }
                
                if start == prev {
                    ranges.push(format!("line {}", start + 1));
                } else {
                    ranges.push(format!("line {}-{}", start + 1, prev + 1));
                }
                
                response.push_str(&format!("   Match locations: {}\n", ranges.join(", ")));
            }
            
            response.push('\n');
        }
        
        response.push_str(&format!(
            "\nSearch stats:\n- Files searched: {}\n- Files matched: {}",
            summary.searched_files, summary.matched_files
        ));
    }
    
    // Add depth limit warning
    if !summary.skipped_dirs.is_empty() {
        response.push_str(&format!(
            "\n\nNote: Search depth limited to {} levels, the following directories were not fully searched:",
            MAX_DEPTH
        ));
        for dir in &summary.skipped_dirs {
            response.push_str(&format!("\n- {}", dir));
        }
        response.push_str("\n\nTo search deeper directories, please search directly in the subdirectory path.");
    }
    
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(response)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_search_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\nThis is a test\nHello again").unwrap();

        let params = FileSearchParams {
            path: file_path.to_string_lossy().to_string(),
            keyword: "hello".to_string(),
        };

        let result = file_search(Parameters(params)).await;
        assert!(result.is_ok());
        
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("Found"));
                assert!(text.text.contains("test.txt"));
            }
        }
    }

    #[tokio::test]
    async fn test_search_directory() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "Hello World").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "Goodbye World").unwrap();

        let params = FileSearchParams {
            path: temp_dir.path().to_string_lossy().to_string(),
            keyword: "hello".to_string(),
        };

        let result = file_search(Parameters(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World").unwrap();

        let params = FileSearchParams {
            path: file_path.to_string_lossy().to_string(),
            keyword: "xyznotfound".to_string(),
        };

        let result = file_search(Parameters(params)).await;
        assert!(result.is_ok());
        
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("No files"));
            }
        }
    }
}
