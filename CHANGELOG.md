# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- New tool `execute_python`: Execute Python code in a RustPython interpreter with local filesystem access. Supports stdout/stderr capture, timeout control (1-30s), automatic last-line expression evaluation, and `__working_dir` injection. Marked as dangerous and disabled by default.

### Security
- `execute_python` is classified as a dangerous tool due to filesystem access capabilities. Enable with caution via WebUI or `--disable-tools` configuration.

## [0.2.0] - 2024-04-22

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

## [0.1.0] - 2024-03-15

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
