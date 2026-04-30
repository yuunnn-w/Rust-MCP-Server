use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(
    name = "rust-mcp-server",
    author = "MCP Server Team",
    version = "0.2.0",
    about = "A high-performance MCP server with WebUI control panel / 高性能 MCP 服务器，带 WebUI 控制面板",
    long_about = "Rust MCP Server - A high-performance Model Context Protocol server\n\
                    Rust MCP 服务器 - 高性能模型上下文协议服务器\n\n\
                    Features / 功能特性:\n\
                    - HTTP/SSE transport modes / HTTP/SSE 传输模式\n\
                    - WebUI control panel for tool management / WebUI 控制面板管理工具\n\
                    - 20 built-in tools (file ops, calculator, HTTP, etc.) / 20个内置工具（文件操作、计算器、HTTP等）\n\
                    - Real-time tool call statistics / 实时工具调用统计\n\
                    - Tool enable/disable control / 工具启用/禁用控制"
)]
pub struct AppConfig {
    /// WebUI listening address (WebUI 监听地址)
    #[arg(long, default_value = "127.0.0.1", env = "MCP_WEBUI_HOST")]
    pub webui_host: String,

    /// WebUI listening port (WebUI 监听端口，默认: 2233)
    #[arg(long, default_value_t = 2233, env = "MCP_WEBUI_PORT")]
    pub webui_port: u16,

    /// MCP transport type: http (default, JSON response), sse (stream response)
    /// MCP 传输模式: http (默认，JSON响应), sse (流式响应)
    #[arg(long, default_value = "http", env = "MCP_TRANSPORT")]
    pub mcp_transport: String,

    /// MCP service listening address (MCP 服务监听地址)
    #[arg(long, default_value = "127.0.0.1", env = "MCP_HOST")]
    pub mcp_host: String,

    /// MCP service listening port (MCP 服务监听端口，默认: 3344)
    #[arg(long, default_value_t = 3344, env = "MCP_PORT")]
    pub mcp_port: u16,

    /// Maximum concurrent tool calls (最大并发调用数，默认: 10)
    #[arg(long, default_value_t = 10, env = "MCP_MAX_CONCURRENCY")]
    pub max_concurrency: usize,

    /// Disabled tools, comma-separated (禁用的工具列表，逗号分隔)
    /// Default enabled: calculator, dir_list, file_read, file_search, image_read, file_stat, path_exists, json_query, git_ops, env_get
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "file_write,file_ops,file_edit,http_request,datetime,execute_command,process_list,base64_codec,hash_compute,system_info",
        env = "MCP_DISABLE_TOOLS"
    )]
    #[serde(default = "default_disable_tools")]
    pub disable_tools: Vec<String>,

    /// Working directory for file operations (dangerous ops restricted here)
    /// 文件操作工作目录（危险操作限制在此目录内）
    #[arg(long, default_value = ".", env = "MCP_WORKING_DIR")]
    pub working_dir: PathBuf,

    /// Log level: trace, debug, info, warn, error (日志级别: trace, debug, info, warn, error)
    #[arg(long, default_value = "info", env = "MCP_LOG_LEVEL")]
    pub log_level: String,

    /// Disable WebUI (禁用 WebUI 控制面板)
    #[arg(long, env = "MCP_DISABLE_WEBUI")]
    pub disable_webui: bool,

    /// Allowed dangerous commands for execute_command tool (允许执行的危险命令序号列表)
    /// 默认全部禁止，可通过序号启用特定命令
    /// 1=rm, 2=del, 3=format, 4=mkfs, 5=dd, 6=fork, 7=eval, 8=exec, 9=system, 10=shred
    /// 11=rd, 12=format, 13=diskpart, 14=reg, 15=net, 16=sc, 17=schtasks, 18=powercfg, 19=bcdedit, 20=wevtutil
    #[arg(
        long,
        value_delimiter = ',',
        env = "MCP_ALLOW_DANGEROUS_COMMANDS",
        help = "允许执行的危险命令序号，逗号分隔。默认全部禁止。\n\
                Dangerous command list / 危险命令列表:\n\
                Linux: 1=rm(delete), 2=del(delete), 3=format, 4=mkfs, 5=dd, 6=fork(:(){:|:&};:), 7=eval, 8=exec, 9=system, 10=shred\n\
                Windows: 11=rd/s, 12=format, 13=diskpart, 14=reg(registry/注册表), 15=net(network/网络), 16=sc(service/服务), 17=schtasks(scheduled tasks/计划任务), 18=powercfg, 19=bcdedit, 20=wevtutil"
    )]
    #[serde(default)]
    pub allow_dangerous_commands: Vec<u8>,

    /// Custom allowed hosts for DNS rebinding protection (覆盖自动推断的 allowed_hosts)
    /// 自定义允许的 Host 头列表，用于 DNS 重绑定保护
    #[arg(long, value_delimiter = ',', env = "MCP_ALLOWED_HOSTS")]
    #[serde(default)]
    pub allowed_hosts: Option<Vec<String>>,

    /// Disable allowed hosts check (NOT recommended for public deployments)
    /// 禁用 allowed_hosts 检查（不推荐用于公网部署）
    #[arg(long, env = "MCP_DISABLE_ALLOWED_HOSTS")]
    #[serde(default)]
    pub disable_allowed_hosts: bool,
}

