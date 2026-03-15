# Rust MCP Server

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust" alt="Rust Version">
  <img src="https://img.shields.io/badge/MCP-Protocol-blue" alt="MCP Protocol">
  <img src="https://img.shields.io/badge/License-GPL%20v3.0-blue" alt="License">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey" alt="Platform">
</p>

<p align="center">
  <b>A high-performance Model Context Protocol (MCP) server with WebUI</b>
</p>

<p align="center">
  <a href="README-zh.md">中文</a> | 
  <a href="#features">Features</a> | 
  <a href="#quick-start">Quick Start</a> | 
  <a href="#documentation">Documentation</a> | 
  <a href="#security">Security</a>
</p>

---

## Overview

Rust MCP Server is a high-performance [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server implementation built with Rust. It provides a comprehensive set of tools for AI assistants to interact with the file system, execute commands, make HTTP requests, and more - all through a modern WebUI control panel.

### Demo Screenshots

<p align="center">
  <i>WebUI Control Panel - Manage tools, view statistics, monitor status in real-time</i>
</p>

## Features

### Core Features
- **18 Built-in Tools**: File operations, HTTP requests, calculations, system info, and more
- **WebUI Control Panel**: Modern web interface for managing tools and monitoring
- **Real-time Updates**: SSE-based live status updates in WebUI
- **Multi-Transport Support**: HTTP (default, JSON response) and SSE (stream response) transports
- **Concurrency Control**: Configurable max concurrent tool calls
- **Internationalization**: Support for English and Chinese

### Security Features
- **Working Directory Restriction**: Dangerous operations restricted to configured directory
- **Dangerous Command Blacklist**: 20 configurable dangerous command patterns
- **Command Injection Detection**: Automatic detection of suspicious characters
- **Two-Step Confirmation**: Dangerous commands require user confirmation
- **Audit Logging**: All command executions are logged

### Available Tools

#### File Operations (Safe)
| Tool | Description | Dangerous |
|------|-------------|-----------|
| `dir_list` | List directory contents with tree structure | No |
| `file_read` | Read text file content with line range support | No |
| `file_search` | Search for keywords in files/directories | No |
| `image_read` | Read image file and return base64 data | No |

#### File Operations (Dangerous - Disabled by Default)
| Tool | Description | Security Check |
|------|-------------|----------------|
| `file_write` | Write content to file | Working directory check |
| `file_copy` | Copy file to new location | Working directory check |
| `file_move` | Move file to new location | Working directory check |
| `file_delete` | Delete file | Working directory check |
| `file_rename` | Rename file | Working directory check |

#### System & Network Tools
| Tool | Description | Default Status |
|------|-------------|----------------|
| `execute_command` | Execute shell commands with safety checks | Disabled |
| `process_list` | List system processes | Disabled |
| `system_info` | Get system information | Disabled |
| `http_request` | Make HTTP GET/POST requests | Disabled |

#### Utility Tools
| Tool | Description |
|------|-------------|
| `calculator` | Calculate mathematical expressions |
| `datetime` | Get current date/time (China timezone) |
| `base64_encode` | Encode string to base64 |
| `base64_decode` | Decode base64 to string |
| `hash_compute` | Compute MD5/SHA1/SHA256 hashes |

## Quick Start

### Installation

#### Option 1: Download Pre-built Binary
Download the latest release from [GitHub Releases](https://github.com/yuunnn-w/Rust-MCP-Server/releases).

#### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/yuunnn-w/Rust-MCP-Server.git
cd Rust-MCP-Server

# Build using the provided script
# On Linux/macOS:
./scripts/build-unix.sh

# On Windows:
.\scripts\build-windows.bat

# Or build manually with cargo
cargo build --release
```

### Usage

```bash
# Start with default settings (HTTP transport + WebUI)
./rust-mcp-server

# Start with custom WebUI port
./rust-mcp-server --webui-port 8080

# Start with SSE transport
./rust-mcp-server --mcp-transport sse --mcp-port 9000

# Enable dangerous commands (by ID)
./rust-mcp-server --allow-dangerous-commands 1,2

# See all options
./rust-mcp-server --help
```

### Access WebUI

Once started, open your browser:
```
http://127.0.0.1:2233
```

## Configuration

### Command Line Options

| Option | Environment Variable | Default | Description |
|--------|---------------------|---------|-------------|
| `--webui-host` | `MCP_WEBUI_HOST` | `127.0.0.1` | WebUI listening address |
| `--webui-port` | `MCP_WEBUI_PORT` | `2233` | WebUI listening port |
| `--mcp-transport` | `MCP_TRANSPORT` | `http` | Transport: `http` or `sse` |
| `--mcp-host` | `MCP_HOST` | `127.0.0.1` | MCP service address |
| `--mcp-port` | `MCP_PORT` | `3344` | MCP service port |
| `--max-concurrency` | `MCP_MAX_CONCURRENCY` | `10` | Max concurrent calls |
| `--working-dir` | `MCP_WORKING_DIR` | `.` | Working directory for file ops |
| `--disable-tools` | `MCP_DISABLE_TOOLS` | See below | Tools to disable (15 tools disabled by default) |
| `--allow-dangerous-commands` | `MCP_ALLOW_DANGEROUS_COMMANDS` | - | Allow dangerous command IDs |
| `--log-level` | `MCP_LOG_LEVEL` | `info` | Log level: trace/debug/info/warn/error |
| `--disable-webui` | - | - | Disable WebUI panel |

**Default Tool Status:**
- **Enabled by default (4):** `calculator`, `dir_list`, `file_read`, `file_search`
- **Disabled by default (14):** `file_write`, `file_copy`, `file_move`, `file_delete`, `file_rename`, `http_request`, `datetime`, `image_read`, `execute_command`, `process_list`, `base64_encode`, `base64_decode`, `hash_compute`, `system_info`

### Dangerous Command IDs

The following commands are blocked by default and require explicit permission:

| ID | Linux Command | Windows Command |
|----|---------------|-----------------|
| 1 | `rm` (delete) | - |
| 2 | `del` (delete) | - |
| 3 | `format` | `format` |
| 4 | `mkfs` | - |
| 5 | `dd` | - |
| 6 | Fork bomb (`:(){:|:&};:`) | - |
| 7 | `eval` | - |
| 8 | `exec` | - |
| 9 | `system` | - |
| 10 | `shred` | - |
| 11 | - | `rd /s` (delete tree) |
| 12 | - | `format` (Windows) |
| 13 | - | `diskpart` |
| 14 | - | `reg` (registry) |
| 15 | - | `net` (network) |
| 16 | - | `sc` (services) |
| 17 | - | `schtasks` |
| 18 | - | `powercfg` |
| 19 | - | `bcdedit` |
| 20 | - | `wevtutil` |

Enable with: `--allow-dangerous-commands 1,3,5`

## Security

### Command Execution Security

The `execute_command` tool implements multiple security layers:

1. **Working Directory Restriction**: Commands can only operate within the configured working directory
2. **Dangerous Command Detection**: Blocks known dangerous commands (see list above)
3. **Injection Pattern Detection**: Detects shell metacharacters (`;`, `|`, `&`, `` ` ``, `$`, etc.)
4. **Two-Step Confirmation**: Suspicious commands require user confirmation via repeated call
5. **Audit Logging**: All commands are logged with timestamp and result

### File Operation Security

All file operations are restricted to the configured working directory:
- Path traversal attacks (`../etc/passwd`) are blocked
- Symbolic link escaping is prevented
- Absolute paths outside working directory are rejected

### Example Security Flow

```
User: execute_command("rm -rf /")
Server: "Security Warning: Dangerous command 'rm (delete files)' detected.
        This command may cause damage to the system or data.
        Please confirm with the user whether to execute this command.
        If the user agrees, please call the execute_command tool again."

User: execute_command("rm -rf /")  [Second call within 5 minutes]
Server: [Command executed after confirmation]
```

## Documentation

- [API Documentation](docs/api.md) - REST API reference
- [Architecture](docs/architecture.md) - System architecture and design
- [User Guide](docs/user-guide.md) - Detailed user guide
- [Security Guide](docs/security.md) - Security features and best practices
- [Contributing](CONTRIBUTING.md) - Contribution guidelines

## Development

### Project Structure

```
Rust-MCP-Server/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration management
│   ├── mcp/
│   │   ├── handler.rs       # MCP protocol handler
│   │   ├── state.rs         # Shared server state
│   │   └── tools/           # Tool implementations
│   ├── utils/               # Utility functions
│   └── web/                 # WebUI and HTTP API
├── scripts/                 # Build scripts
├── docs/                    # Documentation
├── README.md               # This file
├── README-zh.md            # Chinese README
└── LICENSE                 # GPL v3.0 License
```

### Testing with llama.cpp

You can test the MCP server using [llama.cpp](https://github.com/ggerganov/llama.cpp)'s `llama-server` which supports MCP via WebUI configuration.

```bash
# 1. Start the MCP server
./rust-mcp-server --mcp-transport http --mcp-port 8080

# 2. Start llama-server
llama-server -m your-model.gguf

# 3. Open llama.cpp WebUI, go to Settings and configure MCP server URL
#    (e.g., http://localhost:8080)

# 4. Enable MCP tools and start chatting
```

> **Note:** llama.cpp provides experimental MCP CORS proxy support via `--webui-mcp-proxy` flag. See llama.cpp documentation for details and security considerations.

### Building Documentation

```bash
cargo doc --no-deps --open
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and changes.

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) before submitting PRs.

## License

This project is licensed under the GPL v3.0 License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Model Context Protocol](https://modelcontextprotocol.io/) - The protocol specification
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - Official Rust MCP SDK
- [Axum](https://github.com/tokio-rs/axum) - Web framework for Rust
- [Tokio](https://tokio.rs/) - Async runtime for Rust

## Support

- GitHub Issues: [Report bugs or request features](https://github.com/yuunnn-w/Rust-MCP-Server/issues)
- GitHub Discussions: [Ask questions or share ideas](https://github.com/yuunnn-w/Rust-MCP-Server/discussions)

---

<p align="center">
  Made with Rust <br>
  <a href="https://github.com/yuunnn-w">@yuunnn-w</a>
</p>
