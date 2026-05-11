# 更新日志

本文件记录项目的所有重要更新。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本控制](https://semver.org/lang/zh-CN/)。

## [0.4.0] - 2026-05-11

### 新增功能
- **旧版 Office 格式支持**：Read/Edit 工具现在通过 LibreOffice 自动转换支持旧版 .doc、.ppt 和 .xls 格式（.xls 通过 calamine 原生支持）
- **DOCX 阅读模式**：三种模式 — `doc_text`（Markdown 输出，含标题/表格/格式）、`doc_with_images`（Markdown + 内嵌图片）、`doc_images`（仅提取图片）
- **PPTX 幻灯片转图片模式**：`ppt_images` 模式通过 LibreOffice 将幻灯片渲染为 PNG/JPG，以 Base64 编码的 MCP ImageContent 返回，供视觉模型使用
- **PPTX 原生图片提取（v0.4.0 更新）**：`ppt_images` 模式在 LibreOffice 不可用时，自动回退到纯 Rust 原生提取方案。通过 ZIP 解析从每张幻灯片中提取嵌入图片，并与幻灯片文本一起展示——每页先展示所有提取的图片，再展示文本内容。无需任何外部依赖。
- **FileStat 办公文件统计**：`FileStat` 完整模式现在对办公文件（DOCX/PPTX/PDF/XLSX）返回 `document_stats` 信息，包括 `document_type`、`page_count`/`slide_count`/`sheet_count`、嵌入 `image_count` 和 `text_char_count`。帮助语言模型智能选择文本模式或图片模式。
- **PDF 页面转图片模式**：`pdf_images` 模式通过 **pdfium-render v0.9.1**（Chromium PDF 引擎）将每页 PDF 渲染为 PNG/JPG。PDFium 二进制文件通过 `include_bytes!` + zstd 压缩直接嵌入可执行文件，运行时首次使用自动解压至临时目录加载。`build.rs` 在构建时若 `assets/` 下缺失则自动从 GitHub 下载。零运行时外部依赖
- **复杂 DOCX 编辑**：新增 `office_insert`、`office_replace`、`office_delete`、`office_insert_image`、`office_format`、`office_insert_table` 模式，通过 Markdown 进行结构化文档操作
- **PDF 编辑（基于 lopdf）**：新增 `pdf_delete_page`、`pdf_insert_image`、`pdf_insert_text`、`pdf_replace_text` 模式，使用纯 Rust lopdf 库
- **基于 Markdown 的 DOCX/PDF 创建**：Write 工具支持 `office_markdown` 参数以创建带标题/表格/格式的 DOCX；通过 LibreOffice 创建 PDF
- **基于 CSV 的 XLSX 创建**：Write 工具支持 `office_csv` 参数以创建多工作表电子表格
- **新增依赖**：`docx-rs` v0.4、`lopdf` v0.39、`pdfium-render` v0.9.1、`zstd` v0.13、`flate2` v1.0
- **LibreOffice 可用性检测**：服务器在启动时检测 LibreOffice 并报告可用性
- **Archive 工具 AES-256 密码加密**：新增 `password` 参数，支持创建和提取受密码保护的 ZIP 归档
- **Grep 工具精简输出模式**：新增 `output_format: "brief"` 仅返回文件路径和行号
- **Windows 7 原生兼容性（自包含）**：移除 `oldwin` crate，改为在 `build.rs` 中直接集成 **VC-LTL5 v5.3.1**（`assets/vc-ltl/`，CRT 替换）和 **YY-Thunks v1.2.1**（`assets/yy-thunks/`，Win8+ API 桩代码）。两者均嵌入仓库。经 `YY.Depends.Analyzer.exe` 以 `6.1.7600` 目标验证 — 零缺失 API 条目。
- **Windows 版本感知 `system_info`**：运行时通过 `RtlGetVersion` 检测系统版本，在低于 Windows 10 的系统上自动跳过磁盘、网络和温度信息收集
- **Windows 可执行程序图标**：为 Windows 可执行文件添加圆角透明图标
- **自定义系统提示词**：新增 `--system-prompt` CLI 参数和 `MCP_SYSTEM_PROMPT` 环境变量，可通过 WebUI 更新
- **自定义 Shell 路径**：`Bash` 工具新增 `shell_path` 和 `shell_arg` 参数
- **双语 CLI 帮助**：`--help` 输出同时显示中英文说明
- **静态资源缓存**：添加缓存头和 ETag 支持
- **前端 UX 改进**：添加筛选控件、模态框 ESC 关闭和加载状态
- **PDFium 资源目录重构**：移至 `assets/pdfium/`，自动下载支持 curl→PowerShell→wget 回退链，新增 **macOS** `libpdfium.dylib` 支持
- **包描述精简**：缩短为 "Rust Model Context Protocol (MCP) Server"

### 变更
- **DOCX 库迁移**：将 `docx-rust` 替换为 `docx-rs`，以获得更优的图片提取、样式解析和格式支持
- **Read 工具重新设计**：模式系统重组 — `auto`/`text`/`media` 用于通用文件，`doc_text`/`doc_with_images`/`doc_images` 用于 DOC/DOCX，`ppt_text`/`ppt_images` 用于 PPT/PPTX，`pdf_text`/`pdf_images` 用于 PDF
- **Read 工具扩展**：新增 `image_dpi` 和 `image_format` 参数用于幻灯片/页面渲染控制
- **Edit 工具扩展**：Office 格式检测现在包含 .doc/.ppt/.xls；新增复杂编辑参数（`markdown`、`find_text`、`location`、`element_type`、`format_type`、`image_path`、`slide_index`、`page_index`）
- **Write 工具扩展**：新增 `office_markdown` 和 `office_csv` 参数；支持 PDF file_type
- **工具描述更新**：mod.rs、handler.rs 和 WebUI 中的 Read/Edit/Write 描述反映了所有新功能
- **Read 工具图片模式**：`media`、`pdf_images`、`ppt_images`、`doc_images` 模式现在通过 MCP `ImageContent` 返回 Base64 编码的图片内容，供视觉模型（如 llama.cpp）直接使用，替代了之前的文件路径元数据
- **Read 工具 doc_with_images 模式**：图片现在内嵌在文档文本的原始位置（通过替换 `{{IMAGE:N}}` 标记实现），而非追加到末尾
- **Read 工具 line_numbers 参数**：默认值从 `true` 改为 `false`
- **所有工具参数描述**：现在使用 `#[schemars(description)]` 为所有参数字段提供完整的 JSON Schema 覆盖
- **Read 工具 ppt_images 模式**：不再强制要求 LibreOffice。当 LibreOffice 不可用时，自动回退到纯 Rust 原生提取（PPTX ZIP 嵌入图片 + ppt-rs 文本提取）。每页幻灯片先展示图片，再展示文字。
- **Read 和 FileStat 工具描述**：更新了详细的模式选择指导、使用策略和办公文件元数据文档。
- **FileStat 办公文件支持**：现在对 DOCX/PPTX/PDF/XLSX 文件提取并返回 `document_stats`（document_type、page/slide/sheet 数量、图片数量、文本字符数）。
- **工具预设重构**：重新设计了 6 个预设，每个预设现在同时控制 `execute_python` 的文件系统访问状态。服务器启动时默认自动应用 `minimal` 预设。
- **状态优化**：使用原子类型减少锁竞争
- **前端性能改进**：搜索防抖和 Canvas 渲染优化
- **构建系统**：移除 `oldwin`/`oldwin-targets` 依赖，`build.rs` 直接链接 VC-LTL5 `.lib` 文件和 YY-Thunks `.obj` 文件，使用 `/NODEFAULTLIB` 精确控制链接顺序

### 修复
- **PPTX 文本提取失败**：修复了使用非标准占位符类型的文件幻灯片文本为空的问题。改用底层 `PresentationReader` API，可提取所有形状文本（不限于标准占位符分类）
- **旧版格式不兼容**：.doc、.ppt 文件现在可通过 LibreOffice 转换读取；.xls 文件通过 calamine 原生支持
- **DOCX 图片提取**：嵌入 DOCX 文件的图片现在可提取到临时文件，返回尺寸/格式元数据
- **PDF 文本提取乱码修复**：从 `pdf` crate 原始字节解码切换为 `lopdf::Document::extract_text()`，正确处理字体编码、ToUnicode CMap 和 CJK 文本
- **lopdf 良性警告抑制**：将 `lopdf` 日志过滤级别设为 `error`，抑制良性的 Type3 字体编码警告，避免日志输出杂乱
- 修复 broadcast 接收器在延迟时崩溃导致工具列表变更通知丢失的问题
- 修复 edit 工具行替换/插入/删除模式下换行符（\r\n）损坏的问题
- 修复 rename 自动重命名冲突解决中的死代码
- 修复 archive.rs `add_dir_to_zip` 和 read.rs `offset_chars` 中的潜在 OOM 问题
- 修复 enhanced_glob 字符类解析中错误转义 `[]` 的问题
- 修复 system_metrics 中 `process_count` 始终为 0 的问题
- 修复 `data_analysis` 预设中缺少 Diff 和 Archive 工具的问题
- 修复 web 处理器中错误响应使用 `String` 而非 `ApiError` 的问题
- 修复 WebUI 配置渲染中的 XSS 漏洞
- 修复多处可能引起 panic 的 `unwrap()` 调用
- 修复 bash 同步模式超时处理中的竞态条件
- 修复 office_utils 中临时文件的符号链接攻击向量
- 修复 web_fetch 中未检查 HTTP 状态码的问题
- 修复 web_fetch 和 web_search 中每次调用都重新编译正则表达式的问题
- 修复 SSE 序列化失败时静默错误的问题
- **WebUI 预设国际化**：预设按钮现在正确显示中文名称
- **WebUI 毛玻璃闪烁**：修复鼠标移动到模态框上方时 backdrop-filter 动画冲突
- **便签搜索**：修复便签搜索未包含标签和分类
- **ZIP 路径遍历**：修复归档解压中的安全漏洞
- **便签内容 UTF-8 截断**：修复在非 UTF-8 边界处截断时的 panic
- **Git diff 路径**：修复子目录中的路径问题
- **Python 线程泄漏**：修复代码执行期间的泄漏
- **异步运行时阻塞**：修复 `image_read`、`file_search` 和 `diff` 中的同步 I/O 阻塞
- **Handler 任务泄漏**：修复客户端重连时的任务泄漏
- **`set_level` 实现**：现在可以正确重载日志过滤器
- **文档一致性审查**：修复所有 md 文件中文档与代码之间的多处不一致
- **PPTX 关系 XML 解析**：修复正则要求属性顺序（Target 必须在 Type 之前）导致的"PPTX contains no slides"错误
- **`parse_relationship_targets`**：重写为独立解析各属性，不依赖 XML 属性顺序
- **v0.4.0 审计修复**：
  - 修复 `windows_version.rs` 缺少非 Windows 平台的代码分支导致 Linux/macOS 编译失败
  - 修复 WebP VP8L 图片尺寸检测使用错误字节偏移
  - 修复 handler.rs 中的阻塞 `std::fs::read_dir` 调用
  - 修复 MCP 初始化指令中硬编码的 WebUI URL
  - 修复工具启用检查与并发许可获取之间的 TOCTOU 竞态条件
  - 统一 handler.rs 与 tools/mod.rs 之间的工具描述（11 处不一致）
  - 修复 `system_admin` 预设与 `coding` 预设完全相同的问题
  - 移除 `default_disable_tools` 中的死引用 `HttpRequest`
  - 修复高负载下信号量缩减静默失败
  - 修复 `pdf_replace_text` 忽略 `page_index` 参数
  - 修复 `edit_docx_insert` 忽略 `find_text` 和 `location` 参数
  - 移除 `edit_docx_insert` 中的重复 `read_docx` 调用
  - 修复 PPTX 文本提取中的 N× 临时文件写入性能问题
  - 为 `WebFetch` 添加响应大小限制（50MB）
  - 将阻塞的 sysinfo 调用包裹在 `spawn_blocking` 中
  - 修复 update_config 中持有配置锁时进行阻塞 I/O
  - 在 `extract_text_from_bytes` 中添加旧格式保护检查
  - 修复 WebUI 并发 HUD 始终重置为 0
  - 修复 WebUI 终端 DOM 内存泄漏
  - 修复 `apply_format_to_docx` 中标题格式的 occurrence 跟踪
  - 修复 `edit_pptx_string` 中 PPTX 图形重复替换
  - 修复 `resolve_shell` 中 shell_arg 验证
  - 修复异步上下文中的 `std::sync::Mutex` 阻塞
  - WebUI 数据加载从 `Promise.all` 切换为 `Promise.allSettled`
  - 移除未使用的 `chart.min.js`（200KB 死代码）

### 移除
- **HttpRequest 工具**：因功能不完整而废弃；WebFetch 提供网页内容抓取功能。已从 mod.rs、handler.rs、presets.rs 及所有预设中移除。工具总数从 22 减少至 21。
- **`oldwin` crate**：替换为直接的 VC-LTL5 + YY-Thunks 集成
- **`chart.min.js`**：移除未使用的 200KB Chart.js 库，WebUI 图表使用原生 Canvas API 渲染
- **`.cargo/config.toml` `/FORCE:MULTIPLE` 标志**：移除 `oldwin` 后不再需要

### 安全
- NotebookEdit 写操作现在强制使用工作目录沙箱

## [0.3.0] - 2026-05-05

### 新增功能
- 新增工具 `execute_python`：基于 RustPython 解释器执行 Python 代码，支持本地文件系统访问。具备 stdout/stderr 捕获、超时控制（1-30秒）、自动末行表达式求值、`__working_dir` 全局变量注入等特性。文件系统访问默认禁用（沙箱模式）；该工具本身是安全工具，默认启用。
- **完整的 Python 标准库支持**：在 `rustpython-stdlib` 中启用 `host_env` 和 `ssl-rustls` 特性，使网络模块（`socket`、`urllib`、`http`、`ssl`）在沙箱模式和文件系统模式下均可用。
- **内置 HTTP 请求能力**：基于 `urllib` 的 HTTP 请求现在可在 Python 解释器内直接使用，无需外部依赖。
- **4 个新工具**（总计 25 个）：
  - `clipboard`：跨平台剪贴板操作（`read_text`、`write_text`、`read_image`、`clear`），基于 `arboard`
  - `archive`：ZIP 归档操作（`create`、`extract`、`list`、`append`），支持可配置压缩级别，基于 `zip`
  - `diff`：高级差异比较工具，支持 4 种模式（`compare_text`、`compare_files`、`directory_diff`、`git_diff_file`）和 4 种输出格式（`unified`、`side_by_side`、`summary`、`inline`），基于 `similar`
  - `note_storage`：AI 短期内存便签本，30 分钟无操作自动清空。支持 `create`、`list`、`read`、`update`、`delete`、`append`、`search`
- **工具预设**：6 种预定义工具配置（`minimal`、`coding`、`document`、`data_analysis`、`system_admin`、`full_power`），一键启用
  - 新增 REST API：`GET /api/tool-presets`、`GET /api/tool-presets/current`、`POST /api/tool-presets/apply/{name}`
  - WebUI 侧边栏增加预设面板及当前预设指示器
- **批量工具启用**：`POST /api/tools/batch-enable` 批量启用/禁用多个工具
  - WebUI 侧边栏增加"全部启用"和"全部禁用"按钮
- **内存便签系统**：集成到 `ServerState` 的临时便签存储，30 分钟无操作自动清理
- 新增依赖：`arboard = "3.6"`、`zip = "8.6"`、`similar = "3.1"`

### 安全
- `execute_python` 默认以沙箱模式运行（文件系统访问禁用）。该工具本身是安全工具，默认启用。如需开启文件系统访问，请通过 WebUI 谨慎启用。
- **Python 沙箱加固**：将模块黑名单策略替换为文件系统函数拦截策略。保留 `os` 模块可用性，使网络标准库模块（`socket`、`urllib`、`http`）能够正常工作，但所有 `os` 文件系统函数（`open`、`listdir`、`mkdir`、`remove`、`rename`、`stat`、`walk` 等）在沙箱模式下被阻塞。`subprocess` 和 `ctypes` 仍作为安全基线被阻止。启用文件系统访问时，`open()` 和 `os` 文件系统函数均被限制在工作目录内。
- **HTTP SSRF 防护**：新增 IPv4 映射 IPv6 地址拦截（`::ffff:127.0.0.1`），禁用自动重定向，并为全局 HTTP 客户端配置连接超时和连接池限制。
- **命令执行安全**：超时后通过 `Child::kill()` 终止子进程，而非仅取消等待。新增命令长度限制（10,000 字符）。注入检测新增换行符（`\n`、\r`），并正确处理引号内的反斜杠转义。
- **文件操作加固**：重命名操作通过 `ensure_path_within_working_dir` 校验目标路径。跨文件系统移动时自动回退到复制+删除。消除 TOCTOU 竞态条件，移除 copy/move/delete/rename 前的预检查，直接依赖操作系统错误返回。
- **哈希流式计算**：大文件哈希改用 8KB 分块读取，避免一次性加载整个文件导致 OOM。
- **文件大小限制**：`file_write` 新增 100MB 内容限制，`image_read` 新增 50MB 限制。
- **敏感信息过滤**：`env_get` 对包含 `SECRET`、`PASSWORD`、`TOKEN`、`KEY` 的环境变量进行黑名单过滤。
- `archive` 工具通过 `ensure_path_within_working_dir` 校验所有路径
- `diff` 工具的 `compare_files` 和 `directory_diff` 模式限制在工作目录内
- `note_storage` 数据完全临时（仅内存存储），30 分钟无操作后自动清空

### 修复
- **崩溃修复**：修复 `json_query`、`file_read`（highlight_line）、`http_request`、`execute_command` 中的 UTF-8 截断 panic，使用 `char_indices()` 安全边界检测。
- **计算器正确性**：拒绝尾部多余 token（如 `(1+2))`），修复一元运算符链（`+-5`），对负数的非整数次幂返回错误而非 NaN。
- **file_read offset_chars**：续读提示现在正确报告字符偏移量，不再混淆字节长度。
- **file_search 性能**：每次搜索只编译一次正则表达式；`max_results` 在遍历过程中强制执行，避免读取不必要的文件。
- **file_edit CRLF 处理**：行替换、插入、删除模式现在保留 Windows `\r\n` 换行符。
- **损坏符号链接检测**：`file_stat` 和 `path_exists` 使用 `symlink_metadata()`，正确将损坏的符号链接报告为存在。
- **git_ops 路径处理**：从 `GIT_WORK_TREE` 和 `GIT_DIR` 环境变量中去除 Windows UNC 前缀（`\\?\`），使 git 能正确识别仓库路径。
- **Web API 错误码**：REST 端点现在返回正确的 HTTP 状态码 —— 工具不存在返回 404，配置参数无效返回 400，内部错误返回 500 —— 不再全部返回 500。
- **Web 静态文件**：未知的 `/api/*` 路由现在返回正确的 404，不再回退到 SPA 的 `index.html`。

### 变更
- `md5` 依赖替换为 `md-5`（RustCrypto），支持流式哈希计算。
- `datetime` 改为使用系统本地时区，不再硬编码中国/北京 UTC+8。
- `system_info` 使用 `available_memory()` 替代 `free_memory()`，内存使用率更准确。
- `process_list` 内存单位从 KB 修正为 MB。
- `dir_list` 按 `size`/`modified` 排序时复用预缓存的元数据，避免重复 stat 系统调用。
- 默认 `working_dir` `"."` 现在启动时自动解析为实际当前工作目录。
- `default_disable_tools` 现在包含 `archive`。
- 更新现有工具描述，提升清晰度和一致性。
- `execute_python` 在 `list_tools()` 中的描述不再被覆盖；保留完整详细描述。

## [0.2.0] - 2026-04-22

### 新增功能
- **WebUI 关于对话框**：新增"关于"按钮和模态框，展示软件版本、说明、作者及 GitHub 仓库链接
- **REST API**：新增 `GET /api/version` 端点，返回服务器版本元数据
- **file_edit** 工具：精确的文本替换编辑，支持 `old_string`/`new_string`/`occurrence` 参数
- **base64_codec** 工具：将 `base64_encode` 和 `base64_decode` 合并为单个工具，通过 `operation` 参数切换
- **dir_list** 增强：新增 `pattern`（Glob 过滤）、`brief` 精简模式、`sort_by` 排序；默认 `max_depth` 从 1 提升至 2
- **file_read** 增强：默认 `end_line` 从 100 提升至 500；新增 `offset_chars`、`max_chars`（默认 15000）、`line_numbers`
- **file_search** 增强：现在返回匹配内容片段及周围上下文，而非仅行号；新增 `file_pattern`、`use_regex`、`max_results`、`context_lines`、`brief`；搜索深度从 3 提升至 5；跳过黑名单目录
- **http_request** 增强：新增 `extract_json_path`（JSON Pointer）、`include_response_headers`、`max_response_chars`
- **image_read** 增强：新增 `mode` 参数（`"full"` 与 `"metadata"`），避免大量 base64 数据传输
- **系统指标模块**：基于 `sysinfo` 的实时 CPU、内存、负载监控模块
- **REST API**：新增 `GET /api/system-metrics` 端点，返回实时系统资源使用率
- **WebUI 全面重构**：重新设计为 "Cyberpunk AI Command Center" 主题，包含玻璃态 HUD、动态网格粒子背景、终端日志流面板、3D 卡片悬浮倾斜效果
- 搜索操作目录黑名单：自动跳过 `.git`、`target`、`node_modules`、`__pycache__` 等目录
- README 中新增 Windows 7 交叉编译说明
- 新增 `--allowed-hosts` 和 `--disable-allowed-hosts` 命令行参数，用于控制 DNS 重绑定保护
- `--mcp-host 0.0.0.0` 时自动检测本机网卡 IP 地址

### 修复
- **dir_list** `sort_entries` 在 `flatten=true` 时按 `size` 或 `modified` 排序现在能正确解析相对路径
- **image_read** `full` 模式现在返回标准 MCP `ImageContent`（`type: "image"`）及人类可读的 `TextContent` 元数据，使视觉模型客户端（如 llama.cpp）能将图片送入编码器处理，而非将 base64 当作纯文本令牌

### 变更
- `rmcp` 从 1.3.0 升级至 1.5.0（已配置 `allowed_hosts` DNS 重绑定保护）
- `reqwest` 从 0.12 升级至 0.13
- `schemars` 从 1.0 升级至 1.1
- 默认启用工具：现为 10 个（`calculator`、`dir_list`、`file_read`、`file_search`、`image_read`、`file_stat`、`path_exists`、`json_query`、`git_ops`、`env_get`）

### 移除
- 独立的 `base64_encode` 和 `base64_decode` 工具（由统一的 `base64_codec` 替代）

## [0.1.0] - 2026-03-15

### 新增功能
- Rust MCP Server 初始版本发布
- 18 个内置工具：文件操作、系统信息、HTTP 请求等
- WebUI 控制面板，支持实时更新
- 多传输协议支持（SSE 和 HTTP）
- 危险命令黑名单，支持配置 ID（20 个命令）
- 命令注入检测
- 危险操作两步确认机制
- 文件操作工作目录限制
- 所有命令执行的审计日志
- 国际化支持（中文和英文）
- 完整的文档体系

### 安全特性
- 所有文件操作的工作目录限制
- 危险命令黑名单（20 种命令模式）
- 命令注入模式检测
- 可疑命令两步确认
- 自动清理待确认命令（5 分钟超时）

---

For English version, see [CHANGELOG.md](CHANGELOG.md)
