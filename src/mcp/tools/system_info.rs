use rmcp::model::CallToolResult;
use serde::Serialize;
use sysinfo::{
    Components, CpuRefreshKind, DiskKind, Disks, MemoryRefreshKind, Networks, RefreshKind,
    System,
};

fn round_two(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[derive(Debug, Serialize)]
struct SystemInfo {
    os_name: String,
    os_version: String,
    long_os_version: String,
    distribution_id: String,
    kernel_version: String,
    hostname: String,
    cpu_arch: String,
    cpu_count: usize,
    physical_core_count: Option<usize>,
    cpu_brand: String,
    cpu_frequency_mhz: f64,
    cpu_usage_percent: f64,
    memory_total_mb: f64,
    memory_used_mb: f64,
    memory_free_mb: f64,
    memory_usage_percent: f64,
    swap_total_mb: f64,
    swap_used_mb: f64,
    swap_free_mb: f64,
    swap_usage_percent: f64,
    uptime_seconds: u64,
    boot_time: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    load_average: Option<(f64, f64, f64)>,
    disks: Vec<DiskInfo>,
    network_interfaces: Vec<NetworkInterfaceInfo>,
    components: Vec<ComponentInfo>,
}

#[derive(Debug, Serialize)]
struct DiskInfo {
    name: String,
    mount_point: String,
    file_system: String,
    kind: String,
    total_gb: f64,
    available_gb: f64,
    usage_percent: f64,
    is_removable: bool,
    is_read_only: bool,
}

#[derive(Debug, Serialize)]
struct NetworkInterfaceInfo {
    name: String,
    mac_address: String,
    ip_addresses: Vec<String>,
    mtu: u64,
    total_received_mb: f64,
    total_transmitted_mb: f64,
}

#[derive(Debug, Serialize)]
struct ComponentInfo {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    critical_temperature: Option<f64>,
}

pub async fn system_info() -> Result<CallToolResult, String> {
    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    // Wait a bit for CPU usage measurement
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    system.refresh_cpu_all();
    system.refresh_memory();

    // CPU info
    let cpu_usage_percent = round_two(system.global_cpu_usage() as f64);
    let cpu_count = system.cpus().len();
    let cpu_brand = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let cpu_frequency_mhz = system
        .cpus()
        .first()
        .map(|cpu| round_two(cpu.frequency() as f64))
        .unwrap_or(0.0);

    // Memory info (bytes -> MB)
    let total_memory = system.total_memory() as f64;
    let used_memory = system.used_memory() as f64;
    let available_memory = system.available_memory() as f64;

    let memory_total_mb = round_two(total_memory / (1024.0 * 1024.0));
    let memory_used_mb = round_two(used_memory / (1024.0 * 1024.0));
    let memory_free_mb = round_two(available_memory / (1024.0 * 1024.0));
    let memory_usage_percent = if total_memory > 0.0 {
        round_two((used_memory / total_memory) * 100.0)
    } else {
        0.0
    };

    // Swap info (bytes -> MB)
    let total_swap = system.total_swap() as f64;
    let used_swap = system.used_swap() as f64;
    let free_swap = system.free_swap() as f64;

    let swap_total_mb = round_two(total_swap / (1024.0 * 1024.0));
    let swap_used_mb = round_two(used_swap / (1024.0 * 1024.0));
    let swap_free_mb = round_two(free_swap / (1024.0 * 1024.0));
    let swap_usage_percent = if total_swap > 0.0 {
        round_two((used_swap / total_swap) * 100.0)
    } else {
        0.0
    };

    // Load average (Unix only)
    #[cfg(unix)]
    let load_average = {
        let load_avg = System::load_average();
        Some((
            round_two(load_avg.one),
            round_two(load_avg.five),
            round_two(load_avg.fifteen),
        ))
    };
    #[cfg(not(unix))]
    let load_average = None;

    // Disks
    let disks = Disks::new_with_refreshed_list();
    let disks_info: Vec<DiskInfo> = disks
        .iter()
        .map(|disk| {
            let total = disk.total_space() as f64;
            let available = disk.available_space() as f64;
            let used = if total > available { total - available } else { 0.0 };
            let usage_percent = if total > 0.0 {
                round_two((used / total) * 100.0)
            } else {
                0.0
            };
            DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().display().to_string(),
                file_system: disk.file_system().to_string_lossy().to_string(),
                kind: match disk.kind() {
                    DiskKind::HDD => "HDD".to_string(),
                    DiskKind::SSD => "SSD".to_string(),
                    DiskKind::Unknown(_) => "Unknown".to_string(),
                },
                total_gb: round_two(total / (1024.0 * 1024.0 * 1024.0)),
                available_gb: round_two(available / (1024.0 * 1024.0 * 1024.0)),
                usage_percent,
                is_removable: disk.is_removable(),
                is_read_only: disk.is_read_only(),
            }
        })
        .collect();

    // Network interfaces
    let networks = Networks::new_with_refreshed_list();
    let network_interfaces: Vec<NetworkInterfaceInfo> = networks
        .iter()
        .map(|(name, data)| {
            let total_received = data.total_received() as f64;
            let total_transmitted = data.total_transmitted() as f64;
            NetworkInterfaceInfo {
                name: name.clone(),
                mac_address: data.mac_address().to_string(),
                ip_addresses: data.ip_networks().iter().map(|n| n.to_string()).collect(),
                mtu: data.mtu(),
                total_received_mb: round_two(total_received / (1024.0 * 1024.0)),
                total_transmitted_mb: round_two(total_transmitted / (1024.0 * 1024.0)),
            }
        })
        .collect();

    // Hardware components (temperature)
    let components = Components::new_with_refreshed_list();
    let components_info: Vec<ComponentInfo> = components
        .iter()
        .map(|component| {
            let temp = component.temperature();
            let max_temp = component.max();
            let critical_temp = component.critical();
            ComponentInfo {
                label: component.label().to_string(),
                temperature: temp.and_then(|t| {
                    if t.is_nan() {
                        None
                    } else {
                        Some(round_two(t as f64))
                    }
                }),
                max_temperature: max_temp.and_then(|t| {
                    if t.is_nan() {
                        None
                    } else {
                        Some(round_two(t as f64))
                    }
                }),
                critical_temperature: critical_temp.and_then(|t| {
                    if t.is_nan() {
                        None
                    } else {
                        Some(round_two(t as f64))
                    }
                }),
            }
        })
        .collect();

    let info = SystemInfo {
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        long_os_version: System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
        distribution_id: System::distribution_id(),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        cpu_arch: System::cpu_arch(),
        cpu_count,
        physical_core_count: System::physical_core_count(),
        cpu_brand,
        cpu_frequency_mhz,
        cpu_usage_percent,
        memory_total_mb,
        memory_used_mb,
        memory_free_mb,
        memory_usage_percent,
        swap_total_mb,
        swap_used_mb,
        swap_free_mb,
        swap_usage_percent,
        uptime_seconds: System::uptime(),
        boot_time: System::boot_time(),
        load_average,
        disks: disks_info,
        network_interfaces,
        components: components_info,
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
                assert!(text.text.contains("disks"));
                assert!(text.text.contains("network_interfaces"));
                assert!(text.text.contains("components"));
            }
        }
    }
}
