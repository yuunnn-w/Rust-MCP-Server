use crate::utils::enhanced_glob::GlobMatcher;
use crate::utils::file_utils::{glob_match, is_text_file, resolve_path, should_skip_dir};
use crate::utils::office_utils::{detect_office_format, extract_text_from_bytes, OfficeFormat};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_DEPTH: usize = 10;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GrepParams {
    /// Path to file or directory to search
    #[schemars(description = "Path to file or directory to search")]
    pub path: String,
    /// Keyword or regex pattern to search for
    #[schemars(description = "Keyword or regex pattern to search for")]
    pub pattern: String,
    /// Use regex for pattern matching (default: false)
    #[schemars(description = "Use regex for pattern matching (default: false)")]
    pub use_regex: Option<bool>,
    /// Glob pattern to filter files, e.g. "*.rs" (default: no filter)
    #[schemars(description = "Glob pattern to filter files, e.g. '*.rs'")]
    pub file_pattern: Option<String>,
    /// Maximum depth to traverse (default: 5, max: 10)
    #[schemars(description = "Maximum depth to traverse (default: 5, max: 10)")]
    pub max_depth: Option<usize>,
    /// Maximum number of match results to return (default: 20)
    #[schemars(description = "Maximum number of match results to return (default: 20)")]
    pub max_results: Option<usize>,
    /// Number of context lines before and after each match (default: 3)
    #[schemars(description = "Number of context lines before and after each match (default: 3)")]
    pub context_lines: Option<usize>,
    /// Output format: "detailed" (default), "compact", "location", "brief"
    #[schemars(description = "Output format: detailed (default, full context), compact (matched lines only), location (file:line only), brief (file + line numbers)")]
    pub output_mode: Option<String>,
    /// Glob patterns to include files (only search files matching these)
    #[schemars(description = "Glob patterns to include files (only search files matching at least one pattern)")]
    pub include: Option<Vec<String>>,
    /// Glob patterns to exclude files (skip files matching any of these)
    #[schemars(description = "Glob patterns to exclude files (skip files matching any pattern)")]
    pub exclude: Option<Vec<String>>,
    /// Case-sensitive search (default: false)
    #[schemars(description = "Case-sensitive search (default: false)")]
    pub case_sensitive: Option<bool>,
    /// Match whole words only (default: false)
    #[schemars(description = "Match whole words only (default: false)")]
    pub whole_word: Option<bool>,
    /// Enable multiline mode for regex (default: false)
    #[schemars(description = "Enable multiline mode for regex (default: false)")]
    pub multiline: Option<bool>,
    /// Maximum file size in bytes (skip files larger than this, default: 1MB)
    #[schemars(description = "Maximum file size in bytes (skip files larger than this, default: 1048576)")]
    pub max_file_size: Option<u64>,
    /// Search binary/office documents (DOCX/PPTX/XLSX/PDF/IPYNB) for text content (default: true)
    #[schemars(description = "Search binary/office documents (DOCX/PPTX/XLSX/PDF/IPYNB) for text content (default: true)")]
    pub search_binary: Option<bool>,
}

#[derive(Debug, Serialize)]
struct MatchResult {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    lines: Vec<usize>,
}

#[derive(Debug, Serialize)]
struct CompactResult {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    lines: Vec<usize>,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    pattern: String,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_files: Vec<String>,
    truncated: bool,
}

