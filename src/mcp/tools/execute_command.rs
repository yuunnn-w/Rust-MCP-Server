use crate::config::AppConfig;
use crate::mcp::state::ServerState;
use crate::utils::file_utils::is_path_within_working_dir;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

const MAX_OUTPUT_SIZE: usize = 100 * 1024; // 100KB output limit

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExecuteCommandParams {
    /// Command to execute
    #[schemars(description = "The command to execute")]
    pub command: String,
    /// Working directory for the command (default: current working directory)
    #[schemars(description = "Working directory for the command")]
    pub cwd: Option<String>,
    /// Timeout in seconds (default: 30, max: 300)
    #[schemars(description = "Timeout in seconds (default: 30, max: 300)")]
    pub timeout: Option<u64>,
    /// Environment variables as key=value pairs
    #[schemars(description = "Environment variables as JSON object")]
    pub env: Option<serde_json::Map<String, serde_json::Value>>,
    /// Shell to use. On Windows: "cmd" (default), "powershell", "pwsh". On Unix: "sh" (default), "bash", "zsh".
    #[schemars(description = "Shell to use: cmd/powershell/pwsh on Windows; sh/bash/zsh on Unix")]
    pub shell: Option<String>,
}

/// Check for command injection patterns
#[cfg(not(windows))]
fn has_injection_patterns(command: &str) -> bool {
    let dangerous_chars = [';', '|', '&', '`', '$', '(', ')', '<', '>'];
    let command_trimmed = command.trim();

    let mut in_single_quote = false;
    let mut in_double_quote = false;

    for c in command_trimmed.chars() {
        match c {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            _ if !in_single_quote && !in_double_quote && dangerous_chars.contains(&c) => return true,
            _ => {}
        }
    }

    false
}

/// Check for command injection patterns (Windows variant)
#[cfg(windows)]
fn has_injection_patterns(command: &str) -> bool {
    let dangerous_chars = [';', '|', '&', '`', '$', '(', ')', '<', '>', '%', '^'];
    let command_trimmed = command.trim();

    let mut in_double_quote = false;

    for c in command_trimmed.chars() {
        match c {
            '"' => in_double_quote = !in_double_quote,
            _ if !in_double_quote && dangerous_chars.contains(&c) => return true,
            _ => {}
        }
    }

    false
}

/// Truncate output if too large
fn truncate_output(output: String) -> String {
    if output.len() > MAX_OUTPUT_SIZE {
        format!(
            "{}\n\n[... Output truncated, total size {} bytes, limit {} bytes ...]",
            &output[..MAX_OUTPUT_SIZE],
            output.len(),
            MAX_OUTPUT_SIZE
        )
    } else {
        output
    }
}

/// Determine shell executable and argument based on platform and user request
fn resolve_shell(shell: Option<&str>) -> (String, String) {
    #[cfg(windows)]
    {
        match shell {
            Some("powershell") => ("powershell.exe".to_string(), "-Command".to_string()),
            Some("pwsh") => ("pwsh.exe".to_string(), "-Command".to_string()),
            Some("cmd") | _ => ("cmd".to_string(), "/C".to_string()),
        }
    }
    #[cfg(not(windows))]
    {
        match shell {
            Some("bash") => ("bash".to_string(), "-c".to_string()),
            Some("zsh") => ("zsh".to_string(), "-c".to_string()),
            Some("sh") | _ => ("sh".to_string(), "-c".to_string()),
        }
    }
}

