# 更新日志

本文件记录项目的所有重要更新。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本控制](https://semver.org/lang/zh-CN/)。

## [Unreleased]

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
