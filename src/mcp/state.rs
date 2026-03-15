use crate::config::AppConfig;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock, Semaphore};
use tracing::info;

/// Status of a single tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Total number of calls
    pub call_count: u64,
    /// Whether the tool is currently being called
    pub is_calling: bool,
    /// Timestamp of the last call end (for busy status check)
    #[serde(skip)]
    pub last_call_end: Arc<RwLock<Option<Instant>>>,
    /// Recent call timestamps (for statistics)
    #[serde(skip)]
    pub recent_calls: Arc<RwLock<VecDeque<Instant>>>,
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Whether the tool is a dangerous operation
    pub is_dangerous: bool,
}

impl ToolStatus {
    pub fn new(name: impl Into<String>, description: impl Into<String>, enabled: bool, is_dangerous: bool) -> Self {
        Self {
            enabled,
            call_count: 0,
            is_calling: false,
            last_call_end: Arc::new(RwLock::new(None)),
            recent_calls: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            name: name.into(),
            description: description.into(),
            is_dangerous,
        }
    }

    /// Record a call start
    pub async fn record_call_start(&mut self) {
        self.is_calling = true;
        self.call_count += 1;
        
        let mut calls = self.recent_calls.write().await;
        calls.push_back(Instant::now());
        
        // Keep only last 1000 calls
        while calls.len() > 1000 {
            calls.pop_front();
        }
    }

    /// Record a call end
    pub async fn record_call_end(&mut self) {
        self.is_calling = false;
        let mut last_end = self.last_call_end.write().await;
        *last_end = Some(Instant::now());
    }

    /// Get call count in the last N minutes
    pub async fn get_recent_calls_count(&self, minutes: u64) -> usize {
        let cutoff = Instant::now() - std::time::Duration::from_secs(minutes * 60);
        let calls = self.recent_calls.read().await;
        calls.iter().filter(|&&t| t > cutoff).count()
    }

    /// Get statistics for the last N minutes, grouped by interval
    pub async fn get_stats(&self, total_minutes: u64, interval_minutes: u64) -> Vec<usize> {
        if interval_minutes == 0 {
            return vec![];
        }
        
        let num_intervals = (total_minutes + interval_minutes - 1) / interval_minutes;
        let now = Instant::now();
        let mut stats = vec![0usize; num_intervals as usize];
        
        let calls = self.recent_calls.read().await;
        for &call_time in calls.iter() {
            let elapsed_minutes = (now - call_time).as_secs() / 60;
            if elapsed_minutes < total_minutes {
                let interval = (elapsed_minutes / interval_minutes) as usize;
                if interval < stats.len() {
                    stats[interval] += 1;
                }
            }
        }
        
        stats
    }

