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
- **21 Built-in Tools**: File operations, office document support (.docx, .pptx, .xlsx, .pdf, .ipynb), PDF page-to-image rendering via pdfium-render (pre-bundled PDFium library, auto-downloaded on build), HTTP requests, git operations, Python execution, clipboard, archive, diff, note storage, task management, web search, notebook editing, command monitoring, and more
- **Tool Presets**: 6 built-in presets (minimal, coding, data_analysis, system_admin, research, full_power) for one-click tool configuration
- **System Prompt**: Custom instructions appended to MCP `initialize` response via `--system-prompt` or WebUI
- **WebUI Control Panel**: Cyberpunk AI Command Center theme with glassmorphism HUD, animated background, terminal log stream, and 3D card tilt effects
- **Real-time Updates**: SSE-based live status updates in WebUI
- **System Metrics**: Real-time CPU, memory, and load monitoring via HUD and `/api/system-metrics` endpoint
- **Multi-Transport Support**: HTTP (default, JSON response) and SSE (stream response) transports
- **Concurrency Control**: Configurable max concurrent tool calls
- **Internationalization**: Support for English and Chinese

### Security Features
- **Working Directory Restriction**: Write operations restricted to configured directory; read-only tools can access any path
- **Dangerous Command Blacklist**: 20 configurable dangerous command patterns
- **Command Injection Detection**: Automatic detection of suspicious characters
- **Two-Step Confirmation**: Dangerous commands require user confirmation
- **Audit Logging**: All command executions are logged

### Available Tools

#### File Operations (Safe / Enabled by Default)
The following read-only tools are **not** restricted to the working directory:

| Tool | Description | Dangerous |
|------|-------------|-----------|
| `Glob` | List directory contents with enhanced filtering (max depth 10). Supports multi-pattern glob/regex matching, exclude patterns, file size/time filters. | No |
| `Read` | Read files with mode system: `auto`/`text`/`media` for generic files, `doc_text`/`doc_with_images`/`doc_images` for DOC/DOCX, `ppt_text`/`ppt_images` for PPT/PPTX (with native Rust fallback when LibreOffice unavailable), `pdf_text`/`pdf_images` for PDF. Image modes return base64-encoded image content for vision models (e.g., llama.cpp). Supports image_dpi, image_format. Recommendation: use FileStat first. | No |
| `Grep` | Search pattern in files with enhanced filtering. Regex, whole-word, multiline, office document search. | No |
| `FileStat` | Get metadata for files/directories. mode="exist" for lightweight existence check. Full mode returns `document_stats` for office files (DOCX/PPTX/PDF/XLSX) with page/slide/sheet count, embedded image count, and text char count. | No |

#### File Operations (Dangerous / Disabled by Default)
The following write operations are **restricted** to the working directory:

| Tool | Description | Security Check |
|------|-------------|----------------|
| `Edit` | Multi-mode editing: string_replace, line_replace, insert, delete, patch. Complex office modes: office_insert, office_replace, office_delete, office_insert_image, office_format, office_insert_table. PDF modes: pdf_delete_page, pdf_insert_image, pdf_insert_text, pdf_replace_text. Supports .doc/.docx/.ppt/.pptx/.xls/.xlsx. | Working directory check |
| `Write` | Write content to files. Supports creating .docx (office_markdown), .xlsx (office_csv), .pdf (office_markdown), .ipynb files. | Working directory check |
| `FileOps` | Copy, move, delete, or rename files. dry_run preview, conflict_resolution. | Working directory check |

#### System & Network Tools
| Tool | Description | Default Status |
|------|-------------|----------------|
| `Bash` | Execute shell commands with working_dir, stdin, async_mode. Use Monitor for async. | Disabled |
| `SystemInfo` | Get system information with sections parameter (includes processes). | Disabled |
| `Git` | Run git commands (status, diff, log, branch, show) with path and max_count. | Enabled |
| `ExecutePython` | Execute Python code. All standard library modules available. Filesystem toggle. | Enabled |

