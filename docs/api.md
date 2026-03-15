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
      "description": "Read text file content with line range support",
      "enabled": true,
      "call_count": 42,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    },
    {
      "name": "execute_command",
      "description": "Execute shell command in specified directory",
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
- `stats_history`: Array of call counts per 15-minute interval (last 2 hours)
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
  "message": "Tool 'file_read' disabled"
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
  "log_level": "info",
  "disable_webui": false,
  "disable_tools": ["execute_command", "file_write"]
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
  "config": {
    "max_concurrency": 20,
    ...
  }
}
```

**Updatable fields:**
- `max_concurrency`
- `log_level`

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

### Search

#### GET /api/search?q={query}
Search tools by name or description.

**Response:**
```json
{
  "tools": [
    {
      "name": "file_read",
      "description": "Read text file content with line range support",
      "enabled": true
    }
  ]
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
        "description": "Read text file content with line range support",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": {"type": "string"},
            "start_line": {"type": "integer"},
            "end_line": {"type": "integer"}
          },
          "required": ["path"]
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
      "path": "/path/to/file.txt",
      "start_line": 0,
      "end_line": 100
    }
  }
}
```

Success Response:
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
    "data": "Path is outside working directory"
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
| 400 | /api/config | Invalid configuration |
| 404 | /api/tool/{name}/* | Tool not found |
| 500 | All | Internal server error |

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
