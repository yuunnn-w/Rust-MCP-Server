use rmcp::model::CallToolResult;
use serde::Serialize;
use std::cmp::Ordering;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, ProcessesToUpdate};

#[derive(Debug, Serialize)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory_mb: u64,
    status: String,
}

pub async fn process_list() -> Result<CallToolResult, String> {
    let mut system = System::new_with_specifics(
        RefreshKind::everything().with_processes(ProcessRefreshKind::everything()),
    );

    // Refresh to get current CPU usage
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut processes: Vec<ProcessInfo> = system
        .processes()
        .iter()
        .map(|(pid, process)| ProcessInfo {
            pid: pid.as_u32(),
            name: process.name().to_string_lossy().to_string(),
            cpu_usage: process.cpu_usage(),
            memory_mb: process.memory() / (1024 * 1024),
            status: format!("{:?}", process.status()),
        })
        .collect();

    // Sort by CPU usage descending (safely handle NaN)
    processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(Ordering::Equal));

    // Take top 50 processes
    let top_processes: Vec<_> = processes.into_iter().take(50).collect();

    let json = serde_json::to_string_pretty(&top_processes).map_err(|e| e.to_string())?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(
        format!(
            "Top 50 processes by CPU usage:\n\n{}",
            json
        ),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_list() {
        let result = process_list().await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("processes"));
            }
        }
    }
}
