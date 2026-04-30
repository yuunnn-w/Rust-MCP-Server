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
- `stats_history`: 每5分钟间隔的调用次数数组（最近2小时）
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
  "tool": "file_read",
  "enabled": false
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
  "log_level": "info"
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
  "message": "配置更新成功",
  "changes": ["max_concurrency"],
  "restart_required": false
}
```

**可更新字段：**
- `webui_host`
- `webui_port`
- `mcp_transport` (`"http"` 或 `"sse"`)
- `mcp_host`
- `mcp_port`
- `max_concurrency`（范围：1-1000）
- `working_dir`
- `log_level` (`"trace"`、`"debug"`、`"info"`、`"warn"`、`"error"`)

**注意：** 修改 `mcp_transport`、`mcp_host`、`mcp_port`、`webui_host`、`webui_port`、`log_level` 或 `working_dir` 后需要重启服务器才能完全生效。当涉及这些字段时，响应将包含 `restart_required: true`。

### MCP 服务控制

#### GET /api/system-metrics
获取实时系统资源使用情况。

**响应：**
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

**字段说明：**
- `cpu_percent`: 全局 CPU 使用率百分比（0-100）
- `memory_total`: 总物理内存（字节）
- `memory_used`: 已使用物理内存（字节）
- `memory_percent`: 内存使用率百分比（0-100）
- `cpu_cores`: 逻辑 CPU 核心数
- `uptime_seconds`: 系统运行时间（秒）
- `load_average`: 1分钟、5分钟、15分钟平均负载（Windows 上可能为零）
- `process_count`: 运行中的进程总数

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

### Python 文件系统访问切换

#### GET /api/python-fs-access
获取 `execute_python` 工具的当前文件系统访问状态。

**响应：**
```json
{
  "enabled": false
}
```

#### POST /api/python-fs-access
启用或禁用 `execute_python` 工具的文件系统访问。

**请求：**
```json
{
  "enabled": true
}
```

**响应：**
```json
{
  "success": true,
  "enabled": true
}
```

**注意：** 当文件系统访问被禁用（默认）时，`execute_python` 以沙箱模式运行，`builtins.open`、`_io.FileIO` 以及 `os`/`nt`/`posix` 模块被阻止。启用后，Python 代码可以访问配置的工作目录内的文件。

### 搜索

#### GET /api/search?q={query}
按名称或描述搜索工具。

**响应：**
```json
["file_read", "file_search", "dir_list"]
```

### 版本信息

#### GET /api/version
获取服务器版本及元数据。

**响应：**
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
        "description": "并发读取一个或多个文本文件，每个文件可独立设置行号和范围",
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
        "description": "并发编辑一个或多个文件，支持 string_replace、line_replace、insert、delete、patch 模式，可创建新文件",
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
        "description": "使用 JSON Pointer 语法查询 JSON 文件",
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
        "description": "并发获取一个或多个文件或目录的元数据，对文本文件返回字符数和行数",
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
        "description": "检查路径是否存在并返回其类型",
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
        "description": "在仓库中运行 git 命令",
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
        "description": "获取环境变量的值",
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
        "description": "在沙箱环境中执行 Python 代码（默认安全）。将返回值赋给 __result。可用模块：math, random, statistics, datetime, itertools, functools, collections, re, string, json, fractions, decimal, typing, hashlib, base64, bisect, heapq, copy, pprint, enum, types, dataclasses, inspect, sys。文件系统访问可通过 WebUI 切换。"}}
        "inputSchema": {
          "type": "object",
          "properties": {
            "code": {"type": "string", "description": "要执行的 Python 代码"},
            "timeout_ms": {"type": "integer", "minimum": 1000, "maximum": 30000, "default": 5000}
          },
          "required": ["code"]
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
      "files": [
        {"path": "/path/to/file.txt", "start_line": 0, "end_line": 100}
      ]
    }
  }
}
```

成功响应（文本工具）：
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

**image_read 响应（full 模式）：**
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

第一个 content 项为标准 MCP `ImageContent`，包含原始 base64 数据和 MIME 类型（无 JSON 包装），使视觉模型客户端能将图片送入编码器处理。第二个项为人类可读的元数据文本。

错误响应：
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
