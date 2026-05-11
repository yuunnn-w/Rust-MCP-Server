// ============================================================
// Cyberpunk AI Command Center — Frontend Controller
// ============================================================

function debounce(fn, delay) {
    let timer = null;
    return function(...args) {
        clearTimeout(timer);
        timer = setTimeout(() => fn.apply(this, args), delay);
    };
}

class CommandCenter {
    constructor() {
        this.lang = 'zh';
        this.tools = [];
        this.currentFilter = 'all';
        this.currentSort = 'name';
        this.searchQuery = '';
        this.callHistory = {};
        this.currentAlphabet = 'all';
        this.sseSource = null;
        this.terminalLogs = [];
        this.terminalCollapsed = false;
        this.config = null;
        this.currentConcurrency = 0;
        this.metricsInterval = null;
        this.theme = 'system'; // 'dark' | 'light' | 'system'
        this.pythonFsAccessEnabled = false;
        this.presets = [];
        this.currentPreset = null;
        this.canvasAccentColor = '0, 240, 255';
        this._canvasResizeHandler = null;
        this._mediaQueryList = null;
        this._mediaQueryListener = null;
        this._cardTiltState = new WeakMap();

        this.i18n = {
            zh: {
                title: 'AI 命令中心',
                subtitle: 'MCP 工具管理平台',
                safeTools: '安全工具',
                dangerousTools: '危险工具',
                idle: '空闲',
                calling: '执行中',
                enabled: '已启用',
                disabled: '已禁用',
                mcpRunning: 'MCP运行中',
                mcpStopped: 'MCP已停止',
                status: '状态',
                config: '配置',
                tools: '工具',
                searchPlaceholder: '搜索工具...',
                sortByName: '按名称排序',
                sortByCalls: '按调用次数排序',
                sortByTime: '按时间排序',
                toolInfo: '工具信息',
                toolStats: '工具统计',
                recentCalls: '最近调用',
                description: '描述',
                usage: '使用说明',
                close: '关闭',
                enable: '启用',
                disable: '禁用',
                editConfig: '编辑配置',
                configTitle: '系统配置',
                save: '保存',
                cancel: '取消',
                cfgWebuiHost: 'WebUI 主机',
                cfgWebuiPort: 'WebUI 端口',
                cfgMcpTransport: 'MCP 传输模式',
                cfgMcpHost: 'MCP 主机',
                cfgMcpPort: 'MCP 端口',
                cfgMaxConcurrency: '最大并发数',
                cfgWorkingDir: '工作目录',
                cfgLogLevel: '日志级别',
                browse: '浏览',
                restartRequired: '需要重启服务器',
                callCount: '调用次数',
                lastCall: '最后调用',
                avgDuration: '平均耗时',
                errorRate: '错误率',
                callTrend: '调用趋势',
                systemConfig: '系统配置',
                restartServer: '重启服务器',
                noTools: '没有找到匹配的工具',
                connectionError: '连接错误',
                confirmRestart: '确认重启',
                restartConfirmText: '确定要重启服务器吗？这将中断所有正在进行的操作。',
                errorLoading: '加载工具列表失败',
                errorSSE: 'SSE连接断开，正在尝试重连...',
                terminalTitle: '系统日志流',
                cpu: 'CPU',
                memory: '内存',
                calls: '调用',
                concurrency: '并发',
                about: '关于',
                version: '版本',
                author: '作者',
                license: '许可证',
                aboutDescription: '高性能模型上下文协议（MCP）服务器，带 WebUI 控制面板。',
                presetSection: '工具预设',
                batchSection: '批量操作',
                batchEnableAll: '全部启用',
                batchDisableAll: '全部禁用',
                presetCurrent: '当前',
                presetNone: '无',
                presetNameMinimal: '最小模式',
                presetNameCoding: '编码开发',
                presetNameResearch: '研究文档',
                presetNameDataAnalysis: '数据分析',
                presetNameSystemAdmin: '系统管理',
                presetNameFullPower: '全功能',
                presetToolsCount: '{count} 个工具',
                github: 'GitHub',
                loading: '加载中...',
                loadError: '加载失败',
                retry: '重试',
                filterAll: '全部',
                filterSafe: '安全',
                filterDangerous: '危险',
            },
            en: {
                title: 'AI Command Center',
                subtitle: 'MCP Tool Management Platform',
                safeTools: 'Safe Tools',
                dangerousTools: 'Dangerous Tools',
                idle: 'Idle',
                calling: 'Calling',
                enabled: 'Enabled',
                disabled: 'Disabled',
                mcpRunning: 'MCP Running',
                mcpStopped: 'MCP Stopped',
                status: 'Status',
                config: 'Config',
                tools: 'Tools',
                searchPlaceholder: 'Search tools...',
                sortByName: 'Sort by Name',
                sortByCalls: 'Sort by Calls',
                sortByTime: 'Sort by Time',
                toolInfo: 'Tool Info',
                toolStats: 'Tool Statistics',
                recentCalls: 'Recent Calls',
                description: 'Description',
                usage: 'Usage',
                close: 'Close',
                enable: 'Enable',
                disable: 'Disable',
                editConfig: 'Edit Config',
                configTitle: 'System Config',
                save: 'Save',
                cancel: 'Cancel',
                cfgWebuiHost: 'WebUI Host',
                cfgWebuiPort: 'WebUI Port',
                cfgMcpTransport: 'MCP Transport',
                cfgMcpHost: 'MCP Host',
                cfgMcpPort: 'MCP Port',
                cfgMaxConcurrency: 'Max Concurrency',
                cfgWorkingDir: 'Working Directory',
                cfgLogLevel: 'Log Level',
                browse: 'Browse',
                restartRequired: 'Server restart required',
                callCount: 'Call Count',
                lastCall: 'Last Call',
                avgDuration: 'Avg Duration',
                errorRate: 'Error Rate',
                callTrend: 'Call Trend',
                systemConfig: 'System Config',
                restartServer: 'Restart Server',
                noTools: 'No matching tools found',
                connectionError: 'Connection Error',
                confirmRestart: 'Confirm Restart',
                restartConfirmText: 'Are you sure you want to restart the server? This will interrupt all ongoing operations.',
                errorLoading: 'Failed to load tool list',
                errorSSE: 'SSE connection lost, attempting to reconnect...',
                terminalTitle: 'System Log Stream',
                cpu: 'CPU',
                memory: 'MEM',
                calls: 'Calls',
                concurrency: 'Conc',
                about: 'About',
                version: 'Version',
                author: 'Author',
                license: 'License',
                aboutDescription: 'A high-performance Model Context Protocol (MCP) server with WebUI control panel.',
                presetSection: 'Tool Presets',
                batchSection: 'Batch Actions',
                batchEnableAll: 'Enable All',
                batchDisableAll: 'Disable All',
                presetCurrent: 'Current',
                presetNone: 'None',
                presetNameMinimal: 'Minimal',
                presetNameCoding: 'Coding',
                presetNameResearch: 'Research',
                presetNameDataAnalysis: 'Data Analysis',
                presetNameSystemAdmin: 'System Admin',
                presetNameFullPower: 'Full Power',
                presetToolsCount: '{count} tools',
                github: 'GitHub',
                loading: 'Loading...',
                loadError: 'Load failed',
                retry: 'Retry',
                filterAll: 'All',
                filterSafe: 'Safe',
                filterDangerous: 'Dangerous',
            }
        };

        this.toolI18n = {
            zh: {
                Glob: { desc: '列出目录内容，支持增强过滤（最大深度10）。对UTF-8文本文件自动返回字符数和行数', usage: '用法：列出目录内容。\n参数：path（路径），可选 max_depth（默认2，最大10），可选 pattern（glob 如 "*.rs"），可选 brief（默认true），可选 sort_by（name/type/size/modified），可选 flatten\n示例：{"path": "/home/user", "pattern": "*.rs", "brief": true}' },
                Read: { desc: '读取文件并自动检测格式。模式：auto/text/media。DOC/DOCX：doc_text（markdown）、doc_with_images（markdown+内嵌图片）、doc_images（仅图片）。PPT/PPTX：ppt_text、ppt_images（幻灯片转图片）。PDF：pdf_text、pdf_images（页面转图片）。XLS/XLSX：text。支持批量读取。图片模式返回 base64 编码的图片内容供视觉模型读取', usage: '用法：读取文件。\n参数：path（路径），可选mode（auto/text/media/doc_text/doc_with_images/doc_images/ppt_text/ppt_images/pdf_text/pdf_images），可选 start_line/end_line/offset_chars/max_chars/line_numbers/highlight_line/sheet_name/image_dpi/image_format\n示例：{"path": "file.txt", "start_line": 0, "end_line": 100} | {"path": "doc.docx", "mode": "doc_text"}' },
                Grep: { desc: '在文件中搜索模式，支持增强过滤（最大深度10）。支持正则、大小写敏感、全词匹配、多行模式。输出模式：detailed/compact/location/brief。可搜索办公文档内容', usage: '用法：搜索关键词/模式。\n参数：path（路径），pattern（模式），可选 file_pattern（glob），可选 use_regex（默认false），可选 output_mode（detailed/compact/location/brief），可选 max_results（默认20），可选 context_lines（默认3）\n示例：{"path": "/home/user/src", "pattern": "TODO", "context_lines": 3}' },
                Edit: { desc: '并发编辑文件。文本模式：string_replace、line_replace、insert、delete、patch。Office模式：office_insert、office_replace、office_delete、office_insert_image、office_format、office_insert_table（通过markdown操作DOCX复杂格式）。PDF模式：pdf_delete_page、pdf_insert_image、pdf_insert_text、pdf_replace_text。可创建新文件（危险操作）', usage: '用法：并发编辑多个文件。\n参数：operations（操作列表），每个操作包含 path, mode 及对应模式的参数\ntext模式：string_replace(path, old_string, new_string, occurrence)/line_replace(path, start_line, end_line, new_string)/insert(path, start_line, new_string)/delete(path, start_line, end_line)/patch(path, patch)\noffice模式：office_insert(path, markdown)/office_replace(path, find_text, new_string)/office_delete(path, find_text, element_type)/office_insert_image(path, image_path, find_text)/office_format(path, find_text, format_type)/office_insert_table(path, markdown[, find_text, location])\npdf模式：pdf_delete_page(path, page_index)/pdf_insert_image(path, image_path, page_index)/pdf_insert_text(path, new_string, page_index)/pdf_replace_text(path, old_string, new_string)\n示例：{"operations": [{"path": "main.rs", "mode": "string_replace", "old_string": "fn old()", "new_string": "fn new()"}]}' },
                Write: { desc: '并发将内容写入文件。支持创建办公文档：DOCX（docx_paragraphs或office_markdown）、XLSX（xlsx_sheets或office_csv）、PPTX（pptx_slides）、PDF（office_markdown通过LibreOffice）、IPYNB（ipynb_cells）（危险操作）', usage: '用法：并发写入多个文件。\n参数：files（文件列表），可选 file_type/docx_paragraphs/xlsx_sheets/pptx_slides/ipynb_cells/office_markdown/office_csv\n文本模式：{"files": [{"path": "test.txt", "content": "Hello", "mode": "new"}]}\nDOCX创建：{"file_type": "docx", "office_markdown": "# Title", "files": [{"path": "doc.docx"}]}\nCSV创建XLSX：{"file_type": "xlsx", "office_csv": "Name,Age\nAlice,30", "files": [{"path": "data.xlsx"}]}\nPDF创建：{"file_type": "pdf", "office_markdown": "# Title", "files": [{"path": "output.pdf"}]}' },
                FileStat: { desc: '并发获取一个或多个文件或目录的元数据。对UTF-8文本文件额外返回字符数、行数和编码', usage: '用法：并发获取多个文件/目录的元数据。\n参数：paths（路径列表）\n返回：每个路径包含 name, size, file_type, readable, writable, modified/created/accessed。UTF-8文本文件额外包含 is_text, char_count, line_count, encoding\n示例：{"paths": ["src/main.rs", "Cargo.toml"]}' },
                Clipboard: { desc: '读写系统剪贴板内容，支持文本和图片', usage: '用法：读写系统剪贴板。\n参数：operation（read_text/write_text/read_image/clear），可选 text（write_text时需要）\n示例：{"operation": "read_text"} | {"operation": "write_text", "text": "Hello"} | {"operation": "clear"}' },
                Diff: { desc: '比较文本、文件或目录的差异，支持多种输出格式', usage: '用法：比较差异。\n参数：operation（compare_text/compare_files/directory_diff/git_diff_file），可选 old_text/new_text，可选 output_format（unified/side_by_side/summary/inline，默认unified），可选 context_lines（默认3）\n示例：{"operation": "compare_text", "old_text": "foo", "new_text": "bar", "output_format": "unified"}' },
                WebFetch: { desc: '获取URL内容，提取模式：text（去除HTML）/html（原始）/markdown', usage: '用法：获取网页内容。\n参数：url（地址），可选 extract_mode（text/html/markdown，默认text）\n示例：{"url": "https://example.com", "extract_mode": "markdown"}' },
                Archive: { desc: '创建、解压、列出或追加ZIP压缩文件。支持 AES-256 密码加密（危险操作）', usage: '用法：ZIP归档操作。\n参数：operation（create/extract/list/append），source，可选 destination, password\n示例：{"operation": "create", "source": "src/", "destination": "archive.zip", "password": "mypass"}' },
                NoteStorage: { desc: 'AI助手的短期记忆便签板，支持CRUD、搜索和JSON导出', usage: '用法：管理便签。\n参数：operation（create/list/read/update/delete/append/search/export/import），及对应内容\n示例：{"operation": "create", "content": "记住这个信息..."}' },
                Task: { desc: '任务管理，支持增删改查操作', usage: '用法：管理任务。\n参数：operation（create/list/get/update/delete），及对应内容\n示例：{"operation": "create", "title": "完成报告", "priority": "high"}' },
                WebSearch: { desc: '通过DuckDuckGo搜索网页，支持地区和语言过滤', usage: '用法：搜索网页。\n参数：query（搜索内容），可选 region（地区代码），可选 language（语言代码）\n示例：{"query": "Rust MCP server"}' },
                AskUser: { desc: '通过MCP引导向用户提问，支持超时和默认选项', usage: '用法：向用户提问。\n参数：question（问题），可选 timeout_ms，可选 options\n示例：{"question": "确认删除？", "options": ["yes", "no"]}' },
                Bash: { desc: '执行Shell命令，支持可选的工作目录、标准输入、输出限制和异步模式。异步命令需配合Monitor工具使用（危险操作）', usage: '用法：执行Shell命令。\n参数：command（命令），可选 working_dir/cwd，可选 stdin（标准输入），可选 max_output_chars（默认50000），可选 timeout（默认30秒，最大300），可选 async_mode（默认false），可选 shell_path/shell_arg\n示例：{"command": "ls -la", "working_dir": "/home/user"}' },
                SystemInfo: { desc: '获取系统信息，包括进程列表。使用sections参数选择获取的类别：system/cpu/memory/disks/network/temperature/processes（默认全部启用）', usage: '用法：获取系统信息。\n参数：可选 sections（类别列表），可选 process_limit（默认50），可选 process_sort（cpu/memory/name，默认cpu）\n示例：{"sections": ["cpu", "memory", "processes"]}' },
                ExecutePython: { desc: '在RustPython沙箱中执行Python代码，用于计算、数据处理和逻辑判断。全部Python标准库可用', usage: '用法：执行Python代码。\n参数：code（代码），可选 timeout_ms（默认5000，最大30000），可选 packages（暂无效）\n通过 __result 变量返回结果\n示例：{"code": "x = 42\n__result = x * 2"}' },
                Git: { desc: '执行Git命令（status/diff/log/branch/show），支持路径过滤和日志数量限制', usage: '用法：执行Git命令。\n参数：command（status/diff/log/branch/show），可选 repo_path（仓库路径），可选 path（文件路径过滤），可选 max_count（log条数限制）\n示例：{"command": "status", "repo_path": "/home/user/repo"}' },
                Monitor: { desc: '监控Bash工具启动的异步长时间运行命令。支持流式输出、等待完成或发送信号', usage: '用法：监控异步命令。\n参数：command_id（Bash异步模式返回的ID），可选 operation（stream/wait/signal，默认wait），可选 timeout（默认60秒），可选 signal（terminate/kill/interrupt）\n示例：{"command_id": "abc123", "operation": "stream"}' },
                NotebookEdit: { desc: '读取、写入和编辑Jupyter .ipynb笔记本文件。支持add_cell/edit_cell/delete_cell操作（危险操作）', usage: '用法：编辑Jupyter笔记本。\n参数：path（.ipynb文件路径），operation（add_cell/edit_cell/delete_cell），可选 cell_index/cell_id，可选 source/cell_type\n示例：{"operation": "add_cell", "path": "notebook.ipynb", "cell_type": "code", "source": "print(1)"}' },
                FileOps: { desc: '并发复制、移动、删除或重命名文件。支持dry_run预览和conflict_resolution（skip/overwrite/rename）。限制在工作目录内操作（危险操作）', usage: '用法：批量操作文件。\n参数：operations（操作列表），每个包含 path, operation（copy/move/delete/rename），可选 destination/new_path，可选 overwrite\n示例：{"operations": [{"path": "old.txt", "operation": "rename", "new_path": "new.txt"}]}' },
            },
            en: {
                Glob: { desc: 'List directory contents with enhanced filtering (max depth 10). Returns char_count and line_count for UTF-8 text files', usage: 'Usage: List directory contents.\nParameters: path, optional max_depth (default: 2, max: 10), optional pattern (glob e.g. "*.rs"), optional brief (default: true), optional sort_by (name/type/size/modified), optional flatten\nExample: {"path": "/home/user", "pattern": "*.rs", "brief": true}' },
                Read: { desc: 'Read file with format auto-detection. Modes: auto, text, media. DOC/DOCX: doc_text (markdown), doc_with_images (markdown+inline images), doc_images (images only). PPT/PPTX: ppt_text, ppt_images (slides as images). PDF: pdf_text, pdf_images (pages as images). XLS/XLSX: text. Batch mode via paths. Image modes return base64-encoded image content for vision models', usage: 'Usage: Read file.\nParameters: path, optional mode (auto/text/media/doc_text/doc_with_images/doc_images/ppt_text/ppt_images/pdf_text/pdf_images), optional start_line/end_line/offset_chars/max_chars/line_numbers/highlight_line/sheet_name/image_dpi/image_format\nExample: {"path": "file.txt", "start_line": 0, "end_line": 100} | {"path": "doc.docx", "mode": "doc_text"}' },
                Grep: { desc: 'Search pattern in files with enhanced filtering (max depth 10). Searches office documents. Supports regex, case-sensitive, whole-word, multiline modes. Output modes: detailed/compact/location/brief', usage: 'Usage: Search for keyword/pattern.\nParameters: path, pattern, optional file_pattern (glob), optional use_regex (default: false), optional output_mode (detailed/compact/location/brief), optional max_results (default: 20), optional context_lines (default: 3)\nExample: {"path": "/home/user/src", "pattern": "TODO", "context_lines": 3}' },
                Edit: { desc: 'Edit files concurrently. Text modes: string_replace, line_replace, insert, delete, patch. Office modes: office_insert, office_replace, office_delete, office_insert_image, office_format, office_insert_table (manipulate DOCX via markdown). PDF modes: pdf_delete_page, pdf_insert_image, pdf_insert_text, pdf_replace_text. Can create new files (dangerous)', usage: 'Usage: Edit multiple files.\nParameters: operations (list), each with path, mode, and mode-specific params.\ntext: string_replace(path, old_string, new_string, occurrence)/line_replace(path, start_line, end_line, new_string)/insert(path, start_line, new_string)/delete(path, start_line, end_line)/patch(path, patch)\noffice: office_insert(path, markdown)/office_replace(path, find_text, new_string)/office_delete(path, find_text, element_type)/office_insert_image(path, image_path, find_text)/office_format(path, find_text, format_type)/office_insert_table(path, markdown[, find_text, location])\npdf: pdf_delete_page(path, page_index)/pdf_insert_image(path, image_path, page_index)/pdf_insert_text(path, new_string, page_index)/pdf_replace_text(path, old_string, new_string)\nExample: {"operations": [{"path": "main.rs", "mode": "string_replace", "old_string": "fn old()", "new_string": "fn new()"}]}' },
                Write: { desc: 'Write content to files concurrently. Supports office documents: DOCX (docx_paragraphs or office_markdown), XLSX (xlsx_sheets or office_csv), PPTX (pptx_slides), PDF (office_markdown via LibreOffice), IPYNB (ipynb_cells) (dangerous)', usage: 'Usage: Write to multiple files.\nParameters: files (list), optional file_type/docx_paragraphs/xlsx_sheets/pptx_slides/ipynb_cells/office_markdown/office_csv\nText: {"files": [{"path": "test.txt", "content": "Hello", "mode": "new"}]}\nDOCX: {"file_type": "docx", "office_markdown": "# Title", "files": [{"path": "doc.docx"}]}\nCSV to XLSX: {"file_type": "xlsx", "office_csv": "Name,Age\nAlice,30", "files": [{"path": "data.xlsx"}]}\nPDF: {"file_type": "pdf", "office_markdown": "# PDF Title", "files": [{"path": "output.pdf"}]}' },
                FileStat: { desc: 'Get metadata for one or more files or directories concurrently. Returns char_count, line_count, and encoding for UTF-8 text files', usage: 'Usage: Get metadata for multiple files/directories.\nParameters: paths (list of paths)\nReturns: name, size, file_type, readable, writable, modified/created/accessed. For UTF-8 text files also is_text, char_count, line_count, encoding\nExample: {"paths": ["src/main.rs", "Cargo.toml"]}' },
                Clipboard: { desc: 'Read or write system clipboard content, supports text and images', usage: 'Usage: Read or write clipboard.\nParameters: operation (read_text/write_text/read_image/clear), optional text (for write_text)\nExample: {"operation": "read_text"} | {"operation": "write_text", "text": "Hello"} | {"operation": "clear"}' },
                Diff: { desc: 'Compare text, files, or directories with multiple output formats', usage: 'Usage: Compare differences.\nParameters: operation (compare_text/compare_files/directory_diff/git_diff_file), optional old_text/new_text, optional output_format (unified/side_by_side/summary/inline, default: unified), optional context_lines (default: 3)\nExample: {"operation": "compare_text", "old_text": "foo", "new_text": "bar", "output_format": "unified"}' },
                WebFetch: { desc: 'Fetch content from a URL with extract_mode: text (strips HTML), html (raw), or markdown', usage: 'Usage: Fetch web content.\nParameters: url, optional extract_mode (text/html/markdown, default: text)\nExample: {"url": "https://example.com", "extract_mode": "markdown"}' },
                Archive: { desc: 'Create, extract, list, or append ZIP archives with AES-256 password encryption (dangerous)', usage: 'Usage: ZIP archive operations.\nParameters: operation (create/extract/list/append), source, optional destination, password\nExample: {"operation": "create", "source": "src/", "destination": "archive.zip", "password": "mypass"}' },
                NoteStorage: { desc: 'AI assistant short-term memory scratchpad with CRUD, search, export/import JSON', usage: 'Usage: Manage notes.\nParameters: operation (create/list/read/update/delete/append/search/export/import), and content\nExample: {"operation": "create", "content": "Remember this info..."}' },
                Task: { desc: 'Task management with CRUD operations', usage: 'Usage: Manage tasks.\nParameters: operation (create/list/get/update/delete), and content\nExample: {"operation": "create", "title": "Complete report", "priority": "high"}' },
                WebSearch: { desc: 'Search the web via DuckDuckGo with optional region/language filters', usage: 'Usage: Search the web.\nParameters: query, optional region (code), optional language (code)\nExample: {"query": "Rust MCP server"}' },
                AskUser: { desc: 'Ask the user a question via MCP elicitation with timeout and default options', usage: 'Usage: Ask user a question.\nParameters: question, optional timeout_ms, optional options\nExample: {"question": "Confirm deletion?", "options": ["yes", "no"]}' },
                Bash: { desc: 'Execute shell command with optional working_dir, stdin, max_output_chars, and async_mode. Use Monitor tool for async commands (dangerous)', usage: 'Usage: Execute shell command.\nParameters: command, optional working_dir/cwd, optional stdin, optional max_output_chars (default: 50000), optional timeout (default: 30s, max: 300), optional async_mode (default: false), optional shell_path/shell_arg\nExample: {"command": "ls -la", "working_dir": "/home/user"}' },
                SystemInfo: { desc: 'Get system information including processes. Use sections parameter: system/cpu/memory/disks/network/temperature/processes (all enabled by default)', usage: 'Usage: Get system info.\nParameters: optional sections (list of categories), optional process_limit (default: 50), optional process_sort (cpu/memory/name, default: cpu)\nExample: {"sections": ["cpu", "memory", "processes"]}' },
                ExecutePython: { desc: 'Execute Python code in a RustPython sandbox for calculations, data processing, and logic evaluation. All Python standard library modules available', usage: 'Usage: Execute Python code.\nParameters: code, optional timeout_ms (default: 5000, max: 30000), optional packages (unused)\nUse __result variable for return value\nExample: {"code": "x = 42\n__result = x * 2"}' },
                Git: { desc: 'Run git commands (status/diff/log/branch/show) with path filtering and max_count for log', usage: 'Usage: Run git commands.\nParameters: command (status/diff/log/branch/show), optional repo_path, optional path (file filter), optional max_count (log entries)\nExample: {"command": "status", "repo_path": "/home/user/repo"}' },
                Monitor: { desc: 'Monitor long-running Bash commands started with async=true. Stream output, wait for completion, or send signals', usage: 'Usage: Monitor async commands.\nParameters: command_id (from Bash async mode), optional operation (stream/wait/signal, default: wait), optional timeout (default: 60s), optional signal (terminate/kill/interrupt)\nExample: {"command_id": "abc123", "operation": "stream"}' },
                NotebookEdit: { desc: 'Read, write, and edit Jupyter .ipynb notebook files. Supports add_cell/edit_cell/delete_cell (dangerous)', usage: 'Usage: Edit Jupyter notebook.\nParameters: path (.ipynb file), operation (add_cell/edit_cell/delete_cell), optional cell_index/cell_id, optional source/cell_type\nExample: {"operation": "add_cell", "path": "notebook.ipynb", "cell_type": "code", "source": "print(1)"}' },
                FileOps: { desc: 'Copy, move, delete, or rename files concurrently. Supports dry_run and conflict_resolution (skip/overwrite/rename). Restricted to working directory (dangerous)', usage: 'Usage: Batch file operations.\nParameters: operations (list), each with path, operation (copy/move/delete/rename), optional destination/new_path, optional overwrite\nExample: {"operations": [{"path": "old.txt", "operation": "rename", "new_path": "new.txt"}]}' },
            }
        };

        this.init();
    }