    /// Get recent call timestamps as formatted strings
    pub async fn get_recent_call_times(&self, count: usize) -> Vec<String> {
        use chrono::{DateTime, Local};
        
        let calls = self.recent_calls.read().await;
        let now = Instant::now();
        
        calls
            .iter()
            .rev()
            .take(count)
            .map(|&instant| {
                let duration_ago = now - instant;
                let system_time = std::time::SystemTime::now() - duration_ago;
                let datetime: DateTime<Local> = system_time.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .collect()
    }
}

/// Server-wide status update for SSE
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum StatusUpdate {
    #[serde(rename_all = "camelCase")]
    ToolCallCount {
        tool: String,
        count: u64,
        is_calling: bool,
        is_busy: bool,
    },
    #[serde(rename_all = "camelCase")]
    ToolEnabled {
        tool: String,
        enabled: bool,
    },
    McpServiceStatus {
        running: bool,
    },
    #[serde(rename_all = "camelCase")]
    ConcurrentCalls {
        current: usize,
        max: usize,
    },
}

/// Pending command for user confirmation
#[derive(Debug, Clone)]
pub struct PendingCommand {
    #[allow(dead_code)]
    pub command: String,
    #[allow(dead_code)]
    pub cwd: String,
    pub timestamp: Instant,
}

/// Shared server state
pub struct ServerState {
    /// Tool status map
    pub tool_status: DashMap<String, ToolStatus>,
    /// Concurrency limiter
    pub concurrency_limiter: Semaphore,
    /// Configuration (wrapped in RwLock to allow updates)
    pub config: RwLock<AppConfig>,
    /// Status update broadcaster
    pub status_tx: broadcast::Sender<StatusUpdate>,
    /// Current concurrent calls
    pub current_calls: RwLock<usize>,
    /// Maximum concurrent calls allowed
    pub max_concurrency: RwLock<usize>,
    /// MCP service running status
    pub mcp_running: RwLock<bool>,
    /// Pending commands waiting for user confirmation (command_hash -> PendingCommand)
    pub pending_commands: RwLock<HashMap<String, PendingCommand>>,
}

impl ServerState {
    /// Create new server state
    pub fn new(config: AppConfig) -> Arc<Self> {
        let (status_tx, _) = broadcast::channel(100);
        let max_concurrency = config.max_concurrency;

        let state = Arc::new(Self {
            tool_status: DashMap::new(),
            concurrency_limiter: Semaphore::new(max_concurrency),
            config: RwLock::new(config),
            status_tx,
            current_calls: RwLock::new(0),
            max_concurrency: RwLock::new(max_concurrency),
            mcp_running: RwLock::new(false),
            pending_commands: RwLock::new(HashMap::new()),
        });

        state
    }

    /// Initialize tool status (only for tools that don't exist yet)
    pub async fn init_tools(&self, tools: Vec<(String, String, bool)>) {
        let mut initialized_count = 0;
        let mut skipped_count = 0;
        
        // Read config once for all tools
        let config = self.config.read().await;
        
        for (name, description, is_dangerous) in tools {
            // Only insert if tool doesn't already exist
            if !self.tool_status.contains_key(&name) {
                let enabled = !config.is_tool_disabled(&name);
                let status = ToolStatus::new(&name, description, enabled, is_dangerous);
                self.tool_status.insert(name, status);
                initialized_count += 1;
            } else {
                skipped_count += 1;
            }
        }
        
        drop(config);
        
        if initialized_count > 0 {
            info!("Initialized {} tools ({} already existed)", initialized_count, skipped_count);
        } else {
            info!("All {} tools already initialized", skipped_count);
        }
    }

    /// Check if a tool is enabled
    pub async fn is_tool_enabled(&self, tool_name: &str) -> bool {
        match self.tool_status.get(tool_name) {
            Some(status) => status.enabled,
            None => false,
        }
    }

    /// Set tool enabled/disabled
    pub async fn set_tool_enabled(&self, tool_name: &str, enabled: bool) -> Result<(), String> {
        match self.tool_status.get_mut(tool_name) {
            Some(mut status) => {
                status.enabled = enabled;
                let _ = self.status_tx.send(StatusUpdate::ToolEnabled {
                    tool: tool_name.to_string(),
                    enabled,
                });
                info!("Tool '{}' {}abled", tool_name, if enabled { "en" } else { "dis" });
                Ok(())
            }
            None => Err(format!("Tool '{}' not found", tool_name)),
        }
    }

    /// Record a tool call start
    pub async fn record_call_start(&self, tool_name: &str) {
        // Update current calls count
        let current = {
            let mut calls = self.current_calls.write().await;
            *calls += 1;
            *calls
        };
        let max = *self.max_concurrency.read().await;
        let _ = self.status_tx.send(StatusUpdate::ConcurrentCalls {
            current,
            max,
        });

        // Update tool status
        if let Some(mut status) = self.tool_status.get_mut(tool_name) {
            status.record_call_start().await;
            tracing::info!("Tool '{}' call started (count: {})", tool_name, status.call_count);
            let _ = self.status_tx.send(StatusUpdate::ToolCallCount {
                tool: tool_name.to_string(),
                count: status.call_count,
                is_calling: true,
                is_busy: true,
            });
        }
    }

    /// Record a tool call end
    pub async fn record_call_end(&self, tool_name: &str) {
        // Update current calls count (延迟5秒发送SSE)
        let current = {
            let mut calls = self.current_calls.write().await;
            *calls = calls.saturating_sub(1);
            *calls
        };
        let max = *self.max_concurrency.read().await;
        let status_tx = self.status_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let _ = status_tx.send(StatusUpdate::ConcurrentCalls {
                current,
                max,
            });
        });

        // Update tool status
        if let Some(mut status) = self.tool_status.get_mut(tool_name) {
            status.record_call_end().await;
            let call_count = status.call_count;
            tracing::info!("Tool '{}' call ended (count: {})", tool_name, call_count);
            
            // Send immediate update - tool is still busy (within 5 second window)
            let _ = self.status_tx.send(StatusUpdate::ToolCallCount {
                tool: tool_name.to_string(),
                count: call_count,
                is_calling: false,
                is_busy: true,
            });
            
            // Spawn a task to send "not busy" update after 5 seconds
            let status_tx = self.status_tx.clone();
            let tool_name = tool_name.to_string();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let _ = status_tx.send(StatusUpdate::ToolCallCount {
                    tool: tool_name,
                    count: call_count,
                    is_calling: false,
                    is_busy: false,
                });
            });
        }
    }

    /// Get all tool statuses
    pub fn get_all_tool_statuses(&self) -> Vec<ToolStatus> {
        self.tool_status
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get a specific tool's status
    pub fn get_tool_status(&self, tool_name: &str) -> Option<ToolStatus> {
        self.tool_status.get(tool_name).map(|s| s.clone())
    }

    /// Get current concurrent calls count
    pub async fn get_current_calls(&self) -> usize {
        *self.current_calls.read().await
    }

    /// Get maximum concurrent calls
    pub async fn get_max_concurrency(&self) -> usize {
        *self.max_concurrency.read().await
    }

    /// Update maximum concurrency
    pub async fn set_max_concurrency(&self, max: usize) {
        let mut current_max = self.max_concurrency.write().await;
        *current_max = max;
        let _ = self.status_tx.send(StatusUpdate::ConcurrentCalls {
            current: *self.current_calls.read().await,
            max,
        });
    }

    /// Set MCP service running status
    pub async fn set_mcp_running(&self, running: bool) {
        let mut status = self.mcp_running.write().await;
        *status = running;
        let _ = self.status_tx.send(StatusUpdate::McpServiceStatus { running });
    }

    /// Check if MCP service is running
    pub async fn is_mcp_running(&self) -> bool {
        *self.mcp_running.read().await
    }

    /// Subscribe to status updates
    pub fn subscribe_status(&self) -> broadcast::Receiver<StatusUpdate> {
        self.status_tx.subscribe()
    }

    /// Generate a hash for a command
    pub fn hash_command(&self, command: &str, cwd: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        command.hash(&mut hasher);
        cwd.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Check if a command is pending confirmation
    pub async fn is_command_pending(&self, command: &str, cwd: &str) -> bool {
        let hash = self.hash_command(command, cwd);
        let pending = self.pending_commands.read().await;
        
        if let Some(cmd) = pending.get(&hash) {
            // Check if not expired (5 minutes timeout)
            if cmd.timestamp.elapsed() < Duration::from_secs(300) {
                return true;
            }
        }
        false
    }

    /// Add a command to pending list
    pub async fn add_pending_command(&self, command: &str, cwd: &str) {
        let hash = self.hash_command(command, cwd);
        let mut pending = self.pending_commands.write().await;
        pending.insert(hash, PendingCommand {
            command: command.to_string(),
            cwd: cwd.to_string(),
            timestamp: Instant::now(),
        });
    }

    /// Remove a pending command (after execution)
    pub async fn remove_pending_command(&self, command: &str, cwd: &str) {
        let hash = self.hash_command(command, cwd);
        let mut pending = self.pending_commands.write().await;
        pending.remove(&hash);
    }

    /// Clean up expired pending commands
    pub async fn cleanup_expired_pending_commands(&self) {
        let mut pending = self.pending_commands.write().await;
        let expired: Vec<String> = pending
            .iter()
            .filter(|(_, cmd)| cmd.timestamp.elapsed() >= Duration::from_secs(300))
            .map(|(hash, _)| hash.clone())
            .collect();
        for hash in expired {
            pending.remove(&hash);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AppConfig {
        AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "stdio".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 8080,
            max_concurrency: 10,
            disable_tools: vec![],
            working_dir: std::path::PathBuf::from("."),
            log_level: "info".to_string(),
            disable_webui: false,
        }
    }

    #[tokio::test]
    async fn test_tool_status() {
        let mut status = ToolStatus::new("test_tool", "A test tool", true, false);
        
        assert!(status.enabled);
        assert_eq!(status.call_count, 0);
        assert!(!status.is_calling);
        
        status.record_call_start().await;
        assert!(status.is_calling);
        assert_eq!(status.call_count, 1);
        
        status.record_call_end().await;
        assert!(!status.is_calling);
        assert_eq!(status.call_count, 1);
    }

    #[tokio::test]
    async fn test_server_state() {
        let config = create_test_config();
        let state = ServerState::new(config);
        
        state.init_tools(vec![
            ("tool1".to_string(), "Tool 1".to_string(), false),
            ("tool2".to_string(), "Tool 2".to_string(), true),
        ]);
        
        assert!(state.is_tool_enabled("tool1").await);
        assert!(state.is_tool_enabled("tool2").await);
        
        let all_statuses = state.get_all_tool_statuses();
        assert_eq!(all_statuses.len(), 2);
    }

    #[tokio::test]
    async fn test_recent_calls_count() {
        let mut status = ToolStatus::new("test_tool", "A test tool", true, false);
        
        // Simulate some calls
        status.record_call_start().await;
        status.record_call_end().await;
        
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        status.record_call_start().await;
        status.record_call_end().await;
        
        let count = status.get_recent_calls_count(1).await;
        assert_eq!(count, 2);
    }

    #[test]
    fn test_status_update_serialization() {
        let update = StatusUpdate::ToolCallCount {
            tool: "calculator".to_string(),
            count: 5,
            is_calling: true,
            is_busy: true,
        };
        let json = serde_json::to_string(&update).unwrap();
        println!("ToolCallCount JSON: {}", json);
        assert!(json.contains("\"type\":\"ToolCallCount\""));
        assert!(json.contains("\"tool\":\"calculator\""));
        assert!(json.contains("\"count\":5"));
        assert!(json.contains("\"isCalling\":true"));
        assert!(json.contains("\"isBusy\":true"));

        let update2 = StatusUpdate::ConcurrentCalls {
            current: 3,
            max: 10,
        };
        let json2 = serde_json::to_string(&update2).unwrap();
        println!("ConcurrentCalls JSON: {}", json2);
        assert!(json2.contains("\"type\":\"ConcurrentCalls\""));
        assert!(json2.contains("\"current\":3"));
        assert!(json2.contains("\"max\":10"));
    }
}
