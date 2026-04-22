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

The WebUI provides a Cyberpunk AI Command Center:
- **HUD Header**: Live system metrics (CPU ring gauge, memory bar, total calls, concurrency)
- **Tool Grid**: View and manage all tools with 3D tilt hover effects and neon accent borders
- **Terminal Log Panel**: Bottom panel streaming SSE events with colored log levels (INFO/WARN/ERROR)
- **Real-time Updates**: SSE-based live status updates for tool calls, concurrency, and MCP service status
- **Animated Background**: Canvas-based perspective grid and floating particle network

### Enabling/Disabling Tools

**Important:** By default, 10 tools are enabled for security: `calculator`, `dir_list`, `file_read`, `file_search`, `image_read`, `file_stat`, `path_exists`, `json_query`, `git_ops`, and `env_get`.

1. Open WebUI at `http://127.0.0.1:2233`
2. Find the tool card in the grid
3. Toggle the switch on the tool card
4. Changes take effect immediately (no restart needed)

### Viewing Tool Statistics

Click on any tool card to see:
- Total call count
- Recent calls (last 15 minutes)
- Usage chart (last 2 hours, 5-minute intervals)
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
List directory contents with tree structure or flat list.

**Parameters:**
- `path` (string): Directory path (default: current directory)
- `max_depth` (number, optional): Maximum recursion depth (default: 2, max: 5)
- `include_hidden` (boolean, optional): Include hidden files (default: false)
- `pattern` (string, optional): Glob pattern to filter entries, e.g. `"*.rs"`
- `brief` (boolean, optional): Brief mode — only return name, path, is_dir (default: true)
- `sort_by` (string, optional): Sort by `"name"` (default), `"type"`, `"size"`, `"modified"`
- `flatten` (boolean, optional): Return flat list instead of nested tree (default: false)

**Example:**
```json
{
  "path": "/project/src",
  "max_depth": 2,
  "pattern": "*.rs",
  "brief": true,
  "sort_by": "name"
}
```

#### file_read
Read text file content with line range support.

**Parameters:**
- `path` (string): File path
- `start_line` (number, optional): Start line (0-indexed, default: 0)
- `end_line` (number, optional): End line (default: 500)
- `offset_chars` (number, optional): Character offset to start reading (alternative to start_line)
- `max_chars` (number, optional): Maximum characters to return (default: 15000)
- `line_numbers` (boolean, optional): Prefix each line with its line number (default: true)
- `highlight_line` (number, optional): Highlight a specific line with `>>>` marker (1-based)

**Features:**
- 15KB character limit per read (configurable via `max_chars`)
- Automatic truncation with precise continuation hints
- Returns total line count
- Line number prefixing for easy reference
- Highlight line for pinpointing search results

**Example:**
```json
{
  "path": "config.json",
  "start_line": 0,
  "end_line": 500,
  "line_numbers": true,
  "highlight_line": 42
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
- `file_pattern` (string, optional): Glob pattern to filter files, e.g. `"*.rs"`
- `use_regex` (boolean, optional): Use regex matching (default: false)
- `max_results` (number, optional): Maximum match results to return (default: 20)
- `context_lines` (number, optional): Context lines around each match (default: 3)
- `brief` (boolean, optional): Brief mode — only return file paths and line numbers (default: false)
- `output_format` (string, optional): `"detailed"` (default), `"compact"`, or `"location"`

**Features:**
- Recursive directory search (max depth: 5)
- Returns matching content snippets with surrounding context
- Regex and literal keyword support
- Skips binary files and blacklisted directories
- Warns about deeper directories not searched
- Compact mode returns only `file:line:matched_text`
- Location mode returns only `file:line` (minimal token usage)

**Example:**
```json
{
  "path": "/project/src",
  "keyword": "TODO",
  "file_pattern": "*.rs",
  "context_lines": 3,
  "max_results": 10,
  "output_format": "compact"
}
```

#### file_edit
Multi-mode file editing — string replacement, line-based operations, or unified diff patch.

**Parameters:**
- `path` (string): File path to edit
- `mode` (string, optional): `"string_replace"` (default), `"line_replace"`, `"insert"`, `"delete"`, `"patch"`

**string_replace mode:**
- `old_string` (string): String to find (exact match, can span multiple lines)
- `new_string` (string): Replacement string
- `occurrence` (number, optional): Which occurrence to replace — `1`=first (default), `2`=second, `0`=replace all

**line_replace / insert / delete mode:**
- `start_line` (number): Start line (1-based, inclusive)
- `end_line` (number): End line (1-based, inclusive). Not used for insert.
- `new_string` (string): Content for replacement or insertion

**patch mode:**
- `patch` (string): Unified diff patch string

**Features:**
- `string_replace`: Exact string matching, multi-line support
- `line_replace`: Replace lines by number — LLM does not need to output old content
- `insert`: Insert content before a specific line
- `delete`: Delete a range of lines
- `patch`: Apply standard unified diff for complex multi-location changes
- All modes return replacement summary with preview

**Examples:**
```json
// String replacement
{
  "path": "src/main.rs",
  "mode": "string_replace",
  "old_string": "fn main() {",
  "new_string": "fn main() -> Result<(), Box<dyn std::error::Error>> {",
  "occurrence": 1
}

// Line replacement (no old_string needed!)
{
  "path": "src/main.rs",
  "mode": "line_replace",
  "start_line": 10,
  "end_line": 15,
  "new_string": "    let x = 42;\n    println!(\"{}\", x);"
}

