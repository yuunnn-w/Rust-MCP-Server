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

**重要提示:** 服务器默认以 `minimal` 预设启动，启用 9 个安全工具（包含 `ExecutePython`，但处于沙箱模式，无文件系统访问）。您可以通过 WebUI 侧边栏或 `--preset` CLI 选项切换预设。各工具仍可独立开关。

1. 打开 WebUI 访问 `http://127.0.0.1:2233`
2. 在工具网格中找到对应工具卡片
3. 点击工具卡片上的开关
4. 更改立即生效（无需重启）

### 工具预设

侧边栏提供**工具预设**功能，可一键切换工具配置：
- **minimal**：安全只读工具 + 沙箱 Python（9 个，`ExecutePython` 无文件系统访问）
- **coding**：开发相关工具，包含文件编辑、任务管理和命令执行（20 个，`ExecutePython` 可文件系统访问）
- **data_analysis**：数据分析工具，包含 Python、差异比较、归档和网络工具（15 个，`ExecutePython` 可文件系统访问）
- **system_admin**：系统管理工具，包含系统信息、进程列表和命令执行（20 个，`ExecutePython` 可文件系统访问）
- **research**：研究与文档处理工具，包含网页搜索、网页抓取和文件读取（10 个，`ExecutePython` 无文件系统访问）
- **full_power**：启用全部 21 个工具（`ExecutePython` 可文件系统访问）

点击侧边栏中的预设按钮即可应用。当前激活的预设会显示在预设网格上方。

### 批量操作

使用侧边栏中的**批量操作**按钮可快速：
- **全部启用**：一次性启用所有工具
- **全部禁用**：一次性禁用所有工具

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

#### Glob
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

#### Read
读取文件，支持格式自动检测和模式系统。支持文本文件、办公文档、PDF 和图片。

**模式系统：**
- `auto`（默认）：自动检测文件类型并选择适当的模式
- `text`：以纯文本方式读取，支持行号、高亮、字符偏移
- `media`：读取图片并返回 Base64 编码内容，供视觉模型直接使用（如 llama.cpp）
- `doc_text`：以 Markdown 方式读取 DOC/DOCX（含标题、表格、格式）
- `doc_with_images`：以 Markdown + 图片内嵌在文档原始位置的方式读取 DOC/DOCX
- `doc_images`：仅从 DOC/DOCX 文件中提取图片
- `ppt_text`：使用 PresentationReader 读取 PPT/PPTX 文本（提取所有形状文本）
- `ppt_images`：将幻灯片显示为图片。优先使用 LibreOffice（最佳质量），不可用时自动回退到纯 Rust 原生提取（每页先展示嵌入图片，再展示文本）。无需任何外部依赖即可工作。
- `pdf_text`：从 PDF 文件中提取文本
- `pdf_images`：将 PDF 页面渲染为图片（通过嵌入的 PDFium），返回 Base64 编码内容供视觉模型使用

**模式选择策略：**
1. 先使用 **FileStat** 查看文档统计信息（slide_count、image_count、text_char_count）
2. 如果有图片（image_count > 0）且需要视觉内容 → 使用 `{pdf,ppt,doc}_images`
3. 如果文本量大且不需要图片 → 使用 `{pdf,ppt,doc}_text`
4. 对于没有 LibreOffice 的 PPTX：`ppt_images` 仍可通过原生提取正常工作

**参数：**
- `path` (string): 文件路径（auto 模式）
- `mode` (string, 可选): 读取模式（默认：`"auto"`）
- `files` (array, 可选): 批量模式 — 要读取的文件列表
  - `path` (string): 文件路径
  - `start_line` (number, 可选): 起始行（从0开始，默认：0）
  - `end_line` (number, 可选): 结束行（默认：500）
  - `offset_chars` (number, 可选): 字符偏移量，作为 start_line 的替代
  - `max_chars` (number, 可选): 最大返回字符数（默认：15000）
  - `line_numbers` (boolean, 可选): 每行前添加行号（默认：false）
  - `highlight_line` (number, 可选): 高亮指定行，在输出中用 `>>> ` 标记
