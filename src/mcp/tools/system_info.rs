use rmcp::model::CallToolResult;
use serde::Serialize;
use sysinfo::{System, RefreshKind};

#[derive(Debug, Serialize)]
struct SystemInfo {
    os_name: String,
    os_version: String,
    kernel_version: String,
    hostname: String,
    cpu_count: usize,
    cpu_brand: String,
    cpu_usage_percent: f32,
    memory_total_mb: u64,
    memory_used_mb: u64,
    memory_free_mb: u64,
    memory_usage_percent: f64,
    uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    load_average: Option<(f64, f64, f64)>,
}

pub async fn system_info() -> Result<CallToolResult, String> {
    let mut system = System::new_with_specifics(
        RefreshKind::everything()
    );

    // Wait a bit for CPU usage measurement
    std::thread::sleep(std::time::Duration::from_millis(500));
    system.refresh_cpu_all();
    system.refresh_memory();

    // Calculate average CPU usage across all cores
    let cpu_usage: f32 = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() 
        / system.cpus().len().max(1) as f32;

    // Get load average (Unix only, Windows returns 0.0)
    let load_avg = System::load_average();
    let load_average = if load_avg.one > 0.0 || load_avg.five > 0.0 || load_avg.fifteen > 0.0 {
        Some((load_avg.one, load_avg.five, load_avg.fifteen))
    } else {
        None // Windows doesn't have Unix-style load average
    };

    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let memory_usage_percent = if total_memory > 0 {
        (used_memory as f64 / total_memory as f64) * 100.0
    } else {
        0.0
    };

    let info = SystemInfo {
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        cpu_count: system.cpus().len(),
        cpu_brand: system.cpus().first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        cpu_usage_percent: cpu_usage,
        memory_total_mb: total_memory / 1024,
        memory_used_mb: used_memory / 1024,
        memory_free_mb: system.free_memory() / 1024,
        memory_usage_percent,
        uptime_seconds: System::uptime(),
        load_average,
    };

    let json = serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_info() {
        let result = system_info().await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("os_name"));
                assert!(text.text.contains("cpu_count"));
            }
        }
    }
}
