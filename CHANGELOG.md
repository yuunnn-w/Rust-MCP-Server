# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-05-11

### Added
- **Old format office support**: Read/Edit tools now support legacy .doc, .ppt, and .xls formats via LibreOffice auto-conversion (with native calamine support for .xls)
- **DOCX reading modes**: Three modes — `doc_text` (markdown output with headings/tables/formatting), `doc_with_images` (markdown with inline embedded images), `doc_images` (extracted images only)
- **PPTX slide-to-image mode**: `ppt_images` mode renders slides as PNG/JPG via LibreOffice, returning base64-encoded images as MCP ImageContent for vision model consumption
- **PPTX native image extraction (v0.4.0 update)**: `ppt_images` mode now falls back to pure Rust native extraction when LibreOffice is unavailable. Extracts embedded images from each PPTX slide via ZIP parsing and presents them alongside slide text — each slide first shows all extracted images, followed by text content. Requires no external dependencies.
- **FileStat office document stats**: `FileStat` full mode now returns `document_stats` for office files (DOCX/PPTX/PDF/XLSX) including `document_type`, `page_count`/`slide_count`/`sheet_count`, embedded `image_count`, and `text_char_count`. Enables LLMs to intelligently choose between text and image reading modes.
- **PDF page-to-image mode**: `pdf_images` mode renders each PDF page to PNG/JPG via **pdfium-render v0.9.1** (Chromium's PDF engine). The PDFium DLL/SO is embedded directly into the executable via `include_bytes!` with zstd compression, extracted to a temp directory on first use at runtime. `build.rs` auto-downloads PDFium binaries to `assets/pdfium/` if not present. Zero runtime external dependencies
- **Complex DOCX editing**: New `office_insert`, `office_replace`, `office_delete`, `office_insert_image`, `office_format`, `office_insert_table` modes for structured document manipulation via markdown
- **PDF editing via lopdf**: New `pdf_delete_page`, `pdf_insert_image`, `pdf_insert_text`, `pdf_replace_text` modes using pure Rust lopdf library
- **Markdown-based DOCX/PDF creation**: Write tool supports `office_markdown` parameter to create DOCX with headings/tables/formatting; PDF creation via LibreOffice
- **CSV-based XLSX creation**: Write tool supports `office_csv` parameter for multi-sheet spreadsheet creation
- **New dependencies**: `docx-rs` v0.4, `lopdf` v0.39, `pdfium-render` v0.9.1, `zstd` v0.13, `flate2` v1.0
- **LibreOffice availability detection**: Server detects LibreOffice at startup and reports availability
- **Archive tool AES-256 password encryption**: Added `password` parameter for creating and extracting password-protected ZIP archives
- **Grep tool brief output mode**: New `output_format: "brief"` returns file paths and line numbers only
- **Windows 7 native compatibility (self-contained)**: Removed `oldwin` crate. Replaced with direct integration of **VC-LTL5 v5.3.1** (`assets/vc-ltl/`, CRT replacement) and **YY-Thunks v1.2.1** (`assets/yy-thunks/`, Win8+ API stubs) via `build.rs`. Both are embedded in the repository. Verified by `YY.Depends.Analyzer.exe` targeting `6.1.7600` — zero missing API entries.
- **Windows version-aware `system_info`**: Detects Windows version at runtime using `RtlGetVersion`. On pre-Win10 systems, skips disk/network/temperature collection to avoid compatibility issues.
- **Windows Executable Icon**: Added application icon with rounded-corner transparency.
- **Custom System Prompt**: New `--system-prompt` CLI flag and `MCP_SYSTEM_PROMPT` env var. Configurable via WebUI.
- **Custom Shell Path**: New `shell_path` and `shell_arg` parameters for `Bash` tool.
- **Bilingual CLI Help**: `--help` shows both English and Chinese descriptions.
- **Static resource caching**: Cache headers and ETag support for static assets.
- **Frontend UX**: Filter controls, modal ESC close, and loading states.
- **PDFium asset restructuring**: Moved to `assets/pdfium/` directory. Auto-download supports direct library placement (`.dll`/`.so`/`.dylib`), `.tgz` archive extraction, and automatic download from pdfium-binaries with curl→PowerShell→wget fallback chain. Added **macOS** `libpdfium.dylib` support.

### Changed
- **DOCX library migration**: Replaced `docx-rust` with `docx-rs` for superior image extraction, style parsing, and formatting support
- **Read tool redesigned**: Mode system reorganized — `auto`/`text`/`media` for generic files, `doc_text`/`doc_with_images`/`doc_images` for DOC/DOCX, `ppt_text`/`ppt_images` for PPT/PPTX, `pdf_text`/`pdf_images` for PDF
- **Read tool expanded**: New parameters `image_dpi` and `image_format` for slide/page rendering control
- **Edit tool expanded**: Office format detection now includes .doc/.ppt/.xls; new complex editing parameters (`markdown`, `find_text`, `location`, `element_type`, `format_type`, `image_path`, `slide_index`, `page_index`)
- **Write tool expanded**: New `office_markdown` and `office_csv` parameters; PDF file_type support
- **Tool descriptions updated**: Read/Edit/Write descriptions in mod.rs, handler.rs, and WebUI reflect all new capabilities
- **Read tool image modes**: `media`, `pdf_images`, `ppt_images`, and `doc_images` now return base64-encoded image content via MCP `ImageContent` for direct vision model consumption (e.g., llama.cpp) instead of file path metadata
- **Read tool doc_with_images mode**: Images are now embedded inline at their document positions (marked with `{{IMAGE:N}}` markers in text) rather than appended at the end
- **Read tool line_numbers parameter**: Default changed from `true` to `false`
- **All tools parameter descriptions**: Now use `#[schemars(description)]` for complete JSON Schema coverage across all parameter fields
- **Read tool ppt_images mode**: No longer requires LibreOffice. When LibreOffice is unavailable, falls back to pure Rust native extraction (embedded images from PPTX ZIP + slide text via `ppt-rs`). Slides present images first, then text.
- **Read and FileStat tool descriptions**: Updated with detailed mode selection guidance, usage strategies, and office document metadata documentation.
- **FileStat office document support**: Now extracts and returns `document_stats` (document_type, page/slide/sheet count, image count, text character count) for DOCX/PPTX/PDF/XLSX files.
- **Tool Presets overhaul**: Redesigned the 6 presets so each preset now also controls `execute_python` filesystem access state. Server auto-applies the `minimal` preset on startup by default.
- **State optimization**: Optimized `state.rs` with atomic types to reduce lock contention.
- **Frontend performance**: Improved search debouncing and Canvas rendering performance.
- **Build system**: Removed `oldwin`/`oldwin-targets` dependencies. `build.rs` now directly links VC-LTL5 `.lib` files and YY-Thunks `.obj` files with explicit `/NODEFAULTLIB` link order control.
- **Description**: Shortened package description to "Rust Model Context Protocol (MCP) Server".

### Fixed
- **PPTX text extraction failure**: Fixed bug where slide text was empty for files using non-standard placeholder types. Switched to low-level `PresentationReader` API which extracts all shape text regardless of placeholder classification
- **Old format incompatibility**: .doc, .ppt files now readable via LibreOffice conversion; .xls files natively supported via calamine
- **DOCX image extraction**: Images embedded in DOCX files can now be extracted to temporary files with dimension/format metadata returned
- **PDF text extraction returns garbled characters**: Switched from `pdf` crate raw byte decoding to `lopdf::Document::extract_text()` which properly handles font encodings, ToUnicode CMaps, and CJK text
- **lopdf benign warnings suppressed**: Set `lopdf` log filter to `error` level to suppress benign Type3 font encoding warnings that cluttered log output
- Fixed broadcast receiver dying on lag, causing tool list change notifications to be lost
- Fixed line ending (\r\n) corruption in edit tool's line_replace, insert, and delete modes
- Fixed dead code in rename auto-rename conflict resolution
- Fixed potential OOM in archive.rs `add_dir_to_zip` and read.rs `offset_chars`
- Fixed enhanced_glob character class parsing that was incorrectly escaping `[]`
- Fixed `process_count` always returning 0 in system_metrics
- Fixed `data_analysis` preset missing Diff and Archive tools
- Fixed web handlers using `String` instead of `ApiError` for error responses
- Fixed XSS vulnerability in WebUI config rendering
- Fixed various `unwrap()` calls that could cause panics
- Fixed race condition in bash sync mode timeout handling
- Fixed temp file symlink attack vector in office_utils
- Fixed HTTP status code not being checked in web_fetch
- Fixed regex recompilation on every call in web_fetch and web_search
- Fixed SSE serialization failures producing silent errors
- **WebUI preset i18n**: Preset buttons now correctly display Chinese names
- **WebUI glassmorphism flickering**: Fixed backdrop-filter animation conflicts when moving mouse over modals
- **Note storage search**: Fixed note search not including tags and category
- **ZIP path traversal**: Fixed path traversal security vulnerability in archive extraction
- **Note content UTF-8 truncation**: Fixed panic when truncating note content at non-UTF-8 boundaries
- **Git diff paths**: Fixed incorrect file paths for git diff in subdirectories
- **Python thread leaks**: Fixed thread leaks during Python code execution
- **Async runtime blocking**: Fixed sync I/O blocking the async runtime in `image_read`, `file_search`, and `diff`
- **Handler task leaks**: Fixed task leaks in MCP handler on client reconnection
- **`set_level` implementation**: Fixed previously empty `set_level` implementation; now properly reloads the log filter
- **Documentation consistency audit**: Fixed multiple inconsistencies between documentation and code across all md files
- **PPTX relationship XML parsing**: Fixed regex requiring attribute order (Target before Type), causing "PPTX contains no slides" error on some files
- **`parse_relationship_targets`**: Rewrote to parse attributes independently regardless of XML attribute order
- **v0.4.0 audit fixes**:
  - Fixed missing `#[cfg(not(windows))]` stub in `windows_version.rs` causing compilation failure on Linux/macOS
  - Fixed WebP VP8L image dimension detection using wrong byte offsets
  - Fixed blocking `std::fs::read_dir` calls in handler.rs async context
  - Fixed hardcoded WebUI URL in MCP initialize instructions
  - Fixed TOCTOU race between tool enable check and concurrency permit acquisition
  - Unified tool descriptions between handler.rs and tools/mod.rs (11 inconsistencies)
  - Fixed `system_admin` preset being identical to `coding` preset
  - Removed dead `HttpRequest` reference from default_disable_tools
  - Fixed semaphore reduction silently failing under high load
  - Fixed `pdf_replace_text` ignoring `page_index` parameter
  - Fixed `edit_docx_insert` ignoring `find_text` and `location` parameters
  - Removed duplicate `read_docx` call in `edit_docx_insert`
  - Fixed N× temp file writes in PPTX text extraction
  - Added response size limit to `WebFetch` (50MB)
  - Wrapped blocking sysinfo calls in `spawn_blocking`
  - Fixed blocking I/O while holding config write lock in update_config
  - Added old format guard in `extract_text_from_bytes`
  - Fixed WebUI concurrency HUD always resetting to 0
  - Fixed WebUI terminal DOM memory leak
  - Fixed heading format occurrence tracking in `apply_format_to_docx`
  - Fixed PPTX shape double-replacement in `edit_pptx_string`
  - Fixed `shell_arg` validation in `resolve_shell`
  - Fixed `std::sync::Mutex` blocking in async context
  - Switched WebUI data loading from `Promise.all` to `Promise.allSettled`
  - Removed unused `chart.min.js` (200KB dead code)

### Removed
- **HttpRequest tool**: Deprecated as feature-incomplete; WebFetch covers web content fetching. Removed from mod.rs, handler.rs, presets.rs, and all presets. Tool count reduced from 22 to 21.
- **`oldwin` crate**: Replaced by direct VC-LTL5 + YY-Thunks integration in `build.rs`.
- **`chart.min.js`**: Removed unused 200KB Chart.js library. The WebUI chart is rendered via native Canvas API.
- **`.cargo/config.toml` `/FORCE:MULTIPLE` flag**: No longer needed after removing `oldwin`.

### Security
- NotebookEdit write operations now enforce working directory sandbox

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