// Insert before line 5
{
  "path": "src/main.rs",
  "mode": "insert",
  "start_line": 5,
  "new_string": "use std::collections::HashMap;"
}

// Delete lines 20-25
{
  "path": "src/main.rs",
  "mode": "delete",
  "start_line": 20,
  "end_line": 25
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
- `shell` (string, optional): Shell interpreter — `"cmd"` (default Windows), `"powershell"`, `"pwsh"`, `"sh"` (default Unix), `"bash"`, `"zsh"`

**Security:**
- Dangerous commands require two-step confirmation
- Shell injection patterns detected
- Output limited to 100KB

**Example:**
```json
{
  "command": "ls -la",
  "cwd": "/home/user",
  "timeout": 30,
  "shell": "bash"
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
- `timeout` (number, optional): Timeout in seconds (default: 30)
- `extract_json_path` (string, optional): JSON Pointer path to extract from JSON response, e.g. `"/data/0/name"`
- `include_response_headers` (boolean, optional): Include response headers in output (default: false)
- `max_response_chars` (number, optional): Maximum response body characters (default: 15000)

**Example:**
```json
{
  "url": "https://api.example.com/data",
  "method": "GET",
  "extract_json_path": "/data/0/name",
  "max_response_chars": 5000
}
```

#### base64_codec
Encode or decode base64.

**Parameters:**
- `operation` (string): `"encode"` or `"decode"`
- `input` (string): String to encode, or base64 string to decode

**Example:**
```json
{
  "operation": "encode",
  "input": "Hello, World!"
}
```

#### hash_compute
Compute hash of string or file.

**Parameters:**
- `input` (string): String to hash, or path with `file:` prefix for file
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
Read image file and return MCP-standard image content or metadata.

**Parameters:**
- `path` (string): Image file path
- `mode` (string, optional): `"full"` (default) returns image data; `"metadata"` returns only dimensions and type

**Returns (full mode):**
- MCP `ImageContent` with raw base64 data and MIME type (enables vision-model encoding)
- Human-readable `TextContent` with filename, dimensions, file size, and format

**Returns (metadata mode):**
- JSON text with image format, dimensions, and size

### Development Tools

#### file_stat
Get file or directory metadata.

**Parameters:**
- `path` (string): File or directory path

**Returns:**
- `name`, `path`, `exists`
- `file_type`: `"file"`, `"directory"`, `"symlink"`, or `"unknown"`
- `size`: Size in bytes
- `size_human`: Human-readable size string
- `readable`, `writable`, `executable`: Permission booleans
- `modified`, `created`, `accessed`: Timestamp strings
- `is_symlink`: Whether the path is a symbolic link

**Example:**
```json
{
  "path": "src/main.rs"
}
```

#### path_exists
Lightweight path existence check.

**Parameters:**
- `path` (string): Path to check

**Returns:**
- `exists` (boolean)
- `path_type`: `"file"`, `"dir"`, `"symlink"`, or `"none"`

**Example:**
```json
{
  "path": "src/main.rs"
}
```

#### json_query
Query a JSON file directly using JSON Pointer syntax.

**Parameters:**
- `path` (string): JSON file path
- `query` (string): JSON Pointer path, e.g. `"/data/0/name"`
- `max_chars` (number, optional): Maximum characters to return (default: 15000)

**Returns:**
- `found` (boolean)
- `result`: The queried value (pretty-printed JSON)
- `result_type`: Type information (e.g. `"object{5}"`, `"array[3]"`, `"string"`)

**Example:**
```json
{
  "path": "config.json",
  "query": "/database/host"
}
```

#### git_ops
Run git commands in a repository.

**Parameters:**
- `action` (string): `"status"`, `"diff"`, `"log"`, `"branch"`, or `"show"`
- `repo_path` (string, optional): Repository path (default: working directory)
- `options` (array of strings, optional): Extra git arguments

**Example:**
```json
{
  "action": "log",
  "options": ["--oneline", "-n", "10"]
}
```

#### env_get
Get the value of an environment variable.

**Parameters:**
- `name` (string): Environment variable name

**Returns:**
- `name`, `value`, `is_set` (boolean)

**Example:**
```json
{
  "name": "PATH"
}
```

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
      --allowed-hosts <HOSTS>          Custom allowed Host headers for DNS rebinding protection (comma-separated)
      --disable-allowed-hosts          Disable allowed_hosts check (NOT recommended for public deployments)
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
export MCP_ALLOWED_HOSTS="192.168.1.100,example.com"
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

**403 Forbidden: Host header is not allowed**

This error occurs when the MCP server rejects the request due to DNS rebinding protection (rmcp v1.5.0+).

**If using `--mcp-host 0.0.0.0`:** The server auto-detects local network interface IPs. If auto-detection fails, use one of the following:

```bash
# Option 1: Explicitly specify allowed hosts
./rust-mcp-server --mcp-host 0.0.0.0 --allowed-hosts 192.168.1.100

# Option 2: Disable the check (NOT recommended for public deployments)
./rust-mcp-server --mcp-host 0.0.0.0 --disable-allowed-hosts
```

### Performance Issues

**Increase concurrency:**
```bash
./rust-mcp-server --max-concurrency 50
```

**Monitor resource usage:**
```bash
# Via WebUI HUD
Open http://127.0.0.1:2233 and check the header system metrics

# Via API
curl http://127.0.0.1:2233/api/system-metrics

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
