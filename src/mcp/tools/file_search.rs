use crate::utils::file_utils::{ensure_path_within_working_dir, glob_match, is_text_file, should_skip_dir};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_DEPTH: usize = 5;
const MAX_FILE_SIZE: u64 = 1_048_576; // 1 MiB

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileSearchParams {
    /// Path to file or directory to search
    #[schemars(description = "Path to file or directory to search")]
    pub path: String,
    /// Keyword to search for
    #[schemars(description = "Keyword to search for")]
    pub keyword: String,
    /// Glob pattern to filter files, e.g. "*.rs" (default: no filter)
    #[schemars(description = "Glob pattern to filter files, e.g. '*.rs'")]
    pub file_pattern: Option<String>,
    /// Use regex for keyword matching (default: false)
    #[schemars(description = "Use regex for keyword matching (default: false)")]
    pub use_regex: Option<bool>,
    /// Maximum number of match results to return (default: 20)
    #[schemars(description = "Maximum number of match results to return (default: 20)")]
    pub max_results: Option<usize>,
    /// Number of context lines before and after each match (default: 3)
    #[schemars(description = "Number of context lines before and after each match (default: 3)")]
    pub context_lines: Option<usize>,
    /// Brief mode: only return file paths and line numbers (default: false)
    #[schemars(description = "Brief mode: only return file paths and line numbers (default: false)")]
    pub brief: Option<bool>,
    /// Output format: "detailed" (default), "compact", "location"
    /// - detailed: full context lines around each match
    /// - compact: only matched line text with file:line prefix
    /// - location: only file path and line numbers, no text content
    #[schemars(description = "Output format: detailed (default), compact, location")]
    pub output_format: Option<String>,
}

#[derive(Debug, Serialize)]
struct MatchResult {
    file: String,
    matches: Vec<MatchEntry>,
}

#[derive(Debug, Serialize)]
struct MatchEntry {
    line: usize,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<Vec<ContextLine>>,
}

#[derive(Debug, Serialize)]
struct ContextLine {
    line: usize,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_match: Option<bool>,
}

#[derive(Debug, Serialize)]
struct BriefResult {
    file: String,
    lines: Vec<usize>,
}

#[derive(Debug, Serialize)]
struct CompactResult {
    file: String,
    matches: Vec<CompactMatch>,
}

#[derive(Debug, Serialize)]
struct CompactMatch {
    line: usize,
    text: String,
}

#[derive(Debug, Serialize)]
struct LocationResult {
    file: String,
    lines: Vec<usize>,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    keyword: String,
    total_matches: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    results: Option<Vec<MatchResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    brief_results: Option<Vec<BriefResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compact_results: Option<Vec<CompactResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location_results: Option<Vec<LocationResult>>,
    searched_files: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_dirs: Vec<String>,
    truncated: bool,
}

fn search_in_file(
    file_path: &Path,
    keyword: &str,
    use_regex: bool,
    context_lines: usize,
    output_format: &str,
) -> Result<Vec<MatchEntry>, String> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read '{}': {}", file_path.display(), e))?;

    let lines: Vec<&str> = content.lines().collect();
    let re = if use_regex {
        Some(
            regex::Regex::new(keyword)
                .map_err(|e| format!("Invalid regex '{}': {}", keyword, e))?,
        )
    } else {
        None
    };

    let keyword_lower = keyword.to_lowercase();
    let mut matches = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let matched = if let Some(ref re) = re {
            re.is_match(line)
        } else {
            line.to_lowercase().contains(&keyword_lower)
        };

        if matched {
            let context = if output_format == "detailed" {
                let start = idx.saturating_sub(context_lines);
                let end = (idx + context_lines + 1).min(lines.len());
                let mut ctx = Vec::new();
                for ci in start..end {
                    ctx.push(ContextLine {
                        line: ci + 1,
                        text: lines[ci].to_string(),
                        is_match: if ci == idx { Some(true) } else { None },
                    });
                }
                Some(ctx)
            } else {
                None
            };

            matches.push(MatchEntry {
                line: idx + 1,
                text: line.to_string(),
                context,
            });
        }
    }

    Ok(matches)
}

