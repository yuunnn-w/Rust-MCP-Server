# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Custom System Prompt**: New `--system-prompt` CLI flag and `MCP_SYSTEM_PROMPT` environment variable. Custom prompt is appended to MCP `initialize` instructions and can be updated via WebUI (`GET/PUT /api/config`).
- **Custom Shell Path for `execute_command`**: New `shell_path` and `shell_arg` parameters allow specifying custom shell executables (e.g., `C:\Tools\pwh.exe` for Windows 7 + VxKex environments). Smart argument inference: filenames containing "powershell"/"pwsh"/"pwh" use `-Command`, otherwise Windows uses `/C` and Unix uses `-c`.
- **Bilingual CLI Help**: `--help` output now displays both English and Chinese descriptions using clap's `help_template` and bilingual `help` annotations.

### Changed
- **Tool Presets overhaul**: Redesigned the 6 existing presets (`minimal`, `coding`, `document`, `data_analysis`, `system_admin`, `full_power`) so each preset now also controls `execute_python` filesystem access state. `minimal` keeps `execute_python` sandboxed (fs=false); `coding`/`data_analysis`/`system_admin`/`full_power` enable fs access. Server auto-applies the `minimal` preset on startup by default. Use `--preset none` to skip.
- **Documentation overhaul**: All 10 markdown files (README, architecture, user-guide, security, API, and Chinese versions) updated to reflect preset-based tool management, new `system_prompt` feature, and corrected tool counts.
- **Changelog year correction**: All release dates corrected from 2024/2025 to 2026.

### Fixed
- **WebUI preset i18n**: Preset buttons (`data_analysis`, `system_admin`) now correctly display Chinese names. The "Current: " label now shows translated preset names instead of English.
- **WebUI glassmorphism flickering**: Fixed backdrop-filter flickering when moving mouse over modal overlays or sidebar. Removed conflicting `transform: translateZ(0)` / `will-change` from `.modal` and `.modal-content`, replaced with `contain: layout paint`. Optimized `bindCardTilt` to use `requestAnimationFrame` for batched transform updates.

## [0.3.0] - 2026-05-05

### Added
- **New tool `execute_python`**: Execute Python code in a RustPython interpreter with local filesystem access. Supports stdout/stderr capture, timeout control (1-30s), automatic last-line expression evaluation, and `__working_dir` injection. Filesystem access is disabled by default (sandboxed); the tool itself is safe and enabled by default.
- **Full Python standard library support**: Enabled `host_env` and `ssl-rustls` features in `rustpython-stdlib`, making network modules (`socket`, `urllib`, `http`, `ssl`) available in both sandbox and filesystem modes.
- **Embedded HTTP helper**: `urllib`-based HTTP requests now work inside the Python interpreter without external dependencies.
- **4 New Tools** (total 25):
  - `clipboard`: Cross-platform clipboard operations (`read_text`, `write_text`, `read_image`, `clear`) via `arboard`
  - `archive`: ZIP archive operations (`create`, `extract`, `list`, `append`) with configurable compression level via `zip`
  - `diff`: Advanced diff tool with 4 modes (`compare_text`, `compare_files`, `directory_diff`, `git_diff_file`) and 4 output formats (`unified`, `side_by_side`, `summary`, `inline`) via `similar`
  - `note_storage`: In-memory temporary scratchpad for AI short-term memory with auto-expiry after 30 minutes of inactivity. Supports `create`, `list`, `read`, `update`, `delete`, `append`, `search`
- **Tool Presets**: 6 predefined tool configurations (`minimal`, `coding`, `document`, `data_analysis`, `system_admin`, `full_power`) for one-click tool enablement
  - New REST APIs: `GET /api/tool-presets`, `GET /api/tool-presets/current`, `POST /api/tool-presets/apply/{name}`
  - Preset UI in WebUI sidebar with active preset indicator
- **Batch Tool Enable**: `POST /api/tools/batch-enable` to enable/disable multiple tools at once
  - WebUI sidebar buttons for "Enable All" and "Disable All"
