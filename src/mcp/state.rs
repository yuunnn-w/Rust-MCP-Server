use crate::config::AppConfig;
use crate::mcp::presets::get_preset;
use crate::utils::system_metrics::{MetricsCollector, SystemMetrics};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock, Semaphore};
use tracing::info;
use tracing_subscriber::{EnvFilter, Registry, reload};

/// Notification that the tool list has changed, to be sent to MCP clients
#[derive(Debug, Clone)]
pub struct ToolListChangedSignal;

/// Status of a single tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Total number of calls
    pub call_count: u64,
    /// Whether the tool is currently being called
    pub is_calling: bool,
    /// Whether the tool is busy (calling or within 5s cooldown)
    #[serde(skip)]
    pub is_busy: bool,
    /// Timestamp of the last call end (for busy status check)
    #[serde(skip)]
    pub last_call_end: Arc<RwLock<Option<Instant>>>,
    /// Recent call timestamps (for statistics)
    #[serde(skip)]
    pub recent_calls: Arc<RwLock<VecDeque<Instant>>>,
    /// Start time of current call (for duration tracking)
    #[serde(skip)]
    pub call_start_time: Arc<RwLock<Option<Instant>>>,
    /// Recent call durations in milliseconds (for avg duration)
    #[serde(skip)]
    pub call_durations: Arc<RwLock<VecDeque<u64>>>,
    /// Number of failed calls
    pub error_count: u64,
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
            is_busy: false,
            last_call_end: Arc::new(RwLock::new(None)),
            recent_calls: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            call_start_time: Arc::new(RwLock::new(None)),
            call_durations: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            error_count: 0,
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
        
        // Record start time for duration tracking
        let mut start = self.call_start_time.write().await;
        *start = Some(Instant::now());
    }

    /// Record a call end
    pub async fn record_call_end(&mut self) {
        self.is_calling = false;
        let mut last_end = self.last_call_end.write().await;
        *last_end = Some(Instant::now());
        
        // Calculate and store call duration
        let duration_ms = {
            let start_opt = self.call_start_time.read().await;
            start_opt.map(|s| s.elapsed().as_millis() as u64).unwrap_or(0)
        };
        let mut durations = self.call_durations.write().await;
        durations.push_back(duration_ms);
        while durations.len() > 100 {
            durations.pop_front();
        }
    }

    /// Compute is_busy dynamically based on is_calling and last_call_end
    pub async fn compute_is_busy(&self) -> bool {
        if self.is_calling {
            return true;
        }
        let last_end = self.last_call_end.read().await;
        if let Some(end_time) = *last_end {
            end_time.elapsed() < Duration::from_secs(5)
        } else {
            false
        }
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
        
        let num_intervals = total_minutes.div_ceil(interval_minutes);
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
    #[serde(rename_all = "camelCase")]
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

/// A note stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Note {
    pub fn new(id: u64, title: String, content: String, tags: Vec<String>, category: String) -> Self {
        let now = chrono::Local::now().to_rfc3339();
        Self {
            id,
            title,
            content,
            tags,
            category,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// A task stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub tags: Vec<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Task {
    pub fn new(id: u64, title: String, description: String, priority: String, tags: Vec<String>, status: String) -> Self {
        let now = chrono::Local::now().to_rfc3339();
        Self {
            id,
            title,
            description,
            priority,
            tags,
            status,
            created_at: now.clone(),
            updated_at: now,
        }
    }
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
    /// Tool list changed signal broadcaster (for MCP clients)
    pub tool_list_changed_tx: broadcast::Sender<ToolListChangedSignal>,
    /// Current concurrent calls
    pub current_calls: AtomicUsize,
    /// Maximum concurrent calls allowed
    pub max_concurrency: AtomicUsize,
    /// Deficit of permits to drain when reducing max_concurrency under load
    pub permit_deficit: AtomicUsize,
    /// MCP service running status
    pub mcp_running: AtomicBool,
    /// Pending commands waiting for user confirmation (command_hash -> PendingCommand)
    pub pending_commands: RwLock<HashMap<String, PendingCommand>>,
    /// System metrics collector
    pub metrics_collector: MetricsCollector,
    /// Whether execute_python tool has filesystem access enabled
    pub python_fs_access_enabled: AtomicBool,
    /// Current active tool preset name
    pub current_preset: RwLock<Option<String>>,
    /// Custom system prompt appended to MCP instructions (sync RwLock for access from sync get_info)
    pub system_prompt: std::sync::RwLock<Option<String>>,
    /// In-memory notes storage
    pub notes: DashMap<u64, Note>,
    /// Last access time for notes (used for 30min auto-clear)
    pub notes_last_access: RwLock<Instant>,
    /// Next note ID auto-increment
    pub notes_next_id: AtomicU64,
    /// In-memory tasks storage
    pub tasks: DashMap<u64, Task>,
    /// Next task ID auto-increment
    pub tasks_next_id: AtomicU64,
    /// Handle for dynamically reloading the tracing log filter
    pub log_reload_handle: std::sync::RwLock<Option<reload::Handle<EnvFilter, Registry>>>,
    /// Handle for the pending commands cleanup background task
    pub pending_cleanup_handle: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Whether LibreOffice is available on the system
    #[allow(dead_code)]
    pub libreoffice_available: AtomicBool,
}

impl ServerState {
    /// Create new server state
    pub fn new(config: AppConfig) -> Arc<Self> {
        let (status_tx, _) = broadcast::channel(100);
        let (tool_list_changed_tx, _) = broadcast::channel(100);
        let max_concurrency = config.max_concurrency;

        let system_prompt = config.system_prompt.clone();
        Arc::new_cyclic(|weak: &std::sync::Weak<Self>| {
            let weak_clone = weak.clone();
            let handle = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    if let Some(state) = weak_clone.upgrade() {
                        state.cleanup_expired_pending_commands().await;
                    } else {
                        break;
                    }
                }
            });

            Self {
                tool_status: DashMap::new(),
                concurrency_limiter: Semaphore::new(max_concurrency),
                config: RwLock::new(config),
                status_tx,
                tool_list_changed_tx,
                current_calls: AtomicUsize::new(0),
                max_concurrency: AtomicUsize::new(max_concurrency),
                permit_deficit: AtomicUsize::new(0),
                mcp_running: AtomicBool::new(false),
                pending_commands: RwLock::new(HashMap::new()),
                metrics_collector: MetricsCollector::new(),
                python_fs_access_enabled: AtomicBool::new(false),
                current_preset: RwLock::new(None),
                system_prompt: std::sync::RwLock::new(system_prompt),
                notes: DashMap::new(),
                notes_last_access: RwLock::new(Instant::now()),
                notes_next_id: AtomicU64::new(1),
                tasks: DashMap::new(),
                tasks_next_id: AtomicU64::new(1),
                log_reload_handle: std::sync::RwLock::new(None),
                pending_cleanup_handle: std::sync::Mutex::new(Some(handle)),
                libreoffice_available: AtomicBool::new(crate::utils::office_converter::find_libreoffice().is_some()),
            }
        })
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
                // Notify MCP clients that tool list has changed
                let _ = self.tool_list_changed_tx.send(ToolListChangedSignal);
                info!("Tool '{}' {}abled", tool_name, if enabled { "en" } else { "dis" });
                Ok(())
            }
            None => Err(format!("Tool '{}' not found", tool_name)),
        }
    }

    /// Record a tool call start
    pub async fn record_call_start(&self, tool_name: &str) {
        // Update current calls count
        let current = self.current_calls.fetch_add(1, Ordering::Relaxed) + 1;
        let max = self.max_concurrency.load(Ordering::Relaxed);
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
    pub async fn record_call_end(self: &Arc<Self>, tool_name: &str) {
        // Drain permit deficit if any (from concurrency reduction under load)
        let mut deficit = self.permit_deficit.load(Ordering::Relaxed);
        while deficit > 0 {
            match self.concurrency_limiter.try_acquire() {
                Ok(permit) => {
                    permit.forget();
                    deficit -= 1;
                    self.permit_deficit.store(deficit, Ordering::Relaxed);
                }
                Err(_) => break,
            }
        }

        // Update current calls count
        let current = self.current_calls.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
        let max = self.max_concurrency.load(Ordering::Relaxed);
        let _ = self.status_tx.send(StatusUpdate::ConcurrentCalls {
            current,
            max,
        });

        // Update tool status
        if let Some(mut status) = self.tool_status.get_mut(tool_name) {
            status.record_call_end().await;
            let call_count = status.call_count;
            tracing::info!("Tool '{}' call ended (count: {})", tool_name, call_count);
            
            // Send immediate update - tool is still busy (within 5 second window)
            let is_busy = status.compute_is_busy().await;
            let _ = self.status_tx.send(StatusUpdate::ToolCallCount {
                tool: tool_name.to_string(),
                count: call_count,
                is_calling: false,
                is_busy,
            });
        }
    }

    /// Get all tool statuses (is_busy is computed dynamically)
    pub async fn get_all_tool_statuses(&self) -> Vec<ToolStatus> {
        let mut result = Vec::new();
        for entry in self.tool_status.iter() {
            let mut status = entry.value().clone();
            status.is_busy = status.compute_is_busy().await;
            result.push(status);
        }
        result
    }

    /// Get a specific tool's status (is_busy is computed dynamically)
    pub async fn get_tool_status(&self, tool_name: &str) -> Option<ToolStatus> {
        match self.tool_status.get(tool_name) {
            Some(s) => {
                let mut status = s.clone();
                status.is_busy = status.compute_is_busy().await;
                Some(status)
            }
            None => None,
        }
    }

    /// Get current concurrent calls count
    pub async fn get_current_calls(&self) -> usize {
        self.current_calls.load(Ordering::Relaxed)
    }

    /// Get maximum concurrent calls
    pub async fn get_max_concurrency(&self) -> usize {
        self.max_concurrency.load(Ordering::Relaxed)
    }

    /// Update maximum concurrency and dynamically adjust the semaphore
    pub async fn set_max_concurrency(&self, max: usize) {
        let max = if max == 0 { 1 } else { max };
        let old_val = self.max_concurrency.load(Ordering::Relaxed);
        self.max_concurrency.store(max, Ordering::Relaxed);

        if max > old_val {
            self.concurrency_limiter.add_permits(max - old_val);
            self.permit_deficit.store(0, Ordering::Relaxed);
        } else if max < old_val {
            let to_remove = old_val - max;
            let mut removed = 0usize;
            for _ in 0..to_remove {
                match self.concurrency_limiter.try_acquire() {
                    Ok(permit) => {
                        permit.forget();
                        removed += 1;
                    }
                    Err(_) => break,
                }
            }
            let deficit = to_remove - removed;
            self.permit_deficit.store(deficit, Ordering::Relaxed);
        }

        let current = self.current_calls.load(Ordering::Relaxed);
        let _ = self.status_tx.send(StatusUpdate::ConcurrentCalls {
            current,
            max,
        });
    }

    /// Check if python filesystem access is enabled
    pub async fn is_python_fs_access_enabled(&self) -> bool {
        self.python_fs_access_enabled.load(Ordering::Relaxed)
    }

    /// Set python filesystem access enabled/disabled
    pub async fn set_python_fs_access_enabled(&self, enabled: bool) {
        self.python_fs_access_enabled.store(enabled, Ordering::Relaxed);
        tracing::info!("Python filesystem access {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Set MCP service running status
    pub async fn set_mcp_running(&self, running: bool) {
        self.mcp_running.store(running, Ordering::Relaxed);
        let _ = self.status_tx.send(StatusUpdate::McpServiceStatus { running });
    }

    /// Check if MCP service is running
    pub async fn is_mcp_running(&self) -> bool {
        self.mcp_running.load(Ordering::Relaxed)
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

    /// Atomically check if a command is pending and remove it if so (for confirmation)
    pub async fn confirm_and_remove_pending_command(&self, command: &str, cwd: &str) -> bool {
        let hash = self.hash_command(command, cwd);
        let mut pending = self.pending_commands.write().await;
        
        if let Some(cmd) = pending.get(&hash) {
            if cmd.timestamp.elapsed() < Duration::from_secs(300) {
                pending.remove(&hash);
                return true;
            }
        }
        false
    }

    /// Stop the pending commands cleanup background task
    pub fn stop_pending_cleanup(&self) {
        if let Ok(mut guard) = self.pending_cleanup_handle.lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
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

    /// Collect current system metrics
    pub async fn collect_metrics(&self) -> Result<SystemMetrics, String> {
        self.metrics_collector.collect().await
    }

    // ===== Preset Management =====

    /// Get current preset name
    pub async fn get_current_preset(&self) -> Option<String> {
        self.current_preset.read().await.clone()
    }

    /// Set current preset name
    pub async fn set_current_preset(&self, preset: Option<String>) {
        let mut guard = self.current_preset.write().await;
        *guard = preset;
    }

    /// Get system prompt (sync, for use in get_info)
    pub fn get_system_prompt_sync(&self) -> Option<String> {
        self.system_prompt.read().ok().and_then(|g| g.clone())
    }

    /// Set system prompt
    pub fn set_system_prompt(&self, prompt: Option<String>) {
        if let Ok(mut guard) = self.system_prompt.write() {
            *guard = prompt;
        }
    }

    /// Apply a preset by name: enable tools in the preset, disable others
    pub async fn apply_preset(&self, preset_name: &str) -> Result<(), String> {
        let preset = get_preset(preset_name)
            .ok_or_else(|| format!("Preset '{}' not found", preset_name))?;

        let enabled_set: std::collections::HashSet<String> = preset.tools_enabled.iter().cloned().collect();

        for mut entry in self.tool_status.iter_mut() {
            let tool_name = entry.key().clone();
            let should_enable = enabled_set.contains(&tool_name);
            if entry.enabled != should_enable {
                entry.enabled = should_enable;
                let _ = self.status_tx.send(StatusUpdate::ToolEnabled {
                    tool: tool_name.clone(),
                    enabled: should_enable,
                });
            }
        }

        let _ = self.tool_list_changed_tx.send(ToolListChangedSignal);
        self.set_current_preset(Some(preset_name.to_string())).await;
        self.set_python_fs_access_enabled(preset.python_fs_access_enabled).await;
        info!("Applied preset '{}'", preset_name);
        Ok(())
    }

    // ===== Note Management =====

    const NOTE_MAX_COUNT: usize = 100;
    const NOTE_MAX_CONTENT_LEN: usize = 50_000;
    const NOTE_MAX_TITLE_LEN: usize = 200;
    const NOTE_MAX_TAGS: usize = 10;
    const NOTE_MAX_TAG_LEN: usize = 50;
    const NOTE_TIMEOUT_MINUTES: u64 = 30;

    /// Check if notes have expired (30min inactivity) and clear if so
    pub async fn check_notes_timeout(&self) {
        let last_access = *self.notes_last_access.read().await;
        if last_access.elapsed() > Duration::from_secs(Self::NOTE_TIMEOUT_MINUTES * 60) {
            let count = self.notes.len();
            if count > 0 {
                self.notes.clear();
                self.notes_next_id.store(1, Ordering::SeqCst);
                info!("Notes expired after {}min inactivity, cleared {} notes", Self::NOTE_TIMEOUT_MINUTES, count);
            }
        }
    }

    /// Touch notes last access time
    pub async fn touch_notes_access(&self) {
        let mut guard = self.notes_last_access.write().await;
        *guard = Instant::now();
    }

    /// Create a new note
    pub async fn note_create(&self, title: String, content: String, tags: Vec<String>, category: String) -> Result<Note, String> {
        self.check_notes_timeout().await;
        self.touch_notes_access().await;

        if self.notes.len() >= Self::NOTE_MAX_COUNT {
            return Err(format!("Maximum {} notes reached. Delete some notes first.", Self::NOTE_MAX_COUNT));
        }
        if title.len() > Self::NOTE_MAX_TITLE_LEN {
            return Err(format!("Title too long: max {} characters", Self::NOTE_MAX_TITLE_LEN));
        }
        let content = if content.len() > Self::NOTE_MAX_CONTENT_LEN {
            let mut truncated: String = content.chars().take(Self::NOTE_MAX_CONTENT_LEN).collect();
            truncated.push_str("...[truncated]");
            truncated
        } else {
            content
        };
        let tags: Vec<String> = tags.into_iter()
            .filter(|t| !t.is_empty() && t.len() <= Self::NOTE_MAX_TAG_LEN)
            .take(Self::NOTE_MAX_TAGS)
            .collect();

        let id = self.notes_next_id.fetch_add(1, Ordering::SeqCst);

        let note = Note::new(id, title, content, tags, category);
        self.notes.insert(id, note.clone());
        Ok(note)
    }

    /// List notes with optional filtering
    pub async fn note_list(&self, tag_filter: Option<String>, category_filter: Option<String>) -> Vec<Note> {
        self.check_notes_timeout().await;
        self.touch_notes_access().await;

        let mut notes: Vec<Note> = self.notes.iter()
            .filter(|e| {
                let n = e.value();
                if let Some(ref tag) = tag_filter {
                    let tag_lower = tag.to_lowercase();
                    if !n.tags.iter().any(|t| t.to_lowercase() == tag_lower) {
                        return false;
                    }
                }
                if let Some(ref cat) = category_filter {
                    if n.category != *cat { return false; }
                }
                true
            })
            .map(|e| e.value().clone())
            .collect();
        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        notes
    }

    /// Read a note by ID
    pub async fn note_read(&self, id: u64) -> Option<Note> {
        self.check_notes_timeout().await;
        self.touch_notes_access().await;
        self.notes.get(&id).map(|e| e.value().clone())
    }

    /// Update a note
    pub async fn note_update(&self, id: u64, title: Option<String>, content: Option<String>, tags: Option<Vec<String>>, category: Option<String>) -> Result<Note, String> {
        self.check_notes_timeout().await;
        self.touch_notes_access().await;

        match self.notes.get_mut(&id) {
            Some(mut note) => {
                if let Some(t) = title {
                    if t.len() > Self::NOTE_MAX_TITLE_LEN {
                        return Err(format!("Title too long: max {} characters", Self::NOTE_MAX_TITLE_LEN));
                    }
                    note.title = t;
                }
                if let Some(c) = content {
                    note.content = if c.len() > Self::NOTE_MAX_CONTENT_LEN {
                        let mut truncated: String = c.chars().take(Self::NOTE_MAX_CONTENT_LEN).collect();
                        truncated.push_str("...[truncated]");
                        truncated
                    } else {
                        c
                    };
                }
                if let Some(t) = tags {
                    note.tags = t.into_iter()
                        .filter(|t| !t.is_empty() && t.len() <= Self::NOTE_MAX_TAG_LEN)
                        .take(Self::NOTE_MAX_TAGS)
                        .collect();
                }
                if let Some(c) = category {
                    note.category = c;
                }
                note.updated_at = chrono::Local::now().to_rfc3339();
                Ok(note.clone())
            }
            None => Err(format!("Note {} not found", id)),
        }
    }

    /// Delete a note
    pub async fn note_delete(&self, id: u64) -> Result<(), String> {
        self.check_notes_timeout().await;
        self.touch_notes_access().await;

        if self.notes.remove(&id).is_some() {
            Ok(())
        } else {
            Err(format!("Note {} not found", id))
        }
    }

    /// Search notes by keyword in title, content, tags or category
    pub async fn note_search(&self, query: &str) -> Vec<Note> {
        self.check_notes_timeout().await;
        self.touch_notes_access().await;

        let q = query.to_lowercase();
        let mut notes: Vec<Note> = self.notes.iter()
            .filter(|e| {
                let n = e.value();
                n.title.to_lowercase().contains(&q)
                    || n.content.to_lowercase().contains(&q)
                    || n.category.to_lowercase().contains(&q)
                    || n.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .map(|e| e.value().clone())
            .collect();
        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        notes
    }

    /// Append content to a note
    pub async fn note_append(&self, id: u64, append_content: &str) -> Result<Note, String> {
        self.check_notes_timeout().await;
        self.touch_notes_access().await;

        match self.notes.get_mut(&id) {
            Some(mut note) => {
                note.content.push('\n');
                note.content.push_str(append_content);
                if note.content.len() > Self::NOTE_MAX_CONTENT_LEN {
                    let mut truncated: String = note.content.chars().take(Self::NOTE_MAX_CONTENT_LEN).collect();
                    truncated.push_str("...[truncated]");
                    note.content = truncated;
                }
                note.updated_at = chrono::Local::now().to_rfc3339();
                Ok(note.clone())
            }
            None => Err(format!("Note {} not found", id)),
        }
    }

    // ===== Task Management =====

    const TASK_MAX_COUNT: usize = 200;
    const TASK_MAX_TITLE_LEN: usize = 200;
    const TASK_MAX_DESC_LEN: usize = 5000;
    const TASK_MAX_TAGS: usize = 5;
    const TASK_MAX_TAG_LEN: usize = 50;

    pub async fn task_create(&self, title: String, description: String, priority: String, tags: Vec<String>) -> Result<Task, String> {
        if self.tasks.len() >= Self::TASK_MAX_COUNT {
            return Err(format!("Maximum {} tasks reached. Delete some tasks first.", Self::TASK_MAX_COUNT));
        }
        let title = if title.len() > Self::TASK_MAX_TITLE_LEN {
            title.chars().take(Self::TASK_MAX_TITLE_LEN).collect()
        } else {
            title
        };
        let description = if description.len() > Self::TASK_MAX_DESC_LEN {
            let mut truncated: String = description.chars().take(Self::TASK_MAX_DESC_LEN).collect();
            truncated.push_str("...[truncated]");
            truncated
        } else {
            description
        };
        let priority = match priority.to_lowercase().as_str() {
            "low" | "medium" | "high" => priority.to_lowercase(),
            _ => "medium".to_string(),
        };
        let tags: Vec<String> = tags.into_iter()
            .filter(|t| !t.is_empty() && t.len() <= Self::TASK_MAX_TAG_LEN)
            .take(Self::TASK_MAX_TAGS)
            .collect();
        let id = self.tasks_next_id.fetch_add(1, Ordering::SeqCst);
        let task = Task::new(id, title, description, priority, tags, "pending".to_string());
        self.tasks.insert(id, task.clone());
        Ok(task)
    }

    pub async fn task_list(&self, status_filter: Option<String>, priority_filter: Option<String>, tag_filter: Option<String>, sort_by: Option<String>) -> Vec<Task> {
        let status_filter = status_filter.map(|s| s.to_lowercase());
        let priority_filter = priority_filter.map(|p| p.to_lowercase());
        let tag_filter = tag_filter.map(|t| t.to_lowercase());
        let sort_by = sort_by.unwrap_or_else(|| "created".to_string()).to_lowercase();

        let mut tasks: Vec<Task> = self.tasks.iter()
            .filter(|e| {
                let t = e.value();
                if let Some(ref s) = status_filter {
                    if t.status != *s { return false; }
                }
                if let Some(ref p) = priority_filter {
                    if t.priority != *p { return false; }
                }
                if let Some(ref tg) = tag_filter {
                    if !t.tags.iter().any(|tag| tag.to_lowercase() == *tg) { return false; }
                }
                true
            })
            .map(|e| e.value().clone())
            .collect();

        match sort_by.as_str() {
            "priority" => tasks.sort_by(|a, b| {
                let pa = match a.priority.as_str() { "high" => 0, "medium" => 1, _ => 2 };
                let pb = match b.priority.as_str() { "high" => 0, "medium" => 1, _ => 2 };
                pa.cmp(&pb)
            }),
            "status" => tasks.sort_by(|a, b| {
                let sa = match a.status.as_str() { "pending" => 0, "in_progress" => 1, _ => 2 };
                let sb = match b.status.as_str() { "pending" => 0, "in_progress" => 1, _ => 2 };
                sa.cmp(&sb)
            }),
            _ => tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
        }

        tasks
    }

    pub async fn task_read(&self, id: u64) -> Option<Task> {
        self.tasks.get(&id).map(|e| e.value().clone())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn task_update(&self, id: u64, status: Option<String>, title: Option<String>, description: Option<String>, new_priority: Option<String>, add_tags: Option<Vec<String>>, remove_tags: Option<Vec<String>>) -> Result<Task, String> {
        match self.tasks.get_mut(&id) {
            Some(mut task) => {
                if let Some(s) = status {
                    let s_lower = s.to_lowercase();
                    if matches!(s_lower.as_str(), "pending" | "in_progress" | "completed") {
                        task.status = s_lower;
                    } else {
                        return Err(format!("Invalid status: '{}'. Must be pending, in_progress, or completed.", s));
                    }
                }
                if let Some(t) = title {
                    task.title = if t.len() > Self::TASK_MAX_TITLE_LEN {
                        t.chars().take(Self::TASK_MAX_TITLE_LEN).collect()
                    } else {
                        t
                    };
                }
                if let Some(d) = description {
                    task.description = if d.len() > Self::TASK_MAX_DESC_LEN {
                        let mut truncated: String = d.chars().take(Self::TASK_MAX_DESC_LEN).collect();
                        truncated.push_str("...[truncated]");
                        truncated
                    } else {
                        d
                    };
                }
                if let Some(p) = new_priority {
                    let p_lower = p.to_lowercase();
                    if matches!(p_lower.as_str(), "low" | "medium" | "high") {
                        task.priority = p_lower;
                    } else {
                        return Err(format!("Invalid priority: '{}'. Must be low, medium, or high.", p));
                    }
                }
                if let Some(add) = add_tags {
                    for tag in add {
                        let tag = tag.trim().to_string();
                        if tag.is_empty() || tag.len() > Self::TASK_MAX_TAG_LEN {
                            continue;
                        }
                        if task.tags.len() >= Self::TASK_MAX_TAGS {
                            break;
                        }
                        if !task.tags.iter().any(|t| t == &tag) {
                            task.tags.push(tag);
                        }
                    }
                }
                if let Some(remove) = remove_tags {
                    task.tags.retain(|t| !remove.iter().any(|r| r == t));
                }
                task.updated_at = chrono::Local::now().to_rfc3339();
                Ok(task.clone())
            }
            None => Err(format!("Task {} not found", id)),
        }
    }

    pub async fn task_delete(&self, id: u64) -> Result<(), String> {
        if self.tasks.remove(&id).is_some() {
            Ok(())
        } else {
            Err(format!("Task {} not found", id))
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
            allow_dangerous_commands: vec![],
            allowed_hosts: None,
            disable_allowed_hosts: false,
            preset: "minimal".to_string(),
            system_prompt: None,
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
        ]).await;
        
        assert!(state.is_tool_enabled("tool1").await);
        assert!(state.is_tool_enabled("tool2").await);
        
        let all_statuses = state.get_all_tool_statuses().await;
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
            tool: "dir_list".to_string(),
            count: 5,
            is_calling: true,
            is_busy: true,
        };
        let json = serde_json::to_string(&update).unwrap();
        println!("ToolCallCount JSON: {}", json);
        assert!(json.contains("\"type\":\"ToolCallCount\""));
        assert!(json.contains("\"tool\":\"dir_list\""));
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
