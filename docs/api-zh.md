# API 文档

## REST API 端点

基础 URL: `http://127.0.0.1:2233`

### 工具管理

#### GET /api/tools
获取所有工具及其当前状态。

**响应：**
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

**字段说明：**
- `name`: 工具标识符
- `description`: 人类可读的描述
- `enabled`: 工具当前是否启用
- `call_count`: 总调用次数
- `is_calling`: 工具是否正在被调用
- `is_busy`: 是否达到并发限制
- `is_dangerous`: 是否为危险操作工具

#### GET /api/tool/{name}/stats
获取特定工具的详细统计信息。

**响应：**
```json
{
  "name": "file_read",
  "total_calls": 42,
  "recent_calls_15min": 5,
  "stats_history": [0, 1, 2, 3, 2, 1, 0, 0, 0, 0],
  "recent_call_times": ["2024-03-15 12:00:00", "2024-03-15 11:55:00"]
}
```

**字段说明：**
- `name`: 工具名称
- `total_calls`: 启动以来的总调用次数
- `recent_calls_15min`: 最近15分钟的调用次数
- `stats_history`: 每15分钟间隔的调用次数数组（最近2小时）
- `recent_call_times`: 最近调用的时间戳

#### GET /api/tool/{name}/detail
获取工具的详细信息。

**响应：**
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
启用或禁用工具。

**请求：**
```json
{
  "enabled": false
}
```

**响应：**
```json
{
  "success": true,
  "message": "Tool 'file_read' disabled"
}
```

### 服务器状态

#### GET /api/status
`/api/tools` 的别名。返回所有工具的状态。

#### GET /api/server-status
获取服务器运行时状态。

**响应：**
```json
{
  "current_calls": 2,
  "max_concurrency": 10,
  "mcp_running": true
}
```

**字段说明：**
- `current_calls`: 当前正在执行的工具调用数
- `max_concurrency`: 允许的最大并发调用数
- `mcp_running`: MCP 服务是否正在运行

### 配置管理

#### GET /api/config
获取当前服务器配置。

**响应：**
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
更新配置（有限选项）。

**请求：**
```json
{
  "max_concurrency": 20
}
```

**响应：**
```json
{
  "success": true,
  "config": {
    "max_concurrency": 20,
    ...
  }
}
```

**可更新字段：**
- `max_concurrency`
- `log_level`

### MCP 服务控制

#### POST /api/mcp/start
启动 MCP 服务。

**响应：**
```json
{
  "success": true,
  "message": "MCP service started"
}
```

#### POST /api/mcp/stop
停止 MCP 服务。

**响应：**
```json
{
  "success": true,
  "message": "MCP service stopped"
}
```

#### POST /api/mcp/restart
重启 MCP 服务。

**响应：**
```json
{
  "success": true,
  "message": "MCP service restarted"
}
```

### 搜索

#### GET /api/search?q={query}
按名称或描述搜索工具。

**响应：**
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

### 实时更新 (SSE)

#### GET /api/events
服务器推送事件端点，用于实时更新。

**事件类型：**

##### ToolCallCount
工具调用次数变化时触发。

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
并发调用数变化时触发。

```json
{
  "type": "ConcurrentCalls",
  "current": 2,
  "max": 10
}
```

##### McpServiceStatus
MCP 服务状态变化时触发。

```json
{
  "type": "McpServiceStatus",
  "running": true
}
```

## MCP 协议

MCP 服务通过 HTTP 或 SSE 使用 JSON-RPC 2.0。

基础 URL: `http://127.0.0.1:3344`

### 初始化

请求：
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

### 列出工具

请求：
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

响应：
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

### 调用工具

请求：
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

成功响应：
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

错误响应：
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

## 错误代码

### JSON-RPC 错误代码

| 代码 | 含义 | 描述 |
|------|------|------|
| -32700 | 解析错误 | 无效的 JSON |
| -32600 | 无效请求 | 无效的 JSON-RPC 请求 |
| -32601 | 方法未找到 | 未知方法 |
| -32602 | 无效参数 | 无效的方法参数 |
| -32603 | 内部错误 | 服务器内部错误 |
| -32000 | 服务器错误 | 通用服务器错误 |
| -32001 | 工具已禁用 | 工具当前被禁用 |
| -32002 | 安全错误 | 安全检查失败 |

### HTTP 状态代码

| 状态 | 端点 | 含义 |
|------|------|------|
| 200 | 全部 | 成功 |
| 400 | /api/config | 无效配置 |
| 404 | /api/tool/{name}/* | 工具未找到 |
| 500 | 全部 | 服务器内部错误 |

## 示例

### 启用工具

```bash
curl -X POST http://127.0.0.1:2233/api/tool/execute_command/enable \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

### 获取工具统计

```bash
curl http://127.0.0.1:2233/api/tool/file_read/stats
```

### 通过 HTTP 调用 MCP 工具

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

### 监听 SSE 事件

```bash
curl http://127.0.0.1:2233/api/events
```

---

For English version, see [api.md](api.md)
