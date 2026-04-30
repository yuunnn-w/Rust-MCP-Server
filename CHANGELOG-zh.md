# 更新日志

本文件记录项目的所有重要更新。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本控制](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### 新增功能
- 新增工具 `execute_python`：基于 RustPython 解释器执行 Python 代码，支持本地文件系统访问。具备 stdout/stderr 捕获、超时控制（1-30秒）、自动末行表达式求值、`__working_dir` 全局变量注入等特性。标记为危险工具，默认禁用。

### 安全
- `execute_python` 因具备文件系统访问能力被归类为危险工具，请通过 WebUI 或 `--disable-tools` 配置谨慎启用。

## [0.2.0] - 2024-04-22

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

## [0.1.0] - 2024-03-15

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