#[allow(clippy::too_many_arguments)]
fn search_in_text(
    content: &str,
    pattern: &str,
    precompiled_re: Option<&regex::Regex>,
    case_sensitive: bool,
    context_lines: usize,
    output_mode: &str,
    max_matches: usize,
    current_match_count: &mut usize,
) -> Vec<MatchEntry> {
    let lines: Vec<&str> = content.lines().collect();
    let pattern_lower: Option<String> = if case_sensitive { None } else { Some(pattern.to_lowercase()) };
    let mut matches = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if *current_match_count >= max_matches {
            break;
        }

        let matched = if let Some(re) = precompiled_re {
            re.is_match(line)
        } else if case_sensitive {
            line.contains(pattern)
        } else {
            line.to_lowercase().contains(pattern_lower.as_deref().unwrap_or(""))
        };

        if matched {
            let context = if output_mode == "detailed" {
                let start = idx.saturating_sub(context_lines);
                let end = (idx + context_lines + 1).min(lines.len());
                let mut ctx = Vec::new();
                for (ci, line) in lines.iter().enumerate().skip(start).take(end - start) {
                    ctx.push(ContextLine {
                        line: ci + 1,
                        text: line.to_string(),
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
            *current_match_count += 1;
        }
    }

    matches
}

#[allow(clippy::too_many_arguments)]
fn search_office_file(
    file_path: &Path,
    pattern: &str,
    precompiled_re: Option<&regex::Regex>,
    case_sensitive: bool,
    context_lines: usize,
    output_mode: &str,
    max_matches: usize,
    current_match_count: &mut usize,
) -> Result<Vec<MatchEntry>, String> {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let format = detect_office_format(ext);
    if format == OfficeFormat::Unknown {
        return Err("Not an office file".to_string());
    }

    let bytes = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read '{}': {}", file_path.display(), e))?;
    let text = extract_text_from_bytes(&bytes, format, None)?;

    Ok(search_in_text(
        &text,
        pattern,
        precompiled_re,
        case_sensitive,
        context_lines,
        output_mode,
        max_matches,
        current_match_count,
    ))
}

#[allow(clippy::too_many_arguments)]
fn search_text_file(
    file_path: &Path,
    pattern: &str,
    precompiled_re: Option<&regex::Regex>,
    case_sensitive: bool,
    context_lines: usize,
    output_mode: &str,
    max_matches: usize,
    current_match_count: &mut usize,
) -> Result<Vec<MatchEntry>, String> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read '{}': {}", file_path.display(), e))?;

    Ok(search_in_text(
        &content,
        pattern,
        precompiled_re,
        case_sensitive,
        context_lines,
        output_mode,
        max_matches,
        current_match_count,
    ))
}

fn is_office_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    detect_office_format(ext) != OfficeFormat::Unknown
}

#[allow(clippy::too_many_arguments)]
fn search_directory(
    dir_path: &Path,
    pattern: &str,
    precompiled_re: Option<&regex::Regex>,
    case_sensitive: bool,
    file_pattern: Option<&str>,
    file_matcher: Option<&GlobMatcher>,
    context_lines: usize,
    max_results: usize,
    max_depth: usize,
    current_depth: usize,
    max_file_size: u64,
    search_binary: bool,
    searched_files: &mut usize,
    skipped_dirs: &mut Vec<String>,
    skipped_files: &mut Vec<String>,
    results: &mut Vec<MatchResult>,
    total_matches: &mut usize,
    output_mode: &str,
) -> Result<(), String> {
    if current_depth > max_depth {
        skipped_dirs.push(dir_path.to_string_lossy().to_string());
        return Ok(());
    }

    if *total_matches >= max_results {
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

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            if should_skip_dir(&name_str) {
                continue;
            }
            search_directory(
                &path,
                pattern,
                precompiled_re,
                case_sensitive,
                file_pattern,
                file_matcher,
                context_lines,
                max_results,
                max_depth,
                current_depth + 1,
                max_file_size,
                search_binary,
                searched_files,
                skipped_dirs,
                skipped_files,
                results,
                total_matches,
                output_mode,
            )?;
            if *total_matches >= max_results {
                return Ok(());
            }
        } else if file_type.is_file() {
            if let Some(pat) = file_pattern {
                if !glob_match(pat, &name_str) {
                    continue;
                }
            }

            if let Some(matcher) = file_matcher {
                if !matcher.matches(&name_str) {
                    continue;
                }
            }

            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > max_file_size {
                    continue;
                }
            }

            let is_office = is_office_file(&path);

            if !is_office && !is_text_file(&path) {
                continue;
            }

            if is_office && !search_binary {
                continue;
            }

            *searched_files += 1;

            let file_matches = if is_office {
                match search_office_file(
                    &path,
                    pattern,
                    precompiled_re,
                    case_sensitive,
                    context_lines,
                    output_mode,
                    max_results,
                    total_matches,
                ) {
                    Ok(m) => m,
                    Err(e) => {
                        skipped_files.push(format!("{}: {}", path.display(), e));
                        vec![]
                    }
                }
            } else {
                match search_text_file(
                    &path,
                    pattern,
                    precompiled_re,
                    case_sensitive,
                    context_lines,
                    output_mode,
                    max_results,
                    total_matches,
                ) {
                    Ok(m) => m,
                    Err(e) => {
                        skipped_files.push(format!("{}: {}", path.display(), e));
                        vec![]
                    }
                }
            };

            if !file_matches.is_empty() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let format = if is_office {
                    Some(format!("{:?}", detect_office_format(ext)))
                } else {
                    None
                };
                results.push(MatchResult {
                    file: path.to_string_lossy().to_string(),
                    format,
                    matches: file_matches,
                });
            }

            if *total_matches >= max_results {
                return Ok(());
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn perform_file_search(
    canonical_path: std::path::PathBuf,
    pattern: String,
    use_regex: bool,
    case_sensitive: bool,
    whole_word: bool,
    multiline: bool,
    max_results: usize,
    context_lines: usize,
    output_mode: String,
    file_pattern: Option<String>,
    include: Vec<String>,
    exclude: Vec<String>,
    max_depth: usize,
    max_file_size: u64,
    search_binary: bool,
) -> Result<SearchResponse, String> {
    let precompiled_re = if use_regex {
        let mut regex_str = String::new();
        if !case_sensitive {
            regex_str.push_str("(?i)");
        }
        if multiline {
            regex_str.push_str("(?m)");
        }
        if whole_word {
            regex_str.push_str(&format!(r"\b{}\b", pattern));
        } else {
            regex_str.push_str(&pattern);
        }
        Some(
            regex::Regex::new(&regex_str)
                .map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?,
        )
    } else if whole_word {
        let mut regex_str = String::new();
        if !case_sensitive {
            regex_str.push_str("(?i)");
        }
        if multiline {
            regex_str.push_str("(?m)");
        }
        regex_str.push_str(&format!(r"\b{}\b", regex::escape(&pattern)));
        Some(
            regex::Regex::new(&regex_str)
                .map_err(|e| format!("Invalid whole-word pattern '{}': {}", pattern, e))?,
        )
    } else {
        None
    };

    let file_matcher = if !include.is_empty() || !exclude.is_empty() {
        Some(
            GlobMatcher::new(&include, &exclude, false, true)
                .map_err(|e| format!("Invalid file pattern: {}", e))?,
        )
    } else {
        None
    };

    let mut searched_files = 0usize;
    let mut skipped_dirs: Vec<String> = Vec::new();
    let mut skipped_files: Vec<String> = Vec::new();
    let mut results: Vec<MatchResult> = Vec::new();
    let mut total_matches = 0usize;

    if canonical_path.is_file() {
        let is_office = is_office_file(&canonical_path);

        if !is_office && !is_text_file(&canonical_path) {
            return Ok(SearchResponse {
                pattern,
                total_matches: 0,
                results: None,
                brief_results: None,
                compact_results: None,
                location_results: None,
                searched_files: 0,
                skipped_dirs: vec![],
                skipped_files: vec![],
                truncated: false,
            });
        }

        if is_office && !search_binary {
            return Ok(SearchResponse {
                pattern,
                total_matches: 0,
                results: None,
                brief_results: None,
                compact_results: None,
                location_results: None,
                searched_files: 0,
                skipped_dirs: vec![],
                skipped_files: vec![],
                truncated: false,
            });
        }

        searched_files = 1;

        let file_matches = if is_office {
            match search_office_file(
                &canonical_path,
                &pattern,
                precompiled_re.as_ref(),
                case_sensitive,
                context_lines,
                &output_mode,
                max_results,
                &mut total_matches,
            ) {
                Ok(m) => m,
                Err(e) => {
                    skipped_files.push(format!("{}: {}", canonical_path.display(), e));
                    vec![]
                }
            }
        } else {
            match search_text_file(
                &canonical_path,
                &pattern,
                precompiled_re.as_ref(),
                case_sensitive,
                context_lines,
                &output_mode,
                max_results,
                &mut total_matches,
            ) {
                Ok(m) => m,
                Err(e) => {
                    skipped_files.push(format!("{}: {}", canonical_path.display(), e));
                    vec![]
                }
            }
        };

        if !file_matches.is_empty() {
            let ext = canonical_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let format = if is_office {
                Some(format!("{:?}", detect_office_format(ext)))
            } else {
                None
            };
            results.push(MatchResult {
                file: canonical_path.to_string_lossy().to_string(),
                format,
                matches: file_matches,
            });
        }
    } else if canonical_path.is_dir() {
        search_directory(
            &canonical_path,
            &pattern,
            precompiled_re.as_ref(),
            case_sensitive,
            file_pattern.as_deref(),
            file_matcher.as_ref(),
            context_lines,
            max_results,
            max_depth,
            0,
            max_file_size,
            search_binary,
            &mut searched_files,
            &mut skipped_dirs,
            &mut skipped_files,
            &mut results,
            &mut total_matches,
            &output_mode,
        )?;
    }

    let truncated = total_matches >= max_results;

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

    match output_mode.as_str() {
        "compact" => {
            let compact_results: Vec<CompactResult> = final_results
                .iter()
                .map(|r| CompactResult {
                    file: r.file.clone(),
                    format: r.format.clone(),
                    matches: r
                        .matches
                        .iter()
                        .map(|m| CompactMatch {
                            line: m.line,
                            text: m.text.clone(),
                        })
                        .collect(),
                })
                .collect();

            Ok(SearchResponse {
                pattern,
                total_matches,
                results: None,
                brief_results: None,
                compact_results: Some(compact_results),
                location_results: None,
                searched_files,
                skipped_dirs,
                skipped_files,
                truncated,
            })
        }
        "location" => {
            let location_results: Vec<LocationResult> = final_results
                .iter()
                .map(|r| LocationResult {
                    file: r.file.clone(),
                    format: r.format.clone(),
                    lines: r.matches.iter().map(|m| m.line).collect(),
                })
                .collect();

            Ok(SearchResponse {
                pattern,
                total_matches,
                results: None,
                brief_results: None,
                compact_results: None,
                location_results: Some(location_results),
                searched_files,
                skipped_dirs,
                skipped_files,
                truncated,
            })
        }
        "brief" => {
            let brief_results: Vec<BriefResult> = final_results
                .iter()
                .map(|r| BriefResult {
                    file: r.file.clone(),
                    format: r.format.clone(),
                    lines: r.matches.iter().map(|m| m.line).collect(),
                })
                .collect();

            Ok(SearchResponse {
                pattern,
                total_matches,
                results: None,
                brief_results: Some(brief_results),
                compact_results: None,
                location_results: None,
                searched_files,
                skipped_dirs,
                skipped_files,
                truncated,
            })
        }
        _ => {
            Ok(SearchResponse {
                pattern,
                total_matches,
                results: Some(final_results),
                brief_results: None,
                compact_results: None,
                location_results: None,
                searched_files,
                skipped_dirs,
                skipped_files,
                truncated,
            })
        }
    }
}

pub async fn file_search(
    params: Parameters<GrepParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let path = Path::new(&params.path);
    let path_str = params.path.clone();
    let pattern = params.pattern;

    if pattern.is_empty() {
        return Err("Pattern cannot be empty".to_string());
    }

    let canonical_path = resolve_path(path, working_dir)?;

    if !canonical_path.exists() {
        return Err(format!("Path '{}' does not exist", path_str));
    }

    let use_regex = params.use_regex.unwrap_or(false);
    let case_sensitive = params.case_sensitive.unwrap_or(false);
    let whole_word = params.whole_word.unwrap_or(false);
    let multiline = params.multiline.unwrap_or(false);
    let max_results = params.max_results.unwrap_or(20);
    let context_lines = params.context_lines.unwrap_or(3);
    let output_mode = params.output_mode.as_deref().unwrap_or("detailed").to_string();
    let file_pattern = params.file_pattern;
    let include = params.include.unwrap_or_default();
    let exclude = params.exclude.unwrap_or_default();
    let max_depth = params.max_depth.unwrap_or(5).min(MAX_DEPTH);
    let max_file_size = params.max_file_size.unwrap_or(1_048_576);
    let search_binary = params.search_binary.unwrap_or(true);

    let response = tokio::task::spawn_blocking(move || {
        perform_file_search(
            canonical_path,
            pattern,
            use_regex,
            case_sensitive,
            whole_word,
            multiline,
            max_results,
            context_lines,
            output_mode,
            file_pattern,
            include,
            exclude,
            max_depth,
            max_file_size,
            search_binary,
        )
    })
    .await
    .map_err(|e| format!("Search task failed: {}", e))??;

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

        let params = GrepParams {
            path: file_path.to_string_lossy().to_string(),
            pattern: "hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(1),
            output_mode: None,
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
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

        let params = GrepParams {
            path: file_path.to_string_lossy().to_string(),
            pattern: "hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(1),
            output_mode: Some("compact".to_string()),
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
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

        let params = GrepParams {
            path: file_path.to_string_lossy().to_string(),
            pattern: "hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(1),
            output_mode: Some("location".to_string()),
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("location_results"));
                assert!(!text.text.contains("Hello World"));
            }
        }
    }

    #[tokio::test]
    async fn test_search_directory() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "Hello World").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "Goodbye World").unwrap();

        let params = GrepParams {
            path: temp_dir.path().to_string_lossy().to_string(),
            pattern: "hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(0),
            output_mode: None,
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_regex() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}\nfn helper() {}").unwrap();

        let params = GrepParams {
            path: temp_dir.path().to_string_lossy().to_string(),
            pattern: r"fn \w+\(".to_string(),
            use_regex: Some(true),
            file_pattern: Some("*.rs".to_string()),
            max_depth: None,
            max_results: None,
            context_lines: Some(0),
            output_mode: None,
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
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

        let params = GrepParams {
            path: file_path.to_string_lossy().to_string(),
            pattern: "xyznotfound".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(0),
            output_mode: None,
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("total_matches") && text.text.contains("0"));
            }
        }
    }

    #[tokio::test]
    async fn test_search_max_results_early_stop() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "Hello World\nHello Again").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "Hello Universe\nHello Galaxy").unwrap();

        let params = GrepParams {
            path: temp_dir.path().to_string_lossy().to_string(),
            pattern: "hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: Some(2),
            context_lines: Some(0),
            output_mode: None,
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("\"truncated\": true"));
                assert!(text.text.contains("\"total_matches\": 2"));
            }
        }
    }

    #[tokio::test]
    async fn test_search_brief_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\nHello again").unwrap();

        let params = GrepParams {
            path: file_path.to_string_lossy().to_string(),
            pattern: "hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(0),
            output_mode: Some("brief".to_string()),
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("brief_results"));
            }
        }
    }

    #[tokio::test]
    async fn test_search_case_sensitive() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\nhello world").unwrap();

        let params = GrepParams {
            path: file_path.to_string_lossy().to_string(),
            pattern: "Hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(0),
            output_mode: None,
            include: None,
            exclude: None,
            case_sensitive: Some(true),
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
                let count = parsed["total_matches"].as_u64().unwrap();
                assert_eq!(count, 1);
            }
        }
    }

    #[tokio::test]
    async fn test_search_whole_word() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "HelloWorld\nHello World\nHello").unwrap();

        let params = GrepParams {
            path: file_path.to_string_lossy().to_string(),
            pattern: "Hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(0),
            output_mode: None,
            include: None,
            exclude: None,
            case_sensitive: None,
            whole_word: Some(true),
            multiline: None,
            max_file_size: None,
            search_binary: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
                let count = parsed["total_matches"].as_u64().unwrap();
                assert_eq!(count, 2);
                assert!(text.text.contains("Hello World"));
                assert!(text.text.contains("\"Hello\"")); // line 3 "Hello"
            }
        }
    }

    #[tokio::test]
    async fn test_search_exclude_pattern() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "Hello").unwrap();
        fs::write(temp_dir.path().join("file2.log"), "Hello").unwrap();

        let params = GrepParams {
            path: temp_dir.path().to_string_lossy().to_string(),
            pattern: "Hello".to_string(),
            use_regex: None,
            file_pattern: None,
            max_depth: None,
            max_results: None,
            context_lines: Some(0),
            output_mode: None,
            include: None,
            exclude: Some(vec!["*.log".to_string()]),
            case_sensitive: None,
            whole_word: None,
            multiline: None,
            max_file_size: None,
            search_binary: None,
        };

        let result = file_search(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("file1.txt"));
                assert!(!text.text.contains("file2.log"));
            }
        }
    }
}