    // ============================================================
    // INIT
    // ============================================================
    init() {
        this.initTheme();
        this.initBackgroundCanvas();
        this.bindEvents();
        this.loadData();
        this.initSSE();
        this.initMetricsPolling();
        this.addTerminalLog('info', 'Command Center initialized. Waiting for telemetry...');
        // Mobile: collapse terminal by default to save screen space
        if (window.innerWidth <= 768) {
            this.terminalCollapsed = true;
            document.getElementById('terminal-panel')?.classList.add('collapsed');
        }
        this.render();
        this.initCleanup();
    }

    // ============================================================
    // THEME MANAGEMENT
    // ============================================================
    initTheme() {
        const saved = localStorage.getItem('cc-theme');
        this.theme = saved || 'system';
        this.applyTheme();

        if (this.theme === 'system') {
            this._mediaQueryList = window.matchMedia('(prefers-color-scheme: dark)');
            this._mediaQueryListener = () => this.applyTheme();
            this._mediaQueryList.addEventListener('change', this._mediaQueryListener);
        }
    }

    getEffectiveTheme() {
        if (this.theme === 'system') {
            return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }
        return this.theme;
    }

    applyTheme() {
        const effective = this.getEffectiveTheme();
        document.documentElement.setAttribute('data-theme', effective);
        const btn = document.getElementById('theme-toggle');
        if (btn) btn.textContent = this.getThemeIcon();
        const sidebarBtn = document.getElementById('sidebar-theme-toggle');
        if (sidebarBtn) sidebarBtn.textContent = this.getThemeIcon();
        this.updateCanvasAccentColor();
    }

