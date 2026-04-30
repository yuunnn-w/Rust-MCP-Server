use std::path::{Path, PathBuf};
use std::time::SystemTime;
use chrono::{DateTime, Local};

/// Match a glob pattern against a file name. Supports * and ? wildcards.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let mut regex_str = String::new();
    regex_str.push('^');
    for ch in pattern.chars() {
        match ch {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' | '^' | '$' | '|' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            _ => regex_str.push(ch),
        }
    }
    regex_str.push('$');
    regex::Regex::new(&regex_str)
        .map(|re| re.is_match(name))
        .unwrap_or(false)
}

/// Strip Windows UNC prefix (`\\?\`) from path strings.
/// On non-Windows platforms this is a no-op.
pub fn strip_unc_prefix(path_str: &str) -> String {
    if path_str.starts_with("\\\\?\\") {
        path_str[4..].to_string()
    } else {
        path_str.to_string()
    }
}

/// Information about a text file.
#[derive(Debug, Clone)]
pub struct TextFileInfo {
    pub char_count: usize,
    pub line_count: usize,
}

/// Try to read a file and decode it as UTF-8 text (async).
/// Returns `Some(TextFileInfo)` if successful, `None` if the file is binary or unreadable.
pub async fn get_text_file_info(path: &Path) -> Option<TextFileInfo> {
    let bytes = tokio::fs::read(path).await.ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let char_count = text.chars().count();
    let line_count = text.lines().count();
    Some(TextFileInfo { char_count, line_count })
}

/// Synchronous version of `get_text_file_info`.
pub fn get_text_file_info_sync(path: &Path) -> Option<TextFileInfo> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let char_count = text.chars().count();
    let line_count = text.lines().count();
    Some(TextFileInfo { char_count, line_count })
}

/// Format file size to human readable string
pub fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if size == 0 {
        return "0 B".to_string();
    }
    let exp = (size as f64).log(1024.0).min(UNITS.len() as f64 - 1.0) as usize;
    let size = size as f64 / 1024_f64.powi(exp as i32);
    if exp == 0 {
        format!("{} {}", size as u64, UNITS[exp])
    } else {
        format!("{:.2} {}", size, UNITS[exp])
    }
}

/// Check if a path is within the working directory
pub fn is_path_within_working_dir(path: &Path, working_dir: &Path) -> bool {
    match (path.canonicalize(), working_dir.canonicalize()) {
        (Ok(canonical_path), Ok(canonical_working)) => {
            canonical_path.starts_with(&canonical_working)
        }
        _ => {
            // Fallback: normalize and check with absolute paths
            match (path.absolutize(), working_dir.absolutize()) {
                (Ok(abs_path), Ok(abs_working)) => {
                    let norm_path = normalize_path(&abs_path);
                    let norm_working = normalize_path(&abs_working);
                    norm_path.starts_with(&norm_working)
                }
                _ => false,
            }
        }
    }
}

