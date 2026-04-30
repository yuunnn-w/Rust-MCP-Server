# 用户指南

## 目录

- [快速开始](#快速开始)
- [使用 WebUI](#使用-webui)
- [工具参考](#工具参考)
- [配置说明](#配置说明)
- [安全特性](#安全特性)
- [故障排除](#故障排除)

## 快速开始

### 安装

从 [GitHub Releases](https://github.com/yuunnn-w/Rust-MCP-Server/releases) 下载适合您平台的最新版本。

或从源码构建：
```bash
git clone https://github.com/yuunnn-w/Rust-MCP-Server.git
cd Rust-MCP-Server
./scripts/build-unix.sh  # Linux/macOS
# 或
scripts\build-windows.bat  # Windows
```

### 首次运行

```bash
# 使用默认设置启动
./rust-mcp-server

# 访问 WebUI
open http://127.0.0.1:2233  # macOS
# 或
start http://127.0.0.1:2233  # Windows
```

### 连接 MCP 客户端

配置您的 MCP 客户端连接到：
- **HTTP 传输:** `http://127.0.0.1:3344`
- **SSE 传输:** `http://127.0.0.1:3344/sse`

Claude Desktop 配置示例：
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

## 使用 WebUI

### 仪表板概览

WebUI 提供 Cyberpunk AI Command Center 控制面板：
- **HUD 头部**: 实时系统指标（CPU 环形图、内存条形图、总调用数、并发数）
- **工具网格**: 查看和管理全部工具，支持 3D 悬浮倾斜效果和霓虹边框
- **终端日志面板**: 底部面板实时流式显示 SSE 事件，带彩色日志级别（INFO/WARN/ERROR）
- **实时更新**: 基于 SSE 的实时状态更新（工具调用、并发数、MCP 服务状态）
- **动态背景**: Canvas 绘制的透视网格和浮动粒子网络

### 启用/禁用工具

**重要提示:** 默认情况下，以下 11 个工具为安全起见是启用的：`calculator`、`dir_list`、`file_read`、`file_search`、`image_read`、`file_stat`、`path_exists`、`json_query`、`git_ops`、`env_get`、`execute_python`。

1. 打开 WebUI 访问 `http://127.0.0.1:2233`
2. 在工具网格中找到对应工具卡片
3. 点击工具卡片上的开关
4. 更改立即生效（无需重启）

### 查看工具统计

点击任意工具卡片查看：
- 总调用次数
- 最近调用（15分钟内）
- 使用图表（最近2小时，5分钟间隔）
- 最近调用时间戳
- 工具描述和使用说明

### 工具状态指示器

- **空闲**: 工具可用
- **调用中...**: 工具正在被执行（执行结束后保持5秒）
- **已禁用**: 工具已关闭
- **危险**: 带有警告图标标记（需谨慎使用）

## 工具参考

### 文件操作类

#### dir_list
列出目录内容，树形结构展示。

**参数：**
- `path` (string): 目录路径（默认：当前目录）
- `max_depth` (number, 可选): 最大递归深度（默认：2，最大：5）
- `include_hidden` (boolean, 可选): 包含隐藏文件（默认：false）
- `pattern` (string, 可选): Glob 过滤模式，例如 `"*.rs"`
- `brief` (boolean, 可选): 精简模式 — 仅返回名称、路径、是否为目录（默认：true）
- `sort_by` (string, 可选): 排序方式：`"name"`（默认）、`"type"`、`"size"`、`"modified"`
- `flatten` (boolean, 可选): 扁平模式 — 返回一维数组而非嵌套树（默认：false）

**示例：**
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
并发读取一个或多个文本文件内容，支持行范围。

**参数：**
- `files` (array): 要读取的文件列表
  - `path` (string): 文件路径
  - `start_line` (number, 可选): 起始行（从0开始，默认：0）
  - `end_line` (number, 可选): 结束行（默认：500）
  - `offset_chars` (number, 可选): 字符偏移量，作为 start_line 的替代
  - `max_chars` (number, 可选): 最大返回字符数（默认：15000）
  - `line_numbers` (boolean, 可选): 每行前添加行号（默认：true）
  - `highlight_line` (number, 可选): 高亮指定行，在输出中用 `>>> ` 标记

**特性：**
- 支持并发读取多个文件
- 每次读取限制 15KB 字符（可通过 `max_chars` 调整）
- 超出自动截断并提供精确的继续读取提示
- 每个文件返回独立的总行数
- 行号前缀便于引用

**示例：**
```json
{
  "files": [
    {
      "path": "config.json",
      "start_line": 0,
      "end_line": 500,
      "line_numbers": true
    },
    {
      "path": "src/main.rs",
      "start_line": 0,
      "end_line": 100
    }
  ]
}
```

#### file_write
并发写入内容到一个或多个文件（危险操作）。

**参数：**
- `files` (array): 要写入的文件列表
  - `path` (string): 文件路径
  - `content` (string): 要写入的内容
  - `mode` (string, 可选): "new" | "append" | "overwrite"（默认："new"）

**示例：**
```json
{
  "files": [
    {
      "path": "output.txt",
      "content": "Hello, World!",
      "mode": "new"
    },
    {
      "path": "log.txt",
      "content": "日志条目\n",
      "mode": "append"
    }
  ]
}
```

#### file_search
在文件或目录中搜索关键词。

**参数：**
- `path` (string): 文件或目录路径
- `keyword` (string): 搜索关键词
- `file_pattern` (string, 可选): Glob 文件过滤模式，例如 `"*.rs"`
- `use_regex` (boolean, 可选): 使用正则匹配（默认：false）
- `max_results` (number, 可选): 最大返回匹配结果数（默认：20）
- `context_lines` (number, 可选): 匹配行周围的上下文行数（默认：3）
- `brief` (boolean, 可选): 精简模式 — 仅返回文件路径和行号（默认：false）
- `output_format` (string, 可选): 输出格式：`"detailed"`（默认，完整片段）、`"compact"`（`file:line:matched_text`）、`"location"`（仅 `file:line`）

**特性：**
- 递归目录搜索（最大深度：5）
- 返回匹配内容片段及周围上下文
- 支持正则和字面量关键词
- 跳过二进制文件和黑名单目录
- 提示未搜索的深层目录

**示例：**
```json
{
  "path": "/project/src",
  "keyword": "TODO",
  "file_pattern": "*.rs",
  "context_lines": 3,
  "max_results": 10
}
```

#### file_edit
多模式文件编辑 — 支持字符串替换、行级操作和统一差异补丁。支持并发批量操作和创建新文件。

**参数：**
- `operations` (array): 编辑操作列表
  - `path` (string): 要编辑的文件路径
  - `mode` (string, 可选): `"string_replace"`（默认）、`"line_replace"`、`"insert"`、`"delete"`、`"patch"`

**string_replace 模式：**
- `old_string` (string): 要查找的字符串（精确匹配，可跨多行）
- `new_string` (string): 替换字符串
- `occurrence` (number, 可选): 替换第几次出现 — `1`=第一次（默认），`2`=第二次，`0`=替换所有

**line_replace / insert / delete 模式：**
- `start_line` (number): 起始行（1-based，包含）
- `end_line` (number): 结束行（1-based，包含）。insert 模式不使用。
- `new_string` (string): 替换或插入的内容

**patch 模式：**
- `patch` (string): 统一差异补丁字符串

**特性：**
- `string_replace`: 精确字符串匹配，支持多行。如果文件不存在且提供了 new_string，则创建新文件。
- `line_replace`: 按行号替换 — LLM 无需输出旧内容。如果文件不存在且提供了 new_string，则创建新文件。
- `insert`: 在指定行前插入内容。如果文件不存在且提供了 new_string，则创建新文件。
- `delete`: 删除指定范围的行（要求文件已存在）
- `patch`: 应用标准统一差异补丁，支持多位置复杂修改（要求文件已存在）
- 所有模式均返回替换摘要及预览
- 支持并发执行多个操作

**示例：**
```json
// 单操作
{
  "operations": [
    {
      "path": "src/main.rs",
      "mode": "string_replace",
      "old_string": "fn main() {",
      "new_string": "fn main() -> Result<(), Box<dyn std::error::Error>> {",
      "occurrence": 1
    }
  ]
}

// 并发批量操作
{
  "operations": [
    {
      "path": "src/main.rs",
      "mode": "line_replace",
      "start_line": 10,
      "end_line": 15,
      "new_string": "    let x = 42;\n    println!(\"{}\", x);"
    },
    {
      "path": "src/lib.rs",
      "mode": "insert",
      "start_line": 5,
      "new_string": "use std::fs;"
    }
  ]
}

// 创建新文件
{
  "operations": [
    {
      "path": "src/new_module.rs",
      "mode": "string_replace",
      "old_string": "",
      "new_string": "pub fn hello() {\n    println!(\"Hello\");\n}"
    }
  ]
}
```

### 系统工具类

#### execute_command
执行 shell 命令，带安全检查（危险操作）。

**参数：**
- `command` (string): 要执行的命令
- `cwd` (string, 可选): 工作目录（默认：当前目录）
- `timeout` (number, 可选): 超时秒数（默认：30，最大：300）
- `env` (object, 可选): 环境变量（键值对）
- `shell` (string, 可选): 显式指定解释器 — `"cmd"`、`"powershell"`、`"pwsh"`、`"sh"`、`"bash"`、`"zsh"`（默认按平台自动选择）

**安全特性：**
- 危险命令需要两步确认
- 检测 shell 注入模式
- 输出限制为 100KB

**示例：**
```json
{
  "command": "ls -la",
  "cwd": "/home/user",
  "timeout": 30,
  "shell": "bash"
}
```

#### execute_python
在沙箱环境中执行 Python 代码（默认安全）。适用于精确计算、数据处理和逻辑评估。

**沙箱模式（默认）：**
- 文件系统访问被禁用（`open()`、`os` 模块被阻止）
- 可用标准库模块：`math`、`random`、`statistics`、`datetime`、`itertools`、`functools`、`collections`、`re`、`string`、`json`、`fractions`、`decimal`、`typing`、`hashlib`、`base64`、`bisect`、`heapq`、`copy`、`pprint`、`enum`、`types`、`dataclasses`、`inspect`、`sys`
- 将返回值赋给 `__result`；若未设置，最后一行自动作为表达式求值

**文件系统模式：**
- 通过 WebUI 上 `execute_python` 卡片的"文件系统"开关启用
- 启用后，`__working_dir` 被注入到全局变量中
- 所有 Python 文件操作被限制在配置的工作目录内

**参数：**
- `code` (string): 要执行的 Python 代码
- `timeout_ms` (number, 可选): 超时时间（毫秒，默认：5000，最大：30000）

**参数：**
- `code` (string): 要执行的 Python 代码
- `timeout_ms` (number, 可选): 超时毫秒数（默认：5000，最大：30000）

**返回：**
- `result`: `__result` 变量的值（或未设置时自动求值的末行表达式结果）
- `stdout`: 捕获的标准输出
- `stderr`: 捕获的标准错误/提示
- `execution_time_ms`: 执行耗时（毫秒）

**说明：**
- 将返回值赋给变量 `__result`
- 若未设置 `__result`，最后一行将自动作为表达式求值
- 全局变量 `__working_dir` 包含服务器工作目录
- 支持 Python 标准库模块（math、random、statistics、datetime 等）

**示例：**
```json
{
  "code": "import math\n__result = math.pi * 2",
  "timeout_ms": 5000
}
```

#### process_list
列出系统进程。

**返回：**
- 进程 ID、名称、CPU 使用率、内存使用率
- 按 CPU 使用率降序排列

#### system_info
获取系统信息。

**返回：**
- 操作系统名称和版本
- CPU 数量和架构
- 内存信息
- 主机名

### 实用工具类

#### calculator
计算数学表达式。

**支持：**
- 基本运算符：+、-、*、/、^
- 函数：sqrt、sin、cos、tan、log、ln、abs
- 常量：pi、e
- 括号控制优先级

**示例：**
```json
{
  "expression": "2 + 3 * 4"
}
```

#### http_request
发起 HTTP 请求。

**参数：**
- `url` (string): 目标 URL
- `method` (string): "GET" 或 "POST"
- `headers` (object, 可选): HTTP 请求头
- `body` (string, 可选): 请求体
- `timeout` (number, 可选): 超时秒数（默认：30）
- `extract_json_path` (string, 可选): JSON Pointer 路径，从 JSON 响应中提取数据，例如 `"/data/0/name"`
- `include_response_headers` (boolean, 可选): 在输出中包含响应头（默认：false）
- `max_response_chars` (number, 可选): 最大响应体字符数（默认：15000）

**示例：**
```json
{
  "url": "https://api.example.com/data",
  "method": "GET",
  "extract_json_path": "/data/0/name",
  "max_response_chars": 5000
}
```

#### base64_codec
Base64 编码或解码。

**参数：**
- `operation` (string): `"encode"` 或 `"decode"`
- `input` (string): 要编码的字符串，或要解码的 base64 字符串

**示例：**
```json
{
  "operation": "encode",
  "input": "Hello, World!"
}
```

#### hash_compute
计算字符串或文件的哈希值。

**参数：**
- `input` (string): 要哈希的字符串，或以 `file:` 开头的文件路径
- `algorithm` (string): "MD5"、"SHA1" 或 "SHA256"

**示例：**
```json
{
  "input": "Hello, World!",
  "algorithm": "SHA256"
}
```

#### file_stat
并发获取一个或多个文件或目录的元数据。

**参数：**
- `paths` (array): 文件或目录路径列表

**返回：**
- `name`、`path`、`exists`
- `file_type`: `"file"`、`"directory"`、`"symlink"` 或 `"unknown"`
- `size`: 字节大小
- `size_human`: 人类可读的大小字符串
- `readable`、`writable`、`executable`: 权限布尔值
- `modified`、`created`、`accessed`: 时间戳字符串
- `is_symlink`: 是否为符号链接
- `is_text`: 文件是否为有效的 UTF-8 文本文件
- `char_count`: 字符数（仅文本文件）
- `line_count`: 行数（仅文本文件）
- `encoding`: 检测到的编码（例如 "utf-8"）

**示例：**
```json
{
  "paths": ["src/main.rs", "Cargo.toml", "src/"]
}
```

#### path_exists
轻量级路径存在性检查。

**参数：**
- `path` (string): 要检查的路径

**返回：**
- `exists` (boolean)
- `path_type`: `"file"`、`"dir"`、`"symlink"` 或 `"none"`

**示例：**
```json
{
  "path": "src/main.rs"
}
```

#### json_query
使用 JSON Pointer 语法直接查询 JSON 文件。

**参数：**
- `path` (string): JSON 文件路径
- `query` (string): JSON Pointer 路径，例如 `"/data/0/name"`
- `max_chars` (number, 可选): 最大返回字符数（默认：15000）

**返回：**
- `found` (boolean)
- `result`: 查询到的值（格式化 JSON）
- `result_type`: 类型信息（例如 `"object{5}"`、`"array[3]"`、`"string"`）

**示例：**
```json
{
  "path": "config.json",
  "query": "/database/host"
}
```

#### git_ops
在仓库中运行 git 命令。

**参数：**
- `action` (string): `"status"`、`"diff"`、`"log"`、`"branch"` 或 `"show"`
- `repo_path` (string, 可选): 仓库路径（默认：工作目录）
- `options` (string 数组, 可选): 额外的 git 参数

**示例：**
```json
{
  "action": "log",
  "options": ["--oneline", "-n", "10"]
}
```

#### env_get
获取环境变量的值。

**参数：**
- `name` (string): 环境变量名称

**返回：**
- `name`、`value`、`is_set` (boolean)

**示例：**
```json
{
  "name": "PATH"
}
```

### 图像工具类

#### image_read
读取图像文件并返回标准 MCP 图像内容或元数据。

**参数：**
- `path` (string): 图像文件路径
- `mode` (string, 可选): `"full"`（默认）返回图像数据；`"metadata"` 仅返回尺寸和类型

**返回（full 模式）：**
- MCP `ImageContent` 包含原始 base64 数据和 MIME 类型（供视觉模型编码器使用）
- 人类可读的 `TextContent` 包含文件名、尺寸、大小和格式

**返回（metadata 模式）：**
- JSON 文本包含图像格式、尺寸和大小

## 配置说明

### 命令行选项

```bash
./rust-mcp-server [选项]

选项：
      --webui-host <主机>              WebUI 监听地址 [默认：127.0.0.1]
      --webui-port <端口>              WebUI 监听端口 [默认：2233]
      --mcp-transport <传输模式>       MCP 传输：http 或 sse [默认：http]
      --mcp-host <主机>                MCP 服务监听地址 [默认：127.0.0.1]
      --mcp-port <端口>                MCP 服务监听端口 [默认：3344]
      --max-concurrency <数量>         最大并发调用数 [默认：10]
      --disable-tools <工具列表>       要禁用的工具，逗号分隔
      --working-dir <路径>             文件操作工作目录 [默认：.]
      --log-level <级别>               日志级别：trace、debug、info、warn、error [默认：info]
      --disable-webui                  禁用 WebUI 控制面板
      --allow-dangerous-commands <ID>  允许的危险命令 ID（1-20）
      --allowed-hosts <主机列表>       自定义允许的 Host 头，用于 DNS 重绑定保护（逗号分隔）
      --disable-allowed-hosts          禁用 allowed_hosts 检查（不推荐公网部署使用）
  -h, --help                           显示帮助
  -V, --version                        显示版本
```

### 环境变量

所有 CLI 选项都可以通过环境变量设置：

```bash
export MCP_WEBUI_PORT=8080
export MCP_MAX_CONCURRENCY=20
export MCP_LOG_LEVEL=debug
export MCP_DISABLE_TOOLS="execute_command,process_list"
export MCP_ALLOWED_HOSTS="192.168.1.100,example.com"
./rust-mcp-server
```

### 配置文件

在项目根目录创建 `.env` 文件：

```
MCP_WEBUI_PORT=8080
MCP_MAX_CONCURRENCY=20
MCP_WORKING_DIR=/safe/path
MCP_DISABLE_TOOLS=file_write,execute_command
```

## 安全特性

### 工作目录限制

只读文件工具（`dir_list`、`file_read`、`file_search`、`file_stat`、`path_exists`、`json_query`、`image_read`、`hash_compute`、`git_ops`）**不受**工作目录限制。

写操作工具（`file_write`、`file_edit`、`file_ops`）以及执行类工具（`execute_command`、`execute_python`）被限制在配置的工作目录内：

```bash
./rust-mcp-server --working-dir /var/mcp-safe
```

路径穿越尝试（`../`）对受限工具会被阻止。

### 危险命令黑名单

`execute_command` 工具默认阻止 20 种危险命令模式：

| ID | 命令 | 描述 |
|----|------|------|
| 1 | rm | 删除文件（Linux） |
| 2 | del | 删除文件（Windows） |
| 3 | format | 格式化磁盘 |
| 4 | mkfs | 创建文件系统 |
| 5 | dd | 磁盘复制 |
| 6 | :(){:|:&};: | Fork 炸弹 |
| 7 | eval | 代码执行 |
| 8 | exec | 进程替换 |
| 9 | system | 系统调用 |
| 10 | shred | 安全删除 |
| 11 | rd /s | 删除目录树（Windows） |
| 13 | diskpart | 磁盘分区（Windows） |
| 14 | reg | 注册表操作（Windows） |
| 15 | net | 网络/账户管理（Windows） |
| 16 | sc | 服务控制（Windows） |
| 17 | schtasks | 计划任务（Windows） |
| 18 | powercfg | 电源配置（Windows） |
| 19 | bcdedit | 启动配置（Windows） |
| 20 | wevtutil | 事件日志（Windows） |

**允许特定命令：**
```bash
./rust-mcp-server --allow-dangerous-commands 1,3
```

### 两步确认

当检测到危险命令或注入模式时：

1. 首次调用返回安全警告
2. 命令存入待确认列表（5分钟超时）
3. 第二次相同调用执行命令
4. 用户必须通过 AI 助手明确确认

### 注入检测

以下字符会触发确认：
```
;  |  &  `  $  (  )  <  >
```

引号内的字符不触发检测。

## 故障排除

### 服务器无法启动

**检查端口占用：**
```bash
# Linux/macOS
lsof -i :2233
lsof -i :3344

# Windows
netstat -ano | findstr :2233
netstat -ano | findstr :3344
```

**更换端口：**
```bash
./rust-mcp-server --webui-port 8080 --mcp-port 9000
```

### 工具无法工作

**检查工具是否启用：**
- 打开 WebUI 确认工具开关为开启状态
- 或通过 API 检查：`GET http://127.0.0.1:2233/api/tools`

**检查工作目录：**
```bash
./rust-mcp-server --working-dir /correct/path
```

### MCP 客户端无法连接

**验证传输模式：**
```bash
# 检查当前传输模式
curl http://127.0.0.1:2233/api/config
```

**测试 MCP 端点：**
```bash
# HTTP 传输模式
curl -X POST http://127.0.0.1:3344 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

**403 Forbidden: Host header is not allowed**

此错误表示 MCP 服务器因 DNS 重绑定保护而拒绝了请求（rmcp v1.5.0+）。

**如果使用 `--mcp-host 0.0.0.0`：** 服务器会自动检测本机网卡 IP。若自动检测失败，可使用以下方式：

```bash
# 选项 1：显式指定允许的 Host
./rust-mcp-server --mcp-host 0.0.0.0 --allowed-hosts 192.168.1.100

# 选项 2：禁用检查（不推荐公网部署使用）
./rust-mcp-server --mcp-host 0.0.0.0 --disable-allowed-hosts
```

### 性能问题

**增加并发数：**
```bash
./rust-mcp-server --max-concurrency 50
```

**监控资源使用：**
```bash
# 通过 WebUI HUD
打开 http://127.0.0.1:2233 查看头部系统指标

# 通过 API
curl http://127.0.0.1:2233/api/system-metrics

# Linux/macOS
top -p $(pgrep rust-mcp-server)

# Windows
tasklist | findstr rust-mcp-server
```

## 获取帮助

- [GitHub Issues](https://github.com/yuunnn-w/Rust-MCP-Server/issues)
- [GitHub Discussions](https://github.com/yuunnn-w/Rust-MCP-Server/discussions)

---

For English version, see [user-guide.md](user-guide.md)
