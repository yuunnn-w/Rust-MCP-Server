use crate::utils::file_utils::resolve_path;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitOpsParams {
    /// Git repository path (default: current working directory)
    #[schemars(description = "Git repository path (default: current working directory)")]
    pub repo_path: Option<String>,
    /// Git action: status, diff, log, branch, show
    #[schemars(description = "Git action: status, diff, log, branch, show")]
    pub action: String,
    /// Additional options for the git command (e.g. ["--oneline", "-n", "10"])
    #[schemars(description = "Additional options for the git command")]
    pub options: Option<Vec<String>>,
}

pub async fn git_ops(
    params: Parameters<GitOpsParams>,
    working_dir: &Path,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let repo_path = params.repo_path.as_deref().unwrap_or(".");
    let repo_path_buf = Path::new(repo_path);
    let action = params.action.to_lowercase();

    // Resolve repo path without working directory restriction (read-only git operations)
    let canonical_repo = resolve_path(repo_path_buf, working_dir)?;

    if !canonical_repo.exists() {
        return Err(format!("Repository path '{}' does not exist", repo_path));
    }

    // Build git command
    let mut cmd = Command::new("git");
    cmd.current_dir(&canonical_repo);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    match action.as_str() {
        "status" => {
            cmd.arg("status");
        }
        "diff" => {
            cmd.arg("diff");
        }
        "log" => {
            cmd.arg("log");
        }
        "branch" => {
            cmd.arg("branch").arg("-a");
        }
        "show" => {
            cmd.arg("show");
        }
        _ => {
            return Err(format!(
                "Invalid git action '{}'. Use status, diff, log, branch, or show.",
                params.action
            ));
        }
    }

    // Add user options
    if let Some(options) = params.options {
        for opt in options {
            cmd.arg(opt);
        }
    }

    let output = cmd.output().await.map_err(|e| {
        format!(
            "Failed to execute git command: {}. Make sure git is installed.",
            e
        )
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() && stdout.is_empty() {
        return Err(format!(
            "Git command failed: {}",
            if stderr.is_empty() {
                "unknown error".to_string()
            } else {
                stderr.to_string()
            }
        ));
    }

    let mut response = String::new();
    if !stdout.is_empty() {
        response.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !response.is_empty() {
            response.push('\n');
        }
        response.push_str("[stderr]\n");
        response.push_str(&stderr);
    }

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(response)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    fn init_git_repo(path: &std::path::Path) {
        StdCommand::new("git")
            .arg("init")
            .current_dir(path)
            .output()
            .expect("git init failed");
        StdCommand::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .expect("git config failed");
        StdCommand::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .expect("git config failed");
    }

    #[tokio::test]
    async fn test_git_status() {
        let temp_dir = TempDir::new().unwrap();
        init_git_repo(temp_dir.path());

        let params = GitOpsParams {
            repo_path: Some(temp_dir.path().to_string_lossy().to_string()),
            action: "status".to_string(),
            options: None,
        };

        let result = git_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_git_branch() {
        let temp_dir = TempDir::new().unwrap();
        init_git_repo(temp_dir.path());

        let params = GitOpsParams {
            repo_path: Some(temp_dir.path().to_string_lossy().to_string()),
            action: "branch".to_string(),
            options: None,
        };

        let result = git_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_git_invalid_action() {
        let temp_dir = TempDir::new().unwrap();

        let params = GitOpsParams {
            repo_path: Some(temp_dir.path().to_string_lossy().to_string()),
            action: "invalid".to_string(),
            options: None,
        };

        let result = git_ops(Parameters(params), temp_dir.path()).await;
        assert!(result.is_err());
    }
}