    getThemeIcon() {
        if (this.theme === 'dark') return '☀️';
        if (this.theme === 'light') return '🌙';
        return '💻';
    }

    updateCanvasAccentColor() {
        const style = getComputedStyle(document.documentElement);
        this.canvasAccentColor = style.getPropertyValue('--canvas-accent').trim() || '0, 240, 255';
    }

    cycleTheme() {
        const order = ['system', 'light', 'dark'];
        const idx = order.indexOf(this.theme);
        this.theme = order[(idx + 1) % order.length];
        localStorage.setItem('cc-theme', this.theme);
        this.applyTheme();
    }

    // ============================================================
    // BACKGROUND CANVAS — Animated Grid + Particles
    // ============================================================
    initBackgroundCanvas() {
        const canvas = document.getElementById('bg-canvas');
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        let particles = [];
        let gridOffset = 0;

        this._canvasResizeHandler = () => {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
        };
        this._canvasResizeHandler();
        window.addEventListener('resize', this._canvasResizeHandler);

        for (let i = 0; i < 60; i++) {
            particles.push({
                x: Math.random() * canvas.width,
                y: Math.random() * canvas.height,
                vx: (Math.random() - 0.5) * 0.3,
                vy: (Math.random() - 0.5) * 0.3,
                size: Math.random() * 2 + 0.5,
                alpha: Math.random() * 0.5 + 0.2,
            });
        }

        const draw = () => {
            if (!this.bgAnimationPaused) {
                const accent = this.canvasAccentColor;
                ctx.clearRect(0, 0, canvas.width, canvas.height);

                // Perspective grid
                gridOffset = (gridOffset + 0.3) % 40;
                ctx.strokeStyle = `rgba(${accent}, 0.04)`;
                ctx.lineWidth = 1;

                // Horizontal lines with perspective
                for (let i = 0; i < canvas.height / 2; i += 40) {
                    const y = i + gridOffset;
                    if (y > canvas.height / 2) continue;
                    const perspective = 1 - (y / (canvas.height / 2)) * 0.7;
                    ctx.globalAlpha = perspective * 0.5;
                    ctx.beginPath();
                    ctx.moveTo(0, y + canvas.height / 2);
                    ctx.lineTo(canvas.width, y + canvas.height / 2);
                    ctx.stroke();
                }
                ctx.globalAlpha = 1;

                // Vertical lines
                for (let i = 0; i < canvas.width; i += 60) {
                    ctx.beginPath();
                    ctx.moveTo(i, canvas.height / 2);
                    ctx.lineTo(i + (i - canvas.width / 2) * 0.3, canvas.height);
                    ctx.stroke();
                }

                // Particles
                particles.forEach(p => {
                    p.x += p.vx;
                    p.y += p.vy;
                    if (p.x < 0) p.x = canvas.width;
                    if (p.x > canvas.width) p.x = 0;
                    if (p.y < 0) p.y = canvas.height;
                    if (p.y > canvas.height) p.y = 0;

                    ctx.beginPath();
                    ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
                    ctx.fillStyle = `rgba(${accent}, ${p.alpha})`;
                    ctx.fill();
                });

                // Connect nearby particles
                for (let i = 0; i < particles.length; i++) {
                    for (let j = i + 1; j < particles.length; j++) {
                        const dx = particles[i].x - particles[j].x;
                        const dy = particles[i].y - particles[j].y;
                        const dist = Math.sqrt(dx * dx + dy * dy);
                        if (dist < 120) {
                            ctx.beginPath();
                            ctx.moveTo(particles[i].x, particles[i].y);
                            ctx.lineTo(particles[j].x, particles[j].y);
                            ctx.strokeStyle = `rgba(${accent}, ${0.05 * (1 - dist / 120)})`;
                            ctx.stroke();
                        }
                    }
                }
            }
            requestAnimationFrame(draw);
        };
        draw();
    }

