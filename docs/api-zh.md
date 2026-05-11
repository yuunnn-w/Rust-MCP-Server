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
      "name": "Read",
      "description": "读取文件，支持模式系统：通用文件 auto/text/media，DOC/DOCX 用 doc_text/doc_with_images/doc_images，PPT/PPTX 用 ppt_text/ppt_images，PDF 用 pdf_text/pdf_images。图片模式返回 Base64 编码内容供视觉模型使用。支持 image_dpi、image_format。不受工作目录限制。",
      "enabled": true,
      "call_count": 42,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    },
    {
      "name": "Bash",
      "description": "在指定工作目录中执行 Shell 命令，支持 stdin、max_output_chars 和 async_mode。使用 Monitor 工具处理异步命令。",
      "enabled": false,
      "call_count": 5,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": true
    },
    {
      "name": "Clipboard",
      "description": "Read or write system clipboard content (text or image)",
      "enabled": true,
      "call_count": 0,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    },
    {
      "name": "Archive",
      "description": "创建、解压、列出或追加 ZIP 归档，支持 AES-256 密码加密",
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
      "description": "In-memory temporary scratchpad for AI short-term memory",
      "enabled": true,
      "call_count": 0,
      "is_calling": false,
      "is_busy": false,
      "is_dangerous": false
    }
  ]
}
```

**字段说明�?*
- `name`: 工具标识�?- `description`: 人类可读的描�?- `enabled`: 工具当前是否启用
- `call_count`: 总调用次�?- `is_calling`: 工具是否正在被调�?- `is_busy`: 是否达到并发限制
- `is_dangerous`: 是否为危险操作工�?
#### GET /api/tool/{name}/stats
获取特定工具的详细统计信息�?
**响应�?*
```json
{
  "name": "Read",
  "total_calls": 42,
  "recent_calls_15min": 5,
  "stats_history": [0, 1, 2, 3, 2, 1, 0, 0, 0, 0],
  "recent_call_times": ["2024-03-15 12:00:00", "2024-03-15 11:55:00"]
}
```

**字段说明�?*
- `name`: 工具名称
- `total_calls`: 启动以来的总调用次�?- `recent_calls_15min`: 最�?5分钟的调用次�?- `stats_history`: �?分钟间隔的调用次数数组（最�?小时�?- `recent_call_times`: 最近调用的时间�?
#### GET /api/tool/{name}/detail
获取工具的详细信息�?
**响应�?*
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
启用或禁用工具�?
**请求�?*
```json
{
  "enabled": false
}
```

**响应�?*
```json
{
  "success": true,
  "tool": "Read",
  "enabled": false
}
```

#### POST /api/tools/batch-enable
批量启用或禁用多个工具�?
**请求�?*
```json
{
  "tools": ["Read", "Write", "Bash"],
  "enabled": true
}
```

**响应�?*
```json
{
  "success": true,
  "enabled": true,
  "changed": ["Read", "Write", "Bash"],
  "failed": []
}
```

### 工具预设

#### GET /api/tool-presets
获取所有可用的工具预设�?
**响应�?*
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
获取当前激活的预设名称�?
**响应�?*
```json
{
  "success": true,
  "preset": "coding"
}
```

#### POST /api/tool-presets/apply/{name}
应用工具预设。此操作会原子性地根据预设配置启用/禁用工具�?
**响应�?*
```json
{
  "success": true,
  "preset": "coding"
}
```

**可用预设：**
- `minimal`: 安全只读工具 + 沙箱 Python（9 个，`ExecutePython` 无文件系统访问）
- `coding`: 开发相关工具，包含文件编辑、任务管理和命令执行（20 个，`ExecutePython` 可文件系统访问）
- `data_analysis`: 数据分析工具，包含 Python、差异比较、归档和网络工具（15 个，`ExecutePython` 可文件系统访问）
- `system_admin`: 系统管理工具，包含系统信息、进程、命令和文件操作（20 个，`ExecutePython` 可文件系统访问）
- `research`: 研究与文档处理工具，包含网页搜索、网页抓取和文件读取（10 个，`ExecutePython` 无文件系统访问）
- `full_power`: 启用全部 21 个工具（`ExecutePython` 可文件系统访问）

### 服务器状�?
#### GET /api/status
`/api/tools` 的别名。返回所有工具的状态�?
#### GET /api/server-status
获取服务器运行时状态�?
**响应�?*
```json
{
  "current_calls": 2,
  "max_concurrency": 10,
  "mcp_running": true
}
```

**字段说明�?*
- `current_calls`: 当前正在执行的工具调用数
- `max_concurrency`: 允许的最大并发调用数
- `mcp_running`: MCP 服务是否正在运行

### 配置管理

#### GET /api/config
获取当前服务器配置�?
**响应�?*
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
更新配置（有限选项）�?
**请求�?*
```json
{
  "max_concurrency": 20
}
```

**响应�?*
```json
{
  "success": true,
  "message": "Configuration updated.",
  "changes": ["max_concurrency: 20"],
  "restart_required": false
}
```

**可更新字段：**
- `webui_host`
- `webui_port`
- `mcp_transport` (`"http"` �?`"sse"`)
- `mcp_host`
- `mcp_port`
- `max_concurrency`（范围：1-1000�?- `working_dir`
- `log_level`
- `system_prompt`
- `log_level` (`"trace"`、`"debug"`、`"info"`、`"warn"`、`"error"`)

**注意�?* 修改 `mcp_transport`、`mcp_host`、`mcp_port`、`webui_host`、`webui_port`、`log_level` �?`working_dir` 后需要重启服务器才能完全生效。当涉及这些字段时，响应将包�?`restart_required: true`�?
### 系统指标

#### GET /api/system-metrics
获取实时系统资源使用情况�?
**响应�?*
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

**字段说明�?*
- `cpu_percent`: 全局 CPU 使用率百分比�?-100�?- `memory_total`: 总物理内存（字节�?- `memory_used`: 已使用物理内存（字节�?- `memory_percent`: 内存使用率百分比�?-100�?- `cpu_cores`: 逻辑 CPU 核心�?- `uptime_seconds`: 系统运行时间（秒�?- `load_average`: 1分钟�?分钟�?5分钟平均负载（Windows 上可能为零）
- `process_count`: 运行中的进程总数

### MCP 服务控制

#### POST /api/mcp/start
启动 MCP 服务�?
**响应�?*
```json
{
  "success": true,
  "message": "MCP service status set to running. Note: full restart requires process manager."
}
```

#### POST /api/mcp/stop
停止 MCP 服务�?
**响应�?*
```json
{
  "success": true,
  "message": "MCP service status set to stopped. Note: full shutdown requires process manager."
}
```

#### POST /api/mcp/restart
重启 MCP 服务�?
**响应�?*
```json
{
  "success": true,
  "message": "MCP service status restarted. Note: for a full restart, please use your process manager."
}
```

### Python 文件系统访问切换

#### GET /api/python-fs-access
获取 `ExecutePython` 工具的当前文件系统访问状态�?
**响应�?*
```json
{
  "success": true,
  "enabled": false
}
```

#### POST /api/python-fs-access
启用或禁�?`ExecutePython` 工具的文件系统访问�?
**请求�?*
```json
{
  "enabled": true
}
```

**响应�?*
```json
{
  "success": true,
  "enabled": true
}
```

**注意�?* 当文件系统访问被禁用（默认）时，`ExecutePython` 以沙箱模式运行，`builtins.open`、`_io.FileIO` 以及 `os`/`nt`/`posix` 模块被阻止。启用后，Python 代码可以访问配置的工作目录内的文件�?
### 搜索

#### GET /api/search?q={query}
按名称或描述搜索工具�?
**响应�?*
```json
["Read","Grep","Glob"]
```

### 版本信息

#### GET /api/version
获取服务器版本及元数据�?
**响应�?*
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

### 实时更新 (SSE)

#### GET /api/events
服务器推送事件端点，用于实时更新�?
**事件类型�?*

##### ToolCallCount
工具调用次数变化时触发�?
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
并发调用数变化时触发�?
```json
{
  "type": "ConcurrentCalls",
  "current": 2,
  "max": 10
}
```

##### McpServiceStatus
MCP 服务状态变化时触发�?
```json
{
  "type": "McpServiceStatus",
  "running": true
}
```

## MCP 协议

MCP 服务通过 HTTP �?SSE 使用 JSON-RPC 2.0�?
基础 URL: `http://127.0.0.1:3344`

