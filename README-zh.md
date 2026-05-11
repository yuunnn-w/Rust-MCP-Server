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
- **21 个内置工具**: 文件操作、办公文档支持（.docx、.pptx、.xlsx、.pdf、.ipynb）、通过 pdfium-render 实现 PDF 页面转图片渲染（预置 PDFium 库，构建时自动下载）、HTTP 请求、Git 操作、Python 代码执行、剪贴板、归档、差异比较、便签存储、任务管理、网页搜索、笔记本编辑、命令监控等
- **工具预设**: 6 种内置预设（minimal、coding、data_analysis、system_admin、research、full_power），一键切换工具配置
- **系统提示**: 通过 `--system-prompt` 或 WebUI 自定义追加到 MCP `initialize` 响应的 instructions
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
|------|------|------|
| `Glob` | 列出目录内容，支持增强过滤（最大深度10）。多模式 glob/regex 匹配、排除模式、文件大小/时间过滤。 | 否 |
| `Read` | 读取文件，支持模式系统：`auto`/`text`/`media` 用于通用文件，`doc_text`/`doc_with_images`/`doc_images` 用于 DOC/DOCX，`ppt_text`/`ppt_images` 用于 PPT/PPTX（LibreOffice 不可用时纯 Rust 原生回退），`pdf_text`/`pdf_images` 用于 PDF。图片模式返回 Base64 编码的图片内容，供视觉模型直接使用（如 llama.cpp）。支持 image_dpi、image_format。建议先用 FileStat 查看统计信息。 | 否 |
| `Grep` | 在文件中搜索模式，支持增强过滤。正则、整词、多行、办公文档搜索。 | 否 |
| `FileStat` | 获取文件/目录元数据。mode="exist" 用于轻量级存在性检查。完整模式对办公文件（DOCX/PPTX/PDF/XLSX）返回 `document_stats`（页面/幻灯片/工作表数量、嵌入图片数量、文本字符数）。 | 否 |

#### 文件操作（危险 / 默认禁用）
以下写操作工具**受工作目录限制**：

| 工具 | 描述 | 安全检查 |
|------|------|----------|
| `Edit` | 多模式编辑：string_replace、line_replace、insert、delete、patch。复杂 Office 模式：office_insert、office_replace、office_delete、office_insert_image、office_format、office_insert_table。PDF 模式：pdf_delete_page、pdf_insert_image、pdf_insert_text、pdf_replace_text。支持 .doc/.docx/.ppt/.pptx/.xls/.xlsx。 | 工作目录检查 |
| `Write` | 写入文件内容。支持创建 .docx（office_markdown）、.xlsx（office_csv）、.pdf（office_markdown）、.ipynb 文件。 | 工作目录检查 |
| `FileOps` | 复制、移动、删除或重命名文件。dry_run 预览、conflict_resolution。 | 工作目录检查 |

#### 系统与网络工具
| 工具 | 描述 | 默认状态 |
|------|------|----------|
| `Bash` | 执行 shell 命令，支持 working_dir、stdin、async_mode。使用 Monitor 监控异步命令。 | 禁用 |
| `SystemInfo` | 获取系统信息，支持 sections 参数（含进程列表）。 | 禁用 |
| `Git` | 运行 git 命令（status、diff、log、branch、show），支持 path 和 max_count。 | 启用 |
| `ExecutePython` | 执行 Python 代码。所有 Python 标准库模块均可使用。文件系统访问可通过 WebUI 切换。 | 启用 |

#### 实用工具
| 工具 | 描述 |
|------|------|
| `Clipboard` | 读写系统剪贴板（文本和图片） |
| `Archive` | 创建、解压、列出、追加 ZIP 归档，支持 deflate/zstd 压缩和 AES-256 密码加密 |
| `Diff` | 比较文本、文件或目录差异，支持 ignore_blank_lines 和多种输出格式 |
| `NoteStorage` | AI 短期内存便签本，支持 export/import（30 分钟无操作自动清空） |

