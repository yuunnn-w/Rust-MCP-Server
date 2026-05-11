# Architecture Overview

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Rust MCP Server v0.4.0                       │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │   WebUI (Axum)  │  │  MCP Service    │  │   Tool Registry     │  │
│  │                 │  │   (rmcp)        │  │                     │  │
│  │  ┌───────────┐  │  │                 │  │  ┌───────────────┐  │  │
│  │  │ Static    │  │  │  ┌──────────┐   │  │  │ Glob         │  │  │
│  │  │ Files     │  │  │  │ HTTP/SSE │   │  │  │ Read         │  │  │
│  │  └───────────┘  │  │  │ Transport│   │  │  │ Write        │  │  │
│  │  ┌───────────┐  │  │  └──────────┘   │  │  │ Bash         │  │  │
│  │  │ REST API  │  │  │                 │  │  │ ... (21 tools)│  │  │
│  │  │ /api/*    │  │  │                 │  │  └───────────────┘  │  │
│  │  └───────────┘  │  │                 │  │  └───────────────┘  │  │
│  │  ┌───────────┐  │  │                 │  │                     │  │
│  │  │ SSE       │  │  │                 │  │                     │  │
│  │  │ /events   │  │  │                 │  │                     │  │
│  │  └───────────┘  │  │                 │  │                     │  │
│  │  ┌───────────┐  │  │                 │  │                     │  │
│  │  │ System    │  │  │                 │  │                     │  │
│  │  │ Metrics   │  │  │                 │  │                     │  │
│  │  └───────────┘  │  │                 │  │                     │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │  ServerState    │  │  Configuration  │  │  Security Layer     │  │
│  │                 │  │                 │  │                     │  │
│  │  - Tool status  │  │  - CLI args     │  │  - Path validation  │  │
│  │  - Call stats   │  │  - Env vars     │  │  - Dangerous cmd    │  │
│  │  - Concurrent   │  │  - Defaults     │  │  - Injection check  │  │
│  │  - Pending cmds │  │  - Working dir  │  │  - Audit log        │  │
│  │  - Tool presets │  │                 │  │                     │  │
│  │  - Notes (mem)  │  │                 │  │                     │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Component Details

### WebUI (Axum Web Server)

The WebUI provides a modern control panel for managing the MCP server.

**Features:**
- **Static Files**: HTML, CSS, JS for the control panel (embedded at compile time)
- **REST API**: Endpoints for tool management and statistics
- **SSE Endpoint**: Real-time status updates (`/api/events`)

**API Endpoints:**
- `GET /api/tools` - List all tools with status
- `GET /api/tool/{name}/stats` - Get tool statistics
- `GET /api/tool/{name}/detail` - Get tool details
- `POST /api/tool/{name}/enable` - Enable/disable tool
- `POST /api/tools/batch-enable` - Batch enable/disable tools
- `GET /api/tool-presets` - List tool presets
- `GET /api/tool-presets/current` - Get active preset
- `POST /api/tool-presets/apply/{name}` - Apply a preset
- `GET /api/server-status` - Server runtime status
- `GET /api/system-metrics` - Get real-time CPU, memory, and load metrics
- `GET /api/version` - Get server version information
- `GET /api/config` - Get configuration
- `PUT /api/config` - Update configuration
- `POST /api/mcp/start` - Toggle `mcp_running` state flag to `true` (does not control actual process)
- `POST /api/mcp/stop` - Toggle `mcp_running` state flag to `false` (does not control actual process)
- `POST /api/mcp/restart` - Toggle `mcp_running` state flag (does not control actual process; full restart requires external process manager)
- `GET /api/python-fs-access` - Get `execute_python` filesystem access status
- `POST /api/python-fs-access` - Toggle `execute_python` filesystem access

**Default Bind Address**: `127.0.0.1:2233`

### MCP Service (rmcp)

Implements the Model Context Protocol using the `rmcp` crate.

**Transport Modes:**
- **HTTP**: JSON response mode (default)
- **SSE**: Server-Sent Events streaming mode

**Default Bind Address**: `127.0.0.1:3344`

**Protocol Support:**
- JSON-RPC 2.0
- MCP Protocol version 2024-11-05
- Tool discovery and invocation

### Tool Registry

21 built-in tools organized by category:

#### File Operations (6 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `Glob` | List directory contents with enhanced filtering (max depth 10). Supports multi-pattern glob/regex matching, exclude patterns, file size/time filters. Returns char_count and line_count for UTF-8 text files | No | No |
| `Read` | Read files with format auto-detection and mode system. Supports text/auto/media modes for generic files; doc_text/doc_with_images/doc_images for DOC/DOCX; ppt_text/ppt_images for PPT/PPTX; pdf_text/pdf_images for PDF. `ppt_images` uses LibreOffice (best quality) if available, or native pure Rust extraction (embedded images + text per slide) otherwise. Recommendation: use FileStat first to check document stats, then choose optimal mode. Image modes return base64-encoded ImageContent. | No | No |
| `Grep` | Search pattern in files with enhanced filtering (max depth 10). Supports regex, case-sensitive, whole-word, multiline modes. Searches office documents text content. | No | No |
| `Write` | Write content to files concurrently (create/append/overwrite). Supports creating DOCX (office_markdown), XLSX (office_csv), PDF (office_markdown), IPYNB files. | Yes | Yes |
| `FileOps` | Copy, move, delete, or rename files concurrently. Supports dry_run preview and conflict_resolution. | Yes | Yes |
| `Edit` | Multi-mode editing: string_replace, line_replace, insert, delete, patch. Complex office modes (office_insert, office_replace, office_delete, office_insert_image, office_format, office_insert_table) for DOCX. PDF modes (pdf_delete_page, pdf_insert_image, pdf_insert_text, pdf_replace_text) via lopdf. Supports .doc/.docx/.ppt/.pptx/.xls/.xlsx. | Yes | Yes |

#### Query & Data Tools (2 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `FileStat` | Get metadata for files/directories concurrently. mode="exist" for lightweight existence check. Full mode returns text file info for UTF-8 files plus `document_stats` for office files (DOCX/PPTX/PDF/XLSX) with page/slide/sheet count, embedded image count, and text character count. | No | No |
| `Git` | Run git commands (status, diff, log, branch, show). Supports path filtering and max_count. | No | No |

#### System Tools (3 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `SystemInfo` | Get system information including processes via sections parameter. Disk/network/temperature data omitted on legacy Windows. | No | No |
| `Bash` | Execute shell command with optional working_dir, stdin, async_mode. Use Monitor for async commands. | Yes | Yes |
| `ExecutePython` | Execute Python code. All Python standard library modules available. Filesystem access toggleable via WebUI. | No | Yes (when fs access enabled) |

#### Network Tools (1 tool)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `WebFetch` | Fetch and parse content from a URL with extract_mode (text/html/markdown) | No | No |

#### Utility Tools (4 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `Clipboard` | Read or write system clipboard content (text or image). Cross-platform. | No | No |
| `Archive` | Create, extract, list, or append ZIP archives with AES-256 password encryption. Supports deflate and zstd compression. | Yes | Yes |
| `Diff` | Compare text, files, or directories with multiple output formats. Supports ignore_blank_lines. | No | Yes (file/dir modes) |
| `NoteStorage` | AI short-term memory scratchpad with export/import. Notes auto-expire after 30 minutes. | No | No |

#### Web & Interaction Tools (2 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `WebSearch` | Search the web via DuckDuckGo with optional region/language filters | No | No |
| `AskUser` | Prompt the user for input or confirmation with optional timeout and default_value | No | No |

#### Task Management (1 tool)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `Task` | Unified task management with CRUD operations via operation parameter (create/list/get/update/delete) | No | No |

#### Office & Monitoring Tools (2 tools)
| Tool | Description | Dangerous | Working Dir Restriction |
|------|-------------|-----------|------------------------|
| `NotebookEdit` | Read, write, and edit Jupyter .ipynb notebook files. Write operations sandboxed to working directory. | Yes | Yes |
| `Monitor` | Monitor long-running Bash commands started with async=true. Operations: stream, wait, signal. | No | No |

### Resources

The server exposes the working directory as an MCP resource (`file:///`). Clients can:
- **List resources**: Get the working directory entry with `uri`, `name`, and `description`
- **Read resources**: Fetch directory listings or file contents via `file:///{relative_path}`

Resource contents are returned as `TextResourceContents` with MIME type `text/plain`.

### Prompts

3 built-in prompts available for client use:

| Prompt | Description |
|--------|-------------|
| `system_diagnosis` | Guide for analyzing system information and identifying issues |
| `file_analysis` | Guide for analyzing code files and directory structures |
| `security_checklist` | Checklist to review before executing dangerous operations |

Prompts are retrieved via `prompts/get` and return a list of `PromptMessage` with `User` role.

### ServerState

Shared state across all components using `Arc` for thread-safe sharing.

**Components:**
- **Tool Registry**: HashMap of tool names to their metadata
- **Statistics**: Call counts, history, recent calls
- **Concurrency Control**: Semaphore for limiting concurrent calls
- **Pending Commands**: DashMap for storing commands awaiting confirmation

### Security Layer

Multi-layer security system:

1. **Path Validation**: Canonicalization and working directory check
2. **Dangerous Command Detection**: 20 configurable dangerous command patterns
3. **Injection Detection**: Shell metacharacter detection
4. **Two-Step Confirmation**: User confirmation for dangerous operations
5. **Audit Logging**: All command executions logged

## Data Flow

### Tool Execution Flow

```
1. Client sends tool call request (via MCP protocol)
2. MCP Handler receives and parses request
3. Check if tool is enabled in ServerState
4. Acquire concurrency permit from semaphore
5. Route to tool implementation
6. Execute security checks (path/command validation)
7. Execute tool logic with timeout protection
8. Record statistics and update state
9. Release concurrency permit
10. Return result to client
11. Trigger SSE update for WebUI
```

### WebUI Update Flow

```
1. Tool execution updates ServerState
2. State change triggers SSE broadcast
3. Connected WebUI clients receive update
4. UI components refresh automatically
```

### Command Execution Security Flow

```
Command Input
    ↓
Working Directory Validation
    ↓
Dangerous Command Check (Blacklist: 20 patterns)
    ↓
Injection Pattern Detection (metacharacters)
    ↓
Two-Step Confirmation (if needed)
    ↓
Execute with Audit Log + Timeout
    ↓
Output Truncation (100KB limit)
```

## Configuration System

**Sources** (in order of precedence):
1. Command line arguments (highest priority)
2. Environment variables
3. Default values (lowest priority)

**Key Configuration Options:**

| Option | CLI Flag | Env Variable | Default |
|--------|----------|--------------|---------|
| WebUI Host | `--webui-host` | `MCP_WEBUI_HOST` | `127.0.0.1` |
| WebUI Port | `--webui-port` | `MCP_WEBUI_PORT` | `2233` |
| MCP Transport | `--mcp-transport` | `MCP_TRANSPORT` | `http` |
| MCP Host | `--mcp-host` | `MCP_HOST` | `127.0.0.1` |
| MCP Port | `--mcp-port` | `MCP_PORT` | `3344` |
| Max Concurrency | `--max-concurrency` | `MCP_MAX_CONCURRENCY` | `10` |
| Working Directory | `--working-dir` | `MCP_WORKING_DIR` | `.` |
| Log Level | `--log-level` | `MCP_LOG_LEVEL` | `info` |
| Disabled Tools | `--disable-tools` | `MCP_DISABLE_TOOLS` | See below |
| Dangerous Commands | `--allow-dangerous-commands` | `MCP_ALLOW_DANGEROUS_COMMANDS` | (none) |

**Tool Presets:**
The server starts with the `minimal` preset by default (9 tools enabled, `execute_python` sandboxed). Available presets:
- `minimal`: 9 tools, `ExecutePython` fs=false
- `coding`: 20 tools, `ExecutePython` fs=true
- `data_analysis`: 15 tools, `ExecutePython` fs=true
- `system_admin`: 20 tools, `ExecutePython` fs=true
- `research`: 10 tools, `ExecutePython` fs=false
- `full_power`: 21 tools, `ExecutePython` fs=true

Use `--preset <name>` to set the startup preset, or `--preset none` to skip auto-applying.

## Technology Stack

- **Runtime**: Tokio (async runtime)
- **Web Framework**: Axum
- **MCP Protocol**: rmcp crate
- **Serialization**: serde + serde_json
- **Logging**: tracing + tracing-subscriber
- **CLI Parsing**: clap
- **Concurrency**: tokio::sync (Semaphore, RwLock)
- **Collections**: dashmap (concurrent HashMap)
- **Office Documents**: docx-rs v0.4 (DOCX reading/writing), lopdf v0.39 (PDF editing/text extraction), calamine (XLS/XLSX reading)
- **PDF Rendering**: pdfium-render v0.9.1 (PDF page-to-image rendering; PDFium library pre-bundled in assets/, auto-downloaded by build.rs if missing)
- **Document Conversion**: LibreOffice (detected at startup, used for legacy .doc/.ppt and PPTX slide rendering — PPTX now has pure Rust native fallback when LibreOffice unavailable)
- **System Metrics**: sysinfo (CPU, memory, process monitoring)

---

中文版本请查看 [architecture-zh.md](architecture-zh.md)