#### Utility Tools
| Tool | Description |
|------|-------------|
| `Clipboard` | Read/write system clipboard (text and images) |
| `Archive` | Create, extract, list, append ZIP archives with deflate/zstd compression and AES-256 password encryption |
| `Diff` | Compare text, files, or directories. ignore_blank_lines, multiple output formats |
| `NoteStorage` | AI short-term memory scratchpad with export/import (auto-clears after 30min) |

#### Task, Web & Interaction Tools
| Tool | Description | Default Status |
|------|-------------|----------------|
| `Task` | Unified task management (create/list/get/update/delete via operation parameter) | Disabled |
| `WebSearch` | Search the web with region/language filters | Enabled |
| `WebFetch` | Fetch URL content with extract_mode (text/html/markdown) | Enabled |
| `AskUser` | Prompt user for input or confirmation with timeout and default_value | Enabled |

#### Office & Monitoring Tools
| Tool | Description | Default Status |
|------|-------------|----------------|
| `NotebookEdit` | Read, write, and edit Jupyter .ipynb notebook files | Enabled |
| `Monitor` | Monitor long-running Bash commands (stream, wait, signal) | Enabled |

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

### Asset Dependencies

The project includes several pre-bundled assets and relies on `build.rs` to auto-download others at build time. Below is a complete reference.

#### Bundled Assets (committed to git)

