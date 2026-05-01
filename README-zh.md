# Rust MCP 服务器

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust" alt="Rust 版本">
  <img src="https://img.shields.io/badge/MCP-Protocol-blue" alt="MCP 协议">
  <img src="https://img.shields.io/badge/License-GPL%20v3.0-blue" alt="许可证">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey" alt="平台">
</p>

<p align="center">
  <b>高性能模型上下文协议 (MCP) 服务器，带 WebUI 控制面板</b>
</p>

<p align="center">
  <a href="README.md">English</a> | 
  <a href="#功能特性">功能特性</a> | 
  <a href="#快速开始">快速开始</a> | 
  <a href="#文档">文档</a> | 
  <a href="#安全特性">安全特性</a>
</p>

---

## 项目简介

Rust MCP Server 是一个使用 Rust 构建的高性能 [模型上下文协议 (MCP)](https://modelcontextprotocol.io/) 服务器实现。它为 AI 助手提供了一套全面的工具集，用于与文件系统交互、执行命令、发起 HTTP 请求等操作——所有功能都通过现代化的 WebUI 控制面板进行管理。

### 演示截图

<p align="center">
  <i>WebUI 控制面板 - 管理工具、查看统计信息、实时监控状态</i>
</p>

## 功能特性

### 核心功能
- **21 个内置工具**: 文件操作、HTTP 请求、计算、系统信息、Base64 编解码、Git 操作、JSON 查询、Python 代码执行等
- **WebUI 控制面板**: Cyberpunk AI Command Center 主题，玻璃态 HUD、动态背景、终端日志流、3D 卡片悬浮效果
- **实时更新**: 基于 SSE 的实时状态更新
- **系统指标监控**: 实时 CPU、内存、负载监控（HUD + `/api/system-metrics` 端点）
- **多传输支持**: HTTP（默认，JSON 响应）和 SSE（流式响应）传输
- **并发控制**: 可配置的最大并发工具调用数
- **国际化支持**: 支持英文和中文

### 安全特性
- **工作目录限制**: 危险操作被限制在配置的目录内
- **危险命令黑名单**: 20 个可配置的危险命令模式
- **命令注入检测**: 自动检测可疑字符
- **两步确认**: 危险命令需要用户确认
- **审计日志**: 所有命令执行都被记录

### 可用工具

#### 文件操作（安全 / 默认启用）
以下读操作工具**不受工作目录限制**，可访问任意路径：

| 工具 | 描述 | 危险 |
|------|-------------|------|
| `dir_list` | 树形或扁平结构列出目录内容，包含文本文件元数据（字符数、行数） | 否 |
| `file_read` | 并发读取一个或多个文本文件，支持行范围和行高亮 | 否 |
| `file_search` | 搜索关键词，支持详细/紧凑/位置三种输出模式 | 否 |
| `file_stat` | 并发获取一个或多个文件或目录的元数据，包含文本检测（is_text、字符数、行数） | 否 |
| `path_exists` | 轻量级路径存在性检查 | 否 |
| `json_query` | 使用 JSON Pointer 语法查询 JSON 文件 | 否 |
| `image_read` | 读取图像文件，返回标准 MCP ImageContent（供视觉编码器使用）及元数据 | 否 |

#### 文件操作（危险 / 默认禁用）
以下写操作工具**受工作目录限制**：

| 工具 | 描述 | 安全检查 |
|------|-------------|----------|
| `file_edit` | 多模式编辑：string_replace、line_replace、insert、delete、patch。可创建新文件。支持并发批量操作。 | 工作目录检查 |
| `file_write` | 并发写入一个或多个文件 | 工作目录检查 |
| `file_ops` | 并发复制、移动、删除或重命名一个或多个文件 | 工作目录检查 |

#### 系统与网络工具
| 工具 | 描述 | 默认状态 |
|------|-------------|----------|
| `execute_command` | 执行带安全检查的 shell 命令，支持指定解释器 | 禁用 |
| `process_list` | 列出系统进程 | 禁用 |
| `system_info` | 获取系统信息 | 禁用 |
| `http_request` | 发起 HTTP GET/POST/PUT/DELETE/PATCH/HEAD 请求 | 禁用 |
| `git_ops` | 运行 git 命令（status、diff、log、branch、show） | 启用 |
| `env_get` | 获取环境变量值 | 启用 |
| `execute_python` | 执行 Python 代码。所有 Python 标准库模块均可使用。文件系统访问可通过 WebUI 切换。 | 启用 |

#### 实用工具
| 工具 | 描述 |
|------|-------------|
| `calculator` | 计算数学表达式 |
| `datetime` | 获取当前日期/时间（本地时区） |
| `base64_codec` | Base64 编码/解码 |
| `hash_compute` | 计算 MD5/SHA1/SHA256 哈希 |

## 快速开始

### 安装

#### 选项 1: 下载预编译二进制文件
从 [GitHub Releases](https://github.com/yuunnn-w/Rust-MCP-Server/releases) 下载最新版本。

#### 选项 2: 从源码构建

```bash
# 克隆仓库
git clone https://github.com/yuunnn-w/Rust-MCP-Server.git
cd Rust-MCP-Server

# 使用提供的脚本构建
# Linux/macOS:
./scripts/build-unix.sh

# Windows:
.\scripts\build-windows.bat

# 或使用 cargo 手动构建
cargo build --release
```

#### Windows 7 兼容性编译

若需要在 Windows 7 平台上运行，请使用以下命令进行交叉编译：

```bash
rustup update nightly
cargo +nightly build -Z build-std=std,panic_abort --target x86_64-win7-windows-msvc --release
```

### 使用方法

```bash
# 使用默认设置启动（HTTP 传输 + WebUI）
./rust-mcp-server

# 使用自定义 WebUI 端口启动
./rust-mcp-server --webui-port 8080

# 使用 SSE 传输启动
./rust-mcp-server --mcp-transport sse --mcp-port 9000

# 启用危险命令（按 ID）
./rust-mcp-server --allow-dangerous-commands 1,2

# 查看所有选项
./rust-mcp-server --help
```

### 访问 WebUI

启动后，打开浏览器访问：
```
http://127.0.0.1:2233
```

## 配置

### 命令行选项

| 选项 | 环境变量 | 默认值 | 描述 |
|------|----------|--------|------|
| `--webui-host` | `MCP_WEBUI_HOST` | `127.0.0.1` | WebUI 监听地址 |
| `--webui-port` | `MCP_WEBUI_PORT` | `2233` | WebUI 监听端口 |
| `--mcp-transport` | `MCP_TRANSPORT` | `http` | 传输类型: `http` 或 `sse` |
| `--mcp-host` | `MCP_HOST` | `127.0.0.1` | MCP 服务地址 |
| `--mcp-port` | `MCP_PORT` | `3344` | MCP 服务端口 |
| `--max-concurrency` | `MCP_MAX_CONCURRENCY` | `10` | 最大并发调用数 |
| `--working-dir` | `MCP_WORKING_DIR` | `.` | 文件操作工作目录 |
| `--disable-tools` | `MCP_DISABLE_TOOLS` | 见下文 | 禁用的工具列表（默认禁用11个，启用10个） |
| `--allow-dangerous-commands` | `MCP_ALLOW_DANGEROUS_COMMANDS` | - | 允许的危险命令 ID |
| `--log-level` | `MCP_LOG_LEVEL` | `info` | 日志级别 |
| `--disable-webui` | - | - | 禁用 WebUI 面板 |
| `--allowed-hosts` | `MCP_ALLOWED_HOSTS` | - | 自定义允许的 Host 头（逗号分隔） |
| `--disable-allowed-hosts` | `MCP_DISABLE_ALLOWED_HOSTS` | - | 禁用 DNS 重绑定保护（不推荐公网使用） |

**默认工具状态：**
- **默认启用（11个）：** `calculator`、`dir_list`、`file_read`、`file_search`、`image_read`、`file_stat`、`path_exists`、`json_query`、`git_ops`、`env_get`、`execute_python`
- **默认禁用（10个）：** `file_write`、`file_ops`、`file_edit`、`http_request`、`datetime`、`execute_command`、`process_list`、`base64_codec`、`hash_compute`、`system_info`

### 危险命令 ID

以下命令默认被阻止，需要显式授权：

| ID | Linux 命令 | Windows 命令 |
|----|-----------|-------------|
| 1 | `rm` (删除) | - |
| 2 | `del` (删除) | - |
| 3 | `format` | `format` |
| 4 | `mkfs` | - |
| 5 | `dd` | - |
| 6 | Fork 炸弹 (`:(){:|:&};:`) | - |
| 7 | `eval` | - |
| 8 | `exec` | - |
| 9 | `system` | - |
| 10 | `shred` | - |
| 11 | - | `rd /s` (删除树) |
| 12 | - | `format` (Windows) |
| 13 | - | `diskpart` |
| 14 | - | `reg` (注册表) |
| 15 | - | `net` (网络) |
| 16 | - | `sc` (服务) |
| 17 | - | `schtasks` |
| 18 | - | `powercfg` |
| 19 | - | `bcdedit` |
| 20 | - | `wevtutil` |

启用方式：`--allow-dangerous-commands 1,3,5`

## 安全特性

### 命令执行安全

`execute_command` 工具实现了多层安全防护：

1. **工作目录限制**: 命令只能在配置的工作目录内操作
2. **危险命令检测**: 阻止已知的危险命令（见上表）
3. **注入模式检测**: 检测 shell 元字符（`;`, `|`, `&`, `` ` ``, `$` 等）
4. **两步确认**: 可疑命令需要用户通过重复调用确认
5. **审计日志**: 所有命令都记录时间戳和结果

### 文件操作安全

读操作类工具（`dir_list`、`file_read`、`file_search`、`file_stat`、`path_exists`、`json_query`、`image_read`、`hash_compute`、`git_ops`）不受工作目录限制，可访问任意路径。

写操作类工具（`file_write`、`file_edit`、`file_ops`）以及 `execute_command`、`execute_python` 被限制在配置的工作目录内：
- 路径遍历攻击（`../etc/passwd`）被阻止
- 符号链接逃逸被阻止
- 工作目录外的绝对路径被拒绝

### 安全流程示例

```
用户: execute_command("rm -rf /")
服务器: "安全警告：检测到危险命令 'rm (删除文件)'。
        此命令可能对系统或数据造成损害。
        请向用户确认是否执行此命令。
        如果用户同意，请再次调用 execute_command 工具。"

用户: execute_command("rm -rf /")  [5分钟内第二次调用]
服务器: [命令在确认后执行]
```

## 文档

- [API 文档](docs/api.md) - REST API 参考
- [架构说明](docs/architecture.md) - 系统架构和设计
- [用户指南](docs/user-guide.md) - 详细用户指南
- [安全指南](docs/security.md) - 安全特性和最佳实践
- [贡献指南](CONTRIBUTING.md) - 贡献指南

## 开发

### 项目结构

```
Rust-MCP-Server/
├── src/
│   ├── main.rs              # 入口点
│   ├── config.rs            # 配置管理
│   ├── mcp/
│   │   ├── handler.rs       # MCP 协议处理器
│   │   ├── state.rs         # 共享服务器状态
│   │   └── tools/           # 工具实现
│   ├── utils/               # 工具函数（文件、图像、系统指标）
│   └── web/                 # WebUI 和 HTTP API
├── scripts/                 # 构建脚本
├── docs/                    # 文档
├── README.md               # 英文 README
├── README-zh.md            # 中文 README
└── LICENSE                 # GPL v3.0 许可证
```

### 使用 llama.cpp 测试

你可以使用 [llama.cpp](https://github.com/ggerganov/llama.cpp) 的 `llama-server` 来测试 MCP 服务器，它支持通过 WebUI 配置 MCP。

```bash
# 1. 启动 MCP 服务器
./rust-mcp-server --mcp-transport http --mcp-port 8080

# 2. 启动 llama-server
llama-server -m your-model.gguf

# 3. 打开 llama.cpp WebUI，进入设置页面配置 MCP 服务器地址
#    （例如：http://localhost:8080）

# 4. 启用 MCP 工具并开始对话
```

> **注意：** llama.cpp 通过 `--webui-mcp-proxy` 参数提供实验性的 MCP CORS 代理支持。详情请参阅 llama.cpp 文档了解安全注意事项。

### 构建文档

```bash
cargo doc --no-deps --open
```

## 更新日志

查看 [CHANGELOG.md](CHANGELOG.md) 了解版本历史和变更。

## 贡献

欢迎贡献！提交 PR 前请阅读我们的[贡献指南](CONTRIBUTING.md)。

## 许可证

本项目采用 GPL v3.0 许可证 - 详情请见 [LICENSE](LICENSE) 文件。

## 致谢

- [Model Context Protocol](https://modelcontextprotocol.io/) - 协议规范
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - 官方 Rust MCP SDK
- [Axum](https://github.com/tokio-rs/axum) - Rust Web 框架
- [Tokio](https://tokio.rs/) - Rust 异步运行时

## 支持

- GitHub Issues: [报告问题或请求功能](https://github.com/yuunnn-w/Rust-MCP-Server/issues)
- GitHub Discussions: [提问或分享想法](https://github.com/yuunnn-w/Rust-MCP-Server/discussions)

---

<p align="center">
  使用 Rust 构建 <br>
  <a href="https://github.com/yuunnn-w">@yuunnn-w</a>
</p>
