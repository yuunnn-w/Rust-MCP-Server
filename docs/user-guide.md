# User Guide

## Table of Contents

- [Getting Started](#getting-started)
- [Using WebUI](#using-webui)
- [Tool Reference](#tool-reference)
- [Configuration](#configuration)
- [Security Features](#security-features)
- [Troubleshooting](#troubleshooting)

## Getting Started

### Installation

Download the latest release for your platform from [GitHub Releases](https://github.com/yuunnn-w/Rust-MCP-Server/releases).

Or build from source:
```bash
git clone https://github.com/yuunnn-w/Rust-MCP-Server.git
cd Rust-MCP-Server
./scripts/build-unix.sh  # Linux/macOS
# or
scripts\build-windows.bat  # Windows
```

### First Run

```bash
# Start with default settings
./rust-mcp-server

# Access WebUI
open http://127.0.0.1:2233  # macOS
# or
start http://127.0.0.1:2233  # Windows
```

### Connecting from MCP Client

Configure your MCP client to connect to:
- **HTTP Transport:** `http://127.0.0.1:3344`
- **SSE Transport:** `http://127.0.0.1:3344/sse`

Example configuration for Claude Desktop:
```json
{
  "mcpServers": {
    "rust-mcp": {
      "command": "./rust-mcp-server",
      "args": ["--mcp-transport", "http"]
    }
  }
}
```

## Using WebUI

### Dashboard Overview

The WebUI provides a comprehensive control panel:
- **Tool Grid**: View and manage all 18 tools
- **Status Bar**: Monitor concurrent calls and server status
- **Statistics Panel**: View tool usage charts (Chart.js)
- **Real-time Updates**: SSE-based live status updates

### Enabling/Disabling Tools

**Important:** By default, only `calculator`, `dir_list`, `file_read`, and `file_search` are enabled for security.

1. Open WebUI at `http://127.0.0.1:2233`
2. Find the tool card in the grid
3. Toggle the switch on the tool card
4. Changes take effect immediately (no restart needed)

### Viewing Tool Statistics

Click on "Details" button on any tool card to see:
- Total call count
- Recent calls (last 15 minutes)
- Usage chart (last 2 hours, 15-minute intervals)
- Recent call timestamps
- Tool description and usage

### Tool Status Indicators

- **Idle**: Tool is available for use
- **Calling...**: Tool is currently being called (stays for 5 seconds after completion)
- **Disabled**: Tool is turned off
- **Dangerous**: Marked with warning icon (requires careful use)

## Tool Reference

### File Operations

#### dir_list
List directory contents with tree structure.

**Parameters:**
- `path` (string): Directory path (default: current directory)
- `max_depth` (number, optional): Maximum recursion depth (default: 1, max: 1)

**Example:**
```json
{
  "path": "/project/src",
  "max_depth": 1
}
```

#### file_read
Read text file content with line range support.

**Parameters:**
- `path` (string): File path
- `start_line` (number, optional): Start line (0-indexed, default: 0)
- `end_line` (number, optional): End line (default: 100)

**Features:**
- 10KB character limit per read
- Automatic truncation with notification
- Returns total line count
- Provides hint for reading more content

**Example:**
```json
{
  "path": "config.json",
  "start_line": 0,
  "end_line": 50
}
```

#### file_write
Write content to file (dangerous).

**Parameters:**
- `path` (string): File path
- `content` (string): Content to write
- `mode` (string, optional): "new" | "append" | "overwrite" (default: "new")

**Example:**
```json
{
  "path": "output.txt",
  "content": "Hello, World!",
  "mode": "new"
}
```

#### file_search
Search for keywords in files or directories.

**Parameters:**
- `path` (string): File or directory path
- `keyword` (string): Search keyword
- `max_depth` (number, optional): Maximum recursion depth (default: 3)

**Features:**
- Recursive directory search
- Returns file paths and line numbers
- Skips binary files (UTF-8 check)
- Warns about deeper directories not searched

**Example:**
```json
{
  "path": "/project/src",
  "keyword": "TODO",
  "max_depth": 3
}
```

### System Tools

#### execute_command
Execute shell commands with security checks (dangerous).

**Parameters:**
- `command` (string): Command to execute
- `cwd` (string, optional): Working directory (default: current)
- `timeout` (number, optional): Timeout in seconds (default: 30, max: 300)
- `env` (object, optional): Environment variables as key-value pairs

**Security:**
- Dangerous commands require two-step confirmation
- Shell injection patterns detected
- Output limited to 100KB

**Example:**
```json
{
  "command": "ls -la",
  "cwd": "/home/user",
  "timeout": 30
}
```

#### process_list
List system processes.

**Returns:**
- Process ID, name, CPU usage, memory usage
- Sorted by CPU usage (descending)

#### system_info
Get system information.

**Returns:**
- OS name and version
- CPU count and architecture
- Memory information
- Hostname

### Utility Tools

#### calculator
Calculate mathematical expressions.

**Supports:**
- Basic operators: +, -, *, /, ^
- Functions: sqrt, sin, cos, tan, log, ln, abs
- Constants: pi, e
- Parentheses for precedence

**Example:**
```json
{
  "expression": "2 + 3 * 4"
}
```

#### http_request
Make HTTP requests.

**Parameters:**
- `url` (string): Target URL
- `method` (string): "GET" or "POST"
- `headers` (object, optional): HTTP headers
- `body` (string, optional): Request body

**Example:**
```json
{
  "url": "https://api.example.com/data",
  "method": "GET"
}
```

#### base64_encode / base64_decode
Encode/decode base64.

**Example:**
```json
{
  "input": "Hello, World!"
}
```

#### hash_compute
Compute hash of string or file.

**Parameters:**
- `input` (string): String to hash, or path with `@` prefix for file
- `algorithm` (string): "MD5", "SHA1", or "SHA256"

**Example:**
```json
{
  "input": "Hello, World!",
  "algorithm": "SHA256"
}
```

### Image Tools

#### image_read
Read image file and return base64 encoded data.

**Parameters:**
- `path` (string): Image file path

**Returns:**
- Base64 encoded image data
- Image format (png, jpeg, etc.)

## Configuration

### Command Line Options

```bash
./rust-mcp-server [OPTIONS]

Options:
      --webui-host <HOST>              WebUI listening address [default: 127.0.0.1]
      --webui-port <PORT>              WebUI listening port [default: 2233]
      --mcp-transport <TRANSPORT>      MCP transport: http or sse [default: http]
      --mcp-host <HOST>                MCP service listening address [default: 127.0.0.1]
      --mcp-port <PORT>                MCP service listening port [default: 3344]
      --max-concurrency <NUM>          Maximum concurrent tool calls [default: 10]
      --disable-tools <TOOLS>          Comma-separated list of tools to disable
      --working-dir <PATH>             Working directory for file operations [default: .]
      --log-level <LEVEL>              Log level: trace, debug, info, warn, error [default: info]
      --disable-webui                  Disable WebUI control panel
      --allow-dangerous-commands <IDS> Allowed dangerous command IDs (1-20)
  -h, --help                           Print help
  -V, --version                        Print version
```

### Environment Variables

All CLI options can be set via environment variables:

```bash
export MCP_WEBUI_PORT=8080
export MCP_MAX_CONCURRENCY=20
export MCP_LOG_LEVEL=debug
export MCP_DISABLE_TOOLS="execute_command,process_list"
./rust-mcp-server
```

### Configuration File

Create `.env` file in project root:

```
MCP_WEBUI_PORT=8080
MCP_MAX_CONCURRENCY=20
MCP_WORKING_DIR=/safe/path
MCP_DISABLE_TOOLS=file_write,execute_command
```

## Security Features

### Working Directory Restriction

All file operations are restricted to the configured working directory:

```bash
./rust-mcp-server --working-dir /var/mcp-safe
```

Path traversal attempts (`../`) are blocked.

### Dangerous Command Blacklist

The `execute_command` tool blocks 20 dangerous command patterns by default:

| ID | Command | Description |
|----|---------|-------------|
| 1 | rm | Delete files (Linux) |
| 2 | del | Delete files (Windows) |
| 3 | format | Format disk |
| 4 | mkfs | Create filesystem |
| 5 | dd | Disk copy |
| 6 | :(){:|:&};: | Fork bomb |
| 7 | eval | Code execution |
| 8 | exec | Process replacement |
| 9 | system | System call |
| 10 | shred | Secure delete |
| 11 | rd /s | Delete directory tree (Windows) |
| 13 | diskpart | Disk partition (Windows) |
| 14 | reg | Registry operations (Windows) |
| 15 | net | Network/account management (Windows) |
| 16 | sc | Service control (Windows) |
| 17 | schtasks | Scheduled tasks (Windows) |
| 18 | powercfg | Power configuration (Windows) |
| 19 | bcdedit | Boot configuration (Windows) |
| 20 | wevtutil | Event logs (Windows) |

**Allow Specific Commands:**
```bash
./rust-mcp-server --allow-dangerous-commands 1,3
```

### Two-Step Confirmation

When a dangerous command or injection pattern is detected:

1. First call returns a security warning
2. Command is stored in pending list (5-minute timeout)
3. Second identical call executes the command
4. User must explicitly confirm with AI assistant

### Injection Detection

The following characters trigger confirmation:
```
;  |  &  `  $  (  )  <  >
```

Characters inside quoted strings are excluded from detection.

## Troubleshooting

### Server Won't Start

**Check port availability:**
```bash
# Linux/macOS
lsof -i :2233
lsof -i :3344

# Windows
netstat -ano | findstr :2233
netstat -ano | findstr :3344
```

**Change ports:**
```bash
./rust-mcp-server --webui-port 8080 --mcp-port 9000
```

### Tools Not Working

**Check if tool is enabled:**
- Open WebUI and verify tool toggle is ON
- Or check via API: `GET http://127.0.0.1:2233/api/tools`

**Check working directory:**
```bash
./rust-mcp-server --working-dir /correct/path
```

### MCP Client Can't Connect

**Verify transport mode:**
```bash
# Check current transport
curl http://127.0.0.1:2233/api/config
```

**Test MCP endpoint:**
```bash
# For HTTP transport
curl -X POST http://127.0.0.1:3344 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

### Performance Issues

**Increase concurrency:**
```bash
./rust-mcp-server --max-concurrency 50
```

**Monitor resource usage:**
```bash
# Linux/macOS
top -p $(pgrep rust-mcp-server)

# Windows
tasklist | findstr rust-mcp-server
```

## Getting Help

- [GitHub Issues](https://github.com/yuunnn-w/Rust-MCP-Server/issues)
- [GitHub Discussions](https://github.com/yuunnn-w/Rust-MCP-Server/discussions)

---

中文版本请查看 [user-guide-zh.md](user-guide-zh.md)
