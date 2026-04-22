use serde::Serialize;
use std::sync::{Arc, Mutex};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

/// System metrics snapshot
#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    /// CPU usage percentage (0-100)
    pub cpu_percent: f32,
    /// Total memory in bytes
    pub memory_total: u64,
    /// Used memory in bytes
    pub memory_used: u64,
    /// Memory usage percentage (0-100)
    pub memory_percent: f32,
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// Load average (1min, 5min, 15min) - may be zero on Windows
    pub load_average: [f64; 3],
    /// Total number of processes
    pub process_count: usize,
}

/// Metrics collector with internal caching
pub struct MetricsCollector {
    system: Arc<Mutex<System>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        // Need a small delay for CPU measurement to be meaningful on first call
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_all();
        system.refresh_memory();

        Self {
            system: Arc::new(Mutex::new(system)),
        }
    }

    pub fn collect(&self) -> SystemMetrics {
        let mut system = self.system.lock().unwrap();

        // Refresh CPU and memory
        system.refresh_cpu_all();
        system.refresh_memory();

        let cpu_percent = system.global_cpu_usage();
        let memory_total = system.total_memory();
        let memory_used = system.used_memory();
        let memory_percent = if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        };

        let load_average = System::load_average();

        SystemMetrics {
            cpu_percent,
            memory_total,
            memory_used,
            memory_percent,
            cpu_cores: system.cpus().len(),
            uptime_seconds: System::uptime(),
            load_average: [load_average.one, load_average.five, load_average.fifteen],
            process_count: system.processes().len(),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        let metrics = collector.collect();

        // CPU cores should always be > 0 on any real system
        assert!(metrics.cpu_cores > 0);
        // Memory total should be > 0
        assert!(metrics.memory_total > 0);
        // Memory percent should be within valid range
        assert!(metrics.memory_percent >= 0.0 && metrics.memory_percent <= 100.0);
        // CPU percent should be within valid range
        assert!(metrics.cpu_percent >= 0.0 && metrics.cpu_percent <= 100.0);
    }
}
