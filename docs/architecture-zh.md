# 架构概述

## 系统架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Rust MCP Server v0.4.0                       │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │   WebUI (Axum)  │  │  MCP 服务       │  │   工具注册表        │  │
│  │                 │  │   (rmcp)        │  │                     │  │
│  │  ┌───────────┐  │  │                 │  │  ┌───────────────┐  │  │
│  │  │ 静态文件   │  │  │  ┌──────────┐   │  │  │ Glob         │  │  │
│  │  └───────────┘  │  │  │ HTTP/SSE │   │  │  │ Read         │  │  │
│  │  ┌───────────┐  │  │  │ 传输层   │   │  │  │ Write        │  │  │
│  │  │ REST API  │  │  │  └──────────┘   │  │  │ Bash         │  │  │
│  │  │ /api/*    │  │  │                 │  │  │ ... (21工具)  │  │  │
│  │  ┌───────────┐  │  │                 │  │  └───────────────┘  │  │
│  │  │ SSE       │  │  │                 │  │                     │  │
│  │  │ /events   │  │  │                 │  │                     │  │
│  │  └───────────┘  │  │                 │  │                     │  │
│  │  ┌───────────┐  │  │                 │  │                     │  │
│  │  │ 系统指标   │  │  │                 │  │                     │  │
│  │  └───────────┘  │  │                 │  │                     │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │  服务器状态      │  │   配置系统       │  │    安全层          │  │
│  │                 │  │                 │  │                     │  │
│  │  - 工具状态     │  │  - 命令行参数   │  │  - 路径验证         │  │
│  │  - 调用统计     │  │  - 环境变量     │  │  - 危险命令检查     │  │
│  │  - 并发控制     │  │  - 默认值       │  │  - 注入检测         │  │
│  │  - 待确认命令   │  │  - 工作目录     │  │  - 审计日志         │  │
│  │  - 工具预设     │  │                 │  │                     │  │
│  │  - 便签存储     │  │                 │  │                     │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## 组件详情

### WebUI (Axum Web 服务器)

WebUI 提供了一个现代化的控制面板来管理 MCP 服务器。

**功能特性：**
- **静态文件**: HTML、CSS、JS 控制面板文件（编译时嵌入）
- **REST API**: 工具管理和统计接口
- **SSE 端点**: 实时状态更新 (`/api/events`)

**API 端点：**
- `GET /api/tools` - 列出所有工具及其状态
- `GET /api/tool/{name}/stats` - 获取工具统计信息
- `GET /api/tool/{name}/detail` - 获取工具详情
- `POST /api/tool/{name}/enable` - 启用/禁用工具
- `POST /api/tools/batch-enable` - 批量启用/禁用工具
- `GET /api/tool-presets` - 列出工具预设
- `GET /api/tool-presets/current` - 获取当前预设
- `POST /api/tool-presets/apply/{name}` - 应用预设
- `GET /api/server-status` - 服务器运行状态
- `GET /api/system-metrics` - 获取实时 CPU、内存、负载指标
- `GET /api/version` - 获取服务器版本信息
- `GET /api/config` - 获取配置
- `PUT /api/config` - 更新配置
- `POST /api/mcp/{start|stop|restart}` - MCP 服务控制
- `GET /api/python-fs-access` - 查看 `execute_python` 文件系统访问状态
- `POST /api/python-fs-access` - 切换 `execute_python` 文件系统访问

**默认绑定地址**: `127.0.0.1:2233`

### MCP 服务 (rmcp)

使用 `rmcp` crate 实现模型上下文协议。

**传输模式：**
- **HTTP**: JSON 响应模式（默认）
- **SSE**: 服务器推送事件流式模式

**默认绑定地址**: `127.0.0.1:3344`

**协议支持：**
- JSON-RPC 2.0
- MCP 协议版本 2024-11-05
- 工具发现和调用

### 工具注册表

21 个内置工具，按类别组织：

#### 文件操作类（6 个工具）
| 工具名 | 描述 | 危险操作 | 工作目录限制 |
|--------|------|----------|-------------|
| `Glob` | 列出目录内容，支持增强过滤（最大深度10）。多模式 glob/regex 匹配、排除模式、文件大小/时间过滤。对 UTF-8 文本文件返回字符数和行数。 | 否 | 否 |
| `Read` | 读取文件，支持格式自动检测和多种模式。通用文件支持 auto/text/media 模式；DOC/DOCX 支持 doc_text/doc_with_images/doc_images 模式；PPT/PPTX 支持 ppt_text/ppt_images 模式；PDF 支持 pdf_text/pdf_images 模式。`ppt_images` 优先使用 LibreOffice（最佳质量），不可用时自动回退到纯 Rust 原生提取（嵌入式图片+文本每页展示）。建议先用 FileStat 查看文档统计信息，再选择最佳模式。图片模式返回 Base64 编码的 ImageContent。 | 否 | 否 |
| `Grep` | 在文件中搜索模式，支持增强过滤（最大深度10）。正则、区分大小写、整词、多行模式。支持搜索办公文档文本内容。 | 否 | 否 |
| `Write` | 并发写入文件（创建/追加/覆盖）。支持创建 DOCX（office_markdown）、XLSX（office_csv）、PDF（office_markdown）、IPYNB 文件。 | 是 | 是 |
| `FileOps` | 并发复制、移动、删除或重命名文件。支持 dry_run 预览和 conflict_resolution。 | 是 | 是 |
| `Edit` | 多模式编辑：string_replace、line_replace、insert、delete、patch。复杂 Office 模式（office_insert、office_replace、office_delete、office_insert_image、office_format、office_insert_table）用于 DOCX。PDF 模式（pdf_delete_page、pdf_insert_image、pdf_insert_text、pdf_replace_text）通过 lopdf 实现。支持 .doc/.docx/.ppt/.pptx/.xls/.xlsx。 | 是 | 是 |

#### 查询与数据类（2 个工具）
| 工具名 | 描述 | 危险操作 | 工作目录限制 |
|--------|------|----------|-------------|
| `FileStat` | 并发获取文件/目录元数据。mode="exist" 用于轻量级存在性检查。完整模式对 UTF-8 文本文件返回字符数和行数，对办公文件（DOCX/PPTX/PDF/XLSX）额外返回 `document_stats`（页面/幻灯片/工作表数量、嵌入图片数量、文本字符数）。 | 否 | 否 |
| `Git` | 运行 git 命令（status、diff、log、branch、show）。支持 path 过滤和 max_count。 | 否 | 否 |

#### 系统工具类（3 个工具）
| 工具名 | 描述 | 危险操作 | 工作目录限制 |
|--------|------|----------|-------------|
| `SystemInfo` | 获取系统信息，支持通过 sections 参数获取进程列表。在旧版 Windows 上自动跳过磁盘、网络和温度数据。 | 否 | 否 |
| `Bash` | 执行 shell 命令，支持 working_dir、stdin、async_mode。使用 Monitor 监控异步命令。 | 是 | 是 |
| `ExecutePython` | 执行 Python 代码。所有 Python 标准库模块均可使用。文件系统访问可通过 WebUI 切换。 | 否 | 是（仅在启用文件系统访问时） |

#### 网络工具类（1 个工具）
| 工具名 | 描述 | 危险操作 | 工作目录限制 |
|--------|------|----------|-------------|
| `WebFetch` | 抓取并解析 URL 内容，支持 extract_mode（text/html/markdown） | 否 | 否 |

#### 实用工具类（4 个工具）
| 工具名 | 描述 | 危险操作 | 工作目录限制 |
|--------|------|----------|-------------|
| `Clipboard` | 读写系统剪贴板内容（文本或图片）。跨平台。 | 否 | 否 |
| `Archive` | 创建、解压、列出或追加 ZIP 归档，支持 AES-256 密码加密。支持 deflate 和 zstd 压缩。 | 是 | 是 |
| `Diff` | 比较文本、文件或目录差异，支持多种输出格式。支持 ignore_blank_lines。 | 否 | 是（文件/目录模式） |
| `NoteStorage` | AI 短期内存便签本，支持 export/import。便签 30 分钟无操作后自动清空。 | 否 | 否 |

#### 网络与交互工具类（2 个工具）
| 工具名 | 描述 | 危险操作 | 工作目录限制 |
|--------|------|----------|-------------|
| `WebSearch` | 通过 DuckDuckGo 搜索网页，支持 region/language 过滤 | 否 | 否 |
| `AskUser` | 向用户提问或请求确认，支持 timeout 和 default_value | 否 | 否 |

#### 任务管理类（1 个工具）
| 工具名 | 描述 | 危险操作 | 工作目录限制 |
|--------|------|----------|-------------|
| `Task` | 统一任务管理，通过 operation 参数支持 CRUD 操作（create/list/get/update/delete） | 否 | 否 |

#### 办公文档与监控类（2 个工具）
| 工具名 | 描述 | 危险操作 | 工作目录限制 |
|--------|------|----------|-------------|
| `NotebookEdit` | 读取、写入和编辑 Jupyter .ipynb 笔记本文件。写操作受工作目录沙箱限制。 | 是 | 是 |
| `Monitor` | 监控通过 async=true 启动的长时间运行 Bash 命令。操作：stream、wait、signal。 | 否 | 否 |

### 资源 (Resources)

服务器将工作目录暴露为 MCP 资源（`file:///`）。客户端可以：
- **列出资源**：获取工作目录条目，包含 `uri`、`name` 和 `description`
- **读取资源**：通过 `file:///{相对路径}` 获取目录列表或文件内容

资源内容以 `TextResourceContents` 形式返回，MIME 类型为 `text/plain`。

### 提示词 (Prompts)

3 个内置提示词可供客户端使用：

| 提示词 | 描述 |
|--------|-------------|
| `system_diagnosis` | 分析系统信息并识别问题的指南 |
| `file_analysis` | 分析代码文件和目录结构的指南 |
| `security_checklist` | 执行危险操作前的检查清单 |

提示词通过 `prompts/get` 获取，返回一系列 `User` 角色的 `PromptMessage`。

### 服务器状态 (ServerState)

跨所有组件的共享状态，使用 `Arc` 实现线程安全共享。

**组件：**
- **工具注册表**: 工具名称到元数据的 HashMap
- **统计信息**: 调用次数、历史记录、最近调用
- **并发控制**: 用于限制并发调用的信号量
- **待确认命令**: 用于存储待确认命令的 DashMap

### 安全层

多层安全系统：

1. **路径验证**: 规范化和工作目录检查
2. **危险命令检测**: 20 种可配置的危险命令模式
3. **注入检测**: Shell 元字符检测
4. **两步确认**: 危险操作需要用户确认
5. **审计日志**: 所有命令执行都被记录

## 数据流

### 工具执行流程

```
1. 客户端发送工具调用请求（通过 MCP 协议）
2. MCP 处理器接收并解析请求
3. 在 ServerState 中检查工具是否启用
4. 从信号量获取并发许可
5. 路由到工具实现
6. 执行安全检查（路径/命令验证）
7. 在超时保护下执行工具逻辑
8. 记录统计信息并更新状态
9. 释放并发许可
10. 向客户端返回结果
11. 触发 WebUI 的 SSE 更新
```

### WebUI 更新流程

```
1. 工具执行更新 ServerState
2. 状态变更触发 SSE 广播
3. 连接的 WebUI 客户端接收更新
4. UI 组件自动刷新
```

### 命令执行安全流程

```
命令输入
    ↓
工作目录验证
    ↓
危险命令检查（黑名单：20 种模式）
    ↓
注入模式检测（元字符）
    ↓
两步确认（如需要）
    ↓
在审计日志 + 超时保护下执行
    ↓
输出截断（100KB 限制）
```

## 配置系统

**配置源**（按优先级排序）：
1. 命令行参数（最高优先级）
2. 环境变量
3. 默认值（最低优先级）

**主要配置选项：**

| 选项 | CLI 参数 | 环境变量 | 默认值 |
|------|----------|----------|--------|
| WebUI 主机 | `--webui-host` | `MCP_WEBUI_HOST` | `127.0.0.1` |
| WebUI 端口 | `--webui-port` | `MCP_WEBUI_PORT` | `2233` |
| MCP 传输 | `--mcp-transport` | `MCP_TRANSPORT` | `http` |
| MCP 主机 | `--mcp-host` | `MCP_HOST` | `127.0.0.1` |
| MCP 端口 | `--mcp-port` | `MCP_PORT` | `3344` |
| 最大并发 | `--max-concurrency` | `MCP_MAX_CONCURRENCY` | `10` |
| 工作目录 | `--working-dir` | `MCP_WORKING_DIR` | `.` |
| 日志级别 | `--log-level` | `MCP_LOG_LEVEL` | `info` |
| 禁用工具 | `--disable-tools` | `MCP_DISABLE_TOOLS` | 见下方 |
| 危险命令 | `--allow-dangerous-commands` | `MCP_ALLOW_DANGEROUS_COMMANDS` | （无） |

**工具预设：**
服务器默认以 `minimal` 预设启动（启用 9 个工具，`ExecutePython` 处于沙箱模式）。可用预设：
- `minimal`：9 个工具，`ExecutePython` 无文件系统访问
- `coding`：20 个工具，`ExecutePython` 可文件系统访问
- `data_analysis`：15 个工具，`ExecutePython` 可文件系统访问
- `system_admin`：20 个工具，`ExecutePython` 可文件系统访问
- `research`：10 个工具，`ExecutePython` 无文件系统访问
- `full_power`：21 个工具，`ExecutePython` 可文件系统访问

使用 `--preset <name>` 设置启动预设，或 `--preset none` 跳过自动应用。

## 技术栈

- **运行时**: Tokio（异步运行时）
- **Web 框架**: Axum
- **MCP 协议**: rmcp crate
- **序列化**: serde + serde_json
- **日志**: tracing + tracing-subscriber
- **CLI 解析**: clap
- **并发**: tokio::sync (Semaphore, RwLock)
- **集合**: dashmap（并发 HashMap）
- **办公文档**: docx-rs v0.4（DOCX 读写）、lopdf v0.39（PDF 编辑/文本提取）、calamine（XLS/XLSX 读取）
- **PDF 渲染**: pdfium-render v0.9.1（PDF 页面转图片渲染；PDFium 库预置于 assets/，构建时若缺失自动下载）
- **文档转换**: LibreOffice（启动时检测，用于旧版 .doc/.ppt 和 PPTX 幻灯片渲染——PPTX 现在在 LibreOffice 不可用时具有纯 Rust 原生回退方案）
- **系统指标**: sysinfo（CPU、内存、进程监控）

---

For English version, see [architecture.md](architecture.md)