| Path | Purpose | Source |
|------|---------|--------|
| `assets/vc-ltl/x64/*.lib` | VC-LTL5 CRT replacement for Win7 x64 | [VC-LTL5 v5.3.1](https://github.com/Chuyu-Team/VC-LTL5/releases) |
| `assets/vc-ltl/x86/*.lib` | VC-LTL5 CRT replacement for Win7 x86 | Same |
| `assets/yy-thunks/YY_Thunks_for_Win7.obj` | Win8+ API stubs for Win7 x64 | [YY-Thunks v1.2.1](https://github.com/Chuyu-Team/YY-Thunks/releases) |
| `assets/yy-thunks/YY_Thunks_for_Win7_x86.obj` | Win8+ API stubs for Win7 x86 | Same |
| `assets/icon.ico`, `assets/icon.png` | Application icons | — |

#### Auto-Downloaded Assets (downloaded & generated by build.rs)

| File | Purpose | Automatic? |
|------|---------|------------|
| `assets/pdfium/pdfium.dll` / `.so` / `.dylib` | PDFium native library for PDF page rendering | Downloaded from [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases) |
| `assets/pdfium/pdfium-*.tgz` | PDFium download archive | Auto-downloaded |
| `assets/pdfium/pdfium.*.zst` | Zstd-compressed PDFium (embedded in binary at compile time) | Auto-generated from the library file |

> **Note**: If auto-download fails (e.g., no network, GitHub rate limit), `build.rs` prints clear instructions. You can also manually place the PDFium library file or its `.tgz` archive in `assets/pdfium/`.

#### Windows 7 Compatibility Assets (Required for Win7 Target Only)

Only needed when building with `--target x86_64-win7-windows-msvc`:

| Path | Download |
|------|----------|
| `assets/vc-ltl/{x64,x86}/*.lib` | [VC-LTL5 Binary v5.3.1](https://github.com/Chuyu-Team/VC-LTL5/releases/download/v5.3.1/VC-LTL5-Binary-v5.3.1.7z) — extract `TargetPlatform/6.0.6000.0/lib/{x64,Win32}/` |
| `assets/yy-thunks/YY_Thunks_for_Win7.obj` | [YY-Thunks-Objs.zip v1.2.1](https://github.com/Chuyu-Team/YY-Thunks/releases/download/v1.2.1/YY-Thunks-Objs.zip) — extract `objs/x64/YY_Thunks_for_Win7.obj` |
| `assets/yy-thunks/YY_Thunks_for_Win7_x86.obj` | Same archive — extract `objs/x86/YY_Thunks_for_Win7.obj` |

#### Windows 7 Compatibility

The server can run **natively** on Windows 7 without third-party compatibility layers (e.g., VxKex). This is achieved through two embedded compatibility layers:

- **VC-LTL5 v5.3.1** (`assets/vc-ltl/`) — replaces the UCRT/VCRUNTIME CRT with `msvcrt.dll`, eliminating `api-ms-win-crt-*` and `VCRUNTIME140.dll` imports that don't exist on Windows 7
- **YY-Thunks v1.2.1** (`assets/yy-thunks/`) — provides runtime stubs for Windows 8+ APIs (e.g., `GetSystemTimePreciseAsFileTime`, `WaitOnAddress`, `ProcessPrng`), falling back to older equivalents

Both are injected at link time via `build.rs`. The project is fully self-contained — no external downloads required.

To build for Windows 7:

```bash
rustup update nightly
cargo +nightly build -Z build-std=std,panic_abort --target x86_64-win7-windows-msvc --release
```

> **Note**: On Windows 7, the `system_info` tool automatically returns limited information (CPU, memory, and OS basics) and skips disk, network, and hardware temperature enumeration to avoid compatibility issues with the `sysinfo` crate.

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
| `--preset` | `MCP_PRESET` | `minimal` | Startup tool preset: minimal/coding/data_analysis/system_admin/research/full_power/none |
| `--system-prompt` | `MCP_SYSTEM_PROMPT` | - | Custom system prompt appended to MCP instructions |
| `--disable-tools` | `MCP_DISABLE_TOOLS` | See below | Tools to disable on top of preset |
| `--allow-dangerous-commands` | `MCP_ALLOW_DANGEROUS_COMMANDS` | - | Allow dangerous command IDs |
| `--log-level` | `MCP_LOG_LEVEL` | `info` | Log level: trace/debug/info/warn/error |
| `--disable-webui` | - | - | Disable WebUI panel |
| `--allowed-hosts` | `MCP_ALLOWED_HOSTS` | - | Custom allowed Host headers (comma-separated) |
| `--disable-allowed-hosts` | `MCP_DISABLE_ALLOWED_HOSTS` | - | Disable DNS rebinding protection (not recommended for public) |

**Tool Presets:**
The server starts with the `minimal` preset by default. Use `--preset <name>` to choose a different preset, or `--preset none` to skip auto-applying.
- **minimal** (9 tools): Safe read-only tools + sandboxed Python
- **coding** (20 tools): Development-focused, includes file editing, task management, and command execution
- **data_analysis** (15 tools): Data analysis, includes Python, diff, archive, and web tools
- **system_admin** (20 tools): System administration, includes system info, processes, commands, and file operations
- **research** (10 tools): Research and documentation, includes web search, web fetch, and file reading
- **full_power** (21 tools): All tools enabled

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

The `Bash` tool implements multiple security layers:

1. **Working Directory Restriction**: Commands can only operate within the configured working directory
2. **Dangerous Command Detection**: Blocks known dangerous commands (see list above)
3. **Injection Pattern Detection**: Detects shell metacharacters (`;`, `|`, `&`, `` ` ``, `$`, etc.)
4. **Two-Step Confirmation**: Suspicious commands require user confirmation via repeated call
5. **Audit Logging**: All commands are logged with timestamp and result

### File Operation Security

Write file operations are restricted to the configured working directory. Read-only tools (Glob, Read, Grep, FileStat, Git) can access any path on the filesystem:
- Path traversal attacks (`../etc/passwd`) are blocked
- Symbolic link escaping is prevented
- Absolute paths outside working directory are rejected

### Example Security Flow

```
User: Bash("rm -rf /")
Server: "Security Warning: Dangerous command 'rm (delete files)' detected.
        This command may cause damage to the system or data.
        Please confirm with the user whether to execute this command.
        If the user agrees, please call the Bash tool again."

User: Bash("rm -rf /")  [Second call within 5 minutes]
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
│   ├── utils/               # Utility functions (file, image, system metrics, office conversion)
│   │   ├── office_converter.rs  # Office document conversion (docx-rs, lopdf, calamine, LibreOffice)
│   │   ├── office_utils.rs      # Office document utility functions
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