- **In-Memory Notes System**: Temporary note storage integrated into `ServerState` with 30-minute auto-cleanup
- New dependencies: `arboard = "3.6"`, `zip = "8.6"`, `similar = "3.1"`

### Security
- `execute_python` runs in sandbox mode by default (filesystem access disabled). It is classified as a safe tool and enabled by default. Filesystem access can be enabled with caution via WebUI.
- **Python sandbox hardening**: Replaced module blacklist approach with filesystem-function interception. The `os` module is kept available so network stdlib modules (`socket`, `urllib`, `http`) can function, but all `os` filesystem functions (`open`, `listdir`, `mkdir`, `remove`, `rename`, `stat`, `walk`, etc.) are blocked in sandbox mode. `subprocess` and `ctypes` remain blocked as a security baseline. When filesystem access is enabled, both `open()` and `os` filesystem functions are restricted to the working directory.
- **HTTP SSRF protection**: Added IPv4-mapped IPv6 address blocking (`::ffff:127.0.0.1`), disabled automatic redirects, and configured connection timeouts and pool limits for the global HTTP client.
- **Command execution safety**: Timeout now kills the child process via `Child::kill()` instead of just cancelling the wait. Added command length limit (10,000 characters). Added newline (`\n`, `\r`) injection detection and backslash escape handling in quote parsing.
- **File operation hardening**: Rename operations now validate target paths with `ensure_path_within_working_dir`. Move operations fallback to copy+delete on cross-device errors. Removed TOCTOU race conditions by relying on kernel error handling instead of pre-flight `exists()` checks.
- **Hash streaming**: Large file hashing now uses 8KB chunked reads instead of loading entire files into memory, preventing OOM on multi-gigabyte files.
- **File size limits**: Added 100MB content limit for `file_write` and 50MB limit for `image_read`.
- **Sensitive data filtering**: `env_get` now blacklists variables containing `SECRET`, `PASSWORD`, `TOKEN`, or `KEY`.
- `archive` tool validates all paths against `working_dir` using `ensure_path_within_working_dir`
- `diff` tool's `compare_files` and `directory_diff` modes are restricted to `working_dir`
- `note_storage` data is purely ephemeral (memory-only) and automatically cleared after 30 minutes of inactivity

