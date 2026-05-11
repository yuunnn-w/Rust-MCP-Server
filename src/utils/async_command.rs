use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct OutputLine {
    pub stream: StreamType,
    pub line: String,
    #[allow(dead_code)]
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamType {
    Stdout,
    Stderr,
}

pub struct MonitorState {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub start_time: Instant,
    pub last_activity: Instant,
    pub exit_code: Option<i32>,
    #[allow(dead_code)]
    pub pid: Option<u32>,
    #[allow(dead_code)]
    pub command: String,
    pub tx: broadcast::Sender<OutputLine>,
}

pub struct AsyncCommandManager {
    pub states: HashMap<String, MonitorState>,
    pub max_concurrent: usize,
}

impl AsyncCommandManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            states: HashMap::new(),
            max_concurrent,
        }
    }

    pub fn register(
        &mut self,
        id: String,
        command: String,
        pid: Option<u32>,
        tx: broadcast::Sender<OutputLine>,
    ) -> Result<(), String> {
        if self.states.len() >= self.max_concurrent {
            return Err(format!(
                "Maximum concurrent commands reached: {}",
                self.max_concurrent
            ));
        }
        let now = Instant::now();
        self.states.insert(
            id.clone(),
            MonitorState {
                id,
                start_time: now,
                last_activity: now,
                exit_code: None,
                pid,
                command,
                tx,
            },
        );
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&MonitorState> {
        self.states.get(id)
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self, id: &str) -> Option<&mut MonitorState> {
        self.states.get_mut(id)
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, id: &str) -> Option<MonitorState> {
        self.states.remove(id)
    }

    pub fn cleanup_expired(&mut self, timeout_secs: u64) {
        let now = Instant::now();
        self.states.retain(|_, state| {
            let elapsed = now.duration_since(state.last_activity).as_secs();
            elapsed < timeout_secs
        });
    }

    /// Start a background task that periodically cleans up expired command states.
    pub fn start_periodic_cleanup(interval_secs: u64, expire_timeout_secs: u64) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(mut manager) = GLOBAL_ASYNC_COMMANDS.lock() {
                        manager.cleanup_expired(expire_timeout_secs);
                    }
                }).await;
            }
        });
    }

    pub fn send_signal(pid: u32, signal: &str) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::process::Command;
            let sig = match signal {
                "terminate" => "TERM",
                "kill" => "KILL",
                "interrupt" => "INT",
                _ => "TERM",
            };
            Command::new("kill")
                .arg(format!("-{}", sig))
                .arg(pid.to_string())
                .status()
                .map_err(|e| format!("Failed to send signal: {}", e))?;
        }
        #[cfg(windows)]
        {
            use std::process::Command;
            if signal == "kill" {
                Command::new("taskkill")
                    .arg("/PID")
                    .arg(pid.to_string())
                    .arg("/F")
                    .status()
                    .map_err(|e| format!("Failed to send signal: {}", e))?;
            } else {
                Command::new("taskkill")
                    .arg("/PID")
                    .arg(pid.to_string())
                    .status()
                    .map_err(|e| format!("Failed to send signal: {}", e))?;
            }
        }
        Ok(())
    }
}

pub static GLOBAL_ASYNC_COMMANDS: LazyLock<Mutex<AsyncCommandManager>> = LazyLock::new(|| {
    Mutex::new(AsyncCommandManager::new(16))
});
