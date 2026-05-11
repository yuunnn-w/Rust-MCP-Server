# User Guide

## Table of Contents

- [Getting Started](#getting-started)
- [Using WebUI](#using-webui)
- [Tool Reference](#tool-reference)
- [Configuration](#configuration)
- [Security Features](#security-features)
- [Troubleshooting](#troubleshooting)

## Getting Started

### Installation

Download the latest release for your platform from [GitHub Releases](https://github.com/yuunnn-w/Rust-MCP-Server/releases).

Or build from source:
```bash
git clone https://github.com/yuunnn-w/Rust-MCP-Server.git
cd Rust-MCP-Server
./scripts/build-unix.sh  # Linux/macOS
# or
scripts\build-windows.bat  # Windows
```

### First Run

```bash
# Start with default settings
./rust-mcp-server

# Access WebUI
open http://127.0.0.1:2233  # macOS
# or
start http://127.0.0.1:2233  # Windows
```

### Connecting from MCP Client

Configure your MCP client to connect to:
- **HTTP Transport:** `http://127.0.0.1:3344`
- **SSE Transport:** `http://127.0.0.1:3344/sse`

Example configuration for Claude Desktop:
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

## Using WebUI

### Dashboard Overview

The WebUI provides a Cyberpunk AI Command Center:
- **HUD Header**: Live system metrics (CPU ring gauge, memory bar, total calls, concurrency)
- **Tool Grid**: View and manage all tools with 3D tilt hover effects and neon accent borders
- **Terminal Log Panel**: Bottom panel streaming SSE events with colored log levels (INFO/WARN/ERROR)
- **Real-time Updates**: SSE-based live status updates for tool calls, concurrency, and MCP service status
- **Animated Background**: Canvas-based perspective grid and floating particle network

### Enabling/Disabling Tools

**Important:** The server starts with the `minimal` preset by default, which enables 9 safe tools including `ExecutePython` (sandboxed, no filesystem access). You can switch presets via the WebUI sidebar or the `--preset` CLI option. Individual tools can still be toggled independently.

1. Open WebUI at `http://127.0.0.1:2233`
2. Find the tool card in the grid
3. Toggle the switch on the tool card
4. Changes take effect immediately (no restart needed)

### Tool Presets

The sidebar provides **Tool Presets** for one-click configuration:
- **minimal**: Safe read-only tools + sandboxed Python (9 tools, `ExecutePython` fs=false)
- **coding**: Development-focused tools including file editing, task management, and command execution (20 tools, `ExecutePython` fs=true)
- **data_analysis**: Data analysis tools including Python, Diff, Archive, and web tools (15 tools, `ExecutePython` fs=true)
- **system_admin**: System administration tools including system info, process list, command execution (20 tools, `ExecutePython` fs=true)
- **research**: Research & documentation tools including web search, web fetch, and file reading (10 tools, `ExecutePython` fs=false)
- **full_power**: All 21 tools enabled (`ExecutePython` fs=true)

Click any preset button in the sidebar to apply it. The currently active preset is displayed above the preset grid.

### Batch Actions

Use the **Batch Actions** buttons in the sidebar to quickly:
- **Enable All**: Enable all tools at once
- **Disable All**: Disable all tools at once

### Viewing Tool Statistics

Click on any tool card to see:
- Total call count
- Recent calls (last 15 minutes)
- Usage chart (last 2 hours, 5-minute intervals)
- Recent call timestamps
- Tool description and usage

### Tool Status Indicators

- **Idle**: Tool is available for use
- **Calling...**: Tool is currently being called (stays for 5 seconds after completion)
- **Disabled**: Tool is turned off
- **Dangerous**: Marked with warning icon (requires careful use)

## Tool Reference

### File Operations

#### Glob
List directory contents with tree structure or flat list.

**Parameters:**
- `path` (string): Directory path (default: current directory)
- `max_depth` (number, optional): Maximum recursion depth (default: 2, max: 5)
- `include_hidden` (boolean, optional): Include hidden files (default: false)
- `pattern` (string, optional): Glob pattern to filter entries, e.g. `"*.rs"`
- `brief` (boolean, optional): Brief mode — only return name, path, is_dir (default: true)
- `sort_by` (string, optional): Sort by `"name"` (default), `"type"`, `"size"`, `"modified"`
- `flatten` (boolean, optional): Return flat list instead of nested tree (default: false)

**Example:**
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
Read files with format auto-detection and mode system. Supports text files, office documents, PDFs, and images.

**Mode System:**
- `auto` (default): Auto-detect file type and choose appropriate mode
- `text`: Read as plain text with line numbers, highlighting, character offsets
- `media`: Read image and return base64-encoded image content for vision models (llama.cpp etc.)
- `doc_text`: Read DOC/DOCX as markdown with headings, tables, and formatting
- `doc_with_images`: Read DOC/DOCX as markdown with images embedded inline at their positions
- `doc_images`: Extract images only from DOC/DOCX files
- `ppt_text`: Read PPT/PPTX text using PresentationReader (extracts ALL shape text)
- `ppt_images`: Slides as images. Uses LibreOffice (best quality) if installed; otherwise native pure Rust extraction (embedded images + text per slide). Works without any external dependencies.
- `pdf_text`: Extract text from PDF files
- `pdf_images`: Render PDF pages as images via PDFium (embedded in binary), returning base64 image content

**Mode Selection Strategy:**
1. Use **FileStat** first to check document stats (slide_count, image_count, text_char_count)
2. If `image_count > 0` and you need visuals → use `{pdf,ppt,doc}_images`
3. If `text_char_count` is large and no images needed → use `{pdf,ppt,doc}_text`
4. For PPTX without LibreOffice: `ppt_images` still works via native extraction

**Parameters:**
- `path` (string): File path (auto mode)
- `mode` (string, optional): Reading mode (default: `"auto"`)
- `files` (array, optional): Batch read mode - list of files to read
  - `path` (string): File path
  - `start_line` (number, optional): Start line (0-indexed, default: 0)
  - `end_line` (number, optional): End line (default: 500)
  - `offset_chars` (number, optional): Character offset to start reading (alternative to start_line)
  - `max_chars` (number, optional): Maximum characters to return (default: 15000)
  - `line_numbers` (boolean, optional): Prefix each line with its line number (default: false)
  - `highlight_line` (number, optional): Highlight a specific line with `>>>` marker (1-based)
- `image_dpi` (number, optional): DPI for slide/page image rendering (default: 150)
- `image_format` (string, optional): Image format for rendering: `"png"` (default) or `"jpg"`

**Features:**
- Auto-detection of text, image, and office document formats
- Mode-based reading with specialized handlers per format
- Office documents (.doc/.docx/.ppt/.pptx/.xls/.xlsx/.pdf) supported
- Image output returns base64-encoded image content for direct vision model consumption via MCP ImageContent
- Batch mode: read multiple text files concurrently
- 15KB character limit per read (configurable via `max_chars`)
- Automatic truncation with precise continuation hints
- Returns total line/char count per file

**Example:**
```json
// Read a DOCX file as markdown
{
  "path": "document.docx",
  "mode": "doc_text"
}

// Read PDF pages as images
{
  "path": "document.pdf",
  "mode": "pdf_images",
  "image_dpi": 200,
  "image_format": "png"
}

// Batch read text files
{
  "mode": "text",
  "files": [
    {
      "path": "config.json",
      "start_line": 0,
      "end_line": 500,
      "line_numbers": true,
      "highlight_line": 42
    },
    {
      "path": "src/main.rs",
      "start_line": 0,
      "end_line": 100
    }
  ]
}
```
```

#### Write
Write content to one or more files concurrently (dangerous). Supports plain text and office document creation.

**Parameters:**
- `files` (array): List of files to write
  - `path` (string): File path
  - `content` (string): Content to write
  - `mode` (string, optional): "new" | "append" | "overwrite" (default: "new")
- `file_type` (string, optional): Force file type: `"pdf"` for PDF creation via LibreOffice
- `office_markdown` (boolean, optional): Treat content as markdown when creating DOCX (supports headings, tables, formatting)
- `office_csv` (boolean, optional): Treat content as CSV when creating XLSX (multi-sheet support)

**Features:**
- Create DOCX files with markdown formatting (headings, bold, italic, tables)
- Create XLSX spreadsheet files from CSV data (multi-sheet)
- Create PDF files from markdown via LibreOffice
- Plain text file creation with new/append/overwrite modes
- Working directory restriction enforced

**Example:**
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
      "content": "Log entry\n",
      "mode": "append"
    }
  ]
}
```

#### Grep
Search for keywords in files or directories.

**Parameters:**
- `path` (string): File or directory path
- `keyword` (string): Search keyword
- `file_pattern` (string, optional): Glob pattern to filter files, e.g. `"*.rs"`
- `use_regex` (boolean, optional): Use regex matching (default: false)
- `max_results` (number, optional): Maximum match results to return (default: 20)
- `context_lines` (number, optional): Context lines around each match (default: 3)
- `brief` (boolean, optional): Brief mode — only return file paths and line numbers (default: false)
- `output_format` (string, optional): `"detailed"` (default), `"compact"`, or `"location"`

**Features:**
- Recursive directory search (max depth: 5)
- Returns matching content snippets with surrounding context
- Regex and literal keyword support
- Skips binary files and blacklisted directories
- Warns about deeper directories not searched
- Compact mode returns only `file:line:matched_text`
- Location mode returns only `file:line` (minimal token usage)

**Example:**
```json
{
  "path": "/project/src",
  "keyword": "TODO",
  "file_pattern": "*.rs",
  "context_lines": 3,
  "max_results": 10,
  "output_format": "compact"
}
```

#### Edit
Multi-mode file editing — string replacement, line-based operations, unified Diff patch, complex office document manipulation, and PDF editing. Supports concurrent batch operations and creating new files.

**Parameters:**
- `operations` (array): List of edit operations
  - `path` (string): File path to edit
  - `mode` (string, optional): Edit mode (see below)

**Text Edit Modes:**

`string_replace` mode:
- `old_string` (string): String to find (exact match, can span multiple lines)
- `new_string` (string): Replacement string
- `occurrence` (number, optional): Which occurrence to replace — `1`=first (default), `2`=second, `0`=replace all

`line_replace` / `insert` / `delete` mode:
- `start_line` (number): Start line (1-based, inclusive)
- `end_line` (number): End line (1-based, inclusive). Not used for insert.
- `new_string` (string): Content for replacement or insertion

`patch` mode:
- `patch` (string): Unified Diff patch string

**Complex Office Modes (DOCX):**

`office_insert` mode:
- `markdown` (string): Markdown content to insert into the document

`office_replace` mode:
- `find_text` (string): Text to find in the document
- `markdown` (string): Markdown replacement content

`office_delete` mode:
- `find_text` (string): Text to find and delete from the document

`office_insert_image` mode:
- `image_path` (string): Path to image file to insert
- `location` (string, optional): Where to insert — `"end"` (default) or `"after"` with find_text

`office_format` mode:
- `find_text` (string): Text to apply formatting to
- `element_type` (string): Element type to format (e.g., `"paragraph"`, `"table"`)
- `format_type` (string): Formatting operation type

`office_insert_table` mode:
- `location` (string, optional): Where to insert — `"end"` (default)
- `markdown` (string): Markdown table content to insert

**PDF Edit Modes:**

`pdf_delete_page` mode:
- `page_index` (number): Zero-based page index to delete

`pdf_insert_image` mode:
- `page_index` (number): Page to insert image on
- `image_path` (string): Path to image file to insert
- `location` (string, optional): Where to insert — `"end"` (default)

`pdf_insert_text` mode:
- `page_index` (number): Page to insert text on
- `markdown` (string): Text content to insert

`pdf_replace_text` mode:
- `page_index` (number): Page to replace text on
- `find_text` (string): Text to find
- `markdown` (string): Replacement text

**Features:**
- Text modes: string_replace, line_replace, insert, delete, patch — create new files or edit existing
- Complex DOCX modes: structured document manipulation via markdown
- PDF editing via pure Rust lopdf library (no external dependencies)
- Legacy format support: .doc, .ppt, .xls via LibreOffice auto-conversion
- All modes return operation summary with preview
- Multiple operations can be performed concurrently

**Examples:**
```json
// Single operation - text edit
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

// Concurrent batch operations
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
      "new_string": "use std::collections::HashMap;"
    }
  ]
}

// Create new file
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

// Office document editing
{
  "operations": [
    {
      "path": "document.docx",
      "mode": "office_replace",
      "find_text": "old section",
      "markdown": "# New Section\n\nUpdated content with **formatting**"
    },
    {
      "path": "document.docx",
      "mode": "office_insert_table",
      "location": "end",
      "markdown": "| Name | Value |\n|------|-------|\n| A    | 1     |"
    }
  ]
}

// PDF editing
{
  "operations": [
    {
      "path": "document.pdf",
      "mode": "pdf_replace_text",
      "page_index": 0,
      "find_text": "April",
      "markdown": "May"
    },
    {
      "path": "document.pdf",
      "mode": "pdf_delete_page",
      "page_index": 5
    }
  ]
}
```

### System Tools

#### Bash
Execute shell commands with security checks (dangerous).

**Parameters:**
- `command` (string): Command to execute
- `cwd` (string, optional): Working directory (default: current)
- `timeout` (number, optional): Timeout in seconds (default: 30, max: 300)
- `env` (object, optional): Environment variables as key-value pairs
- `shell` (string, optional): Shell interpreter — `"cmd"` (default Windows), `"powershell"`, `"pwsh"`, `"sh"` (default Unix), `"bash"`, `"zsh"`
- `shell_path` (string, optional): Custom shell executable path (e.g., `C:\Tools\pwh.exe`). Overrides `shell` when provided.
- `shell_arg` (string, optional): Custom shell argument (e.g., `-Command`, `/C`). Inferred from shell type if not provided.

**Security:**
- Dangerous commands require two-step confirmation
- Shell injection patterns detected
- Output limited to 100KB

**Example:**
```json
{
  "command": "ls -la",
  "cwd": "/home/user",
  "timeout": 30,
  "shell": "bash"
}
```

#### ExecutePython
Execute Python code for calculations, data processing, and logic evaluation. **All Python standard library modules are available.**

**Sandbox Mode (Default):**
- Filesystem operations are disabled (`builtins.open`, `_io.open`, `_io.FileIO`, and `os` filesystem functions are blocked)
- Network modules (`socket`, `urllib`, `http`, `ssl`) and data processing modules remain fully functional
- If a filesystem operation is attempted, the error message will indicate that the execution is in sandbox mode
- `subprocess` and `ctypes` are blocked as a security baseline
- Assign return value to `__result`; last non-comment line auto-evaluates if `__result` is not set
- Execution timeout uses trace-based self-termination inside the VM

**Filesystem Mode:**
- Enable via WebUI "Filesystem" toggle on the `ExecutePython` card
- When enabled, `__working_dir` is injected into globals
- `open()` and `os` filesystem functions are wrapped to restrict paths to the configured working directory
- All Python standard library modules including network and filesystem modules are available

**Parameters:**
- `code` (string): Python code to execute (max 10,000 characters)
- `timeout_ms` (number, optional): Timeout in milliseconds (default: 5000, max: 30000)

**Returns:**
- `result`: Value of `__result` variable (or auto-evaluated last line expression)
- `stdout`: Captured standard output
- `stderr`: Captured standard error / hints
- `execution_time_ms`: Execution duration in milliseconds

**Notes:**
- Assign the desired return value to `__result`
- If `__result` is not set, the last non-comment line is automatically evaluated as an expression
- The global variable `__working_dir` contains the server working directory when filesystem access is enabled
- All Python standard library modules are available regardless of mode

**Example:**
```json
{
  "code": "import math\n__result = math.pi * 2",
  "timeout_ms": 5000
}
```

#### SystemInfo
Get comprehensive system information including processes via `sections` parameter.

**Returns:**
- OS name, version, detailed version, distribution ID, kernel version, hostname
- CPU architecture, logical count, physical core count, brand, frequency (MHz), usage (%)
- Memory: total, used, free (MB), usage (%)
- Swap: total, used, free (MB), usage (%)
- System uptime (seconds), boot time (Unix timestamp)
- Load average (1min, 5min, 15min) — Unix only
- Disks: name, mount point, file system, type (HDD/SSD), total/available (GB), usage (%), removable, read-only
- Network interfaces: name, MAC address, IP addresses (CIDR), MTU, total received/transmitted (MB)
- Hardware temperature: component label, current/max/critical temperature (°C) where available

> **Platform Note**: On Windows versions older than Windows 10, the `disks`, `network_interfaces`, and `components` fields will be empty arrays to avoid compatibility issues. All other fields (CPU, memory, OS info) remain fully populated.

All floating-point values are rounded to 2 decimal places.

### Utility Tools

### Clipboard & Archive Tools

#### Clipboard
Read or write system clipboard content. Supports text and image operations.

**Parameters:**
- `operation` (string): `"read_text"`, `"write_text"`, `"read_image"`, or `"clear"`
- `text` (string, optional): Text to write (required for `write_text`)

**Example:**
```json
{"operation": "read_text"}
{"operation": "write_text", "text": "Hello, World!"}
{"operation": "clear"}
```

#### Archive
Create, extract, list, or append ZIP archives with AES-256 password encryption. All paths are restricted to the working directory.

**Parameters:**
- `operation` (string): `"create"`, `"extract"`, `"list"`, or `"append"`
- `archive_path` (string): Path to the ZIP archive
- `source_paths` (array, optional): Files/directories to include (for `create`/`append`)
- `destination` (string, optional): Extract destination (for `extract`, defaults to working directory)
- `compression_level` (number, optional): 1-9 (default: 6, only for `create`)
- `password` (string, optional): Password for AES-256 encryption/decryption

**Example:**
```json
{"operation": "create", "archive_path": "backup.zip", "source_paths": ["src", "Cargo.toml"]}
{"operation": "extract", "archive_path": "backup.zip", "destination": "./extracted"}
{"operation": "list", "archive_path": "backup.zip"}
```

### Diff & Note Tools

#### Diff
Compare text, files, or directories with multiple output formats.

**Parameters:**
- `operation` (string): `"compare_text"`, `"compare_files"`, `"directory_Diff"`, or `"git_Diff_file"`
- `old_text` / `new_text` (string, optional): For `compare_text`
- `old_path` / `new_path` (string, optional): For `compare_files` / `directory_Diff`
- `file_path` (string, optional): For `git_Diff_file` (compares working copy vs HEAD)
- `output_format` (string, optional): `"unified"` (default), `"side_by_side"`, `"summary"`, or `"inline"`
- `context_lines` (number, optional): 1-20 (default: 3)
- `ignore_whitespace` (boolean, optional): Default false
- `ignore_case` (boolean, optional): Default false
- `max_output_lines` (number, optional): Default 500
- `word_level` (boolean, optional): Enable word-level inline highlighting (default: true)

**Example:**
```json
{"operation": "compare_text", "old_text": "foo\nbar", "new_text": "foo\nbaz", "output_format": "unified"}
{"operation": "git_Diff_file", "file_path": "src/main.rs"}
```

#### NoteStorage
AI short-term memory scratchpad with export/import. Notes are stored only in memory and auto-cleared after 30 minutes of inactivity.

**Limits:** Max 100 notes, 50,000 chars per note, 200 chars per title, 10 tags per note.

**Parameters:**
- `operation` (string): `"create"`, `"list"`, `"read"`, `"update"`, `"delete"`, `"search"`, `"append"`, `"export"`, or `"import"`
- `id` (number, optional): Note ID (for `read`/`update`/`delete`/`append`)
- `title` (string, optional): For `create`/`update`
- `content` (string, optional): For `create`/`update`
- `tags` (array, optional): For `create`/`update`
- `category` (string, optional): For `create`/`update`/`list`
- `query` (string, optional): For `search`
- `append_content` (string, optional): For `append`

**Example:**
```json
{"operation": "create", "title": "User prefers dark mode", "content": "...", "tags": ["preference"], "category": "user_prefs"}
{"operation": "search", "query": "preference"}
```

### Development Tools

#### FileStat
Get metadata for files/directories. Use `mode="exist"` for lightweight existence check (replaces `path_exists`).

**Parameters:**
- `paths` (array): List of file or directory paths
- `mode` (string, optional): `"full"` (default) or `"exist"` (lightweight check)

**Returns (full mode):**
- `name`, `path`, `exists`
- `file_type`: `"file"`, `"directory"`, `"symlink"`, or `"unknown"`
- `size`: Size in bytes
- `size_human`: Human-readable size string
- `is_text`: Whether the file is a valid UTF-8 text file
- `char_count`: Number of characters (for text files)
- `line_count`: Number of lines (for text files)
- `encoding`: Detected encoding (e.g., "utf-8")
- `readable`, `writable`, `executable`: Permission booleans
- `modified`, `created`, `accessed`: Timestamp strings
- `is_symlink`: Whether the path is a symbolic link
- `document_stats` (office files only): Document metadata including:
  - `document_type`: `"docx"`, `"pptx"`, `"pdf"`, or `"xlsx"`
  - `slide_count` / `page_count` / `sheet_count`: number of slides/pages/sheets
  - `image_count`: number of embedded images
  - `text_char_count`: total text character count

**Returns (exist mode):**
- `exists` (boolean)
- `path_type`: `"file"`, `"dir"`, `"symlink"`, or `"none"`

**Example:**
```json
{"paths": ["src/main.rs"]}
```

#### Git
Run git commands in a repository with `path` and `max_count` filters.

**Parameters:**
- `action` (string): `"status"`, `"diff"`, `"log"`, `"branch"`, or `"show"`
- `repo_path` (string, optional): Repository path (default: working directory)
- `path` (string, optional): Filter by file path
- `max_count` (number, optional): Max log entries

**Example:**
```json
{"action": "log", "options": ["--oneline", "-n", "10"]}
```

### Task, Web & Interaction Tools

#### Task
Unified task management with CRUD operations via `operation` parameter.

**Parameters:**
- `operation` (string): `"create"`, `"list"`, `"get"`, `"update"`, or `"delete"`

**create operation:**
- `title` (string): Task title
- `description` (string, optional): Task description
- `priority` (string, optional): `"low"`, `"medium"`, `"high"` (default: `"medium"`)
- `tags` (array of strings, optional): Tags for categorization

**list operation:**
- `status` (string, optional): Filter by `"pending"`, `"in_progress"`, `"completed"`
- `priority` (string, optional): Filter by `"low"`, `"medium"`, `"high"`
- `tags` (array of strings, optional): Filter by tags

**get operation:**
- `id` (number): Task ID to retrieve

**update operation:**
- `id` (number): Task ID
- `title` (string, optional): New title
- `description` (string, optional): New description
- `status` (string, optional): New status
- `priority` (string, optional): New priority
- `tags` (array of strings, optional): New tags

**delete operation:**
- `ids` (array of numbers): Task IDs to delete

**Examples:**
```json
{"operation": "create", "title": "Implement login page", "priority": "high", "tags": ["frontend", "auth"]}
{"operation": "list", "status": "pending", "priority": "high"}
{"operation": "get", "id": 1}
{"operation": "update", "id": 1, "status": "completed"}
{"operation": "delete", "ids": [1, 2, 3]}
```

#### WebSearch
Search the web via DuckDuckGo with optional `region` and `language` filters.

**Parameters:**
- `query` (string): Search query
- `max_results` (number, optional): Maximum results (default: 10, max: 20)
- `region` (string, optional): Region code (e.g., `"us-en"`, `"cn-zh"`)
- `language` (string, optional): Language code (e.g., `"en"`, `"zh"`)

**Example:**
```json
{"query": "Rust MCP server tutorial", "max_results": 5}
```

#### WebFetch
Fetch and parse content from a URL with `extract_mode` (text/html/markdown).

**Parameters:**
- `url` (string): URL to fetch
- `max_chars` (number, optional): Maximum response characters (default: 15000)
- `extract_mode` (string, optional): `"text"`, `"html"`, or `"markdown"`

**Example:**
```json
{"url": "https://example.com/article", "max_chars": 5000}
```

#### AskUser
Prompt the user for input or confirmation with `timeout` and `default_value`.

**Parameters:**
- `message` (string): Message to display to the user
- `type` (string, optional): `"confirm"` or `"input"` (default: `"confirm"`)
- `timeout` (number, optional): Timeout in seconds
- `default_value` (string, optional): Default value for input mode

**Example:**
```json
{"message": "Do you want to proceed with the installation?", "type": "confirm"}
```

### Office & Monitoring Tools

#### NotebookEdit
Read, write, and edit Jupyter .ipynb notebook files. Write operations are sandboxed to working directory.

**Parameters:**
- `operation` (string): `"read"`, `"write"`, `"add_cell"`, `"edit_cell"`, or `"delete_cell"`
- `path` (string): Path to the .ipynb file

**read operation:** Returns notebook cells and metadata.

**write operation:**
- `cells` (array): List of cell objects with `cell_type`, `source`, and optional `outputs`

**add_cell operation:**
- `cell` (object): Cell to add with `cell_type`, `source`
- `index` (number, optional): Insert position (default: end)

**edit_cell operation:**
- `index` (number): Cell index to edit
- `source` (string): New cell source

**delete_cell operation:**
- `index` (number): Cell index to delete

**Example:**
```json
{"operation": "read", "path": "notebook.ipynb"}
```

#### Monitor
Monitor long-running Bash commands started with `async=true`. Operations: stream, wait, signal.

**Parameters:**
- `operation` (string): `"stream"`, `"wait"`, or `"signal"`
- `id` (string): Command ID from async Bash execution
- `signal` (string, optional): Signal to send (`"SIGTERM"`, `"SIGKILL"`, `"SIGINT"`)

**Example:**
```json
{"operation": "stream", "id": "cmd_abc123"}
```

### Command Line Options

```bash
./rust-mcp-server [OPTIONS]

Options:
      --webui-host <HOST>              WebUI listening address [default: 127.0.0.1]
      --webui-port <PORT>              WebUI listening port [default: 2233]
      --mcp-transport <TRANSPORT>      MCP transport: http or sse [default: http]
      --mcp-host <HOST>                MCP service listening address [default: 127.0.0.1]
      --mcp-port <PORT>                MCP service listening port [default: 3344]
      --max-concurrency <NUM>          Maximum concurrent tool calls [default: 10]
      --disable-tools <TOOLS>          Comma-separated list of tools to disable
      --working-dir <PATH>             Working directory for file operations [default: .]
      --log-level <LEVEL>              Log level: trace, debug, info, warn, error [default: info]
      --disable-webui                  Disable WebUI control panel
      --preset <PRESET>              Tool preset on startup: minimal/coding/data_analysis/system_admin/research/full_power/none [default: minimal]
      --system-prompt <PROMPT>       Custom system prompt passed to LLM via MCP instructions
      --allow-dangerous-commands <IDS> Allowed dangerous command IDs (1-20)
      --allowed-hosts <HOSTS>          Custom allowed Host headers for DNS rebinding protection (comma-separated)
      --disable-allowed-hosts          Disable allowed_hosts check (NOT recommended for public deployments)
  -h, --help                           Print help
  -V, --version                        Print version
```

### Environment Variables

All CLI options can be set via environment variables:

```bash
export MCP_WEBUI_PORT=8080
export MCP_MAX_CONCURRENCY=20
export MCP_LOG_LEVEL=debug
export MCP_DISABLE_TOOLS="Bash,SystemInfo"
export MCP_ALLOWED_HOSTS="192.168.1.100,example.com"
./rust-mcp-server
```

### Configuration File

Create `.env` file in project root:

```
MCP_WEBUI_PORT=8080
MCP_MAX_CONCURRENCY=20
MCP_WORKING_DIR=/safe/path
MCP_DISABLE_TOOLS=Write,Bash
```

## Security Features

### Working Directory Restriction

Read-only file tools (`Glob`, `Read`, `Grep`, `FileStat`, `Git`) are **not** restricted to the working directory.

Write operations (`Write`, `Edit`, `FileOps`) and execution tools (`Bash`, `ExecutePython`) are restricted to the configured working directory:

```bash
./rust-mcp-server --working-dir /var/mcp-safe
```

Path traversal attempts (`../`) are blocked for restricted tools.

### Dangerous Command Blacklist

The `Bash` tool blocks 20 dangerous command patterns by default:

| ID | Command | Description |
|----|---------|-------------|
| 1 | rm | Delete files (Linux) |
| 2 | del | Delete files (Windows) |
| 3 | format | Format disk |
| 4 | mkfs | Create filesystem |
| 5 | dd | Disk copy |
| 6 | :(){:|:&};: | Fork bomb |
| 7 | eval | Code execution |
| 8 | exec | Process replacement |
| 9 | system | System call |
| 10 | shred | Secure delete |
| 11 | rd /s | Delete directory tree (Windows) |
| 12 | format | Format disk (Windows) |
| 13 | diskpart | Disk partition (Windows) |
| 14 | reg | Registry operations (Windows) |
| 15 | net | Network/account management (Windows) |
| 16 | sc | Service control (Windows) |
| 17 | schtasks | Scheduled tasks (Windows) |
| 18 | powercfg | Power configuration (Windows) |
| 19 | bcdedit | Boot configuration (Windows) |
| 20 | wevtutil | Event logs (Windows) |

**Allow Specific Commands:**
```bash
./rust-mcp-server --allow-dangerous-commands 1,3
```

### Two-Step Confirmation

When a dangerous command or injection pattern is detected:

1. First call returns a security warning
2. Command is stored in pending list (5-minute timeout)
3. Second identical call executes the command
4. User must explicitly confirm with AI assistant

### Injection Detection

The following characters trigger confirmation:
```
;  |  &  `  $  (  )  <  >
```

Characters inside quoted strings are excluded from detection.

## Troubleshooting

### Server Won't Start

**Check port availability:**
```bash
# Linux/macOS
lsof -i :2233
lsof -i :3344

# Windows
netstat -ano | findstr :2233
netstat -ano | findstr :3344
```

**Change ports:**
```bash
./rust-mcp-server --webui-port 8080 --mcp-port 9000
```

### Tools Not Working

**Check if tool is enabled:**
- Open WebUI and verify tool toggle is ON
- Or check via API: `GET http://127.0.0.1:2233/api/tools`

**Check working directory:**
```bash
./rust-mcp-server --working-dir /correct/path
```

### MCP Client Can't Connect

**Verify transport mode:**
```bash
# Check current transport
curl http://127.0.0.1:2233/api/config
```

**Test MCP endpoint:**
```bash
# For HTTP transport
curl -X POST http://127.0.0.1:3344 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

**403 Forbidden: Host header is not allowed**

This error occurs when the MCP server rejects the request due to DNS rebinding protection (rmcp v1.5.0+).

**If using `--mcp-host 0.0.0.0`:** The server auto-detects local network interface IPs. If auto-detection fails, use one of the following:

```bash
# Option 1: Explicitly specify allowed hosts
./rust-mcp-server --mcp-host 0.0.0.0 --allowed-hosts 192.168.1.100

# Option 2: Disable the check (NOT recommended for public deployments)
./rust-mcp-server --mcp-host 0.0.0.0 --disable-allowed-hosts
```

### Performance Issues

**Increase concurrency:**
```bash
./rust-mcp-server --max-concurrency 50
```

**Monitor resource usage:**
```bash
# Via WebUI HUD
Open http://127.0.0.1:2233 and check the header system metrics

# Via API
curl http://127.0.0.1:2233/api/system-metrics

# Linux/macOS
top -p $(pgrep rust-mcp-server)

# Windows
tasklist | findstr rust-mcp-server
```

## Getting Help

- [GitHub Issues](https://github.com/yuunnn-w/Rust-MCP-Server/issues)
- [GitHub Discussions](https://github.com/yuunnn-w/Rust-MCP-Server/discussions)

---

中文版本请查看 [user-guide-zh.md](user-guide-zh.md)
