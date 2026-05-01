use crate::utils::file_utils::resolve_path;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use md5::Md5;
use sha1::Sha1;
use sha2::Sha256;
use sha2::digest::Digest;
use std::path::Path;
use tokio::io::AsyncReadExt;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HashComputeParams {
    /// Input string or file path (prefix with 'file:' for files)
    #[schemars(description = "Input string or file path (prefix with 'file:' for files)")]
    pub input: String,
    /// Hash algorithm: md5, sha1, sha256 (default: sha256)
    #[schemars(description = "Hash algorithm: md5, sha1, sha256 (default: sha256)")]
    pub algorithm: Option<String>,
}

pub async fn hash_compute(
    params: Parameters<HashComputeParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let algorithm = params.algorithm.unwrap_or_else(|| "sha256".to_string());

    let result = if params.input.starts_with("file:") {
        let file_path = &params.input[5..];
        compute_file_hash(file_path, &algorithm, working_dir).await?
    } else {
        compute_hash(params.input.as_bytes(), &algorithm)?
    };

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("{} hash: {}", algorithm.to_uppercase(), result),
    )]))
}

/// Compute hash of byte data using the specified algorithm
fn compute_hash(data: &[u8], algorithm: &str) -> Result<String, String> {
    match algorithm.to_lowercase().as_str() {
        "md5" => {
            let mut hasher = Md5::new();
            hasher.update(data);
            Ok(format!("{:x}", hasher.finalize()))
        }
        "sha1" => {
            let mut hasher = Sha1::new();
            hasher.update(data);
            Ok(format!("{:x}", hasher.finalize()))
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            Ok(format!("{:x}", hasher.finalize()))
        }
        _ => Err(format!("Unsupported algorithm: {}", algorithm)),
    }
}

async fn compute_file_hash(
    file_path: &str,
    algorithm: &str,
    working_dir: &Path,
) -> Result<String, String> {
    let path = Path::new(file_path);

    // Resolve path without working directory restriction (read-only operation)
    let canonical_path = resolve_path(path, working_dir)?;

    if !canonical_path.exists() {
        return Err(format!("File '{}' does not exist", file_path));
    }

    if !canonical_path.is_file() {
        return Err(format!("Path '{}' is not a file", file_path));
    }

    // Stream file in chunks to avoid OOM with large files
    let mut file = tokio::fs::File::open(&canonical_path)
        .await
        .map_err(|e| format!("Failed to open file: {}", e))?;

    match algorithm.to_lowercase().as_str() {
        "md5" => {
            let mut hasher = Md5::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf).await.map_err(|e| format!("Failed to read file: {}", e))?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        "sha1" => {
            let mut hasher = Sha1::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf).await.map_err(|e| format!("Failed to read file: {}", e))?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf).await.map_err(|e| format!("Failed to read file: {}", e))?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        _ => Err(format!("Unsupported algorithm: {}", algorithm)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_hash_string() {
        let temp_dir = TempDir::new().unwrap();
        let params = HashComputeParams {
            input: "Hello, World!".to_string(),
            algorithm: Some("sha256".to_string()),
        };

        let result = hash_compute(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hash_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let params = HashComputeParams {
            input: format!("file:{}", file_path.to_string_lossy()),
            algorithm: Some("md5".to_string()),
        };

        let result = hash_compute(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hash_invalid_algorithm() {
        let temp_dir = TempDir::new().unwrap();
        let params = HashComputeParams {
            input: "test".to_string(),
            algorithm: Some("invalid".to_string()),
        };

        let result = hash_compute(Parameters(params), temp_dir.path()).await;
        assert!(result.is_err());
    }
}
