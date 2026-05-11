# API Documentation

## REST API Endpoints

Base URL: `http://127.0.0.1:2233`

### Tool Management

#### GET /api/tools
Get all tools with their current status.

**Response:**
```json
{
  "tools": [
    {
      "name": "Read",
      "description": "Read file with mode system: auto/text/media for generic files, doc_text/doc_with_images/doc_images for DOC/DOCX, ppt_text/ppt_images for PPT/PPTX, pdf_text/pdf_images for PDF. Image modes return base64 image content for vision models. Supports image_dpi, image_format. Not restricted to working directory.",
      "enabled": true,
      "call_count": 42,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    },
    {
      "name": "Bash",
      "description": "Execute shell command with optional working_dir, stdin, max_output_chars, and async_mode. Use Monitor tool for async commands.",
      "enabled": false,
      "call_count": 5,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": true
    },
    {
      "name": "Clipboard",
      "description": "Read or write system clipboard content. Supports read_text, write_text, read_image, and clear operations.",
      "enabled": true,
      "call_count": 0,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    },
    {
      "name": "Archive",
      "description": "Create, extract, list, or append ZIP archives with AES-256 password encryption",
      "enabled": false,
      "call_count": 0,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": true
    },
    {
      "name": "Diff",
      "description": "Compare text, files, or directories with multiple output formats",
      "enabled": true,
      "call_count": 0,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    },
    {
      "name": "NoteStorage",
      "description": "AI assistant's short-term memory scratchpad with CRUD, search, export/import JSON. Notes auto-expire after 30 minutes of inactivity.",
      "enabled": true,
      "call_count": 0,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    }
  ]
}
```

**Fields:**
- `name`: Tool identifier
- `description`: Human-readable description
- `enabled`: Whether the tool is currently enabled
- `call_count`: Total number of calls
- `is_calling`: Whether tool is currently being called
- `is_busy`: Whether concurrency limit is reached
- `is_dangerous`: Whether tool performs dangerous operations

#### GET /api/tool/{name}/stats
Get detailed statistics for a specific tool.

**Response:**
```json
{
  "name": "Read",
  "total_calls": 42,
  "recent_calls_15min": 5,
  "stats_history": [0, 1, 2, 3, 2, 1, 0, 0, 0, 0],
  "recent_call_times": ["2024-03-15 12:00:00", "2024-03-15 11:55:00"]
}
```

**Fields:**
- `name`: Tool name
- `total_calls`: Total number of calls since startup
- `recent_calls_15min`: Calls in last 15 minutes
- `stats_history`: Array of call counts per 5-minute interval (last 2 hours)
- `recent_call_times`: Timestamps of recent calls

#### GET /api/tool/{name}/detail
Get detailed information about a tool.

**Response:**
```json
{
  "name": "Read",
  "description": "Read text file content with line range support",
  "usage": "Usage: Provide a 'path' parameter with optional 'start_line' and 'end_line'...",
  "enabled": true,
  "is_dangerous": false
}
```

#### POST /api/tool/{name}/enable
Enable or disable a tool.

**Request:**
```json
{
  "enabled": false
}
```

**Response:**
```json
{
  "success": true,
  "tool": "Read",
  "enabled": false
}
```

#### POST /api/tools/batch-enable
Enable or disable multiple tools at once.

**Request:**
```json
{
  "tools": ["Read", "Write", "Bash"],
  "enabled": true
}
```

**Response:**
```json
{
  "success": true,
  "enabled": true,
  "changed": ["Read", "Write", "Bash"],
  "failed": []
}
```

### Tool Presets

#### GET /api/tool-presets
Get all available tool presets.

**Response:**
```json
[
  {
    "name": "minimal",
    "description": "Minimal safe mode: read-only, computation, and sandboxed Python tools only",
    "tool_count": 9
  },
  {
    "name": "coding",
    "description": "Coding & development: full file operations, git, commands, Python with FS access, HTTP, clipboard, archive, task management",
    "tool_count": 20
  },
  {
    "name": "data_analysis",
    "description": "Data analysis: Python with FS access, web tools, HTTP, file reading/writing, Diff, Archive, NotebookEdit",
    "tool_count": 15
  },
  {
    "name": "system_admin",
    "description": "System administration: system info, processes, commands, Python with FS access, file operations, archive, Monitor",
    "tool_count": 20
  },
  {
    "name": "research",
    "description": "Research & documentation: web search, content fetching, file reading, notes, task tracking, user elicitation, NotebookEdit",
    "tool_count": 10
  },
  {
    "name": "full_power",
    "description": "Full power: all 21 tools enabled",
    "tool_count": 21
  }
]
```

