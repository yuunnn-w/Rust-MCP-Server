# Architecture Overview

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Rust MCP Server v0.1.0                       │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │   WebUI (Axum)  │  │  MCP Service    │  │   Tool Registry     │  │
│  │                 │  │   (rmcp)        │  │                     │  │
│  │  ┌───────────┐  │  │                 │  │  ┌───────────────┐  │  │
│  │  │ Static    │  │  │  ┌──────────┐   │  │  │ dir_list      │  │  │
│  │  │ Files     │  │  │  │ HTTP/SSE │   │  │  │ file_read     │  │  │
│  │  └───────────┘  │  │  │ Transport│   │  │  │ file_write    │  │  │
│  │  ┌───────────┐  │  │  └──────────┘   │  │  │ execute_cmd   │  │  │
│  │  │ REST API  │  │  │                 │  │  │ calculator    │  │  │
│  │  │ /api/*    │  │  │                 │  │  │ ... (18 tools)│  │  │
│  │  └───────────┘  │  │                 │  │  └───────────────┘  │  │
│  │  ┌───────────┐  │  │                 │  │                     │  │
│  │  │ SSE       │  │  │                 │  │                     │  │
│  │  │ /events   │  │  │                 │  │                     │  │
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
- `GET /api/config` - Get configuration
- `PUT /api/config` - Update configuration
- `POST /api/mcp/{start|stop|restart}` - MCP service control

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

18 built-in tools organized by category:

#### File Operations (6 tools)
| Tool | Description | Dangerous |
|------|-------------|-----------|
| `dir_list` | List directory contents with tree structure (max depth 3) | No |
| `file_read` | Read text file content with line range support | No |
| `file_search` | Search for keyword in file or directory (max depth 3) | No |
| `file_write` | Write content to file (create/append/overwrite) | Yes |
| `file_copy` | Copy a file to a new location | Yes |
| `file_move` | Move a file to a new location | Yes |
| `file_delete` | Delete a file | Yes |
| `file_rename` | Rename a file | Yes |

#### System Tools (3 tools)
| Tool | Description | Dangerous |
|------|-------------|-----------|
| `process_list` | List system processes | No |
| `system_info` | Get system information | No |
| `execute_command` | Execute shell command in specified directory | Yes |

#### Utility Tools (5 tools)
| Tool | Description | Dangerous |
|------|-------------|-----------|
| `calculator` | Calculate mathematical expressions | No |
| `datetime` | Get current date and time in China format | No |
| `base64_encode` | Encode string to base64 | No |
| `base64_decode` | Decode base64 to string | No |
| `hash_compute` | Compute hash (MD5, SHA1, SHA256) of string or file | No |

#### Network & Image Tools (2 tools)
| Tool | Description | Dangerous |
|------|-------------|-----------|
| `http_request` | Make HTTP GET or POST requests | No |
| `image_read` | Read image file and return base64 encoded data | No |

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
file_write,file_copy,file_move,file_delete,file_rename,
http_request,datetime,image_read,execute_command,process_list,
base64_encode,base64_decode,hash_compute,system_info
```

Only `calculator`, `dir_list`, `file_read`, `file_search` are enabled by default.

## Technology Stack

- **Runtime**: Tokio (async runtime)
- **Web Framework**: Axum
- **MCP Protocol**: rmcp crate
- **Serialization**: serde + serde_json
- **Logging**: tracing + tracing-subscriber
- **CLI Parsing**: clap
- **Concurrency**: tokio::sync (Semaphore, RwLock)
- **Collections**: dashmap (concurrent HashMap)

---

中文版本请查看 [architecture-zh.md](architecture-zh.md)