pub async fn execute_command(
    params: Parameters<ExecuteCommandParams>,
    working_dir: &Path,
    state: Arc<ServerState>,
    _context: RequestContext<RoleServer>,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let timeout_secs = params.timeout.unwrap_or(30).min(300);
    let cwd = params.cwd.as_deref().unwrap_or(".");
    let cwd_path = Path::new(cwd);
    let command = params.command.trim();
    let shell = params.shell.as_deref();

    // Audit log
    info!(
        "[AUDIT] Execute command attempt: cwd={}, command={}, shell={:?}",
        cwd, command, shell
    );

    // Security check 1: working directory must be within allowed working directory
    if !is_path_within_working_dir(cwd_path, working_dir) {
        warn!(
            "[AUDIT] Rejected command - outside working dir: cwd={}, command={}",
            cwd, command
        );
        return Err(format!(
            "Working directory '{}' is outside the allowed working directory",
            cwd
        ));
    }

    // Security check 2: check for dangerous commands
    let config = state.config.read().await;
    let dangerous_check = config.check_dangerous_command(command);
    drop(config);

    if let Some(dangerous_id) = dangerous_check {
        if state.is_command_pending(command, cwd).await {
            state.remove_pending_command(command, cwd).await;
            warn!(
                "[AUDIT] Dangerous command executed after confirmation: id={}, command={}",
                dangerous_id, command
            );
        } else {
            state.add_pending_command(command, cwd).await;

            let cmd_name = AppConfig::get_dangerous_command_name(dangerous_id)
                .unwrap_or("Unknown dangerous command");

            info!(
                "[AUDIT] Dangerous command pending confirmation: id={}, command={}",
                dangerous_id, command
            );

            return Err(format!(
                "Security Warning: Dangerous command '{}' detected.\n\
                \n\
                Command: {}\n\
                \n\
                This command may cause damage to the system or data. Please confirm with the user whether to execute this command.\n\
                \n\
                If the user agrees, please call the execute_command tool again with the same parameters to confirm execution.",
                cmd_name, command
            ));
        }
    }

    // Security check 3: check for injection patterns
    if has_injection_patterns(command) {
        if state.is_command_pending(command, cwd).await {
            state.remove_pending_command(command, cwd).await;
            warn!(
                "[AUDIT] Command with injection patterns executed after confirmation: command={}",
                command
            );
        } else {
            state.add_pending_command(command, cwd).await;

            info!(
                "[AUDIT] Command with injection patterns pending confirmation: command={}",
                command
            );

            return Err(format!(
                "Security Warning: Command contains special characters that may pose command injection risks.\n\
                \n\
                Command: {}\n\
                \n\
                The command contains the following special characters: ; | & $ ` ( ) < >\n\
                These characters may be used to execute additional malicious commands.\n\
                \n\
                Please confirm with the user whether to execute this command.\n\
                \n\
                If the user agrees, please call the execute_command tool again with the same parameters to confirm execution.",
                command
            ));
        }
    }

    // Clean up expired pending commands
    state.cleanup_expired_pending_commands().await;

    // Determine shell based on OS and user preference
    let (shell_exec, shell_arg) = resolve_shell(shell);

    // Build command
    let mut cmd = Command::new(&shell_exec);
    cmd.arg(&shell_arg).arg(command);
    cmd.current_dir(cwd_path);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Set environment variables
    if let Some(env_vars) = params.env {
        for (key, value) in env_vars {
            let value_str = value
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| value.to_string());
            cmd.env(key, value_str);
        }
    }

    // Execute with timeout
    let result = timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);

            let stdout = truncate_output(stdout.to_string());
            let stderr = truncate_output(stderr.to_string());

            let mut response = format!("Exit code: {}\n", exit_code);

            if !stdout.is_empty() {
                response.push_str(&format!("\nSTDOUT:\n{}", stdout));
            }

            if !stderr.is_empty() {
                response.push_str(&format!("\nSTDERR:\n{}", stderr));
            }

            info!(
                "[AUDIT] Command executed: exit_code={}, cwd={}, command={}",
                exit_code, cwd, command
            );

            if output.status.success() {
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    response,
                )]))
            } else {
                Ok(CallToolResult::error(vec![rmcp::model::Content::text(
                    response,
                )]))
            }
        }
        Ok(Err(e)) => {
            warn!(
                "[AUDIT] Command execution failed: error={}, cwd={}, command={}",
                e, cwd, command
            );
            Err(format!("Failed to execute command: {}", e))
        }
        Err(_) => {
            warn!(
                "[AUDIT] Command timed out: timeout={}, cwd={}, command={}",
                timeout_secs, cwd, command
            );
            Err(format!("Command timed out after {} seconds", timeout_secs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_injection_patterns() {
        assert!(has_injection_patterns("ls ; rm -rf /"));
        assert!(has_injection_patterns("cat file | grep test"));
        assert!(has_injection_patterns("cmd1 && cmd2"));
        assert!(has_injection_patterns("echo $(whoami)"));
        assert!(has_injection_patterns("cmd `whoami`"));
        assert!(!has_injection_patterns("ls -la"));
        assert!(!has_injection_patterns(r#"cat "file with ; in name""#));
        assert!(!has_injection_patterns("echo \"hello world\""));
    }

    #[test]
    fn test_truncate_output() {
        let small = "small output".to_string();
        assert_eq!(truncate_output(small.clone()), small);

        let large = "x".repeat(MAX_OUTPUT_SIZE + 1000);
        let truncated = truncate_output(large.clone());
        assert!(truncated.len() < large.len());
        assert!(truncated.contains("truncated"));
    }

    #[test]
    fn test_resolve_shell() {
        #[cfg(windows)]
        {
            assert_eq!(resolve_shell(Some("cmd")), ("cmd".to_string(), "/C".to_string()));
            assert_eq!(resolve_shell(Some("powershell")), ("powershell.exe".to_string(), "-Command".to_string()));
            assert_eq!(resolve_shell(None), ("cmd".to_string(), "/C".to_string()));
        }
        #[cfg(not(windows))]
        {
            assert_eq!(resolve_shell(Some("sh")), ("sh".to_string(), "-c".to_string()));
            assert_eq!(resolve_shell(Some("bash")), ("bash".to_string(), "-c".to_string()));
            assert_eq!(resolve_shell(None), ("sh".to_string(), "-c".to_string()));
        }
    }
}