#### GET /api/tool-presets/current
Get the currently active preset name.

**Response:**
```json
{
  "success": true,
  "preset": "coding"
}
```

#### POST /api/tool-presets/apply/{name}
Apply a tool preset. This atomically enables/disables tools according to the preset configuration.

**Response:**
```json
{
  "success": true,
  "preset": "coding"
}
```

**Available presets:**
- `minimal`: Safe read-only tools + sandboxed Python (9 tools, `ExecutePython` fs=false)
- `coding`: Development-focused tools including file editing, task management, and command execution (20 tools, `ExecutePython` fs=true)
- `data_analysis`: Data analysis tools including Python, Diff, Archive, and web tools (15 tools, `ExecutePython` fs=true)
- `system_admin`: System administration tools including system info, processes, commands, and file operations (20 tools, `ExecutePython` fs=true)
- `research`: Research & documentation tools including web search, web fetch, and file reading (10 tools, `ExecutePython` fs=false)
- `full_power`: All 21 tools enabled (`ExecutePython` fs=true)

### Server Status

#### GET /api/status
Alias for `/api/tools`. Returns all tools with status.

#### GET /api/server-status
Get server runtime status.

**Response:**
```json
{
  "current_calls": 2,
  "max_concurrency": 10,
  "mcp_running": true
}
```

**Fields:**
- `current_calls`: Number of currently executing tool calls
- `max_concurrency`: Maximum allowed concurrent calls
- `mcp_running`: Whether MCP service is running

### Configuration

#### GET /api/config
Get current server configuration.

**Response:**
```json
{
  "webui_host": "127.0.0.1",
  "webui_port": 2233,
  "mcp_transport": "http",
  "mcp_host": "127.0.0.1",
  "mcp_port": 3344,
  "max_concurrency": 10,
  "working_dir": ".",
  "log_level": "info",
  "system_prompt": null
}
```

#### PUT /api/config
Update configuration (limited options).

**Request:**
```json
{
  "max_concurrency": 20
}
```

**Response:**
```json
{
  "success": true,
  "message": "Configuration updated.",
  "changes": ["max_concurrency: 20"],
  "restart_required": false
}
```

**Updatable fields:**
- `webui_host`
- `webui_port`
- `mcp_transport` (`"http"` or `"sse"`)
- `mcp_host`
- `mcp_port`
- `max_concurrency` (range: 1-1000)
- `working_dir`
- `log_level`
- `system_prompt`
- `log_level` (`"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`)

**Note:** Changes to `mcp_transport`, `mcp_host`, `mcp_port`, `webui_host`, `webui_port`, `log_level`, or `working_dir` require a server restart to take full effect. The response will include `restart_required: true` when this is the case.

### System Metrics

#### GET /api/system-metrics
Get real-time system resource usage.

**Response:**
```json
{
  "cpu_percent": 12.5,
  "memory_total": 17179869184,
  "memory_used": 8589934592,
  "memory_percent": 50.0,
  "cpu_cores": 8,
  "uptime_seconds": 3600,
  "load_average": [0.5, 0.3, 0.2],
  "process_count": 245
}
```

**Fields:**
- `cpu_percent`: Global CPU usage percentage (0-100)
- `memory_total`: Total physical memory in bytes
- `memory_used`: Used physical memory in bytes
- `memory_percent`: Memory usage percentage (0-100)
- `cpu_cores`: Number of logical CPU cores
- `uptime_seconds`: System uptime in seconds
- `load_average`: 1min, 5min, 15min average load (may be zero on Windows)
- `process_count`: Total number of running processes

### MCP Service Control

#### POST /api/mcp/start
Toggle the `mcp_running` state flag to `true`. This does not actually start a new MCP process; a full restart requires an external process manager.

**Response:**
```json
{
  "success": true,
  "message": "MCP service status set to running. Note: full restart requires process manager."
}
```

