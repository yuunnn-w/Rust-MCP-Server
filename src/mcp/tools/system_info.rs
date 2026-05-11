use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use sysinfo::{
    Components, CpuRefreshKind, DiskKind, Disks, MemoryRefreshKind, Networks, RefreshKind,
    System,
};

fn round_two(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SystemInfoParams {
    #[schemars(description = "Sections to include: system, cpu, memory, disks, network, temperature, processes. Default: all sections enabled. On Windows 7, disk/network/temperature are skipped for compatibility.")]
    pub sections: Option<Vec<String>>,
    #[schemars(description = "Maximum number of processes to return (default: 50)")]
    pub process_limit: Option<usize>,
    #[schemars(description = "Sort processes by: cpu, memory, name (default: cpu)")]
    pub process_sort: Option<String>,
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
    interface_ips: Vec<InterfaceIpInfo>,
    components: Vec<ComponentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    processes: Option<Vec<ProcessInfo>>,
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
struct InterfaceIpInfo {
    interface_name: String,
    ip: String,
    is_loopback: bool,
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

pub async fn system_info(params: Parameters<SystemInfoParams>) -> Result<CallToolResult, String> {
    let params = params.0;
    let sections_raw = params.sections;
    let sections_lower: Option<Vec<String>> = sections_raw.map(|v| v.iter().map(|s| s.to_lowercase()).collect());
    let sections_all = sections_lower.as_ref().map(|s| s.is_empty()).unwrap_or(false)
        || sections_lower.is_none();
    let section_enabled = |name: &str| -> bool {
        sections_all || sections_lower.as_ref().map(|s| s.contains(&name.to_string())).unwrap_or(false)
    };
    let process_limit = params.process_limit.unwrap_or(50);
    let process_sort = params.process_sort.unwrap_or_else(|| "cpu".to_string());

    // Detect Windows version at runtime. On systems older than Windows 10,
    // skip collecting disk, network, and component (temperature) data to avoid
    // potential crashes caused by sysinfo/windows crate compatibility issues.
    #[cfg(windows)]
    let is_modern_windows = crate::utils::windows_version::is_windows_10_or_later();
    #[cfg(not(windows))]
    let is_modern_windows = true;

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
    let disks_info: Vec<DiskInfo> = if is_modern_windows && section_enabled("disks") {
        tokio::task::spawn_blocking(move || {
            let disks = Disks::new_with_refreshed_list();
            disks
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
                .collect::<Vec<DiskInfo>>()
        }).await.unwrap_or_else(|e| {
            tracing::error!("Failed to collect disk info: {}", e);
            vec![]
        })
    } else {
        vec![]
    };

    // Network interfaces
    let network_interfaces: Vec<NetworkInterfaceInfo> = if is_modern_windows && section_enabled("network") {
        tokio::task::spawn_blocking(move || {
            let networks = Networks::new_with_refreshed_list();
            networks
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
                .collect::<Vec<NetworkInterfaceInfo>>()
        }).await.unwrap_or_else(|e| {
            tracing::error!("Failed to collect network info: {}", e);
            vec![]
        })
    } else {
        vec![]
    };

    // Hardware components (temperature)
    let components_info: Vec<ComponentInfo> = if is_modern_windows && (section_enabled("temperature") || section_enabled("components")) {
        tokio::task::spawn_blocking(move || {
            let components = Components::new_with_refreshed_list();
            components
                .iter()
                .map(|component| {
                    let temp = component.temperature();
                    let max_temp = component.max();
                    let critical_temp = component.critical();
                    ComponentInfo {
                        label: component.label().to_string(),
                        temperature: temp.and_then(|t| {
                            if t.is_nan() { None } else { Some(round_two(t as f64)) }
                        }),
                        max_temperature: max_temp.and_then(|t| {
                            if t.is_nan() { None } else { Some(round_two(t as f64)) }
                        }),
                        critical_temperature: critical_temp.and_then(|t| {
                            if t.is_nan() { None } else { Some(round_two(t as f64)) }
                        }),
                    }
                })
                .collect::<Vec<ComponentInfo>>()
        }).await.unwrap_or_else(|e| {
            tracing::error!("Failed to collect component info: {}", e);
            vec![]
        })
    } else {
        vec![]
    };

    // Interface IPs from get_if_addrs
    let interface_ips: Vec<InterfaceIpInfo> = if section_enabled("network") {
        match tokio::task::spawn_blocking(get_if_addrs::get_if_addrs).await {
            Ok(Ok(ifaces)) => ifaces
                .into_iter()
                .map(|iface| InterfaceIpInfo {
                    interface_name: iface.name.clone(),
                    ip: iface.ip().to_string(),
                    is_loopback: iface.is_loopback(),
                })
                .collect(),
            _ => vec![],
        }
    } else {
        vec![]
    };

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
        interface_ips,
        components: components_info,
        processes: None,
    };

    // Build processes if requested
    if section_enabled("processes") {
        let mut info = info;
        info.processes = Some(get_process_list(process_limit, &process_sort).await);
        let json = serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?;
        return Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]));
    }

    let json = serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?;

    Ok(CallToolResult::success(vec![rmcp::model::Content::text(json)]))
}

async fn get_process_list(limit: usize, sort_by: &str) -> Vec<ProcessInfo> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System as Sys};
    let mut system = Sys::new_with_specifics(
        RefreshKind::everything().with_processes(ProcessRefreshKind::everything()),
    );
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

    match sort_by {
        "memory" => processes.sort_by_key(|b| std::cmp::Reverse(b.memory_mb)),
        "name" => processes.sort_by_key(|a| a.name.to_lowercase()),
        _ => processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(Ordering::Equal)),
    }

    processes.into_iter().take(limit).collect()
}

#[derive(Debug, Serialize)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory_mb: u64,
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_info() {
        let params = SystemInfoParams {
            sections: None,
            process_limit: None,
            process_sort: None,
        };
        let result = system_info(Parameters(params)).await;
        assert!(result.is_ok());

        if let Ok(ref call_result) = result {
            if let Some(text) = call_result.content.first().and_then(|c| c.as_text()) {
                assert!(text.text.contains("os_name"));
                assert!(text.text.contains("cpu_count"));
                assert!(text.text.contains("disks"));
                assert!(text.text.contains("network_interfaces"));
                assert!(text.text.contains("interface_ips"));
            }
        }
    }
}