    // ============================================================
    // METRICS POLLING
    // ============================================================
    initMetricsPolling() {
        this.updateMetrics();
        this.metricsInterval = setInterval(() => this.updateMetrics(), 3000);
    }

    async updateMetrics() {
        try {
            const res = await fetch('/api/system-metrics');
            if (!res.ok) return;
            const data = await res.json();

            // Update HUD CPU
            const cpuRing = document.querySelector('.hud-ring[data-metric="cpu"] .hud-ring-fill');
            const cpuVal = document.querySelector('.hud-ring[data-metric="cpu"] .hud-value');
            if (cpuRing && cpuVal) {
                const pct = Math.min(data.cpu_percent || 0, 100);
                const circ = 2 * Math.PI * 18;
                cpuRing.style.strokeDasharray = `${circ * pct / 100} ${circ}`;
                cpuVal.textContent = pct.toFixed(0) + '%';
                cpuRing.style.stroke = pct > 80 ? 'var(--neon-red)' : 'var(--neon-cyan)';
            }

            // Update HUD Memory
            const memBar = document.querySelector('.hud-bar-container[data-metric="memory"] .hud-bar-fill');
            const memVal = document.querySelector('.hud-bar-container[data-metric="memory"] .hud-bar-value');
            if (memBar && memVal) {
                const pct = Math.min(data.memory_percent || 0, 100);
                memBar.style.width = pct + '%';
                memVal.textContent = pct.toFixed(0) + '%';
                memBar.style.background = pct > 80
                    ? 'linear-gradient(90deg, var(--neon-red), var(--neon-amber))'
                    : 'linear-gradient(90deg, var(--neon-cyan), var(--neon-purple))';
            }
        } catch (e) {
            // silently ignore
        }
    }

    initCleanup() {
        window.addEventListener('beforeunload', () => {
            if (this.sseSource) {
                this.sseSource.close();
                this.sseSource = null;
            }
            if (this.metricsInterval) {
                clearInterval(this.metricsInterval);
                this.metricsInterval = null;
            }
            if (this._canvasResizeHandler) {
                window.removeEventListener('resize', this._canvasResizeHandler);
            }
            if (this._mediaQueryList && this._mediaQueryListener) {
                this._mediaQueryList.removeEventListener('change', this._mediaQueryListener);
            }
            if (this._keydownHandler) {
                document.removeEventListener('keydown', this._keydownHandler);
            }
        });
    }

    // ============================================================
    // SSE
    // ============================================================
    initSSE() {
        const connect = () => {
            if (this.sseSource) {
                this.sseSource.close();
                this.sseSource = null;
            }
            this.sseSource = new EventSource('/api/events');

            this.sseSource.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    this.handleSSE(data);
                } catch (e) {
                    console.error('SSE parse error:', e);
                }
            };

            this.sseSource.onopen = () => {
                this._sseReconnectAttempts = 0;
            };

