use crate::config::AppConfig;
use crate::mcp::state::ServerState;
use crate::utils::async_command::{GLOBAL_ASYNC_COMMANDS, OutputLine, StreamType};
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
use std::time::SystemTime;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tracing::{info, warn};

const MAX_COMMAND_LENGTH: usize = 10000; // 10000 character command limit

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
    /// Custom shell executable path (e.g., C:\Tools\pwh.exe). Overrides `shell` when provided.
    #[schemars(description = "Custom shell executable path. Overrides shell when provided")]
    pub shell_path: Option<String>,
    /// Custom shell argument (e.g., -Command, /C). If not provided, inferred from shell type.
    #[schemars(description = "Custom shell argument. Inferred from shell type if not provided")]
    pub shell_arg: Option<String>,
    /// Alternative working directory (overrides cwd if both provided)
    #[schemars(description = "Alternative working directory (overrides cwd if both provided)")]
    pub working_dir: Option<String>,
    /// Content to pipe to the command's stdin
    #[schemars(description = "Content to pipe to the command's stdin")]
    pub stdin: Option<String>,
    /// Maximum characters in output before truncation (default: 50000)
    #[schemars(description = "Maximum characters in output before truncation (default: 50000)")]
    pub max_output_chars: Option<usize>,
    /// Execute asynchronously and return a command_id for monitoring (default: false)
    #[schemars(description = "Execute asynchronously and return a command_id for monitoring (default: false)")]
    pub async_mode: Option<bool>,
}

/// Check for command injection patterns
#[cfg(not(windows))]
fn has_injection_patterns(command: &str) -> bool {
    let dangerous_chars = [';', '|', '&', '`', '$', '(', ')', '<', '>', '\n', '\r'];
    let command_trimmed = command.trim();

    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for c in command_trimmed.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match c {
            '\\' if in_double_quote => escape_next = true,
            '\'' if !in_double_quote && !escape_next => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && !escape_next => in_double_quote = !in_double_quote,
            _ if !in_single_quote && !in_double_quote && dangerous_chars.contains(&c) => return true,
            _ => {}
        }
    }

    false
}

/// Check for command injection patterns (Windows variant)
#[cfg(windows)]
fn has_injection_patterns(command: &str) -> bool {
    let dangerous_chars = [';', '|', '&', '`', '$', '(', ')', '<', '>', '%', '^', '\n', '\r'];
    let command_trimmed = command.trim();

    let mut in_double_quote = false;
    let mut escape_next = false;

    for c in command_trimmed.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match c {
            '\\' if in_double_quote => escape_next = true,
            '"' if !escape_next => in_double_quote = !in_double_quote,
            _ if !in_double_quote && dangerous_chars.contains(&c) => return true,
            _ => {}
        }
    }

    false
}

/// Truncate output if too large (UTF-8 safe)
fn truncate_output(output: String, max_chars: usize) -> String {
    if max_chars == 0 || output.len() <= max_chars {
        return output;
    }
    let trunc_point = output.char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(output.len());
    format!(
        "{}\n\n[... Output truncated, total size {} bytes, limit {} bytes ...]",
        &output[..trunc_point],
        output.len(),
        max_chars
    )
}

