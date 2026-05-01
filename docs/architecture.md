# Architecture Overview

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Rust MCP Server v0.2.0                       │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │   WebUI (Axum)  │  │  MCP Service    │  │   Tool Registry     │  │
│  │                 │  │   (rmcp)        │  │                     │  │
│  │  ┌───────────┐  │  │                 │  │  ┌───────────────┐  │  │
│  │  │ Static    │  │  │  ┌──────────┐   │  │  │ dir_list      │  │  │
│  │  │ Files     │  │  │  │ HTTP/SSE │   │  │  │ file_read     │  │  │
│  │  └───────────┘  │  │  │ Transport│   │  │  │ file_write    │  │  │
│  │  ┌───────────┐  │  │  └──────────┘   │  │  │ execute_cmd   │  │  │
│  │  │ execute_python│  │  │
│  │  │ REST API  │  │  │                 │  │  │ calculator    │  │  │
│  │  │ /api/*    │  │  │                 │  │  │ ... (21 tools)│  │  │
│  │  └───────────┘  │  │                 │  │  └───────────────┘  │  │
│  │  ┌───────────┐  │  │                 │  │                     │  │
│  │  │ SSE       │  │  │                 │  │                     │  │
│  │  │ /events   │  │  │                 │  │                     │  │
│  │  └───────────┘  │  │                 │  │                     │  │
│  │  ┌───────────┐  │  │                 │  │                     │  │
│  │  │ System    │  │  │                 │  │                     │  │
│  │  │ Metrics   │  │  │                 │  │                     │  │
│  │  └───────────┘  │  │                 │  │                     │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │  ServerState    │  │  Configuration  │  │  Security Layer     │  │
│  │                 │  │                 │  │                     │  │
│  │  - Tool status  │  │  - CLI args     │  │  - Path validation  │  │
│  │  - Call stats   │  │  - Env vars     │  │  - Dangerous cmd    │  │
│  │  - Concurrent   │  │  - Defaults     │  │  - Injection check  │  │
│  │  - Pending cmds │  │  - Working dir  │  │  - Audit log        │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Component Details

### WebUI (Axum Web Server)

The WebUI provides a modern control panel for managing the MCP server.

**Features:**
- **Static Files**: HTML, CSS, JS for the control panel (embedded at compile time)
- **REST API**: Endpoints for tool management and statistics
- **SSE Endpoint**: Real-time status updates (`/api/events`)

**API Endpoints:**
- `GET /api/tools` - List all tools with status
- `GET /api/tool/{name}/stats` - Get tool statistics
- `GET /api/tool/{name}/detail` - Get tool details
- `POST /api/tool/{name}/enable` - Enable/disable tool
- `GET /api/server-status` - Server runtime status
- `GET /api/system-metrics` - Get real-time CPU, memory, and load metrics
- `GET /api/version` - Get server version information
- `GET /api/config` - Get configuration
- `PUT /api/config` - Update configuration
- `POST /api/mcp/start` - Start MCP service
- `POST /api/mcp/stop` - Stop MCP service
- `POST /api/mcp/restart` - Restart MCP service
- `GET /api/python-fs-access` - Get `execute_python` filesystem access status
- `POST /api/python-fs-access` - Toggle `execute_python` filesystem access

**Default Bind Address**: `127.0.0.1:2233`

### MCP Service (rmcp)

Implements the Model Context Protocol using the `rmcp` crate.

**Transport Modes:**
- **HTTP**: JSON response mode (default)
- **SSE**: Server-Sent Events streaming mode

**Default Bind Address**: `127.0.0.1:3344`

**Protocol Support:**
- JSON-RPC 2.0
- MCP Protocol version 2024-11-05
- Tool discovery and invocation

### Tool Registry

21 built-in tools organized by category:

#### File Operations (8 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `dir_list` | List directory contents with tree structure (max depth 5). Returns char_count and line_count for UTF-8 text files | No | No |
| `file_read` | Read one or more text files concurrently with line range support | No | No |
| `file_search` | Search for keyword in file or directory (max depth 5) | No | No |
| `file_write` | Write content to one or more files concurrently (create/append/overwrite) | Yes | Yes |
| `file_ops` | Copy, move, delete, or rename one or more files concurrently | Yes | Yes |
| `file_edit` | Edit one or more files concurrently (string_replace, line_replace, insert, delete, patch). Can create new files | Yes | Yes |
| `file_stat` | Get metadata for one or more files/directories concurrently. Returns text file info for UTF-8 files | No | No |
| `path_exists` | Lightweight path existence check | No | No |

#### Query & Environment Tools (3 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `json_query` | Query a JSON file using JSON Pointer syntax | No | No |
| `git_ops` | Run git commands in a repository | No | No |
| `env_get` | Get the value of an environment variable | No | No |

#### System Tools (4 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `process_list` | List system processes | No | No |
| `system_info` | Get system information | No | No |
| `execute_command` | Execute shell command in specified directory | Yes | Yes |
| `execute_python` | Execute Python code in a sandboxed environment (filesystem access toggleable) | No | Yes (when fs access enabled) |

#### Utility Tools (4 tools)
| Tool | Description | Dangerous |
|------|-------------|-----------|
| `calculator` | Calculate mathematical expressions | No |
| `datetime` | Get current date and time in China format | No |
| `base64_codec` | Encode or decode base64 strings | No |
| `hash_compute` | Compute hash (MD5, SHA1, SHA256) of string or file | No |

#### Network & Image Tools (2 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `http_request` | Make HTTP GET or POST requests | No | No |
| `image_read` | Read image file and return MCP-standard ImageContent + TextContent metadata | No | No |

### Resources

The server exposes the working directory as an MCP resource (`file:///`). Clients can:
- **List resources**: Get the working directory entry with `uri`, `name`, and `description`
- **Read resources**: Fetch directory listings or file contents via `file:///{relative_path}`

Resource contents are returned as `TextResourceContents` with MIME type `text/plain`.

### Prompts

3 built-in prompts available for client use:

| Prompt | Description |
|--------|-------------|
| `system_diagnosis` | Guide for analyzing system information and identifying issues |
| `file_analysis` | Guide for analyzing code files and directory structures |
| `security_checklist` | Checklist to review before executing dangerous operations |

Prompts are retrieved via `prompts/get` and return a list of `PromptMessage` with `User` role.

### ServerState

Shared state across all components using `Arc` for thread-safe sharing.

**Components:**
- **Tool Registry**: HashMap of tool names to their metadata
- **Statistics**: Call counts, history, recent calls
- **Concurrency Control**: Semaphore for limiting concurrent calls
- **Pending Commands**: DashMap for storing commands awaiting confirmation

### Security Layer

Multi-layer security system:

1. **Path Validation**: Canonicalization and working directory check
2. **Dangerous Command Detection**: 20 configurable dangerous command patterns
3. **Injection Detection**: Shell metacharacter detection
4. **Two-Step Confirmation**: User confirmation for dangerous operations
5. **Audit Logging**: All command executions logged

## Data Flow

### Tool Execution Flow

```
1. Client sends tool call request (via MCP protocol)
2. MCP Handler receives and parses request
3. Check if tool is enabled in ServerState
4. Acquire concurrency permit from semaphore
5. Route to tool implementation
6. Execute security checks (path/command validation)
7. Execute tool logic with timeout protection
8. Record statistics and update state
9. Release concurrency permit
10. Return result to client
11. Trigger SSE update for WebUI
```

### WebUI Update Flow

```
1. Tool execution updates ServerState
2. State change triggers SSE broadcast
3. Connected WebUI clients receive update
4. UI components refresh automatically
```

### Command Execution Security Flow

```
Command Input
    ↓
Working Directory Validation
    ↓
Dangerous Command Check (Blacklist: 20 patterns)
    ↓
Injection Pattern Detection (metacharacters)
    ↓
Two-Step Confirmation (if needed)
    ↓
Execute with Audit Log + Timeout
    ↓
Output Truncation (100KB limit)
```

## Configuration System

**Sources** (in order of precedence):
1. Command line arguments (highest priority)
2. Environment variables
3. Default values (lowest priority)

**Key Configuration Options:**

| Option | CLI Flag | Env Variable | Default |
|--------|----------|--------------|---------|
| WebUI Host | `--webui-host` | `MCP_WEBUI_HOST` | `127.0.0.1` |
| WebUI Port | `--webui-port` | `MCP_WEBUI_PORT` | `2233` |
| MCP Transport | `--mcp-transport` | `MCP_TRANSPORT` | `http` |
| MCP Host | `--mcp-host` | `MCP_HOST` | `127.0.0.1` |
| MCP Port | `--mcp-port` | `MCP_PORT` | `3344` |
| Max Concurrency | `--max-concurrency` | `MCP_MAX_CONCURRENCY` | `10` |
| Working Directory | `--working-dir` | `MCP_WORKING_DIR` | `.` |
| Log Level | `--log-level` | `MCP_LOG_LEVEL` | `info` |
| Disabled Tools | `--disable-tools` | `MCP_DISABLE_TOOLS` | See below |
| Dangerous Commands | `--allow-dangerous-commands` | `MCP_ALLOW_DANGEROUS_COMMANDS` | (none) |

**Default Disabled Tools:**
```
file_write,file_ops,file_edit,http_request,datetime,
execute_command,process_list,base64_codec,hash_compute,system_info,execute_python
```

The following 11 tools are enabled by default: `calculator`, `dir_list`, `file_read`, `file_search`, `image_read`, `file_stat`, `path_exists`, `json_query`, `git_ops`, `env_get`, `execute_python`. Dangerous tools (`execute_command`, `file_write`, `file_ops`, `file_edit`) are disabled by default.

## Technology Stack

- **Runtime**: Tokio (async runtime)
- **Web Framework**: Axum
- **MCP Protocol**: rmcp crate
- **Serialization**: serde + serde_json
- **Logging**: tracing + tracing-subscriber
- **CLI Parsing**: clap
- **Concurrency**: tokio::sync (Semaphore, RwLock)
- **Collections**: dashmap (concurrent HashMap)
- **System Metrics**: sysinfo (CPU, memory, process monitoring)

---

中文版本请查看 [architecture-zh.md](architecture-zh.md)