fn search_directory(
    dir_path: &Path,
    keyword: &str,
    use_regex: bool,
    file_pattern: Option<&str>,
    context_lines: usize,
    max_results: usize,
    current_depth: usize,
    searched_files: &mut usize,
    skipped_dirs: &mut Vec<String>,
    results: &mut Vec<MatchResult>,
    total_matches: &mut usize,
    output_format: &str,
) -> Result<(), String> {
    if current_depth > MAX_DEPTH {
        skipped_dirs.push(dir_path.to_string_lossy().to_string());
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
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if path.is_dir() {
            if should_skip_dir(&name_str) {
                continue;
            }
            search_directory(
                &path,
                keyword,
                use_regex,
                file_pattern,
                context_lines,
                max_results,
                current_depth + 1,
                searched_files,
                skipped_dirs,
                results,
                total_matches,
                output_format,
            )?;
        } else if path.is_file() {
            if let Some(pat) = file_pattern {
                if !glob_match(pat, &name_str) {
                    continue;
                }
            }

            if !is_text_file(&path) {
                continue;
            }

            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > MAX_FILE_SIZE {
                    continue;
                }
            }

            *searched_files += 1;

            if let Ok(file_matches) = search_in_file(&path, keyword, use_regex, context_lines, output_format) {
                if !file_matches.is_empty() {
                    *total_matches += file_matches.len();
                    results.push(MatchResult {
                        file: path.to_string_lossy().to_string(),
                        matches: file_matches,
                    });
                }
            }
        }
    }

    Ok(())
}