### 初始�?
请求�?```json
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

请求�?```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

响应�?```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "Read",
        "description": "读取文件，支持格式自动检测和模式系统（通用文件：auto/text/media；DOC/DOCX：doc_text/doc_with_images/doc_images；PPT/PPTX：ppt_text/ppt_images；PDF：pdf_text/pdf_images）。图片模式返回 Base64 编码内容供视觉模型使用。支持 image_dpi、image_format。",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": {"type": "string", "description": "文件路径（auto 模式）"},
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
            "image_dpi": {"type": "integer", "description": "幻灯片/页面渲染 DPI（默认：150）"},
            "image_format": {"type": "string", "enum": ["png", "jpg"], "description": "渲染图片格式（默认：png）"}
          }
        }
      },
      {
        "name": "Edit",
        "description": "多模式编辑：string_replace、line_replace、insert、delete、patch。复杂 Office 模式：office_insert、office_replace、office_delete、office_insert_image、office_format、office_insert_table。PDF 模式：pdf_delete_page、pdf_insert_image、pdf_insert_text、pdf_replace_text。支持 .doc/.docx/.ppt/.pptx/.xls/.xlsx。",
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
        "description": "获取文件/目录元数据，或检查路径是否存在（mode: metadata/exist�?,
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
      "name": "ExecutePython",
      "description": "执行 Python 代码，用于精确计算、数据处理和逻辑评估。将返回值赋�?__result。所�?Python 标准库模块均可使用。文件系统访问可通过 WebUI 切换�?},
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

请求�?```json
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