/// Ensure path is within working directory, returning canonicalized path or error
/// For new files, the parent directory must exist for canonicalization to work
pub fn ensure_path_within_working_dir(
    path: &Path,
    working_dir: &Path,
) -> Result<PathBuf, String> {
    // Get canonical working directory first
    let canonical_working = working_dir
        .canonicalize()
        .map_err(|e| format!("Invalid working directory '{}': {}", working_dir.display(), e))?;

    // Try to canonicalize the path (works if path exists)
    let path_to_check = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist, try to resolve it relative to working dir
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                canonical_working.join(path)
            }
        }
    };

    // For paths that don't exist, we need to check the parent
    let check_path = if path_to_check.exists() {
        &path_to_check
    } else {
        // Check if the resolved path would be within working dir
        // by comparing parent directories
        match path_to_check.parent() {
            Some(parent) => {
                // If parent doesn't exist either, we can't verify,
                // but we can check if the path itself starts with working dir
                if !parent.exists() {
                    if path_to_check.starts_with(&canonical_working) {
                        return Ok(path_to_check);
                    } else {
                        return Err(format!(
                            "Access denied: path '{}' is outside working directory '{}'",
                            path_to_check.display(),
                            canonical_working.display()
                        ));
                    }
                }
                // Canonicalize parent to verify
                match parent.canonicalize() {
                    Ok(canonical_parent) => {
                        if canonical_parent.starts_with(&canonical_working) {
                            return Ok(path_to_check);
                        } else {
                            return Err(format!(
                                "Access denied: path '{}' is outside working directory '{}'",
                                path_to_check.display(),
                                canonical_working.display()
                            ));
                        }
                    }
                    Err(e) => {
                        return Err(format!(
                            "Cannot resolve parent directory '{}': {}",
                            parent.display(),
                            e
                        ));
                    }
                }
            }
            None => {
                // No parent (root path)
                let norm_path = normalize_path(&path_to_check);
                if norm_path.starts_with(&canonical_working) {
                    return Ok(path_to_check);
                } else {
                    return Err(format!(
                        "Access denied: path '{}' is outside working directory '{}'",
                        path_to_check.display(),
                        canonical_working.display()
                    ));
                }
            }
        }
    };

    // Check if canonical path is within working directory
    let canonical_check = check_path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path '{}': {}", check_path.display(), e))?;

    if canonical_check.starts_with(&canonical_working) {
        Ok(canonical_check)
    } else {
        Err(format!(
            "Access denied: path '{}' is outside working directory '{}'",
            canonical_check.display(),
            canonical_working.display()
        ))
    }
}

/// Format system time to local datetime string
pub fn format_datetime(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Get file extension
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

/// Check if path is a text file based on extension
pub fn is_text_file(path: &Path) -> bool {
    const TEXT_EXTENSIONS: &[&str] = &[
        "txt", "md", "rs", "py", "js", "ts", "json", "yaml", "yml", "toml",
        "html", "css", "xml", "csv", "log", "conf", "config", "ini",
        "sh", "bash", "zsh", "ps1", "bat", "cmd",
        "c", "cpp", "h", "hpp", "java", "go", "rb", "php", "swift",
        "kt", "scala", "r", "m", "mm", "sql", "lua", "pl", "pm",
        // Extended for modern projects
        "vue", "svelte", "tsx", "jsx", "dart", "nix", "dockerfile",
        "gradle", "kts", "purs", "hs", "lhs", "elm", "erl", "hrl",
        "ex", "exs", "clj", "cljs", "edn", "tf", "hcl", "proto",
        "graphql", "gql", "prisma", "makefile", "mk", "cmake",
        "ninja", "patch", "diff", "lock", "sum", "mod",
    ];

    match get_file_extension(path) {
        Some(ext) => TEXT_EXTENSIONS.contains(&ext.as_str()),
        None => true, // Files without extension are treated as text
    }
}

/// Check if a directory should be skipped during recursive traversal
/// (common build/output directories that waste time and context)
pub fn should_skip_dir(name: &str) -> bool {
    const SKIP_DIRS: &[&str] = &[
        ".git",
        "target",
        "node_modules",
        "__pycache__",
        ".venv",
        "venv",
        "dist",
        "build",
        ".idea",
        ".vscode",
        "out",
        "coverage",
        ".cargo",
    ];
    SKIP_DIRS.contains(&name)
}

/// Read file lines with optional line number prefix and character limit.
/// Returns the rendered text, number of lines rendered, whether truncated,
/// and the total line count of the file.
pub async fn read_file_with_options(
    path: &Path,
    start_line: usize,
    end_line: usize,
    max_chars: usize,
    line_numbers: bool,
) -> Result<(String, usize, bool, usize), String> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let start = start_line.min(total_lines);
    let end = end_line.min(total_lines);

    let selected: Vec<&str> = if start < total_lines && start < end {
        lines[start..end].to_vec()
    } else {
        vec![]
    };

    let mut result = String::new();
    let mut chars_count = 0;
    let mut truncated = false;
    let mut lines_included = 0;

    for (idx, line) in selected.iter().enumerate() {
        let line_num = start + idx;
        let formatted = if line_numbers {
            format!("{:4} | {}\n", line_num, line)
        } else {
            format!("{}\n", line)
        };

        if chars_count + formatted.len() > max_chars {
            truncated = true;
            break;
        }

        chars_count += formatted.len();
        result.push_str(&formatted);
        lines_included += 1;
    }

    // Trim trailing newline for cleaner output
    if result.ends_with('\n') {
        result.pop();
    }

    Ok((result, lines_included, truncated, total_lines))
}

