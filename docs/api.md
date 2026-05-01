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
      "name": "file_read",
      "description": "Read text file content with line range support. Not restricted to working directory.",
      "enabled": true,
      "call_count": 42,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    },
    {
      "name": "execute_command",
      "description": "Execute shell command in specified directory (restricted to working directory)",
      "enabled": false,
      "call_count": 5,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": true
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
  "name": "file_read",
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
  "name": "file_read",
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
  "tool": "file_read",
  "enabled": false
}
```

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
  "log_level": "info"
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
  "message": "Configuration updated successfully",
  "changes": ["max_concurrency"],
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
Start MCP service.

**Response:**
```json
{
  "success": true,
  "message": "MCP service started"
}
```

#### POST /api/mcp/stop
Stop MCP service.

**Response:**
```json
{
  "success": true,
  "message": "MCP service stopped"
}
```

#### POST /api/mcp/restart
Restart MCP service.

**Response:**
```json
{
  "success": true,
  "message": "MCP service restarted"
}
```

### Python Filesystem Access Toggle

#### GET /api/python-fs-access
Get the current filesystem access status for the `execute_python` tool.

**Response:**
```json
{
  "enabled": false
}
```

#### POST /api/python-fs-access
Enable or disable filesystem access for the `execute_python` tool.

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

**Note:** When filesystem access is disabled (default), `execute_python` runs in sandbox mode where `builtins.open`, `_io.FileIO`, and `os`/`nt`/`posix` modules are blocked. When enabled, Python code can access files within the configured working directory.

### Search

#### GET /api/search?q={query}
Search tools by name or description.

**Response:**
```json
["file_read", "file_search", "dir_list"]
```

### Version Information

#### GET /api/version
Get server version and metadata.

**Response:**
```json
{
  "name": "rust-mcp-server",
  "version": "0.2.0",
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
  "tool": "file_read",
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
        "name": "file_read",
        "description": "Read one or more text files concurrently with line numbers and range support",
        "inputSchema": {
          "type": "object",
          "properties": {
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
            }
          },
          "required": ["files"]
        }
      },
      {
        "name": "file_edit",
        "description": "Edit one or more files concurrently using string_replace, line_replace, insert, delete, or patch mode. Can create new files.",
        "inputSchema": {
          "type": "object",
          "properties": {
            "operations": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "path": {"type": "string"},
                  "mode": {"type": "string"},
                  "old_string": {"type": "string"},
                  "new_string": {"type": "string"},
                  "occurrence": {"type": "integer"},
                  "start_line": {"type": "integer"},
                  "end_line": {"type": "integer"},
                  "patch": {"type": "string"}
                },
                "required": ["path"]
              }
            }
          },
          "required": ["operations"]
        }
      },
      {
        "name": "json_query",
        "description": "Query a JSON file using JSON Pointer syntax",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": {"type": "string"},
            "query": {"type": "string"}
          },
          "required": ["path", "query"]
        }
      },
      {
        "name": "file_stat",
        "description": "Get metadata for one or more files or directories concurrently",
        "inputSchema": {
          "type": "object",
          "properties": {
            "paths": {
              "type": "array",
              "items": {"type": "string"}
            }
          },
          "required": ["paths"]
        }
      },
      {
        "name": "path_exists",
        "description": "Check if a path exists and get its type",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": {"type": "string"}
          },
          "required": ["path"]
        }
      },
      {
        "name": "git_ops",
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
        "name": "env_get",
        "description": "Get the value of an environment variable",
        "inputSchema": {
          "type": "object",
          "properties": {
            "name": {"type": "string"}
          },
          "required": ["name"]
        }
      },
      {
        "name": "execute_python",
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
    "name": "file_read",
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

**image_read Response (full mode):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "image",
        "data": "iVBORw0KGgoAAAANSUhEUgAA...",
        "mimeType": "image/png"
      },
      {
        "type": "text",
        "text": "Image: screenshot.png, Dimensions: 1920x1080, Size: 1.2 MB, Type: image/png"
      }
    ]
  }
}
```

The first content item is an MCP-standard `ImageContent` with raw base64 data (no JSON wrapper), enabling vision-model clients to route the image through their encoder. The second item is human-readable metadata text.

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
curl -X POST http://127.0.0.1:2233/api/tool/execute_command/enable \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

### Get Tool Statistics

```bash
curl http://127.0.0.1:2233/api/tool/file_read/stats
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
      "name": "calculator",
      "arguments": {
        "expression": "2 + 2"
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
