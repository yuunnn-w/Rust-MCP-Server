use std::path::{Path, PathBuf};
use std::time::SystemTime;
use chrono::{DateTime, Local};

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
            // Fallback: try to check with absolute paths
            match (path.absolutize(), working_dir.absolutize()) {
                (Ok(abs_path), Ok(abs_working)) => abs_path.starts_with(&abs_working),
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
    ];

    match get_file_extension(path) {
        Some(ext) => TEXT_EXTENSIONS.contains(&ext.as_str()),
        None => true, // Files without extension are treated as text
    }
}

/// Safe path join that prevents directory traversal
pub fn safe_join(base: &Path, subpath: &str) -> Option<PathBuf> {
    let joined = base.join(subpath);
    let normalized = normalize_path(&joined)?;
    
    if normalized.starts_with(base) {
        Some(normalized)
    } else {
        None
    }
}

/// Normalize a path (resolve . and ..)
fn normalize_path(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    
    for component in path.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                result.push(component);
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !result.pop() {
                    return None; // Path traverses above root
                }
            }
            std::path::Component::Normal(part) => {
                result.push(part);
            }
        }
    }
    
    Some(result)
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
