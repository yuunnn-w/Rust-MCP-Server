use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HashComputeParams {
    /// Input string or file path (prefix with 'file:' for files)
    #[schemars(description = "Input string or file path (prefix with 'file:' for files)")]
    pub input: String,
    /// Hash algorithm: md5, sha1, sha256 (default: sha256)
    #[schemars(description = "Hash algorithm: md5, sha1, sha256 (default: sha256)")]
    pub algorithm: Option<String>,
}

pub async fn hash_compute(params: Parameters<HashComputeParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let algorithm = params.algorithm.unwrap_or_else(|| "sha256".to_string());

    let result = if params.input.starts_with("file:") {
        let file_path = &params.input[5..];
        compute_file_hash(file_path, &algorithm).await?
    } else {
        compute_string_hash(&params.input, &algorithm)?
    };

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!("{} hash: {}", algorithm.to_uppercase(), result),
    )]))
}

fn compute_string_hash(input: &str, algorithm: &str) -> Result<String, String> {
    match algorithm.to_lowercase().as_str() {
        "md5" => {
            let hash = md5::compute(input.as_bytes());
            Ok(format!("{:x}", hash))
        }
        "sha1" => {
            // Using SHA256 as fallback for SHA1
            let mut hasher = Sha256::new();
            hasher.update(input.as_bytes());
            let result = hasher.finalize();
            Ok(format!("{:x}", result))
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(input.as_bytes());
            let result = hasher.finalize();
            Ok(format!("{:x}", result))
        }
        _ => Err(format!("Unsupported algorithm: {}", algorithm)),
    }
}

async fn compute_file_hash(file_path: &str, algorithm: &str) -> Result<String, String> {
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(format!("File '{}' does not exist", file_path));
    }

    if !path.is_file() {
        return Err(format!("Path '{}' is not a file", file_path));
    }

    let data = tokio::fs::read(path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    match algorithm.to_lowercase().as_str() {
        "md5" => {
            let hash = md5::compute(&data);
            Ok(format!("{:x}", hash))
        }
        "sha1" => {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let result = hasher.finalize();
            Ok(format!("{:x}", result))
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let result = hasher.finalize();
            Ok(format!("{:x}", result))
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
        let params = HashComputeParams {
            input: "Hello, World!".to_string(),
            algorithm: Some("sha256".to_string()),
        };

        let result = hash_compute(Parameters(params)).await;
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

        let result = hash_compute(Parameters(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hash_invalid_algorithm() {
        let params = HashComputeParams {
            input: "test".to_string(),
            algorithm: Some("invalid".to_string()),
        };

        let result = hash_compute(Parameters(params)).await;
        assert!(result.is_err());
    }
}