成功响应（文本工具）�?```json
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
    "data": "Path is outside working directory (write operation)"
  }
}
```

## 错误代码

### JSON-RPC 错误代码

| 代码 | 含义 | 描述 |
|------|------|------|
| -32700 | 解析错误 | 无效�?JSON |
| -32600 | 无效请求 | 无效�?JSON-RPC 请求 |
| -32601 | 方法未找�?| 未知方法 |
| -32602 | 无效参数 | 无效的方法参�?|
| -32603 | 内部错误 | 服务器内部错�?|
| -32000 | 服务器错�?| 通用服务器错�?|
| -32001 | 工具已禁�?| 工具当前被禁�?|
| -32002 | 安全错误 | 安全检查失�?|

### HTTP 状态代�?
| 状�?| 端点 | 含义 |
|------|------|------|
| 200 | 全部 | 成功 |
| 400 | `/api/config`、`/api/tool/{name}/enable` | 参数无效（如并发数越界、传输协�?日志级别非法�?|
| 404 | `/api/tool/{name}/*`、`/api/search`、未�?`/api/*` 路由 | 工具未找到或端点不存�?|
| 500 | 全部 | 服务器内部错�?|

**错误响应体（REST API）：**
```json
{
  "success": false,
  "error": "Tool 'unknown_tool' not found"
}
```

## 示例

### 启用工具

```bash
curl -X POST http://127.0.0.1:2233/api/tool/Bash/enable \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

### 获取工具统计

```bash
curl http://127.0.0.1:2233/api/tool/Read/stats
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
      "name": "ExecutePython",
      "arguments": {
        "code": "import math\nprint(math.pi * 2)"
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