            this.sseSource.onerror = () => {
                this.addTerminalLog('error', this.t('errorSSE'));
                if (this.sseSource) {
                    this.sseSource.close();
                    this.sseSource = null;
                }
                this._sseReconnectAttempts = (this._sseReconnectAttempts || 0) + 1;
                const delay = Math.min(1000 * Math.pow(2, this._sseReconnectAttempts), 30000);
                setTimeout(connect, delay);
            };
        };
        connect();
    }

    handleSSE(data) {
        if (data.type === 'ToolCallCount') {
            const tool = this.tools.find(t => t.name === data.tool);
            if (tool) {
                tool.call_count = data.count;
                tool.is_calling = data.isCalling;
                tool.is_busy = data.isBusy;
                if (!this.callHistory[tool.name]) this.callHistory[tool.name] = [];
                this.callHistory[tool.name].push({
                    time: new Date().toLocaleTimeString(),
                    count: data.count,
                    is_calling: data.isCalling
                });
                if (this.callHistory[tool.name].length > 50) {
                    this.callHistory[tool.name].shift();
                }
                this.addTerminalLog(data.isCalling ? 'info' : 'success',
                    `${data.tool}: call_count=${data.count}, is_calling=${data.isCalling}`);
                this.render();
            }
        } else if (data.type === 'ToolEnabled') {
            const tool = this.tools.find(t => t.name === data.tool);
            if (tool) {
                tool.enabled = data.enabled;
                this.addTerminalLog('info', `${data.tool}: enabled=${data.enabled}`);
                this.render();
            }
        } else if (data.type === 'McpServiceStatus') {
            this.updateMCPStatus(data.running);
            this.addTerminalLog('info', `MCP service running=${data.running}`);
        } else if (data.type === 'ConcurrentCalls') {
            this.currentConcurrency = data.current;
            const el = document.querySelector('.hud-number[data-metric="concurrency"]');
            if (el) el.textContent = `${data.current}/${data.max}`;
            this.addTerminalLog('info', `Concurrent calls: ${data.current}/${data.max}`);
        }
    }

    updateMCPStatus(running) {
        const statusEl = document.getElementById('mcp-status');
        const toggleEl = document.getElementById('mcp-toggle');
        if (statusEl) {
            statusEl.className = `mcp-status ${running ? 'running' : 'stopped'}`;
            statusEl.textContent = running ? this.t('mcpRunning') : this.t('mcpStopped');
        }
        if (toggleEl) toggleEl.checked = running;
    }

    // ============================================================
    // TERMINAL LOG
    // ============================================================
    addTerminalLog(level, message) {
        const time = new Date().toLocaleTimeString();
        this.terminalLogs.push({ time, level, message });
        if (this.terminalLogs.length > 200) this.terminalLogs.shift();

        const container = document.getElementById('terminal-logs');
        if (!container) return;

        const line = document.createElement('div');
        line.className = 'terminal-line';
        line.innerHTML = `<span class="terminal-timestamp">${time}</span><span class="terminal-level ${level}">${level.toUpperCase()}</span><span class="terminal-msg">${this.escapeHtml(message)}</span>`;
        container.appendChild(line);

        while (container.children.length > 200) {
            container.removeChild(container.firstChild);
        }
        container.scrollTop = container.scrollHeight;
    }

    clearTerminal() {
        this.terminalLogs = [];
        const container = document.getElementById('terminal-logs');
        if (container) container.innerHTML = '';
    }

    // ============================================================
    // DATA LOADING
    // ============================================================
    async loadData() {
        this.showLoading(true);
        this.showRetry(false);
        try {
            const results = await Promise.allSettled([
                fetch('/api/tools'),
                fetch('/api/config'),
                fetch('/api/python-fs-access'),
                fetch('/api/tool-presets'),
                fetch('/api/tool-presets/current')
            ]);

            const [toolsRes, configRes, fsRes, presetsRes, currentPresetRes] = results.map(r => {
                if (r.status === 'fulfilled') return r.value;
                return null;
            });

            if (toolsRes && toolsRes.ok) {
                const toolsData = await toolsRes.json();
                this.tools = Array.isArray(toolsData) ? toolsData : (toolsData.tools || []);
                this.tools.forEach(t => {
                    if (!this.callHistory[t.name]) this.callHistory[t.name] = [];
                });
            } else {
                this.tools = [];
            }

            if (configRes && configRes.ok) {
                this.config = await configRes.json();
                this.renderConfig();
            }

            if (fsRes && fsRes.ok) {
                const fsData = await fsRes.json();
                this.pythonFsAccessEnabled = fsData.enabled || false;
            }

            if (presetsRes && presetsRes.ok) {
                this.presets = await presetsRes.json();
            }

            if (currentPresetRes && currentPresetRes.ok) {
                const data = await currentPresetRes.json();
                this.currentPreset = data.preset || null;
            }

            this.showLoading(false);
            this.render();
        } catch (err) {
            this.showLoading(false);
            this.showRetry(true, this.t('errorLoading'));
        }
    }

    async loadToolInfo(name) {
        try {
            const res = await fetch(`/api/tool/${encodeURIComponent(name)}/detail`);
            if (res.ok) return await res.json();
        } catch (e) {}
        return null;
    }

    // ============================================================
    // 3D CARD TILT (Event Delegation)
    // ============================================================
    initCardTiltDelegation() {
        const safeContainer = document.getElementById('safe-tools');
        const dangerContainer = document.getElementById('dangerous-tools');

        const getCard = (el) => el?.closest('.tool-card');

        const handleMouseEnter = (e) => {
            const card = getCard(e.target);
            if (!card) return;
            const relatedCard = getCard(e.relatedTarget);
            if (relatedCard === card) return;
            if (!this._cardTiltState.has(card)) {
                this._cardTiltState.set(card, { rafId: null, targetTransform: '' });
            }
        };

        const handleMouseMove = (e) => {
            const card = getCard(e.target);
            if (!card) return;
            let state = this._cardTiltState.get(card);
            if (!state) {
                state = { rafId: null, targetTransform: '' };
                this._cardTiltState.set(card, state);
            }
            const rect = card.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;
            const cx = rect.width / 2;
            const cy = rect.height / 2;
            const dx = (x - cx) / cx;
            const dy = (y - cy) / cy;
            state.targetTransform = `perspective(800px) rotateY(${dx * 5}deg) rotateX(${-dy * 5}deg) translateZ(8px)`;
            if (!state.rafId) {
                state.rafId = requestAnimationFrame(() => {
                    if (state.targetTransform) {
                        card.style.transform = state.targetTransform;
                        card.style.transition = 'transform 0.1s ease-out';
                    }
                    state.rafId = null;
                });
            }
        };

        const handleMouseLeave = (e) => {
            const card = getCard(e.target);
            if (!card) return;
            const relatedCard = getCard(e.relatedTarget);
            if (relatedCard === card) return;
            const state = this._cardTiltState.get(card);
            if (state) {
                if (state.rafId) {
                    cancelAnimationFrame(state.rafId);
                    state.rafId = null;
                }
                state.targetTransform = '';
            }
            card.style.transform = 'perspective(800px) rotateY(0) rotateX(0) translateZ(0)';
            card.style.transition = 'transform 0.3s ease-out';
        };

        [safeContainer, dangerContainer].forEach(container => {
            if (!container) return;
            container.addEventListener('mouseenter', handleMouseEnter, true);
            container.addEventListener('mousemove', handleMouseMove);
            container.addEventListener('mouseleave', handleMouseLeave, true);
        });
    }

    // ============================================================
    // EVENT BINDING
    // ============================================================
    bindEvents() {
        // Language
        document.getElementById('lang-toggle')?.addEventListener('click', () => {
            this.lang = this.lang === 'zh' ? 'en' : 'zh';
            document.getElementById('lang-toggle').textContent = this.lang === 'zh' ? 'EN' : '中文';
            this.render();
            this.renderConfig();
        });

        // Theme toggle
        document.getElementById('theme-toggle')?.addEventListener('click', () => {
            this.cycleTheme();
        });

        // Sidebar
        document.getElementById('menu-btn')?.addEventListener('click', () => {
            document.getElementById('sidebar')?.classList.add('open');
            document.getElementById('overlay')?.classList.add('show');
        });
        document.getElementById('close-sidebar')?.addEventListener('click', this.closeSidebar.bind(this));
        document.getElementById('overlay')?.addEventListener('click', this.closeSidebar.bind(this));

        // MCP Toggle
        document.getElementById('mcp-toggle')?.addEventListener('change', (e) => {
            this.toggleMCPService(e.target.checked);
        });

        // Search
        const searchInput = document.getElementById('search-input');
        if (searchInput) {
            const debouncedSearch = debounce((e) => {
                this.searchQuery = e.target.value.toLowerCase();
                this.render();
            }, 300);
            searchInput.addEventListener('input', debouncedSearch);
        }

        // Sort
        document.getElementById('sort-select')?.addEventListener('change', (e) => {
            this.currentSort = e.target.value;
            this.render();
        });

        // Terminal toggle
        document.getElementById('terminal-header')?.addEventListener('click', () => {
            this.terminalCollapsed = !this.terminalCollapsed;
            document.getElementById('terminal-panel')?.classList.toggle('collapsed', this.terminalCollapsed);
        });
        document.getElementById('terminal-clear')?.addEventListener('click', (e) => {
            e.stopPropagation();
            this.clearTerminal();
        });
        document.getElementById('terminal-toggle')?.addEventListener('click', (e) => {
            e.stopPropagation();
            this.terminalCollapsed = !this.terminalCollapsed;
            document.getElementById('terminal-panel')?.classList.toggle('collapsed', this.terminalCollapsed);
        });

        // Modals
        document.getElementById('close-config-modal')?.addEventListener('click', () => this.closeModal('config-modal'));
        document.getElementById('close-restart-modal')?.addEventListener('click', () => this.closeModal('restart-modal'));
        document.getElementById('cancel-restart')?.addEventListener('click', () => this.closeModal('restart-modal'));
        document.getElementById('confirm-restart')?.addEventListener('click', () => this.doRestart());
        document.getElementById('edit-config-btn')?.addEventListener('click', () => this.openConfigModal());
        document.getElementById('restart-btn')?.addEventListener('click', () => this.openRestartModal());
        document.getElementById('save-config-btn')?.addEventListener('click', () => this.saveConfig());
        document.getElementById('cancel-config-btn')?.addEventListener('click', () => this.closeModal('config-modal'));

        // Working directory picker
        document.getElementById('browse-working-dir')?.addEventListener('click', () => {
            document.getElementById('working-dir-picker')?.click();
        });
        document.getElementById('working-dir-picker')?.addEventListener('change', (e) => {
            const files = e.target.files;
            if (files && files.length > 0) {
                // webkitdirectory returns files; use the first file's path logic
                const path = files[0].path || files[0].name;
                // Try to derive directory from full path if available (Electron/Tauri) or use name
                const dirInput = document.getElementById('cfg-working-dir');
                if (dirInput) {
                    // For web browsers, webkitdirectory doesn't give full path due to security.
                    // We use a heuristic: if path contains separator, take dirname.
                    const lastSep = path.lastIndexOf('/');
                    const lastSepWin = path.lastIndexOf('\\');
                    const sepIdx = Math.max(lastSep, lastSepWin);
                    dirInput.value = sepIdx > 0 ? path.substring(0, sepIdx) : path;
                }
            }
        });

        // Tool modal close
        document.getElementById('close-tool-modal')?.addEventListener('click', () => this.closeModal('tool-modal'));
        document.getElementById('close-tool-modal-btn')?.addEventListener('click', () => this.closeModal('tool-modal'));
        document.getElementById('tool-modal-toggle')?.addEventListener('click', () => this.toggleCurrentTool());

        // About modal
        document.getElementById('about-btn')?.addEventListener('click', () => this.openAboutModal());
        document.getElementById('close-about-modal')?.addEventListener('click', () => this.closeModal('about-modal'));
        document.getElementById('close-about-btn')?.addEventListener('click', () => this.closeModal('about-modal'));

        // Batch actions
        document.getElementById('batch-enable-all')?.addEventListener('click', () => this.batchEnableTools(true));
        document.getElementById('batch-disable-all')?.addEventListener('click', () => this.batchEnableTools(false));

        // Filter buttons
        document.querySelectorAll('#filter-group .filter-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                this.currentFilter = btn.dataset.filter;
                document.querySelectorAll('#filter-group .filter-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                this.render();
            });
        });

        // Retry button
        document.getElementById('retry-btn')?.addEventListener('click', () => this.loadData());

        // Modal background click & ESC
        document.querySelectorAll('.modal').forEach(modal => {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    this.closeModal(modal.id);
                }
            });
        });
        this._keydownHandler = (e) => {
            if (e.key === 'Escape') {
                const openModal = document.querySelector('.modal.show');
                if (openModal) {
                    this.closeModal(openModal.id);
                }
                if (document.getElementById('sidebar')?.classList.contains('open')) {
                    this.closeSidebar();
                }
            }
        };
        document.addEventListener('keydown', this._keydownHandler);

        // Sidebar mobile actions
        document.getElementById('sidebar-lang-toggle')?.addEventListener('click', () => {
            this.lang = this.lang === 'zh' ? 'en' : 'zh';
            const text = this.lang === 'zh' ? 'EN' : '中文';
            document.getElementById('lang-toggle').textContent = text;
            const sidebarLang = document.getElementById('sidebar-lang-toggle');
            if (sidebarLang) sidebarLang.textContent = text;
            this.render();
            this.renderConfig();
        });
        document.getElementById('sidebar-theme-toggle')?.addEventListener('click', () => {
            this.cycleTheme();
        });
        document.getElementById('sidebar-about-btn')?.addEventListener('click', () => {
            this.closeSidebar();
            this.openAboutModal();
        });

        this.initCardTiltDelegation();
    }

    closeSidebar() {
        document.getElementById('sidebar')?.classList.remove('open');
        document.getElementById('overlay')?.classList.remove('show');
    }

    closeModal(id) {
        document.getElementById(id)?.classList.remove('show');
        this.bgAnimationPaused = false;
        document.body.classList.remove('modal-open');
    }

    openModal(id) {
        document.getElementById(id)?.classList.add('show');
        this.bgAnimationPaused = true;
        document.body.classList.add('modal-open');
    }

    // ============================================================
    // ACTIONS
    // ============================================================
    async toggleMCPService(enable) {
        try {
            const endpoint = enable ? '/api/mcp/start' : '/api/mcp/stop';
            const res = await fetch(endpoint, { method: 'POST' });
            if (!res.ok) throw new Error('Failed');
            const data = await res.json();
            this.updateMCPStatus(enable);
        } catch (err) {
            this.showError(this.t('connectionError'));
            document.getElementById('mcp-toggle').checked = !enable;
        }
    }

    async toggleTool(name, enable) {
        try {
            const res = await fetch(`/api/tool/${encodeURIComponent(name)}/enable`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ enabled: enable })
            });
            if (!res.ok) throw new Error('Failed');
            const tool = this.tools.find(t => t.name === name);
            if (tool) { tool.enabled = enable; this.render(); }
        } catch (err) {
            this.showError(this.t('connectionError'));
        }
    }

    async batchEnableTools(enabled) {
        try {
            const toolNames = this.tools.map(t => t.name);
            const res = await fetch('/api/tools/batch-enable', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ tools: toolNames, enabled })
            });
            if (!res.ok) throw new Error('Failed');
            this.tools.forEach(t => t.enabled = enabled);
            this.currentPreset = null; // Custom selection overrides preset
            this.render();
            this.showSuccess(enabled ? 'All tools enabled' : 'All tools disabled');
        } catch (err) {
            this.showError(this.t('connectionError'));
        }
    }

    async applyPreset(name) {
        try {
            const res = await fetch(`/api/tool-presets/apply/${encodeURIComponent(name)}`, {
                method: 'POST'
            });
            if (!res.ok) throw new Error('Failed');
            const data = await res.json();
            if (data.success) {
                this.currentPreset = name;
                // Refresh tools to get updated enabled states
                const toolsRes = await fetch('/api/tools');
                if (toolsRes.ok) {
                    const toolsData = await toolsRes.json();
            this.tools = toolsData.tools || [];
                }
                this.render();
                this.showSuccess(`Preset "${name}" applied`);
            }
        } catch (err) {
            this.showError(this.t('connectionError'));
        }
    }

    async togglePythonFsAccess(enabled) {
        try {
            const res = await fetch('/api/python-fs-access', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ enabled })
            });
            if (!res.ok) throw new Error('Failed');
            this.pythonFsAccessEnabled = enabled;
            this.render();
            this.showSuccess(enabled ? 'Filesystem access enabled' : 'Filesystem access disabled');
        } catch (err) {
            this.showError(this.t('connectionError'));
        }
    }

    currentModalToolName = null;

    getToolDescription(name) {
        const t = this.toolI18n[this.lang]?.[name];
        return t?.desc || this.tools.find(x => x.name === name)?.description || '';
    }

    getToolUsage(name) {
        const t = this.toolI18n[this.lang]?.[name];
        return t?.usage || this.tools.find(x => x.name === name)?.description || 'N/A';
    }

    async openToolModal(name) {
        this.currentModalToolName = name;
        const [info, stats] = await Promise.all([
            this.loadToolInfo(name),
            this.loadToolStats(name)
        ]);
        const tool = this.tools.find(t => t.name === name);
        if (!tool) return;

        document.getElementById('modal-tool-name').textContent = name;
        document.getElementById('modal-tool-description').textContent = this.getToolDescription(name);
        document.getElementById('modal-tool-usage').textContent = this.getToolUsage(name);

        const toggleBtn = document.getElementById('tool-modal-toggle');
        toggleBtn.textContent = tool.enabled ? this.t('disable') : this.t('enable');
        toggleBtn.className = `btn ${tool.enabled ? 'btn-warning' : 'btn-primary'}`;

        // Stats
        document.getElementById('modal-call-count').textContent = tool.call_count || 0;
        const recentCalls = stats?.recent_call_times || [];
        document.getElementById('modal-last-call').textContent = recentCalls[0] || 'N/A';
        document.getElementById('modal-avg-duration').textContent =
            (stats?.avg_duration_ms && stats.avg_duration_ms > 0) ? `${stats.avg_duration_ms.toFixed(0)} ms` : 'N/A';
        document.getElementById('modal-error-rate').textContent =
            (stats?.error_rate !== undefined) ? `${stats.error_rate.toFixed(1)}%` : 'N/A';

        // Chart — defer draw until layout is complete
        requestAnimationFrame(() => {
            this.drawToolChart(name, stats?.stats_history || []);
        });

        // Recent calls
        const callsList = document.getElementById('recent-calls-list');
        callsList.innerHTML = recentCalls.length
            ? recentCalls.map(t => `<li>${t}</li>`).join('')
            : '<li>No recent calls</li>';

        this.openModal('tool-modal');
    }

    async loadToolStats(name) {
        try {
            const res = await fetch(`/api/tool/${encodeURIComponent(name)}/stats`);
            if (res.ok) return await res.json();
        } catch (e) {}
        return null;
    }

    toggleCurrentTool() {
        if (!this.currentModalToolName) return;
        const tool = this.tools.find(t => t.name === this.currentModalToolName);
        if (tool) this.toggleTool(tool.name, !tool.enabled);
        this.closeModal('tool-modal');
    }

    drawToolChart(name, history) {
        const canvas = document.getElementById('tool-chart');
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        const data = history.slice(-24);
        if (data.length < 2) { ctx.clearRect(0, 0, canvas.width, canvas.height); return; }

        canvas.width = canvas.offsetWidth || 300;
        canvas.height = canvas.offsetHeight || 150;
        if (canvas.width === 0 || canvas.height === 0) return;
        ctx.clearRect(0, 0, canvas.width, canvas.height);

        const max = Math.max(...data, 1);
        const w = canvas.width, h = canvas.height;
        const padLeft = 42, padRight = 15, padTop = 15, padBottom = 28;
        const chartW = w - padLeft - padRight;
        const chartH = h - padTop - padBottom;

        // Theme-aware colors
        const rootStyle = getComputedStyle(document.documentElement);
        const accent = rootStyle.getPropertyValue('--neon-cyan').trim() || '#2dd4bf';
        const muted = rootStyle.getPropertyValue('--text-muted').trim() || '#64748b';
        const gridColor = rootStyle.getPropertyValue('--border-color').trim() || 'rgba(45, 212, 191, 0.25)';

        // Parse accent hex to rgba for gradients
        let r = 45, g = 212, b = 191;
        const hexMatch = accent.match(/^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i);
        if (hexMatch) {
            r = parseInt(hexMatch[1], 16);
            g = parseInt(hexMatch[2], 16);
            b = parseInt(hexMatch[3], 16);
        }

        // Compute points
        const points = data.map((count, i) => ({
            x: padLeft + (i / (data.length - 1)) * chartW,
            y: padTop + chartH - (count / max) * chartH
        }));

        // Grid lines (horizontal, dashed)
        ctx.strokeStyle = gridColor;
        ctx.lineWidth = 1;
        ctx.setLineDash([4, 4]);
        const ySteps = 4;
        for (let i = 0; i <= ySteps; i++) {
            const y = padTop + (i / ySteps) * chartH;
            ctx.beginPath();
            ctx.moveTo(padLeft, y);
            ctx.lineTo(padLeft + chartW, y);
            ctx.stroke();
        }
        ctx.setLineDash([]);

        // Y-axis labels (call count)
        ctx.fillStyle = muted;
        ctx.font = '10px var(--font-mono)';
        ctx.textAlign = 'right';
        ctx.textBaseline = 'middle';
        for (let i = 0; i <= ySteps; i++) {
            const val = Math.round(max * (1 - i / ySteps));
            const y = padTop + (i / ySteps) * chartH;
            ctx.fillText(val.toString(), padLeft - 8, y);
        }

        // X-axis labels (time intervals)
        ctx.textAlign = 'center';
        ctx.textBaseline = 'top';
        const xLabels = ['now', '30m', '60m', '90m', '120m'];
        const xIndices = [0, Math.floor(data.length / 4), Math.floor(data.length / 2), Math.floor(data.length * 3 / 4), data.length - 1];
        xIndices.forEach((idx, i) => {
            if (idx >= 0 && idx < data.length) {
                const x = padLeft + (idx / (data.length - 1)) * chartW;
                ctx.fillText(xLabels[i] || '', x, padTop + chartH + 8);
            }
        });

        // Smooth curve (Catmull-Rom spline → cubic Bezier with clamped control points)
        ctx.beginPath();
        ctx.strokeStyle = accent;
        ctx.lineWidth = 2.5;
        ctx.shadowColor = accent;
        ctx.shadowBlur = 14;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';

        const chartMinY = padTop;
        const chartMaxY = padTop + chartH;

        ctx.moveTo(points[0].x, points[0].y);
        for (let i = 0; i < points.length - 1; i++) {
            const p0 = points[Math.max(0, i - 1)];
            const p1 = points[i];
            const p2 = points[i + 1];
            const p3 = points[Math.min(points.length - 1, i + 2)];

            // Lower tension (0.15 vs original 1/6≈0.167) to reduce overshoot
            const tension = 0.15;
            let cp1x = p1.x + (p2.x - p0.x) * tension;
            let cp1y = p1.y + (p2.y - p0.y) * tension;
            let cp2x = p2.x - (p3.x - p1.x) * tension;
            let cp2y = p2.y - (p3.y - p1.y) * tension;

            // Clamp control-point Y to chart bounds so the curve never dips below/above
            cp1y = Math.max(chartMinY, Math.min(chartMaxY, cp1y));
            cp2y = Math.max(chartMinY, Math.min(chartMaxY, cp2y));

            ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
        }
        ctx.stroke();
        ctx.shadowBlur = 0;

        // Fill area under curve
        ctx.lineTo(points[points.length - 1].x, padTop + chartH);
        ctx.lineTo(points[0].x, padTop + chartH);
        ctx.closePath();
        const grad = ctx.createLinearGradient(0, padTop, 0, padTop + chartH);
        grad.addColorStop(0, `rgba(${r}, ${g}, ${b}, 0.22)`);
        grad.addColorStop(1, `rgba(${r}, ${g}, ${b}, 0.0)`);
        ctx.fillStyle = grad;
        ctx.fill();
    }

    openConfigModal() {
        this.openModal('config-modal');
        if (this.config) {
            const setVal = (name, val) => {
                const el = document.querySelector(`#config-form [name="${name}"]`);
                if (el) el.value = val !== undefined ? val : '';
            };
            setVal('webui_host', this.config.webui_host);
            setVal('webui_port', this.config.webui_port);
            setVal('mcp_transport', this.config.mcp_transport);
            setVal('mcp_host', this.config.mcp_host);
            setVal('mcp_port', this.config.mcp_port);
            setVal('max_concurrency', this.config.max_concurrency);
            setVal('working_dir', this.config.working_dir);
            setVal('log_level', this.config.log_level);
        }
    }

    renderConfig() {
        if (!this.config) return;
        const grid = document.getElementById('config-grid');
        if (!grid) return;

        const fields = [
            ['cfgWebuiHost', this.config.webui_host],
            ['cfgWebuiPort', this.config.webui_port],
            ['cfgMcpTransport', this.config.mcp_transport],
            ['cfgMcpHost', this.config.mcp_host],
            ['cfgMcpPort', this.config.mcp_port],
            ['cfgMaxConcurrency', this.config.max_concurrency],
            ['cfgWorkingDir', this.config.working_dir],
            ['cfgLogLevel', this.config.log_level],
        ];
        grid.innerHTML = fields.map(([key, val]) => `
            <div class="config-item">
                <label>${this.t(key)}</label>
                <div class="config-value">${val !== undefined ? this.escapeHtml(String(val)) : 'N/A'}</div>
            </div>
        `).join('');
    }

    async saveConfig() {
        const form = document.getElementById('config-form');
        if (!form) { this.closeModal('config-modal'); return; }

        const body = {};
        const fd = new FormData(form);
        for (const [key, val] of fd.entries()) {
            if (val !== '') {
                if (key === 'webui_port' || key === 'mcp_port' || key === 'max_concurrency') {
                    const n = parseInt(val, 10);
                    if (!isNaN(n)) body[key] = n;
                } else {
                    body[key] = val;
                }
            }
        }

        try {
            const res = await fetch('/api/config', {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body)
            });
            if (!res.ok) throw new Error('Failed');
            const data = await res.json();
            this.showSuccess(data.message || 'Config saved');
            if (data.restart_required) {
                setTimeout(() => this.openRestartModal(), 500);
            }
        } catch (err) {
            this.showError(this.t('connectionError'));
        }
        this.closeModal('config-modal');
    }

    openRestartModal() {
        document.getElementById('restart-message').textContent = this.t('restartConfirmText');
        this.openModal('restart-modal');
    }

    async openAboutModal() {
        // Update static i18n texts
        document.getElementById('about-modal-title').textContent = this.t('about');
        document.getElementById('about-desc').textContent = this.t('aboutDescription');
        document.getElementById('about-author-label').textContent = this.t('author');
        document.getElementById('about-license-label').textContent = this.t('license');
        document.getElementById('about-github').querySelector('span').textContent = this.t('github');

        // Try to fetch version from API
        try {
            const res = await fetch('/api/version');
            if (res.ok) {
                const data = await res.json();
                document.getElementById('about-version').textContent = 'v' + (data.version || '0.4.0');
                if (data.authors) {
                    document.getElementById('about-author').textContent = data.authors;
                }
            }
        } catch (e) {
            // fallback to defaults
        }

        this.openModal('about-modal');
    }

    async doRestart() {
        this.closeModal('restart-modal');
        try {
            const res = await fetch('/api/mcp/restart', { method: 'POST' });
            if (res.ok) {
                this.showSuccess('Server restarting...');
                setTimeout(() => location.reload(), 3000);
            }
        } catch (err) {
            this.showError(this.t('connectionError'));
        }
    }

    setAlphabetFilter(letter) {
        this.currentAlphabet = letter;
        this.render();
    }

    // ============================================================
    // RENDER
    // ============================================================
    t(key) { return this.i18n[this.lang][key] || key; }

    // Convert snake_case to camelCase for i18n key lookup
    toCamelCase(s) { return s.replace(/_([a-z])/g, (_, ch) => ch.toUpperCase()); }

    // Translate a preset name (e.g. 'data_analysis' -> '数据分析')
    translatePresetName(name) {
        const key = 'presetName' + this.toCamelCase(name.charAt(0).toUpperCase() + name.slice(1));
        return this.i18n[this.lang][key] || name;
    }

    escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    filterTools() {
        let filtered = [...this.tools];
        if (this.currentFilter !== 'all') {
            filtered = filtered.filter(t => t.is_dangerous === (this.currentFilter === 'dangerous'));
        }
        if (this.searchQuery) {
            filtered = filtered.filter(t =>
                t.name.toLowerCase().includes(this.searchQuery) ||
                (t.description && t.description.toLowerCase().includes(this.searchQuery))
            );
        }
        if (this.currentAlphabet !== 'all') {
            filtered = filtered.filter(t => t.name.toLowerCase().startsWith(this.currentAlphabet));
        }
        return filtered;
    }

    sortTools(tools) {
        const sorted = [...tools];
        switch (this.currentSort) {
            case 'name':
                sorted.sort((a, b) => a.name.localeCompare(b.name));
                break;
            case 'calls':
                sorted.sort((a, b) => (b.call_count || 0) - (a.call_count || 0));
                break;

        }
        return sorted;
    }

    render() {
        this.renderAlphabetNav();
        this.renderTools();
        this.renderPresets();
        this.updateLangUI();
        this.updateCallsHud();
    }

    renderPresets() {
        const grid = document.getElementById('preset-grid');
        const currentEl = document.getElementById('preset-current');
        if (!grid) return;
        if (currentEl) {
            const currentName = this.currentPreset ? this.translatePresetName(this.currentPreset) : this.t('presetNone');
            currentEl.textContent = `${this.t('presetCurrent')}: ${currentName}`;
        }
        grid.innerHTML = this.presets.map(p => {
            const name = this.translatePresetName(p.name);
            const countText = this.t('presetToolsCount').replace('{count}', p.tool_count);
            return `
            <button class="preset-btn ${this.currentPreset === p.name ? 'active' : ''}" data-preset="${p.name}">
                <span class="preset-name">${name}</span>
                <span class="preset-count">${countText}</span>
            </button>
        `}).join('');

        document.querySelectorAll('#preset-grid .preset-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const name = btn.dataset.preset;
                this.applyPreset(name);
            });
        });
    }

    updateCallsHud() {
        const callsEl = document.querySelector('.hud-number[data-metric="calls"]');
        if (callsEl) {
            const total = this.tools.reduce((sum, t) => sum + (t.call_count || 0), 0);
            callsEl.textContent = total;
        }
        const concEl = document.querySelector('.hud-number[data-metric="concurrency"]');
        if (concEl && this.config) {
            concEl.textContent = `${this.currentConcurrency}/${this.config.max_concurrency}`;
        }
    }

    renderAlphabetNav() {
        const nav = document.getElementById('alphabet-nav');
        if (!nav) return;
        const letters = ['all', ...Array.from('abcdefghijklmnopqrstuvwxyz')];
        nav.innerHTML = letters.map(l =>
            `<button class="${this.currentAlphabet === l ? 'active' : ''}" data-letter="${l}">${l === 'all' ? (this.lang === 'zh' ? '全部' : 'All') : l.toUpperCase()}</button>`
        ).join('');
        nav.querySelectorAll('button[data-letter]').forEach(btn => {
            btn.addEventListener('click', () => {
                this.setAlphabetFilter(btn.dataset.letter);
            });
        });
    }

    renderTools() {
        const safeContainer = document.getElementById('safe-tools');
        const dangerContainer = document.getElementById('dangerous-tools');
        const safeSection = document.getElementById('safe-section');
        const dangerSection = document.getElementById('danger-section');
        if (!safeContainer || !dangerContainer) return;

        const filtered = this.sortTools(this.filterTools());
        const safeTools = filtered.filter(t => !t.is_dangerous);
        const dangerTools = filtered.filter(t => t.is_dangerous);

        safeSection.style.display = safeTools.length ? '' : 'none';
        dangerSection.style.display = dangerTools.length ? '' : 'none';

        safeContainer.innerHTML = safeTools.length
            ? safeTools.map(t => this.renderToolCard(t)).join('')
            : `<div class="no-tools">${this.t('noTools')}</div>`;

        dangerContainer.innerHTML = dangerTools.length
            ? dangerTools.map(t => this.renderToolCard(t)).join('')
            : `<div class="no-tools">${this.t('noTools')}</div>`;

        // Re-bind toggle buttons
        document.querySelectorAll('.tool-toggle-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const name = btn.dataset.tool;
                const enable = btn.dataset.action === 'enable';
                this.toggleTool(name, enable);
            });
        });

        // Re-bind fs-access toggles
        document.querySelectorAll('input[data-toggle="fs-access"]').forEach(input => {
            input.addEventListener('change', (e) => {
                e.stopPropagation();
                this.togglePythonFsAccess(e.target.checked);
            });
        });

        // Re-bind card click for info
        document.querySelectorAll('.tool-card[data-tool]').forEach(card => {
            card.addEventListener('click', (e) => {
                if (e.target.closest('.tool-toggle-btn')) return;
                if (e.target.closest('.tool-fs-toggle')) return;
                this.openToolModal(card.dataset.tool);
            });
        });
    }

    renderToolCard(tool) {
        const statusClass = tool.is_calling ? 'calling' : 'idle';
        const statusText = tool.is_calling ? this.t('calling') : this.t('idle');
        const dotClass = tool.enabled ? (tool.is_calling ? 'calling' : '') : 'disabled';
        const description = this.getToolDescription(tool.name);
        const fsToggle = tool.name === 'ExecutePython' ? `
            <div class="tool-fs-toggle">
                <span class="fs-toggle-label">${this.lang === 'zh' ? '文件系统' : 'Filesystem'}</span>
                <label class="neon-switch small">
                    <input type="checkbox" data-tool="ExecutePython" data-toggle="fs-access" ${this.pythonFsAccessEnabled ? 'checked' : ''}>
                    <span class="slider"></span>
                </label>
            </div>
        ` : '';

        return `
            <div class="tool-card ${tool.is_dangerous ? 'dangerous' : ''} ${!tool.enabled ? 'disabled' : ''}" data-tool="${tool.name}">
                <div class="tool-header">
                    <div class="tool-name">
                        <span class="tool-status-dot ${dotClass}"></span>
                        ${this.escapeHtml(tool.name)}
                    </div>
                    <button class="tool-toggle-btn btn ${tool.enabled ? 'btn-warning' : 'btn-primary'}" 
                            data-tool="${tool.name}" data-action="${tool.enabled ? 'disable' : 'enable'}"
                            style="padding:0.3rem 0.6rem;font-size:0.7rem;">
                        ${tool.enabled ? this.t('disable') : this.t('enable')}
                    </button>
                </div>
                <div class="tool-description">${this.escapeHtml(description)}</div>
                <div class="tool-stats-summary">
                    <div class="tool-stat">
                        <span class="tool-stat-label">${this.t('calls')}:</span>
                        <span class="tool-stat-value">${tool.call_count || 0}</span>
                    </div>
                    ${fsToggle}
                </div>
                <div class="tool-footer">
                    <span class="tool-status ${statusClass}">${statusText}</span>
                    <span style="font-size:0.7rem;color:var(--text-muted);font-family:var(--font-mono);">${tool.enabled ? this.t('enabled') : this.t('disabled')}</span>
                </div>
            </div>
        `;
    }

    updateLangUI() {
        const t = document.getElementById('brand-title');
        const s = document.getElementById('brand-subtitle');
        if (t) t.textContent = this.t('title');
        if (s) s.textContent = this.t('subtitle');

        const safeHeader = document.getElementById('safe-header-text');
        const dangerHeader = document.getElementById('danger-header-text');
        if (safeHeader) safeHeader.textContent = this.t('safeTools');
        if (dangerHeader) dangerHeader.textContent = this.t('dangerousTools');

        const search = document.getElementById('search-input');
        if (search) search.placeholder = this.t('searchPlaceholder');

        const sort = document.getElementById('sort-select');
        if (sort) {
            sort.innerHTML = `
                <option value="name">${this.t('sortByName')}</option>
                <option value="calls">${this.t('sortByCalls')}</option>
            `;
            sort.value = this.currentSort;
        }

        const mcpStatus = document.getElementById('mcp-status');
        if (mcpStatus) {
            const running = mcpStatus.classList.contains('running');
            mcpStatus.textContent = running ? this.t('mcpRunning') : this.t('mcpStopped');
        }

        const terminalTitle = document.getElementById('terminal-title-text');
        if (terminalTitle) terminalTitle.textContent = this.t('terminalTitle');

        const sidebarTitle = document.getElementById('sidebar-title');
        if (sidebarTitle) sidebarTitle.textContent = this.t('config');

        const presetSectionTitle = document.getElementById('preset-section-title');
        if (presetSectionTitle) presetSectionTitle.textContent = this.t('presetSection');

        const batchSectionTitle = document.getElementById('batch-section-title');
        if (batchSectionTitle) batchSectionTitle.textContent = this.t('batchSection');

        const batchEnableAll = document.getElementById('batch-enable-all');
        if (batchEnableAll) batchEnableAll.textContent = this.t('batchEnableAll');

        const batchDisableAll = document.getElementById('batch-disable-all');
        if (batchDisableAll) batchDisableAll.textContent = this.t('batchDisableAll');

        const presetCurrent = document.getElementById('preset-current');
        if (presetCurrent) {
            const currentName = this.currentPreset ? this.translatePresetName(this.currentPreset) : this.t('presetNone');
            presetCurrent.textContent = `${this.t('presetCurrent')}: ${currentName}`;
        }

        const configTitle = document.getElementById('config-modal-title');
        if (configTitle) configTitle.textContent = this.t('configTitle');

        const toolInfoTitle = document.getElementById('tool-info-title');
        if (toolInfoTitle) toolInfoTitle.textContent = this.t('toolInfo');

        const recentCallsTitle = document.getElementById('recent-calls-title');
        if (recentCallsTitle) recentCallsTitle.textContent = this.t('recentCalls');

        const toolUsageTitle = document.getElementById('tool-usage-title');
        if (toolUsageTitle) toolUsageTitle.textContent = this.t('usage');

        const statCallCount = document.getElementById('stat-call-count-label');
        if (statCallCount) statCallCount.textContent = this.t('callCount');

        const statLastCall = document.getElementById('stat-last-call-label');
        if (statLastCall) statLastCall.textContent = this.t('lastCall');

        const statAvgDur = document.getElementById('stat-avg-duration-label');
        if (statAvgDur) statAvgDur.textContent = this.t('avgDuration');

        const statErrorRate = document.getElementById('stat-error-rate-label');
        if (statErrorRate) statErrorRate.textContent = this.t('errorRate');

        const chartTitle = document.getElementById('chart-title');
        if (chartTitle) chartTitle.textContent = this.t('callTrend');

        const configSectionTitle = document.getElementById('config-section-title');
        if (configSectionTitle) configSectionTitle.textContent = this.t('systemConfig');

        const btnEditConfig = document.getElementById('edit-config-btn');
        if (btnEditConfig) btnEditConfig.textContent = this.t('editConfig');

        const btnRestart = document.getElementById('restart-btn');
        if (btnRestart) btnRestart.textContent = this.t('restartServer');

        const btnSave = document.getElementById('save-config-btn');
        if (btnSave) btnSave.textContent = this.t('save');

        const btnCancel = document.getElementById('cancel-config-btn');
        if (btnCancel) btnCancel.textContent = this.t('cancel');

        const restartModalTitle = document.querySelector('#restart-modal .modal-header h3');
        if (restartModalTitle) restartModalTitle.textContent = this.t('confirmRestart');

        const btnConfirmRestart = document.getElementById('confirm-restart');
        if (btnConfirmRestart) btnConfirmRestart.textContent = this.t('confirmRestart');

        const btnCancelRestart = document.getElementById('cancel-restart');
        if (btnCancelRestart) btnCancelRestart.textContent = this.t('cancel');

        // Config form labels
        document.querySelectorAll('#config-form [data-i18n-key]').forEach(el => {
            const key = el.getAttribute('data-i18n-key');
            if (key) el.textContent = this.t(key);
        });

        // Browse button
        const browseBtn = document.getElementById('browse-working-dir');
        if (browseBtn) browseBtn.textContent = this.t('browse');

        // Filter buttons
        const filterAll = document.getElementById('filter-all');
        const filterSafe = document.getElementById('filter-safe');
        const filterDanger = document.getElementById('filter-danger');
        if (filterAll) filterAll.textContent = this.t('filterAll');
        if (filterSafe) filterSafe.textContent = this.t('filterSafe');
        if (filterDanger) filterDanger.textContent = this.t('filterDangerous');

        this.renderPresets();
    }

    showError(msg) {
        const el = document.getElementById('error-message');
        if (el) { el.textContent = msg; el.style.display = 'block'; setTimeout(() => el.style.display = 'none', 5000); }
    }

    showSuccess(msg) {
        const el = document.getElementById('success-message');
        if (el) { el.textContent = msg; el.style.display = 'block'; setTimeout(() => el.style.display = 'none', 3000); }
    }

    showLoading(show) {
        const el = document.getElementById('loading-indicator');
        const text = document.getElementById('loading-text');
        if (el) el.style.display = show ? 'flex' : 'none';
        if (text) text.textContent = this.t('loading');
    }

    showRetry(show, msg) {
        const el = document.getElementById('retry-message');
        const text = document.getElementById('retry-text');
        const btn = document.getElementById('retry-btn');
        if (el) el.style.display = show ? 'flex' : 'none';
        if (text) text.textContent = msg || '';
        if (btn) btn.textContent = this.t('retry');
    }
}

// ============================================================
// BOOT
// ============================================================
document.addEventListener('DOMContentLoaded', () => {
    window.cc = new CommandCenter();
});