/// Default value for disable_tools
fn default_disable_tools() -> Vec<String> {
    vec![
        "file_write".to_string(),
        "file_ops".to_string(),
        "file_edit".to_string(),
        "http_request".to_string(),
        "datetime".to_string(),
        "execute_command".to_string(),
        "process_list".to_string(),
        "base64_codec".to_string(),
        "hash_compute".to_string(),
        "system_info".to_string(),
    ]
}

impl AppConfig {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        let mut config = Self::parse();
        // If working_dir is the default ".", resolve it to the actual current working directory
        if config.working_dir.as_os_str() == "." {
            if let Ok(cwd) = std::env::current_dir() {
                config.working_dir = cwd;
            }
        }
        config
    }

    /// Get WebUI bind address
    pub fn webui_bind_addr(&self) -> String {
        format!("{}:{}", self.webui_host, self.webui_port)
    }

    /// Get MCP bind address (for sse/http)
    pub fn mcp_bind_addr(&self) -> String {
        format!("{}:{}", self.mcp_host, self.mcp_port)
    }

    /// Check if a tool is disabled
    pub fn is_tool_disabled(&self, tool_name: &str) -> bool {
        self.disable_tools.iter().any(|t| t.trim() == tool_name)
    }

    /// Check if a dangerous command is allowed
    pub fn is_dangerous_command_allowed(&self, command_id: u8) -> bool {
        self.allow_dangerous_commands.contains(&command_id)
    }

    /// Get dangerous command name by ID (for logging/display)
    pub fn get_dangerous_command_name(command_id: u8) -> Option<&'static str> {
        match command_id {
            // Linux commands
            1 => Some("rm (delete files)"),
            2 => Some("del (delete files)"),
            3 => Some("format (format disk)"),
            4 => Some("mkfs (create filesystem)"),
            5 => Some("dd (disk copy)"),
            6 => Some("fork bomb (:(){:|:&};:)"),
            7 => Some("eval (code execution)"),
            8 => Some("exec (process replacement)"),
            9 => Some("system (system call)"),
            10 => Some("shred (secure delete)"),
            // Windows commands
            11 => Some("rd /s (delete directory tree)"),
            12 => Some("format (format disk)"),
            13 => Some("diskpart (disk partition)"),
            14 => Some("reg (registry operations)"),
            15 => Some("net (network/account management)"),
            16 => Some("sc (service control)"),
            17 => Some("schtasks (scheduled tasks)"),
            18 => Some("powercfg (power configuration)"),
            19 => Some("bcdedit (boot configuration)"),
            20 => Some("wevtutil (event logs)"),
            _ => None,
        }
    }

    /// Check command for dangerous patterns
    /// Returns Some(command_id) if dangerous command found and not allowed
    pub fn check_dangerous_command(&self, command: &str) -> Option<u8> {
        let cmd_lower = command.to_lowercase();
        let cmd_trimmed = cmd_lower.trim();

        // Dangerous command profiles: (id, base_command_prefixes, required_substrings)
        // If prefixes is empty, only substrings are checked.
        // If substrings is empty, matching any prefix is sufficient.
        let profiles: Vec<(u8, Vec<&str>, Vec<&str>)> = vec![
            // 1: rm with recursive/delete flags
            (1, vec!["rm "], vec!["-r", "-rf", "--recursive", "-fr"]),
            // 2: del / erase with recursive/force flags
            (2, vec!["del ", "erase "], vec!["/s", "/q", "/f"]),
            // 3: format disk
            (3, vec!["format "], vec![]),
            // 4: mkfs
            (4, vec!["mkfs.", "mkfs "], vec![]),
            // 5: dd with if=/of=
            (5, vec!["dd "], vec!["if=", "of="]),
            // 6: fork bomb
            (6, vec![], vec![":(){:|:&};:", "fork bomb"]),
            // 7: eval with code execution patterns
            (7, vec!["eval "], vec!["$(", "`"]),
            // 8: exec
            (8, vec!["exec "], vec![]),
            // 9: system
            (9, vec!["system(", "system "], vec![]),
            // 10: shred
            (10, vec!["shred "], vec![]),
            // 11: rd / rmdir with /s
            (11, vec!["rd ", "rmdir "], vec!["/s", "/q"]),
            // 12: format (Windows, same as 3)
            // 13: diskpart
            (13, vec!["diskpart", "diskpart.exe"], vec![]),
            // 14: reg modifications
            (14, vec!["reg "], vec!["delete", "add", "import"]),
            // 15: net commands
            (15, vec!["net "], vec!["user", "localgroup", "stop", "start", "share", "use"]),
            // 16: sc service control
            (16, vec!["sc "], vec!["delete", "config", "stop", "start"]),
            // 17: schtasks
            (17, vec!["schtasks "], vec!["/create", "/delete", "/run"]),
            // 18: powercfg
            (18, vec!["powercfg ", "powercfg-"], vec![]),
            // 19: bcdedit
            (19, vec!["bcdedit ", "bcdedit-"], vec![]),
            // 20: wevtutil clear
            (20, vec!["wevtutil "], vec!["cl", "clear-log"]),
        ];

        for (id, prefixes, substrings) in profiles {
            if self.is_dangerous_command_allowed(id) {
                continue;
            }

            let matched = if prefixes.is_empty() {
                // Pure substring match (e.g. fork bomb)
                substrings.iter().any(|s| cmd_trimmed.contains(s))
            } else {
                // Must match a prefix first
                prefixes.iter().any(|prefix| {
                    if !cmd_trimmed.starts_with(prefix) {
                        return false;
                    }
                    if substrings.is_empty() {
                        return true;
                    }
                    substrings.iter().any(|s| cmd_trimmed.contains(s))
                })
            };

            if matched {
                return Some(id);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig {
            webui_host: "127.0.0.1".to_string(),
            webui_port: 2233,
            mcp_transport: "sse".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 8080,
            max_concurrency: 10,
            disable_tools: vec![
                "execute_command".to_string(),
                "process_list".to_string(),
            ],
            working_dir: PathBuf::from("."),
            log_level: "info".to_string(),
            disable_webui: false,
            allow_dangerous_commands: vec![],
            allowed_hosts: None,
            disable_allowed_hosts: false,
        };

        assert!(config.is_tool_disabled("execute_command"));
        assert!(config.is_tool_disabled("process_list"));
        assert!(!config.is_tool_disabled("file_read"));
    }
}