- `image_dpi` (number, 可选): 幻灯片/页面渲染 DPI（默认：150）
- `image_format` (string, 可选): 渲染图片格式：`"png"`（默认）或 `"jpg"`

**特性：**
- 自动检测文本、图片和办公文档格式
- 基于模式的读取，每种格式有专门的处理程序
- 支持办公文档（.doc/.docx/.ppt/.pptx/.xls/.xlsx/.pdf）
- 图片输出返回 Base64 编码的图片内容，通过 MCP ImageContent 供视觉模型直接使用
- 批量模式：并发读取多个文本文件
- 每次读取限制 15KB 字符（可通过 `max_chars` 调整）
- 超出自动截断并提供精确的继续读取提示
- 每个文件返回独立的行数/字符数

**示例：**
```json
// 以 Markdown 方式读取 DOCX 文件
{
  "path": "document.docx",
  "mode": "doc_text"
}

// 将 PDF 页面渲染为图片
{
  "path": "document.pdf",
  "mode": "pdf_images",
  "image_dpi": 200,
  "image_format": "png"
}

// 批量读取文本文件
{
  "mode": "text",
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

#### Write
并发写入内容到一个或多个文件（危险操作）。支持纯文本和办公文档创建。

**参数：**
- `files` (array): 要写入的文件列表
  - `path` (string): 文件路径
  - `content` (string): 要写入的内容
  - `mode` (string, 可选): "new" | "append" | "overwrite"（默认："new"）
- `file_type` (string, 可选): 强制文件类型：`"pdf"` 通过 LibreOffice 创建 PDF
- `office_markdown` (boolean, 可选): 创建 DOCX 时将内容视为 Markdown（支持标题、表格、格式）
- `office_csv` (boolean, 可选): 创建 XLSX 时将内容视为 CSV（支持多工作表）

**特性：**
- 使用 Markdown 格式创建 DOCX 文件（标题、粗体、斜体、表格）
- 从 CSV 数据创建 XLSX 电子表格文件（多工作表）
- 通过 LibreOffice 从 Markdown 创建 PDF 文件
- 纯文本文件创建（新建/追加/覆盖模式）
- 强制工作目录限制

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

#### Grep
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

#### Edit
多模式文件编辑 — 字符串替换、行级操作、统一差异补丁、复杂办公文档操作和 PDF 编辑。支持并发批量操作和创建新文件。

**参数：**
- `operations` (array): 编辑操作列表
  - `path` (string): 要编辑的文件路径
  - `mode` (string, 可选): 编辑模式（见下方）

**文本编辑模式：**

`string_replace` 模式：
- `old_string` (string): 要查找的字符串（精确匹配，可跨多行）
- `new_string` (string): 替换字符串
- `occurrence` (number, 可选): 替换第几次出现 — `1`=第一次（默认），`2`=第二次，`0`=替换所有

`line_replace` / `insert` / `delete` 模式：
- `start_line` (number): 起始行（1-based，包含）
- `end_line` (number): 结束行（1-based，包含）。insert 模式不使用。
- `new_string` (string): 替换或插入的内容

`patch` 模式：
- `patch` (string): 统一差异补丁字符串

**复杂 Office 模式（DOCX）：**

`office_insert` 模式：
- `markdown` (string): 要插入文档的 Markdown 内容

`office_replace` 模式：
- `find_text` (string): 要在文档中查找的文本
- `markdown` (string): Markdown 替换内容

`office_delete` 模式：
- `find_text` (string): 要从文档中查找并删除的文本

`office_insert_image` 模式：
- `image_path` (string): 要插入的图片文件路径
- `location` (string, 可选): 插入位置 — `"end"`（默认）或使用 find_text 的 `"after"`

`office_format` 模式：
- `find_text` (string): 要应用格式的文本
- `element_type` (string): 要格式化的元素类型（如 `"paragraph"`、`"table"`）
- `format_type` (string): 格式化操作类型

`office_insert_table` 模式：
- `location` (string, 可选): 插入位置 — `"end"`（默认）
- `markdown` (string): 要插入的 Markdown 表格内容

**PDF 编辑模式：**

`pdf_delete_page` 模式：
- `page_index` (number): 从零开始的要删除的页面索引

`pdf_insert_image` 模式：
- `page_index` (number): 要插入图片的页面
- `image_path` (string): 要插入的图片文件路径
- `location` (string, 可选): 插入位置 — `"end"`（默认）

`pdf_insert_text` 模式：
- `page_index` (number): 要插入文本的页面
- `markdown` (string): 要插入的文本内容

`pdf_replace_text` 模式：
- `page_index` (number): 要替换文本的页面
- `find_text` (string): 要查找的文本
- `markdown` (string): 替换文本

**特性：**
- 文本模式：string_replace、line_replace、insert、delete、patch — 创建新文件或编辑现有文件
- 复杂 DOCX 模式：通过 Markdown 进行结构化文档操作
- 通过纯 Rust lopdf 库进行 PDF 编辑（无需外部依赖）
- 旧版格式支持：.doc、.ppt、.xls 通过 LibreOffice 自动转换
- 所有模式均返回操作摘要及预览
- 支持并发执行多个操作

**示例：**
```json
// 单操作 - 文本编辑
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

