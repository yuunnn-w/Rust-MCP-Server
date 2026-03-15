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

WebUI 提供了全面的控制面板：
- **工具网格**: 查看和管理全部 18 个工具
- **状态栏**: 监控并发调用和服务器状态
- **统计面板**: 查看工具使用图表（Chart.js）
- **实时更新**: 基于 SSE 的实时状态更新

### 启用/禁用工具

**重要提示:** 默认情况下，只有 `calculator`、`dir_list`、`file_read` 和 `file_search` 为安全起见是启用的。

1. 打开 WebUI 访问 `http://127.0.0.1:2233`
2. 在工具网格中找到对应工具卡片
3. 点击工具卡片上的开关
4. 更改立即生效（无需重启）

### 查看工具统计

点击任意工具卡片上的"详情"按钮查看：
- 总调用次数
- 最近调用（15分钟内）
- 使用图表（最近2小时，15分钟间隔）
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
- `max_depth` (number, 可选): 最大递归深度（默认：3，最大：5）

**示例：**
```json
{
  "path": "/project/src",
  "max_depth": 2
}
```

#### file_read
读取文本文件内容，支持行范围。

**参数：**
- `path` (string): 文件路径
- `start_line` (number, 可选): 起始行（从0开始，默认：0）
- `end_line` (number, 可选): 结束行（默认：100）

**特性：**
- 每次读取限制 10KB 字符
- 超出自动截断并提示
- 返回总行数
- 提供继续读取的提示

**示例：**
```json
{
  "path": "config.json",
  "start_line": 0,
  "end_line": 50
}
```

#### file_write
写入内容到文件（危险操作）。

**参数：**
- `path` (string): 文件路径
- `content` (string): 要写入的内容
- `mode` (string, 可选): "new" | "append" | "overwrite"（默认："new"）

**示例：**
```json
{
  "path": "output.txt",
  "content": "Hello, World!",
  "mode": "new"
}
```

#### file_search
在文件或目录中搜索关键词。

**参数：**
- `path` (string): 文件或目录路径
- `keyword` (string): 搜索关键词
- `max_depth` (number, 可选): 最大递归深度（默认：3）

**特性：**
- 递归目录搜索
- 返回文件路径和行号
- 跳过二进制文件（UTF-8 检测）
- 提示未搜索的深层目录

**示例：**
```json
{
  "path": "/project/src",
  "keyword": "TODO",
  "max_depth": 3
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

**安全特性：**
- 危险命令需要两步确认
- 检测 shell 注入模式
- 输出限制为 100KB

**示例：**
```json
{
  "command": "ls -la",
  "cwd": "/home/user",
  "timeout": 30
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

**示例：**
```json
{
  "url": "https://api.example.com/data",
  "method": "GET"
}
```

#### base64_encode / base64_decode
Base64 编码/解码。

**示例：**
```json
{
  "input": "Hello, World!"
}
```

#### hash_compute
计算字符串或文件的哈希值。

**参数：**
- `input` (string): 要哈希的字符串，或以 `@` 开头的文件路径
- `algorithm` (string): "MD5"、"SHA1" 或 "SHA256"

**示例：**
```json
{
  "input": "Hello, World!",
  "algorithm": "SHA256"
}
```

### 图像工具类

#### image_read
读取图像文件并返回 base64 编码数据。

**参数：**
- `path` (string): 图像文件路径

**返回：**
- Base64 编码的图像数据
- 图像格式（png、jpeg 等）

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

所有文件操作都被限制在配置的工作目录内：

```bash
./rust-mcp-server --working-dir /var/mcp-safe
```

路径穿越尝试（`../`）会被阻止。

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

### 性能问题

**增加并发数：**
```bash
./rust-mcp-server --max-concurrency 50
```

**监控资源使用：**
```bash
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