/// Determine shell executable and argument based on platform and user request.
/// Priority: shell_arg > shell_path (with inference) > shell shortcut > default
fn resolve_shell(shell: Option<&str>, shell_path: Option<&str>, shell_arg: Option<&str>) -> (String, String) {
    if let Some(arg) = shell_arg {
        let valid_args = ["-c", "-C", "-Command", "/C", "/c", "--"];
        if !valid_args.contains(&arg) {
            return (
                if cfg!(windows) { "cmd".to_string() } else { "sh".to_string() },
                if cfg!(windows) { "/C".to_string() } else { "-c".to_string() }
            );
        }
        let exec = shell_path.map(|s| s.to_string()).unwrap_or_else(|| {
            if arg == "/C" || arg == "/c" {
                "cmd".to_string()
            } else if cfg!(windows) {
                "powershell.exe".to_string()
            } else {
                "sh".to_string()
            }
        });
        return (exec, arg.to_string());
    }

    // If shell_path is provided, infer argument from executable name
    if let Some(path) = shell_path {
        let file_stem = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let uses_powershell = file_stem == "powershell" || file_stem == "pwsh" || file_stem == "pwh";
        let arg = if uses_powershell {
            "-Command".to_string()
        } else {
            #[cfg(windows)]
            { "/C".to_string() }
            #[cfg(not(windows))]
            { "-c".to_string() }
        };
        return (path.to_string(), arg);
    }

    // Fall back to shell shortcut name
    #[cfg(windows)]
    {
        match shell {
            Some("powershell") => ("powershell.exe".to_string(), "-Command".to_string()),
            Some("pwsh") => ("pwsh.exe".to_string(), "-Command".to_string()),
            Some("cmd") => ("cmd".to_string(), "/C".to_string()),
            _ => ("cmd".to_string(), "/C".to_string()),
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
    let effective_cwd = params.working_dir.as_deref().unwrap_or(cwd);
    let effective_cwd_path = Path::new(effective_cwd);
    let command = params.command.trim();
    if command.chars().count() > MAX_COMMAND_LENGTH {
        return Err(format!("Command exceeds maximum length of {} characters", MAX_COMMAND_LENGTH));
    }
    let shell = params.shell.as_deref();
    let shell_path = params.shell_path.as_deref();
    let shell_arg_param = params.shell_arg.as_deref();
    let max_output = params.max_output_chars.unwrap_or(50000);

    // Audit log
    info!(
        "[AUDIT] Execute command attempt: cwd={}, command={}, shell={:?}, shell_path={:?}, shell_arg={:?}",
        effective_cwd, command, shell, shell_path, shell_arg_param
    );

    // Security check 1: working directory must be within allowed working directory
    if !is_path_within_working_dir(effective_cwd_path, working_dir) {
        warn!(
            "[AUDIT] Rejected command - outside working dir: cwd={}, command={}",
            effective_cwd, command
        );
        return Err(format!(
            "Working directory '{}' is outside the allowed working directory",
            effective_cwd
        ));
    }

    // Security check 2: check for dangerous commands
    let config = state.config.read().await;
    let dangerous_check = config.check_dangerous_command(command);
    drop(config);

    if let Some(dangerous_id) = dangerous_check {
        if state.confirm_and_remove_pending_command(command, effective_cwd).await {
            warn!(
                "[AUDIT] Dangerous command executed after confirmation: id={}, command={}",
                dangerous_id, command
            );
        } else {
            state.add_pending_command(command, effective_cwd).await;

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
        if state.confirm_and_remove_pending_command(command, effective_cwd).await {
            warn!(
                "[AUDIT] Command with injection patterns executed after confirmation: command={}",
                command
            );
        } else {
            state.add_pending_command(command, effective_cwd).await;

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
    let (shell_exec, shell_arg) = resolve_shell(shell, shell_path, shell_arg_param);

    // Async mode: spawn child directly with streaming output
    if params.async_mode.unwrap_or(false) {
        let command_id = format!(
            "cmd_{}",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let (tx, _rx) = broadcast::channel(64);

        // Build and spawn child
        let mut cmd = Command::new(&shell_exec);
        cmd.arg(&shell_arg).arg(command);
        cmd.current_dir(effective_cwd_path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        if params.stdin.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }
        if let Some(ref env_vars) = params.env {
            for (key, value) in env_vars {
                let s = value
                    .as_str()
                    .ok_or_else(|| format!("Environment variable '{}' must be a string value, got {}", key, value))?;
                cmd.env(key, s);
            }
        }

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let pid = child.id();

        // Write stdin if provided
        if let Some(stdin_content) = params.stdin {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(stdin_content.as_bytes()).await
                    .map_err(|e| format!("Failed to write to stdin: {}", e))?;
            }
        }

        // Register in global manager
        let reg_err = {
            let mut manager = GLOBAL_ASYNC_COMMANDS.lock().map_err(|e| format!("Internal error: {}", e))?;
            manager.register(
                command_id.clone(),
                command.to_string(),
                pid,
                tx.clone(),
            ).err()
        };
        if let Some(e) = reg_err {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(e);
        }

        let cid = command_id.clone();
        let cmd_timeout = timeout_secs;

        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};

            // Take stdout/stderr readers
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            // Spawn streaming readers
            let tx_stdout = tx.clone();
            let tx_stderr = tx.clone();
            let stdout_task = tokio::spawn(async move {
                if let Some(reader) = stdout {
                    let mut lines = BufReader::new(reader).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let _ = tx_stdout.send(OutputLine {
                            stream: StreamType::Stdout,
                            line,
                            timestamp: 0,
                        });
                    }
                }
            });
            let stderr_task = tokio::spawn(async move {
                if let Some(reader) = stderr {
                    let mut lines = BufReader::new(reader).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let _ = tx_stderr.send(OutputLine {
                            stream: StreamType::Stderr,
                            line,
                            timestamp: 0,
                        });
                    }
                }
            });

            // Wait for child to exit with timeout
            let exit_code = match tokio::time::timeout(
                Duration::from_secs(cmd_timeout),
                child.wait(),
            ).await {
                Ok(Ok(status)) => status.code().unwrap_or(-1),
                Ok(Err(e)) => {
                    let _ = tx.send(OutputLine {
                        stream: StreamType::Stderr,
                        line: format!("Process wait error: {}", e),
                        timestamp: 0,
                    });
                    -1
                }
                Err(_) => {
                    // Timeout: kill the process
                    child.start_kill().ok();
                    let _ = tx.send(OutputLine {
                        stream: StreamType::Stderr,
                        line: format!("Command timed out after {} seconds", cmd_timeout),
                        timestamp: 0,
                    });
                    match child.wait().await {
                        Ok(s) => s.code().unwrap_or(-1),
                        Err(_) => -1,
                    }
                }
            };

            // Wait for stream tasks to finish
            let _ = tokio::join!(stdout_task, stderr_task);

            // Update monitor state
            let mut manager = match GLOBAL_ASYNC_COMMANDS.lock() {
                Ok(m) => m,
                Err(_) => {
                    let _ = tx.send(OutputLine {
                        stream: StreamType::Stderr,
                        line: "Internal error: async command manager lock poisoned".to_string(),
                        timestamp: 0,
                    });
                    return;
                }
            };
            if let Some(state) = manager.states.get_mut(&cid) {
                state.exit_code = Some(exit_code);
                state.last_activity = std::time::Instant::now();
            }

            // Cleanup: drop tx to allow channel to close gracefully
            drop(tx);
        });

        info!(
            "[AUDIT] Command started in async mode: id={}, pid={:?}, cwd={}, command={}",
            command_id, pid, effective_cwd, command
        );

        return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            format!("Command started in async mode. Use Monitor tool with command_id: {}", command_id)
        )]));
    }

    // Sync mode: build and execute command
    let mut cmd = Command::new(&shell_exec);
    cmd.arg(&shell_arg).arg(command);
    cmd.current_dir(effective_cwd_path);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let has_stdin = params.stdin.is_some();
    if has_stdin {
        cmd.stdin(Stdio::piped());
    }

    // Set environment variables (only string values allowed)
    if let Some(ref env_vars) = params.env {
        for (key, value) in env_vars {
            let value_str = value
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| format!("Environment variable '{}' must be a string value, got {}", key, value))?;
            cmd.env(key, value_str);
        }
    }

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Write stdin if provided
    if let Some(stdin_content) = params.stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(stdin_content.as_bytes()).await
                .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        }
    }

    // Take stdout/stderr handles before waiting
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    // Wait for child with timeout using select! for reliable kill
    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(|e| format!("Failed to execute command: {}", e))?
        }
        _ = tokio::time::sleep(Duration::from_secs(timeout_secs)) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            warn!(
                "[AUDIT] Command timed out: timeout={}, cwd={}, command={}",
                timeout_secs, effective_cwd, command
            );
            return Err(format!("Command timed out after {} seconds", timeout_secs));
        }
    };

    let read_stdout = async {
        let mut buf = Vec::new();
        if let Some(mut reader) = stdout_handle {
            let _ = reader.read_to_end(&mut buf).await;
        }
        buf
    };
    let read_stderr = async {
        let mut buf = Vec::new();
        if let Some(mut reader) = stderr_handle {
            let _ = reader.read_to_end(&mut buf).await;
        }
        buf
    };
    let (stdout_buf, stderr_buf) = tokio::join!(read_stdout, read_stderr);

    let stdout = String::from_utf8_lossy(&stdout_buf);
    let stderr = String::from_utf8_lossy(&stderr_buf);
    let exit_code = status.code().unwrap_or(-1);

    let stdout = truncate_output(stdout.to_string(), max_output);
    let stderr = truncate_output(stderr.to_string(), max_output);

    let mut response = format!("Exit code: {}\n", exit_code);

    if !stdout.is_empty() {
        response.push_str(&format!("\nSTDOUT:\n{}", stdout));
    }

    if !stderr.is_empty() {
        response.push_str(&format!("\nSTDERR:\n{}", stderr));
    }

    info!(
        "[AUDIT] Command executed: exit_code={}, cwd={}, command={}",
        exit_code, effective_cwd, command
    );

    if status.success() {
        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            response,
        )]))
    } else {
        Ok(CallToolResult::error(vec![rmcp::model::Content::text(
            response,
        )]))
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
        const MAX_OUTPUT_SIZE: usize = 100 * 1024;
        let small = "small output".to_string();
        assert_eq!(truncate_output(small.clone(), 500), small);

        let large = "x".repeat(MAX_OUTPUT_SIZE + 1000);
        let truncated = truncate_output(large.clone(), MAX_OUTPUT_SIZE);
        assert!(truncated.len() < large.len());
        assert!(truncated.contains("truncated"));
    }

    #[test]
    fn test_truncate_output_unlimited() {
        let s = "x".repeat(5000);
        assert_eq!(truncate_output(s.clone(), 0), s);
    }

    #[test]
    fn test_resolve_shell() {
        #[cfg(windows)]
        {
            assert_eq!(resolve_shell(Some("cmd"), None, None), ("cmd".to_string(), "/C".to_string()));
            assert_eq!(resolve_shell(Some("powershell"), None, None), ("powershell.exe".to_string(), "-Command".to_string()));
            assert_eq!(resolve_shell(None, None, None), ("cmd".to_string(), "/C".to_string()));
            // Custom shell path
            assert_eq!(resolve_shell(None, Some(r"C:\Tools\pwh.exe"), None), (r"C:\Tools\pwh.exe".to_string(), "-Command".to_string()));
            assert_eq!(resolve_shell(None, Some(r"C:\Tools\mysh.exe"), None), (r"C:\Tools\mysh.exe".to_string(), "/C".to_string()));
            // Custom shell arg
            assert_eq!(resolve_shell(None, Some(r"C:\Tools\sh.exe"), Some("-c")), (r"C:\Tools\sh.exe".to_string(), "-c".to_string()));
            // shell_arg without shell_path
            assert_eq!(resolve_shell(None, None, Some("/C")), ("cmd".to_string(), "/C".to_string()));
        }
        #[cfg(not(windows))]
        {
            assert_eq!(resolve_shell(Some("sh"), None, None), ("sh".to_string(), "-c".to_string()));
            assert_eq!(resolve_shell(Some("bash"), None, None), ("bash".to_string(), "-c".to_string()));
            assert_eq!(resolve_shell(None, None, None), ("sh".to_string(), "-c".to_string()));
            // Custom shell path
            assert_eq!(resolve_shell(None, Some("/usr/local/bin/pwh"), None), ("/usr/local/bin/pwh".to_string(), "-Command".to_string()));
            assert_eq!(resolve_shell(None, Some("/usr/local/bin/myshell"), None), ("/usr/local/bin/myshell".to_string(), "-c".to_string()));
            // Custom shell arg
            assert_eq!(resolve_shell(None, Some("/bin/bash"), Some("--login -c")), ("/bin/bash".to_string(), "--login -c".to_string()));
        }
    }
}