#### POST /api/mcp/stop
Toggle the `mcp_running` state flag to `false`. This does not actually stop the underlying MCP process; a full restart requires an external process manager.

**Response:**
```json
{
  "success": true,
  "message": "MCP service status set to stopped. Note: full shutdown requires process manager."
}
```

#### POST /api/mcp/restart
Toggle the `mcp_running` state flag (off then on). This does not actually restart the underlying MCP process; a full restart requires an external process manager.

**Response:**
```json
{
  "success": true,
  "message": "MCP service status restarted. Note: for a full restart, please use your process manager."
}
```

### Python Filesystem Access Toggle

#### GET /api/python-fs-access
Get the current filesystem access status for the `ExecutePython` tool.

**Response:**
```json
{
  "success": true,
  "enabled": false
}
```

#### POST /api/python-fs-access
Enable or disable filesystem access for the `ExecutePython` tool.

**Request:**
```json
{
  "enabled": true
}
```

**Response:**
```json
{
  "success": true,
  "enabled": true
}
```

**Note:** When filesystem access is disabled (default), `ExecutePython` runs in sandbox mode where `builtins.open`, `_io.FileIO`, and `os`/`nt`/`posix` modules are blocked. When enabled, Python code can access files within the configured working directory.

### Search

#### GET /api/search?q={query}
Search tools by name or description.

**Response:**
```json
["Read","Grep","Glob"]
```

### Version Information

#### GET /api/version
Get server version and metadata.

**Response:**
```json
{
  "name": "rust-mcp-server",
  "version": "0.4.0",
  "description": "A high-performance MCP server with WebUI control panel",
  "authors": "MCP Server Team",
  "repository": "https://github.com/yuunnn-w/Rust-MCP-Server",
  "license": "GPL-3.0"
}
```

### Real-time Updates (SSE)

#### GET /api/events
Server-Sent Events endpoint for real-time updates.

**Event Types:**

##### ToolCallCount
Triggered when a tool's call count changes.

```json
{
  "type": "ToolCallCount",
  "tool": "Read",
  "count": 42,
  "isCalling": false,
  "isBusy": false
}
```

##### ConcurrentCalls
Triggered when concurrent call count changes.

```json
{
  "type": "ConcurrentCalls",
  "current": 2,
  "max": 10
}
```

##### McpServiceStatus
Triggered when MCP service status changes.

```json
{
  "type": "McpServiceStatus",
  "running": true
}
```

## MCP Protocol

The MCP service uses JSON-RPC 2.0 over HTTP or SSE.

Base URL: `http://127.0.0.1:3344`

### Initialize