### Fixed
- **Crash fixes**: Fixed UTF-8 truncation panics in `json_query`, `file_read` (highlight_line), `http_request`, and `execute_command` by using `char_indices()` safe boundary detection.
- **Calculator correctness**: Rejects trailing tokens (e.g., `(1+2))`), fixes unary operator chains (`+-5`), and adds error for negative-base non-integer powers.
- **file_read offset_chars**: Continuation hint now correctly reports character offsets instead of mixing byte lengths.
- **file_search performance**: Regex is compiled once per search instead of per-file; `max_results` is enforced during traversal to avoid reading unnecessary files.
- **file_edit CRLF handling**: Line replace, insert, and delete modes now preserve Windows `\r\n` line endings.
- **Broken symlink detection**: `file_stat` and `path_exists` now correctly report broken symlinks as existing using `symlink_metadata()`.
- **git_ops path handling**: Windows UNC prefix (`\\?\`) is stripped from `GIT_WORK_TREE` and `GIT_DIR` environment variables so git recognizes repository paths.
- **Web API errors**: REST endpoints now return proper HTTP status codes — 404 for missing tools, 400 for invalid config parameters, 500 for internal errors — instead of always returning 500.
- **Web static files**: Unknown `/api/*` routes now return proper 404 instead of falling back to SPA `index.html`.

### Changed
- `md5` dependency replaced with `md-5` (RustCrypto) to support streaming hash computation.
- `datetime` now uses the system local timezone instead of hardcoded China/Beijing UTC+8.
- `system_info` now reports `available_memory()` instead of `free_memory()` for more accurate memory usage metrics.
- `process_list` memory units corrected from KB to MB.
- `dir_list` sort by `size`/`modified` now uses pre-cached metadata to avoid repeated stat syscalls.
- Default `working_dir` `"."` is now automatically resolved to the actual current working directory at startup.
- `default_disable_tools` now includes `archive`.
- Updated tool descriptions for improved clarity and consistency.
- `execute_python` description in `list_tools()` no longer overridden; full detailed module description is preserved.

## [0.2.0] - 2026-04-22

### Added
- **About dialog in WebUI**: New "About" button and modal displaying software version, description, authors, and GitHub repository link
- **REST API**: New `GET /api/version` endpoint returning server version metadata
- **file_edit** tool: Precise in-file string replacement with `old_string`/`new_string`/`occurrence` parameters
- **base64_codec** tool: Merged `base64_encode` and `base64_decode` into a single tool with `operation` parameter
- **dir_list** enhancements: `pattern` (glob filter), `brief` mode, `sort_by`, default `max_depth` increased from 1 to 2
- **file_read** enhancements: Default `end_line` increased from 100 to 500; added `offset_chars`, `max_chars` (default 15000), and `line_numbers`
- **file_search** enhancements: Now returns content snippets with surrounding context lines instead of bare line numbers; added `file_pattern`, `use_regex`, `max_results`, `context_lines`, `brief`; search depth increased from 3 to 5; skips blacklisted directories
- **http_request** enhancements: Added `extract_json_path` (JSON Pointer), `include_response_headers`, `max_response_chars`
- **image_read** enhancements: Added `mode` parameter (`"full"` vs `"metadata"`) to avoid huge base64 transfers
- **System Metrics**: New `sysinfo`-based module for real-time CPU, memory, and load monitoring
- **REST API**: New `GET /api/system-metrics` endpoint returning live system resource usage
- **WebUI Overhaul**: Complete redesign into "Cyberpunk AI Command Center" theme with glassmorphism HUD, animated grid/particle background, terminal log panel, and 3D card tilt effects
- Directory blacklist in search operations: skips `.git`, `target`, `node_modules`, `__pycache__`, etc.
- Windows 7 cross-compilation instructions in README
- `--allowed-hosts` and `--disable-allowed-hosts` CLI options for DNS rebinding protection control
- Auto-detection of network interface IPs when `--mcp-host 0.0.0.0` is used

### Fixed
- **dir_list** `sort_entries` with `flatten=true` now correctly resolves relative paths when sorting by `size` or `modified`
- **image_read** `full` mode now returns standard MCP `ImageContent` (`type: "image"`) plus human-readable `TextContent` metadata, enabling vision-model clients (e.g., llama.cpp) to route images through their encoder instead of treating base64 as raw text tokens

### Changed
- Upgraded `rmcp` from 1.3.0 to 1.5.0 (with `allowed_hosts` DNS rebinding protection configured)
- Upgraded `reqwest` from 0.12 to 0.13
- Upgraded `schemars` from 1.0 to 1.1
- Default enabled tools: now 10 (`calculator`, `dir_list`, `file_read`, `file_search`, `image_read`, `file_stat`, `path_exists`, `json_query`, `git_ops`, `env_get`)

### Removed
- Standalone `base64_encode` and `base64_decode` tools (replaced by unified `base64_codec`)

## [0.1.0] - 2026-03-15

### Added
- Initial release of Rust MCP Server
- 18 built-in tools for file operations, system info, HTTP requests, and more
- WebUI control panel with real-time updates
- Multi-transport support (SSE and HTTP)
- Dangerous command blacklist with configurable IDs (20 commands)
- Command injection detection
- Two-step confirmation for dangerous operations
- Working directory restriction for file operations
- Audit logging for all command executions
- Internationalization support (English and Chinese)
- Comprehensive documentation

### Security
- Working directory restriction for all file operations
- Dangerous command blacklist (20 command patterns)
- Command injection pattern detection
- Two-step confirmation for suspicious commands
- Automatic pending command cleanup (5-minute timeout)

---

中文版本请查看 [CHANGELOG-zh.md](CHANGELOG-zh.md)