// 办公文档编辑
{
  "operations": [
    {
      "path": "document.docx",
      "mode": "office_replace",
      "find_text": "旧章节",
      "markdown": "# 新章节\n\n带 **格式** 的更新内容"
    },
    {
      "path": "document.docx",
      "mode": "office_insert_table",
      "location": "end",
      "markdown": "| 名称 | 值 |\n|------|-----|\n| A    | 1   |"
    }
  ]
}

// PDF 编辑
{
  "operations": [
    {
      "path": "document.pdf",
      "mode": "pdf_replace_text",
      "page_index": 0,
      "find_text": "四月",
      "markdown": "五月"
    },
    {
      "path": "document.pdf",
      "mode": "pdf_delete_page",
      "page_index": 5
    }
  ]
}
```

### 系统工具类

#### Bash
执行 shell 命令，带安全检查（危险操作）。

**参数：**
- `command` (string): 要执行的命令
- `cwd` (string, 可选): 工作目录（默认：当前目录）
- `timeout` (number, 可选): 超时秒数（默认：30，最大：300）
- `env` (object, 可选): 环境变量（键值对）
- `shell` (string, 可选): 显式指定解释器 — `"cmd"`、`"powershell"`、`"pwsh"`、`"sh"`、`"bash"`、`"zsh"`（默认按平台自动选择）
- `shell_path` (string, 可选): 自定义 shell 可执行文件路径（例如 `C:\Tools\pwh.exe`）。提供时覆盖 `shell`。
- `shell_arg` (string, 可选): 自定义 shell 参数（例如 `-Command`、`/C`）。未提供时按 shell 类型自动推断。

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

#### ExecutePython
执行 Python 代码，用于精确计算、数据处理和逻辑评估。**所有 Python 标准库模块均可使用。**

**沙箱模式（默认）：**
- 文件系统操作被禁用（`builtins.open`、`_io.open`、`_io.FileIO` 以及 `os` 的文件系统函数被阻塞）
- 网络模块（`socket`、`urllib`、`http`、`ssl`）和数据处理模块保持完全可用
- 若尝试文件系统操作，错误信息将提示当前处于沙箱模式
- `subprocess` 和 `ctypes` 作为安全基线被阻止
- 将返回值赋给 `__result`；若未设置，最后一行非注释内容自动作为表达式求值
- 执行超时通过 `sys.settrace` 在 VM 内部注入自终止检查

**文件系统模式：**
- 通过 WebUI 上 `ExecutePython` 卡片的"文件系统"开关启用
- 启用后，`__working_dir` 被注入到全局变量中
- `open()` 和 `os` 文件系统函数被包装为仅限配置的工作目录内路径
- 所有 Python 标准库模块（包括网络和文件系统模块）均可使用

**参数：**
- `code` (string): 要执行的 Python 代码（最大 10,000 字符）
- `timeout_ms` (number, 可选): 超时时间（毫秒，默认：5000，最大：30000）

**返回：**
- `result`: `__result` 变量的值（或未设置时自动求值的末行表达式结果）
- `stdout`: 捕获的标准输出
- `stderr`: 捕获的标准错误/提示
- `execution_time_ms`: 执行耗时（毫秒）

**说明：**
- 将返回值赋给变量 `__result`
- 若未设置 `__result`，最后一行非注释内容将自动作为表达式求值
- 启用文件系统访问时，全局变量 `__working_dir` 包含服务器工作目录
- 无论处于何种模式，所有 Python 标准库模块均可用

**示例：**
```json
{
  "code": "import math\n__result = math.pi * 2",
  "timeout_ms": 5000
}
```

> **提示**：`process_list` 已合并入 `SystemInfo`。使用 `SystemInfo` 并指定 `sections: ["processes"]` 获取进程列表。

#### SystemInfo
获取全面的系统信息。

**返回：**
- 操作系统名称、版本、详细版本、发行版 ID、内核版本、主机名
- CPU 架构、逻辑核心数、物理核心数、品牌、频率（MHz）、使用率（%）
- 内存：总量、已用、空闲（MB）、使用率（%）
- 交换空间：总量、已用、空闲（MB）、使用率（%）
- 系统运行时间（秒）、启动时间（Unix 时间戳）
- 平均负载（1分钟、5分钟、15分钟）——仅限 Unix
- 磁盘：名称、挂载点、文件系统、类型（HDD/SSD）、总/可用容量（GB）、使用率（%）、是否可移动、是否只读
- 网络接口：名称、MAC 地址、IP 地址（CIDR）、MTU、总接收/发送量（MB）
- 硬件温度：组件标签、当前/最高/临界温度（°C，平台支持时）

> **平台说明**：在低于 Windows 10 的系统上，`disks`（磁盘）、`network_interfaces`（网络接口）和 `components`（硬件温度）字段将返回空数组，以避免兼容性问题。其余字段（CPU、内存、操作系统信息）不受影响，正常返回。

所有浮点数值均保留两位小数。

### 实用工具类

### 剪贴板与归档工具

#### Clipboard
读写系统剪贴板内容，支持文本和图片操作。

**参数：**
- `operation` (string): `"read_text"`、`"write_text"`、`"read_image"` 或 `"clear"`
- `text` (string, 可选): 要写入的文本（`write_text` 时必需）

**示例：**
```json
{"operation": "read_text"}
{"operation": "write_text", "text": "Hello, World!"}
{"operation": "clear"}
```

#### Archive
创建、解压、列出或追加 ZIP 归档文件，支持 AES-256 密码加密。所有路径均限制在工作目录内。

**参数：**
- `operation` (string): `"create"`、`"extract"`、`"list"` 或 `"append"`
- `archive_path` (string): ZIP 归档文件路径
- `source_paths` (array, 可选): 要包含的文件/目录（`create`/`append` 时使用）
- `destination` (string, 可选): 解压目标路径（`extract` 时使用，默认工作目录）
- `compression_level` (number, 可选): 1-9（默认: 6，仅 `create` 时有效）
- `password` (string, 可选): AES-256 加密/解密的密码

**示例：**
```json
{"operation": "create", "Archive_path": "backup.zip", "source_paths": ["src", "Cargo.toml"]}
{"operation": "extract", "Archive_path": "backup.zip", "destination": "./extracted"}
{"operation": "list", "Archive_path": "backup.zip"}
```

### 差异比较与便签工具

#### Diff
比较文本、文件或目录差异，支持多种输出格式。

**参数：**
- `operation` (string): `"compare_text"`、`"compare_files"`、`"directory_Diff"` 或 `"git_Diff_file"`
- `old_text` / `new_text` (string, 可选): `compare_text` 时使用
- `old_path` / `new_path` (string, 可选): `compare_files` / `directory_Diff` 时使用
- `file_path` (string, 可选): `git_Diff_file` 时使用（对比工作区与 HEAD 版本）
- `output_format` (string, 可选): `"unified"`（默认）、`"side_by_side"`、`"summary"` 或 `"inline"`
- `context_lines` (number, 可选): 1-20（默认: 3）
- `ignore_whitespace` (boolean, 可选): 默认 false
- `ignore_case` (boolean, 可选): 默认 false
- `max_output_lines` (number, 可选): 默认 500
- `word_level` (boolean, 可选): 启用词级行内高亮（默认: true）

**示例：**
```json
{"operation": "compare_text", "old_text": "foo\nbar", "new_text": "foo\nbaz", "output_format": "unified"}
{"operation": "git_Diff_file", "file_path": "src/main.rs"}
```

#### NoteStorage
AI 短期内存便签本。便签仅保存在内存中，30 分钟无操作后自动清空。

**限制：** 最多 100 条便签，每条最多 50000 字符，标题最多 200 字符，最多 10 个标签。

**参数：**
- `operation` (string): `"create"`、`"list"`、`"read"`、`"update"`、`"delete"`、`"search"`、`"append"`、`"export"` 或 `"import"`
- `id` (number, 可选): 便签 ID（`read`/`update`/`delete`/`append` 时使用）
- `title` (string, 可选): `create`/`update` 时使用
- `content` (string, 可选): `create`/`update` 时使用
- `tags` (array, 可选): `create`/`update` 时使用
- `category` (string, 可选): `create`/`update`/`list` 时使用
- `query` (string, 可选): `search` 时使用
- `append_content` (string, 可选): `append` 时使用

**示例：**
```json
{"operation": "create", "title": "用户偏好暗黑模式", "content": "...", "tags": ["偏好"], "category": "用户偏好"}
{"operation": "search", "query": "偏好"}
```

#### FileStat
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
- `document_stats`（仅办公文件）：文档元数据，包括：
  - `document_type`：`"docx"`、`"pptx"`、`"pdf"` 或 `"xlsx"`
  - `slide_count` / `page_count` / `sheet_count`：幻灯片/页面/工作表数量
  - `image_count`：嵌入图片数量
  - `text_char_count`：文本总字符数

**示例：**
```json
{
  "paths": ["src/main.rs", "Cargo.toml", "src/"]
}
```

> **提示**：`path_exists` 已合并入 `FileStat`。使用 `FileStat` 并指定 `mode: "exist"` 进行轻量级存在性检查。

#### Git
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
      --preset <预设>                启动时应用的工具预设：minimal/coding/data_analysis/system_admin/research/full_power/none [默认：minimal]
      --system-prompt <提示词>       自定义系统提示词，通过 MCP instructions 传递给 LLM
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
export MCP_DISABLE_TOOLS="Bash,SystemInfo"
export MCP_ALLOWED_HOSTS="192.168.1.100,example.com"
./rust-mcp-server
```

### 配置文件

在项目根目录创建 `.env` 文件：

```
MCP_WEBUI_PORT=8080
MCP_MAX_CONCURRENCY=20
MCP_WORKING_DIR=/safe/path
MCP_DISABLE_TOOLS=Write,Bash
```

## 安全特性

### 工作目录限制

只读文件工具（`Glob`、`Read`、`Grep`、`FileStat`、`Git`）**不受**工作目录限制。

写操作工具（`Write`、`Edit`、`FileOps`）以及执行类工具（`Bash`、`ExecutePython`）被限制在配置的工作目录内：

```bash
./rust-mcp-server --working-dir /var/mcp-safe
```

路径穿越尝试（`../`）对受限工具会被阻止。

### 危险命令黑名单

`Bash` 工具默认阻止 20 种危险命令模式：

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
| 12 | format | 格式化磁盘（Windows） |
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