Request:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "example-client",
      "version": "1.0.0"
    }
  }
}
```

### List Tools

Request:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "Read",
        "description": "Read file with format auto-detection and mode system (auto/text/media for generic; doc_text/doc_with_images/doc_images for DOC/DOCX; ppt_text/ppt_images for PPT/PPTX; pdf_text/pdf_images for PDF). Image modes return base64 image content for vision models. Supports image_dpi, image_format.",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": {"type": "string", "description": "File path (auto mode)"},
            "mode": {"type": "string", "enum": ["auto", "text", "media", "doc_text", "doc_with_images", "doc_images", "ppt_text", "ppt_images", "pdf_text", "pdf_images"]},
            "files": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "path": {"type": "string"},
                  "start_line": {"type": "integer"},
                  "end_line": {"type": "integer"},
                  "offset_chars": {"type": "integer"},
                  "max_chars": {"type": "integer"},
                  "line_numbers": {"type": "boolean"},
                  "highlight_line": {"type": "integer"}
                },
                "required": ["path"]
              }
            },
            "image_dpi": {"type": "integer", "description": "DPI for slide/page image rendering (default: 150)"},
            "image_format": {"type": "string", "enum": ["png", "jpg"], "description": "Image format for rendering (default: png)"}
          }
        }
      },
      {
        "name": "Edit",
        "description": "Multi-mode editing: string_replace, line_replace, insert, delete, patch. Complex office modes: office_insert, office_replace, office_delete, office_insert_image, office_format, office_insert_table. PDF modes: pdf_delete_page, pdf_insert_image, pdf_insert_text, pdf_replace_text. Supports .doc/.docx/.ppt/.pptx/.xls/.xlsx.",
        "inputSchema": {
          "type": "object",
          "properties": {
            "operations": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "path": {"type": "string"},
                  "mode": {"type": "string", "enum": ["string_replace", "line_replace", "insert", "delete", "patch", "office_insert", "office_replace", "office_delete", "office_insert_image", "office_format", "office_insert_table", "pdf_delete_page", "pdf_insert_image", "pdf_insert_text", "pdf_replace_text"]},
                  "old_string": {"type": "string"},
                  "new_string": {"type": "string"},
                  "occurrence": {"type": "integer"},
                  "start_line": {"type": "integer"},
                  "end_line": {"type": "integer"},
                  "patch": {"type": "string"},
                  "markdown": {"type": "string"},
                  "find_text": {"type": "string"},
                  "location": {"type": "string"},
                  "element_type": {"type": "string"},
                  "format_type": {"type": "string"},
                  "image_path": {"type": "string"},
                  "slide_index": {"type": "integer"},
                  "page_index": {"type": "integer"}
                },
                "required": ["path"]
              }
            }
          },
          "required": ["operations"]
        }
      },
      {
        "name": "FileStat",
        "description": "Get metadata for one or more files or directories, or check path existence (mode: metadata/exist)",
        "inputSchema": {
          "type": "object",
          "properties": {
            "paths": {
              "type": "array",
              "items": {"type": "string"}
            },
            "mode": {
              "type": "string",
              "enum": ["metadata", "exist"]
            }
          },
          "required": ["paths"]
        }
      },
      {
        "name": "Git",
        "description": "Run git commands in a repository",
        "inputSchema": {
          "type": "object",
          "properties": {
            "action": {"type": "string"},
            "repo_path": {"type": "string"},
            "options": {"type": "array", "items": {"type": "string"}}
          },
          "required": ["action"]
        }
      },
      {
      "name": "ExecutePython",
      "description": "Execute Python code for calculations, data processing, and logic evaluation. Set __result for return value. All Python standard library modules are available. Filesystem access is toggleable via WebUI.",
        "inputSchema": {
          "type": "object",
          "properties": {
            "code": {"type": "string", "description": "Python code to execute"},
            "timeout_ms": {"type": "integer", "minimum": 1000, "maximum": 30000, "default": 5000}
          },
          "required": ["code"]
        }
      }
    ]
  }
}
```

### Call Tool

Request:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "Read",
    "arguments": {
      "files": [
        {"path": "/path/to/file.txt", "start_line": 0, "end_line": 100}
      ]
    }
  }
}
```

Success Response (text tool):
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "File content here...\n\nTotal lines: 150\nUse start_line=100 end_line=200 for more"
      }
    ]
  }
}
```

Error Response:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": "Path is outside working directory (write operation)"
  }
}
```

## Error Codes

### JSON-RPC Error Codes

| Code | Meaning | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Invalid JSON-RPC request |
| -32601 | Method Not Found | Unknown method |
| -32602 | Invalid Params | Invalid method parameters |
| -32603 | Internal Error | Internal server error |
| -32000 | Server Error | Generic server error |
| -32001 | Tool Disabled | Tool is currently disabled |
| -32002 | Security Error | Security check failed |

### HTTP Status Codes

| Status | Endpoint | Meaning |
|--------|----------|---------|
| 200 | All | Success |
| 400 | `/api/config`, `/api/tool/{name}/enable` | Invalid parameters (e.g., out-of-range concurrency, invalid transport/log_level) |
| 404 | `/api/tool/{name}/*`, `/api/search`, unknown `/api/*` routes | Tool not found or endpoint does not exist |
| 500 | All | Internal server error |

**Error Response Body (REST API):**
```json
{
  "success": false,
  "error": "Tool 'unknown_tool' not found"
}
```

## Examples

### Enable a Tool

```bash
curl -X POST http://127.0.0.1:2233/api/tool/Bash/enable \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

### Get Tool Statistics

```bash
curl http://127.0.0.1:2233/api/tool/Read/stats
```

### Call MCP Tool via HTTP

```bash
curl -X POST http://127.0.0.1:3344 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "ExecutePython",
      "arguments": {
        "code": "import math\nprint(math.pi * 2)"
      }
    }
  }'
```

### Listen to SSE Events

```bash
curl http://127.0.0.1:2233/api/events
```

---

中文版本请查看 [api-zh.md](api-zh.md)
