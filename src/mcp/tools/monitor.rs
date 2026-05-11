use crate::utils::async_command::GLOBAL_ASYNC_COMMANDS;
use crate::utils::async_command::StreamType;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MonitorParams {
    #[schemars(description = "Command ID returned by Bash tool when executed with async=true")]
    pub command_id: String,
    #[schemars(description = "Operation: stream, wait, or signal. Default: wait")]
    pub operation: Option<String>,
    #[schemars(description = "Timeout in seconds (default: 60 for wait, 30 for stream)")]
    pub timeout: Option<u64>,
    #[schemars(description = "Signal to send: terminate, kill, or interrupt (for signal operation)")]
    pub signal: Option<String>,
    #[schemars(description = "Maximum output lines to return in stream mode (default: 100)")]
    pub max_lines: Option<usize>,
}

pub async fn monitor(
    params: Parameters<MonitorParams>,
) -> Result<CallToolResult, String> {
    let params = params.0;
    let op = params.operation.as_deref().unwrap_or("wait");

    match op {
        "stream" => {
            // Lock, extract rx, drop guard before awaits
            let (mut rx, max_lines, timeout_secs) = {
                let manager = GLOBAL_ASYNC_COMMANDS.lock().map_err(|e| e.to_string())?;
                let state = manager
                    .get(&params.command_id)
                    .ok_or_else(|| format!("Command not found: {}", params.command_id))?;
                (state.tx.subscribe(), params.max_lines.unwrap_or(100), params.timeout.unwrap_or(30))
            };

            let mut lines = Vec::new();
            let timeout = tokio::time::Duration::from_secs(timeout_secs);

            let completed = tokio::time::timeout(timeout, async {
                loop {
                    match rx.recv().await {
                        Ok(line) => {
                            lines.push(format!(
                                "[{}] {}",
                                if matches!(line.stream, StreamType::Stderr) {
                                    "stderr"
                                } else {
                                    "stdout"
                                },
                                line.line
                            ));
                            if lines.len() >= max_lines {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            })
            .await;

            if completed.is_err() {
                if lines.is_empty() {
                    lines.push("[timeout] Monitor timed out with no output".to_string());
                } else {
                    lines.push(format!("[timeout] Monitor timed out after {} lines", lines.len()));
                }
            }

            let text = lines.join("\n");
            if text.is_empty() {
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    "(no output yet)"
                )]))
            } else {
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(text)]))
            }
        }
        "wait" => {
            let timeout = tokio::time::Duration::from_secs(params.timeout.unwrap_or(60));
            let start = tokio::time::Instant::now();

            loop {
                if start.elapsed() > timeout {
                    return Err("Monitor wait timed out".to_string());
                }

                let (exit_code, still_exists) = {
                    let mgr = GLOBAL_ASYNC_COMMANDS.lock().map_err(|e| e.to_string())?;
                    let state = mgr.get(&params.command_id);
                    match state {
                        Some(s) => (s.exit_code, true),
                        None => (Some(-2), false),
                    }
                };

                if let Some(code) = exit_code {
                    let text = format!("Command completed with exit code: {}", code);
                    return Ok(CallToolResult::success(vec![rmcp::model::Content::text(text)]));
                }
                if !still_exists {
                    return Err(format!("Command '{}' has been cleaned up", params.command_id));
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
        "signal" => {
            let signal = params.signal.as_deref().unwrap_or("terminate");
            // Negative sentinel exit codes used to record the signal type.
            // On Unix, real signal-kill exit codes are 128+N, so negative values
            // never conflict. On Windows, exit codes are DWORD (0..=4294967295),
            // so negative sentinel values are also safe there.
            let exit_code = match signal {
                "kill" => -9,
                "terminate" => -15,
                "interrupt" => -2,
                _ => -15,
            };
            let (found, pid_to_signal) = {
                let mut manager = GLOBAL_ASYNC_COMMANDS.lock().map_err(|e| e.to_string())?;
                match manager.states.get_mut(&params.command_id) {
                    Some(state) => {
                        let pid_to_signal = state.pid;
                        state.exit_code = Some(exit_code);
                        state.last_activity = std::time::Instant::now();
                        (true, pid_to_signal)
                    }
                    None => (false, None),
                }
            };
            if let Some(p) = pid_to_signal {
                let signal_owned = signal.to_string();
                match tokio::task::spawn_blocking(move || {
                    crate::utils::async_command::AsyncCommandManager::send_signal(p, &signal_owned)
                }).await {
                    Ok(Err(e)) => tracing::warn!("Failed to send signal to pid {}: {}", p, e),
                    Err(e) => tracing::warn!("spawn_blocking failed for signal: {}", e),
                    _ => {}
                }
            }
            if found {
                let msg = format!(
                    "Sent signal {} (exit_code={}) to command {}. PID: {:?}. Use wait operation to check completion.",
                    signal, exit_code, params.command_id, pid_to_signal
                );
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(msg)]))
            } else {
                Err(format!("Command not found: {}", params.command_id))
            }
        }
        _ => Err(format!(
            "Unknown operation: {}. Use: stream, wait, signal",
            op
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[test]
    fn test_monitor_params_deserialization() {
        let json = r#"{"command_id":"cmd-123","operation":"stream","timeout":10,"max_lines":50}"#;
        let params: MonitorParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.command_id, "cmd-123");
        assert_eq!(params.operation, Some("stream".to_string()));
        assert_eq!(params.timeout, Some(10));
        assert_eq!(params.max_lines, Some(50));
        assert!(params.signal.is_none());
    }

    #[test]
    fn test_monitor_params_defaults() {
        let json = r#"{"command_id":"cmd-456"}"#;
        let params: MonitorParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.command_id, "cmd-456");
        assert_eq!(params.operation, None);
        assert_eq!(params.timeout, None);
        assert_eq!(params.signal, None);
        assert_eq!(params.max_lines, None);
    }

    #[tokio::test]
    async fn test_monitor_unknown_operation() {
        let (tx, _rx) = broadcast::channel(16);
        let cmd_id = "test-unknown-op-1".to_string();
        {
            let mut mgr = GLOBAL_ASYNC_COMMANDS.lock().unwrap();
            mgr.register(cmd_id.clone(), "echo test".to_string(), None, tx).unwrap();
        }

        let params = Parameters(MonitorParams {
            command_id: cmd_id,
            operation: Some("invalid_op".to_string()),
            timeout: None,
            signal: None,
            max_lines: None,
        });
        let result = monitor(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown operation"));
    }

    #[tokio::test]
    async fn test_monitor_signal_terminate() {
        let (tx, _rx) = broadcast::channel(16);
        let cmd_id = "test-signal-term-1".to_string();
        {
            let mut mgr = GLOBAL_ASYNC_COMMANDS.lock().unwrap();
            mgr.register(cmd_id.clone(), "sleep 100".to_string(), None, tx).unwrap();
        }

        let params = Parameters(MonitorParams {
            command_id: cmd_id.clone(),
            operation: Some("signal".to_string()),
            timeout: None,
            signal: Some("terminate".to_string()),
            max_lines: None,
        });
        let result = monitor(params).await.unwrap();
        assert!(!result.is_error.unwrap_or(false));

        let mgr = GLOBAL_ASYNC_COMMANDS.lock().unwrap();
        let exit_code = mgr.get(&cmd_id).and_then(|s| s.exit_code);
        assert_eq!(exit_code, Some(-15));
    }

    #[tokio::test]
    async fn test_monitor_signal_kill() {
        let (tx, _rx) = broadcast::channel(16);
        let cmd_id = "test-signal-kill-1".to_string();
        {
            let mut mgr = GLOBAL_ASYNC_COMMANDS.lock().unwrap();
            mgr.register(cmd_id.clone(), "sleep 100".to_string(), None, tx).unwrap();
        }

        let params = Parameters(MonitorParams {
            command_id: cmd_id.clone(),
            operation: Some("signal".to_string()),
            timeout: None,
            signal: Some("kill".to_string()),
            max_lines: None,
        });
        let result = monitor(params).await.unwrap();
        assert!(!result.is_error.unwrap_or(false));

        let mgr = GLOBAL_ASYNC_COMMANDS.lock().unwrap();
        let exit_code = mgr.get(&cmd_id).and_then(|s| s.exit_code);
        assert_eq!(exit_code, Some(-9));
    }

    #[tokio::test]
    async fn test_monitor_signal_not_found() {
        let params = Parameters(MonitorParams {
            command_id: "nonexistent-cmd".to_string(),
            operation: Some("signal".to_string()),
            timeout: None,
            signal: Some("terminate".to_string()),
            max_lines: None,
        });
        let result = monitor(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Command not found"));
    }

    #[tokio::test]
    async fn test_monitor_wait_completed() {
        let (tx, _rx) = broadcast::channel(16);
        let cmd_id = "test-wait-completed-1".to_string();
        {
            let mut mgr = GLOBAL_ASYNC_COMMANDS.lock().unwrap();
            mgr.register(cmd_id.clone(), "echo done".to_string(), None, tx).unwrap();
            mgr.states.get_mut(&cmd_id).unwrap().exit_code = Some(0);
        }

        let params = Parameters(MonitorParams {
            command_id: cmd_id,
            operation: Some("wait".to_string()),
            timeout: Some(5),
            signal: None,
            max_lines: None,
        });
        let result = monitor(params).await.unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }
}