/// Normalize a path by resolving `.` and `..` components without touching the filesystem.
/// Unlike `canonicalize()`, this does not require the path to exist.
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => result.push(prefix.as_os_str()),
            Component::RootDir => result.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                // Only pop normal components, never pop root or prefix
                if result.file_name().is_some() {
                    result.pop();
                }
            }
            Component::Normal(part) => result.push(part),
        }
    }
    result
}

/// Resolve a path to its canonical or absolute form without enforcing working directory restriction.
/// Used by read-only tools that are allowed to access any path on the filesystem.
pub fn resolve_path(path: &Path, working_dir: &Path) -> Result<PathBuf, String> {
    // Try to canonicalize the path (works if path exists)
    match path.canonicalize() {
        Ok(p) => Ok(p),
        Err(_) => {
            // Path doesn't exist, resolve it relative to working dir if relative
            if path.is_absolute() {
                Ok(path.to_path_buf())
            } else {
                let canonical_working = working_dir
                    .canonicalize()
                    .map_err(|e| format!("Invalid working directory '{}': {}", working_dir.display(), e))?;
                Ok(canonical_working.join(path))
            }
        }
    }
}

/// Safe path join that prevents directory traversal
/// Note: This function is currently only used in tests
#[cfg(test)]
pub fn safe_join(base: &Path, subpath: &str) -> Option<PathBuf> {
    let joined = base.join(subpath);
    let normalized = normalize_path(&joined);
    
    if normalized.starts_with(base) {
        Some(normalized)
    } else {
        None
    }
}

trait Absolutize {
    fn absolutize(&self) -> Result<PathBuf, std::io::Error>;
}

impl Absolutize for Path {
    fn absolutize(&self) -> Result<PathBuf, std::io::Error> {
        if self.is_absolute() {
            Ok(self.to_path_buf())
        } else {
            std::env::current_dir().map(|cwd| cwd.join(self))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1536), "1.50 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn test_is_text_file() {
        assert!(is_text_file(Path::new("test.txt")));
        assert!(is_text_file(Path::new("test.rs")));
        assert!(is_text_file(Path::new("test.json")));
        assert!(!is_text_file(Path::new("test.exe")));
        assert!(!is_text_file(Path::new("test.bin")));
    }

    #[test]
    fn test_safe_join() {
        let base = Path::new("/home/user");
        
        assert_eq!(
            safe_join(base, "documents"),
            Some(PathBuf::from("/home/user/documents"))
        );
        
        assert_eq!(
            safe_join(base, ".."),
            None
        );
        
        assert_eq!(
            safe_join(base, "documents/../../etc/passwd"),
            None
        );
    }

    #[test]
    fn test_ensure_path_within_working_dir() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let working_dir = temp_dir.path();
        
        // Create a test file
        let test_file = working_dir.join("test.txt");
        fs::write(&test_file, "test content")?;
        
        // Valid path
        let result = ensure_path_within_working_dir(&test_file, working_dir);
        assert!(result.is_ok());
        
        // Invalid path (outside working dir)
        let outside_path = Path::new("/etc/passwd");
        let result = ensure_path_within_working_dir(outside_path, working_dir);
        assert!(result.is_err());
        
        Ok(())
    }
}