#### 任务、网络与交互工具
| 工具 | 描述 | 默认状态 |
|------|------|----------|
| `Task` | 统一任务管理（通过 operation 参数支持 create/list/get/update/delete） | 禁用 |
| `WebSearch` | 使用可配置的搜索引擎搜索网页，支持 region/language 过滤 | 启用 |
| `WebFetch` | 抓取 URL 内容，支持 extract_mode（text/html/markdown） | 启用 |
| `AskUser` | 向用户提问或请求确认，支持 timeout 和 default_value | 启用 |

#### 办公文档与监控工具
| 工具 | 描述 | 默认状态 |
|------|------|----------|
| `NotebookEdit` | 读取、写入和编辑 Jupyter .ipynb 笔记本文件 | 启用 |
| `Monitor` | 监控长时间运行的 Bash 命令（stream、wait、signal） | 启用 |

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

### 资源依赖

项目包含一些预打包的静态资源，`build.rs` 会在构建时自动下载缺失的文件。以下是完整参考。

#### 预打包资源（已提交到 Git）

| 路径 | 用途 | 来源 |
|------|------|------|
| `assets/vc-ltl/x64/*.lib` | VC-LTL5 CRT 替换（Win7 x64） | [VC-LTL5 v5.3.1](https://github.com/Chuyu-Team/VC-LTL5/releases) |
| `assets/vc-ltl/x86/*.lib` | VC-LTL5 CRT 替换（Win7 x86） | 同上 |
| `assets/yy-thunks/YY_Thunks_for_Win7.obj` | Win8+ API 桩代码（Win7 x64） | [YY-Thunks v1.2.1](https://github.com/Chuyu-Team/YY-Thunks/releases) |
| `assets/yy-thunks/YY_Thunks_for_Win7_x86.obj` | Win8+ API 桩代码（Win7 x86） | 同上 |
| `assets/icon.ico`, `assets/icon.png` | 应用图标 | — |

#### 自动下载资源（由 build.rs 下载和生成）

| 文件 | 用途 | 是否自动？ |
|------|------|------------|
| `assets/pdfium/pdfium.dll` / `.so` / `.dylib` | PDFium 原生库，用于 PDF 页面渲染 | 从 [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases) 自动下载 |
| `assets/pdfium/pdfium-*.tgz` | PDFium 下载压缩包 | 自动下载 |
| `assets/pdfium/pdfium.*.zst` | Zstd 压缩后的 PDFium（编译时嵌入二进制） | 自动从库文件生成 |

> **注意**：如果自动下载失败（如无网络、GitHub 限速），`build.rs` 会打印详细说明。你也可以手动将 PDFium 库文件或其 `.tgz` 压缩包放置到 `assets/pdfium/`。

#### Windows 7 兼容性资源（仅 Win7 Target 需要）

仅在 `--target x86_64-win7-windows-msvc` 构建时需要：

| 路径 | 下载 |
|------|------|
| `assets/vc-ltl/{x64,x86}/*.lib` | [VC-LTL5 Binary v5.3.1](https://github.com/Chuyu-Team/VC-LTL5/releases/download/v5.3.1/VC-LTL5-Binary-v5.3.1.7z) — 解压 `TargetPlatform/6.0.6000.0/lib/{x64,Win32}/` |
| `assets/yy-thunks/YY_Thunks_for_Win7.obj` | [YY-Thunks-Objs.zip v1.2.1](https://github.com/Chuyu-Team/YY-Thunks/releases/download/v1.2.1/YY-Thunks-Objs.zip) — 解压 `objs/x64/YY_Thunks_for_Win7.obj` |
| `assets/yy-thunks/YY_Thunks_for_Win7_x86.obj` | 同上 — 解压 `objs/x86/YY_Thunks_for_Win7.obj` |

#### Windows 7 兼容性编译

本服务器通过两层嵌入式兼容层在 Windows 7 上**原生运行**：

- **VC-LTL5 v5.3.1**（`assets/vc-ltl/`）— 将 UCRT/VCRUNTIME CRT 替换为 `msvcrt.dll`，消除 Win7 上不存在的 `api-ms-win-crt-*` 和 `VCRUNTIME140.dll` 导入
- **YY-Thunks v1.2.1**（`assets/yy-thunks/`）— 为 Windows 8+ API 提供运行时桩代码，回退到旧版等效实现

```bash
rustup update nightly
cargo +nightly build -Z build-std=std,panic_abort --target x86_64-win7-windows-msvc --release
```

> **注意**：在 Windows 7 上，`system_info` 工具会自动返回有限的信息（CPU、内存和操作系统基本信息），并跳过磁盘、网络和硬件温度枚举，以避免 `sysinfo` crate 的兼容性问题。

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
| `--preset` | `MCP_PRESET` | `minimal` | 启动工具预设: minimal/coding/data_analysis/system_admin/research/full_power/none |
| `--system-prompt` | `MCP_SYSTEM_PROMPT` | - | 自定义系统提示，追加到 MCP instructions |
| `--disable-tools` | `MCP_DISABLE_TOOLS` | 见下文 | 在预设基础上额外禁用的工具 |
| `--allow-dangerous-commands` | `MCP_ALLOW_DANGEROUS_COMMANDS` | - | 允许的危险命令 ID |
| `--log-level` | `MCP_LOG_LEVEL` | `info` | 日志级别 |
| `--disable-webui` | - | - | 禁用 WebUI 面板 |
| `--allowed-hosts` | `MCP_ALLOWED_HOSTS` | - | 自定义允许的 Host 头（逗号分隔） |
| `--disable-allowed-hosts` | `MCP_DISABLE_ALLOWED_HOSTS` | - | 禁用 DNS 重绑定保护（不推荐公网使用） |

**工具预设：**
服务器默认以 `minimal` 预设启动。使用 `--preset <name>` 选择其他预设，或 `--preset none` 跳过自动应用。
- **minimal**（9 个工具）：安全只读工具 + 沙箱 Python
- **coding**（20 个工具）：开发相关，包含文件编辑、任务管理和命令执行
- **data_analysis**（15 个工具）：数据分析，包含 Python、差异比较、归档和网络工具
- **system_admin**（20 个工具）：系统管理，包含系统信息、进程、命令和文件操作
- **research**（10 个工具）：研究与文档处理，包含网页搜索、网页抓取和文件读取
- **full_power**（21 个工具）：启用全部工具

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

`Bash` 工具实现了多层安全防护：

1. **工作目录限制**: 命令只能在配置的工作目录内操作
2. **危险命令检测**: 阻止已知的危险命令（见上表）
3. **注入模式检测**: 检测 shell 元字符（`;`, `|`, `&`, `` ` ``, `$` 等）
4. **两步确认**: 可疑命令需要用户通过重复调用确认
5. **审计日志**: 所有命令都记录时间戳和结果

### 文件操作安全

只读文件工具（`Glob`、`Read`、`Grep`、`FileStat`、`Git`）不受工作目录限制，可访问任意路径。

写操作类工具（`Write`、`Edit`、`FileOps`）以及 `Bash`、`ExecutePython` 被限制在配置的工作目录内：
- 路径遍历攻击（`../etc/passwd`）被阻止
- 符号链接逃逸被阻止
- 工作目录外的绝对路径被拒绝

### 安全流程示例

```
用户: Bash("rm -rf /")
服务器: "安全警告：检测到危险命令 'rm (删除文件)'。
        此命令可能对系统或数据造成损害。
        请向用户确认是否执行此命令。
        如果用户同意，请再次调用 Bash 工具。"

用户: Bash("rm -rf /")  [5分钟内第二次调用]
服务器: [命令在确认后执行]
```

## 文档

- [API 文档](docs/api-zh.md) - REST API 参考
- [架构说明](docs/architecture-zh.md) - 系统架构和设计
- [用户指南](docs/user-guide-zh.md) - 详细用户指南
- [安全指南](docs/security-zh.md) - 安全特性和最佳实践
- [贡献指南](CONTRIBUTING-zh.md) - 贡献指南

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
│   ├── utils/               # 工具函数（文件、图像、系统指标、办公文档转换）
│   │   ├── office_converter.rs  # 办公文档转换（docx-rs、lopdf、calamine、LibreOffice）
│   │   ├── office_utils.rs      # 办公文档工具函数
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

查看 [CHANGELOG-zh.md](CHANGELOG-zh.md) 了解版本历史和变更。

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