pub async fn file_search(
    params: Parameters<FileSearchParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);
    let keyword = &params.keyword;

    if keyword.is_empty() {
        return Err("Keyword cannot be empty".to_string());
    }

    let canonical_path = ensure_path_within_working_dir(path, working_dir)?;

    if !canonical_path.exists() {
        return Err(format!("Path '{}' does not exist", params.path));
    }

    let use_regex = params.use_regex.unwrap_or(false);
    let max_results = params.max_results.unwrap_or(20);
    let context_lines = params.context_lines.unwrap_or(3);
    let brief = params.brief.unwrap_or(false);
    let output_format = params.output_format.as_deref().unwrap_or("detailed");
    let file_pattern = params.file_pattern.as_deref();

    let mut searched_files = 0usize;
    let mut skipped_dirs: Vec<String> = Vec::new();
    let mut results: Vec<MatchResult> = Vec::new();
    let mut total_matches = 0usize;

    if canonical_path.is_file() {
        searched_files = 1;
        if is_text_file(&canonical_path) {
            if let Ok(file_matches) = search_in_file(&canonical_path, keyword, use_regex, context_lines, output_format) {
                total_matches = file_matches.len();
                if !file_matches.is_empty() {
                    results.push(MatchResult {
                        file: canonical_path.to_string_lossy().to_string(),
                        matches: file_matches,
                    });
                }
            }
        }
    } else if canonical_path.is_dir() {
        search_directory(
            &canonical_path,
            keyword,
            use_regex,
            file_pattern,
            context_lines,
            max_results,
            0,
            &mut searched_files,
            &mut skipped_dirs,
            &mut results,
            &mut total_matches,
            output_format,
        )?;
    }

    let truncated = total_matches > max_results;

    // Truncate results if exceeding max_results
    let mut remaining = max_results;
    let mut final_results: Vec<MatchResult> = Vec::new();
    for mut r in results {
        if remaining == 0 {
            break;
        }
        if r.matches.len() <= remaining {
            remaining -= r.matches.len();
            final_results.push(r);
        } else {
            r.matches.truncate(remaining);
            remaining = 0;
            final_results.push(r);
        }
    }

    let response = if brief {
        let brief_results: Vec<BriefResult> = final_results
            .iter()
            .map(|r| BriefResult {
                file: r.file.clone(),
                lines: r.matches.iter().map(|m| m.line).collect(),
            })
            .collect();

        SearchResponse {
            keyword: keyword.clone(),
            total_matches,
            results: None,
            brief_results: Some(brief_results),
            compact_results: None,
            location_results: None,
            searched_files,
            skipped_dirs,
            truncated,
        }
    } else if output_format == "compact" {
        let compact_results: Vec<CompactResult> = final_results
            .iter()
            .map(|r| CompactResult {
                file: r.file.clone(),
                matches: r.matches.iter().map(|m| CompactMatch {
                    line: m.line,
                    text: m.text.clone(),
                }).collect(),
            })
            .collect();

        SearchResponse {
            keyword: keyword.clone(),
            total_matches,
            results: None,
            brief_results: None,
            compact_results: Some(compact_results),
            location_results: None,
            searched_files,
            skipped_dirs,
            truncated,
        }
    } else if output_format == "location" {
        let location_results: Vec<LocationResult> = final_results
            .iter()
            .map(|r| LocationResult {
                file: r.file.clone(),
                lines: r.matches.iter().map(|m| m.line).collect(),
            })
            .collect();

        SearchResponse {
            keyword: keyword.clone(),
            total_matches,
            results: None,
            brief_results: None,
            compact_results: None,
            location_results: Some(location_results),
            searched_files,
            skipped_dirs,
            truncated,
        }
    } else {
        SearchResponse {
            keyword: keyword.clone(),
            total_matches,
            results: Some(final_results),
            brief_results: None,
            compact_results: None,
            location_results: None,
            searched_files,
            skipped_dirs,
            truncated,
        }
    };

    let json = serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
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
            file_pattern: None,
            use_regex: None,
            max_results: None,
            context_lines: Some(1),
            brief: Some(false),
            output_format: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("Hello World") || text.text.contains("Hello again"));
                assert!(text.text.contains("context"));
            }
        }
    }

    #[tokio::test]
    async fn test_search_compact_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\nThis is a test\nHello again").unwrap();

        let params = FileSearchParams {
            path: file_path.to_string_lossy().to_string(),
            keyword: "hello".to_string(),
            file_pattern: None,
            use_regex: None,
            max_results: None,
            context_lines: Some(1),
            brief: Some(false),
            output_format: Some("compact".to_string()),
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("compact_results"));
                assert!(!text.text.contains("context"));
            }
        }
    }

    #[tokio::test]
    async fn test_search_location_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\nThis is a test\nHello again").unwrap();

        let params = FileSearchParams {
            path: file_path.to_string_lossy().to_string(),
            keyword: "hello".to_string(),
            file_pattern: None,
            use_regex: None,
            max_results: None,
            context_lines: Some(1),
            brief: Some(false),
            output_format: Some("location".to_string()),
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("location_results"));
                assert!(!text.text.contains("Hello World")); // no text content
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
            file_pattern: None,
            use_regex: None,
            max_results: None,
            context_lines: Some(0),
            brief: Some(false),
            output_format: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_regex() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}\nfn helper() {}").unwrap();

        let params = FileSearchParams {
            path: temp_dir.path().to_string_lossy().to_string(),
            keyword: r"fn \w+\(".to_string(),
            file_pattern: Some("*.rs".to_string()),
            use_regex: Some(true),
            max_results: None,
            context_lines: Some(0),
            brief: Some(false),
            output_format: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("fn main()") || text.text.contains("fn helper()"));
            }
        }
    }

    #[tokio::test]
    async fn test_search_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World").unwrap();

        let params = FileSearchParams {
            path: file_path.to_string_lossy().to_string(),
            keyword: "xyznotfound".to_string(),
            file_pattern: None,
            use_regex: None,
            max_results: None,
            context_lines: Some(0),
            brief: Some(false),
            output_format: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("total_matches") && text.text.contains("0"));
            }
        }
    }
}
