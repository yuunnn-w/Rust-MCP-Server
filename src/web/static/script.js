// ============================================================
// Cyberpunk AI Command Center — Frontend Controller
// ============================================================

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
        this.metricsInterval = null;
        this.theme = 'system'; // 'dark' | 'light' | 'system'
        this.pythonFsAccessEnabled = false;

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
                description: '高性能模型上下文协议（MCP）服务器，带 WebUI 控制面板。',
                github: 'GitHub',
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
                description: 'A high-performance Model Context Protocol (MCP) server with WebUI control panel.',
                github: 'GitHub',
            }
        };

        this.toolI18n = {
            zh: {
                dir_list: { desc: '列出目录内容，支持过滤和精简模式（最大深度5）。对文本文件自动返回字符数和行数', usage: '用法：列出目录内容。\n参数：path（路径），可选 max_depth（默认2，最大5），可选 include_hidden，可选 pattern（glob 如 "*.rs"），可选 brief（默认true），可选 sort_by（name/type/size/modified），可选 flatten（默认false，扁平列表）\n返回：对每个文件，若为 UTF-8 文本则包含 char_count 和 line_count\n示例：{"path": "/home/user", "pattern": "*.rs", "brief": true}' },
                file_read: { desc: '并发读取一个或多个文本文件，每个文件可独立设置行号、范围等参数', usage: '用法：并发读取多个文本文件。\n参数：files（文件列表），每个文件项包含 path, 可选 start_line（默认0），可选 end_line（默认500），可选 offset_chars，可选 max_chars（默认15000），可选 line_numbers（默认true），可选 highlight_line（1-based）\n返回：每个文件一个结果对象，包含 success, content, lines_displayed, total_lines, truncated\n示例：{"files": [{"path": "a.txt", "start_line": 0, "end_line": 100}, {"path": "b.txt", "start_line": 0, "end_line": 50}]}' },
                file_search: { desc: '搜索关键词并返回匹配片段及上下文（最大深度5）', usage: '用法：在文件或目录中搜索关键词。\n参数：path（路径），keyword（关键词），可选 file_pattern（glob），可选 use_regex（默认false），可选 max_results（默认20），可选 context_lines（默认3），可选 brief（默认false），可选 output_format（detailed/compact/location，默认detailed）\n示例：{"path": "/home/user/src", "keyword": "TODO", "context_lines": 3}' },
                file_edit: { desc: '并发编辑一个或多个文件，支持字符串替换、行号替换、插入、删除或补丁模式。string_replace/line_replace/insert 可自动创建新文件（危险操作）', usage: '用法：并发编辑多个文件。\n参数：operations（操作列表），每个操作包含 path, mode, 以及对应模式的参数。\nstring_replace: path, old_string, new_string, 可选 occurrence（1=默认第一处，0=全部）。若文件不存在且有 new_string，则创建新文件。\nline_replace: path, start_line, end_line, new_string。若文件不存在且有 new_string，则创建新文件。\ninsert: path, start_line, new_string。若文件不存在且有 new_string，则创建新文件。\ndelete: path, start_line, end_line\npatch: path, patch（unified diff 字符串）\n示例：{"operations": [{"path": "main.rs", "mode": "string_replace", "old_string": "fn old()", "new_string": "fn new()"}, {"path": "new.rs", "mode": "insert", "new_string": "fn main() {}"}]}' },
                file_write: { desc: '并发将内容写入一个或多个文件（危险操作）', usage: '用法：并发写入多个文件。\n参数：files（文件列表），每个文件项包含 path, content, 可选 mode（new/append/overwrite，默认new）\n返回：每个文件一个结果对象，包含 success, message, bytes_written\n示例：{"files": [{"path": "test.txt", "content": "Hello", "mode": "new"}, {"path": "log.txt", "content": "Line", "mode": "append"}]}' },
                file_ops: { desc: '并发复制、移动、删除或重命名一个或多个文件（危险操作）', usage: '用法：并发执行多个文件操作。\n参数：operations（操作列表），每个操作包含 action（copy/move/delete/rename），source，可选 target，可选 overwrite（默认false）\n返回：每个操作一个结果对象，包含 success, message\n示例：{"operations": [{"action": "copy", "source": "a.txt", "target": "b.txt"}, {"action": "delete", "source": "file.txt"}]}' },
                file_stat: { desc: '并发获取一个或多个文件或目录的元数据。对文本文件额外返回字符数、行数和编码', usage: '用法：并发获取多个文件/目录的元数据。\n参数：paths（路径列表）\n返回：每个路径一个结果对象，包含 name, size, file_type, readable, writable, modified/created/accessed。对 UTF-8 文本文件额外包含 is_text, char_count, line_count, encoding\n示例：{"paths": ["src/main.rs", "Cargo.toml"]}' },
                path_exists: { desc: '检查路径是否存在并返回其类型', usage: '用法：检查路径存在性。\n参数：path（路径）\n返回：exists (bool), path_type (file/dir/symlink/none)\n示例：{"path": "src/main.rs"}' },
                json_query: { desc: '使用 JSON Pointer 语法查询 JSON 文件', usage: '用法：查询 JSON 文件。\n参数：path（JSON 文件路径），query（JSON Pointer 如 "/data/0/name"），可选 max_chars（默认15000）\n示例：{"path": "config.json", "query": "/database/host"}' },
                git_ops: { desc: '在仓库中运行 git 命令（status, diff, log, branch, show）', usage: '用法：运行 git 命令。\n参数：action（status/diff/log/branch/show），可选 repo_path（默认工作目录），可选 options（额外参数数组）\n示例：{"action": "status"} | {"action": "log", "options": ["--oneline", "-n", "10"]}' },
                calculator: { desc: '计算数学表达式', usage: '用法：计算数学表达式。\n参数：expression（表达式）\n支持：+, -, *, /, ^, sqrt, sin, cos, tan, log, ln, abs, pi, e\n示例：{"expression": "2 + 3 * 4"}' },
                http_request: { desc: '发起 HTTP 请求，支持 JSON 提取和响应限制', usage: '用法：发起 HTTP 请求。\n参数：url（地址），method（GET/POST），可选 headers，可选 body，可选 extract_json_path（如 "/data/0/name"），可选 include_response_headers（默认false），可选 max_response_chars（默认15000）\n示例：{"url": "https://api.example.com", "method": "GET"}' },
                datetime: { desc: '获取当前日期和时间', usage: '用法：获取当前日期时间。\n无需参数。\n示例：{}' },
                image_read: { desc: '读取图像文件并返回 base64 数据或仅元数据', usage: '用法：读取图像文件。\n参数：path（路径），可选 mode（full/metadata，默认 full）\n示例：{"path": "image.png", "mode": "metadata"}' },
                execute_command: { desc: '执行 shell 命令（默认禁用，危险操作）', usage: '用法：执行 shell 命令。\n参数：command（命令），可选 cwd（工作目录），可选 timeout，可选 shell（Windows: cmd/powershell/pwsh; Unix: sh/bash/zsh）\n示例：{"command": "ls -la", "cwd": "/home/user"}' },
                process_list: { desc: '列出系统进程', usage: '用法：列出系统进程。\n无需参数。\n示例：{}' },
                base64_codec: { desc: '对字符串进行 Base64 编码或解码', usage: '用法：Base64 编解码。\n参数：operation（encode/decode），input（输入）\n示例：{"operation": "encode", "input": "Hello, World!"}' },
                hash_compute: { desc: '计算字符串或文件的哈希值（MD5/SHA1/SHA256）', usage: '用法：计算哈希。\n参数：input（输入），algorithm（MD5/SHA1/SHA256）\n文件需前缀 "file:"\n示例：{"input": "hello", "algorithm": "SHA256"}' },
                system_info: { desc: '获取系统信息', usage: '用法：获取系统信息。\n无需参数。\n示例：{}' },
                env_get: { desc: '获取环境变量的值', usage: '用法：获取环境变量。\n参数：name（变量名）\n示例：{"name": "PATH"}' },
                execute_python: { desc: '执行 Python 代码。所有 Python 标准库模块均可使用。', usage: '用法：执行 Python 代码。\n参数：code（Python 代码），可选 timeout_ms（默认5000，最大30000）\n将返回值赋给变量 __result。若未设置，最后一行将自动作为表达式求值。\n所有 Python 标准库模块均可使用。\n通过 WebUI 上的"文件系统"开关可启用本地文件系统访问。\n示例：{"code": "import math\n__result = math.pi * 2"}' },
            },
            en: {
                dir_list: { desc: 'List directory contents with filtering and brief mode (max depth 5). Returns char_count and line_count for UTF-8 text files', usage: 'Usage: List directory contents.\nParameters: path, optional max_depth (default: 2, max: 5), optional include_hidden, optional pattern (glob e.g. "*.rs"), optional brief (default: true), optional sort_by (name/type/size/modified), optional flatten (default: false)\nReturns: For each file, if it is UTF-8 text, includes char_count and line_count\nExample: {"path": "/home/user", "pattern": "*.rs", "brief": true}' },
                file_read: { desc: 'Read one or more text files concurrently, each with independent line/range settings', usage: 'Usage: Read multiple text files concurrently.\nParameters: files (list of file items), each with path, optional start_line (default: 0), optional end_line (default: 500), optional offset_chars, optional max_chars (default: 15000), optional line_numbers (default: true), optional highlight_line (1-based)\nReturns: One result object per file with success, content, lines_displayed, total_lines, truncated\nExample: {"files": [{"path": "a.txt", "start_line": 0, "end_line": 100}, {"path": "b.txt", "start_line": 0, "end_line": 50}]}' },
                file_search: { desc: 'Search for keyword and return matching content fragments with context (max depth 5)', usage: 'Usage: Search for keyword.\nParameters: path, keyword, optional file_pattern (glob), optional use_regex (default: false), optional max_results (default: 20), optional context_lines (default: 3), optional brief (default: false), optional output_format (detailed/compact/location, default: detailed)\nExample: {"path": "/home/user/src", "keyword": "TODO", "context_lines": 3}' },
                file_edit: { desc: 'Edit one or more files concurrently using string_replace, line_replace, insert, delete, or patch mode. string_replace/line_replace/insert can create new files (dangerous operation)', usage: 'Usage: Edit multiple files concurrently.\nParameters: operations (list of operations), each with path, mode, and mode-specific args.\nstring_replace: path, old_string, new_string, optional occurrence (1=first default, 0=all). Creates new file if not exists and new_string is provided.\nline_replace: path, start_line, end_line, new_string. Creates new file if not exists and new_string is provided.\ninsert: path, start_line, new_string. Creates new file if not exists and new_string is provided.\ndelete: path, start_line, end_line\npatch: path, patch (unified diff string)\nExample: {"operations": [{"path": "main.rs", "mode": "string_replace", "old_string": "fn old()", "new_string": "fn new()"}, {"path": "new.rs", "mode": "insert", "new_string": "fn main() {}"}]}' },
                file_write: { desc: 'Write content to one or more files concurrently (dangerous operation)', usage: 'Usage: Write to multiple files concurrently.\nParameters: files (list of file items), each with path, content, optional mode (new/append/overwrite, default: new)\nReturns: One result object per file with success, message, bytes_written\nExample: {"files": [{"path": "test.txt", "content": "Hello", "mode": "new"}, {"path": "log.txt", "content": "Line", "mode": "append"}]}' },
                file_ops: { desc: 'Copy, move, delete, or rename one or more files concurrently (dangerous operation)', usage: 'Usage: Perform multiple file operations concurrently.\nParameters: operations (list of operations), each with action (copy/move/delete/rename), source, optional target, optional overwrite (default: false)\nReturns: One result object per operation with success, message\nExample: {"operations": [{"action": "copy", "source": "a.txt", "target": "b.txt"}, {"action": "delete", "source": "file.txt"}]}' },
                file_stat: { desc: 'Get metadata for one or more files or directories concurrently. Returns char_count, line_count, and encoding for UTF-8 text files', usage: 'Usage: Get metadata for multiple files/directories concurrently.\nParameters: paths (list of paths)\nReturns: One result object per path with name, size, file_type, readable, writable, modified/created/accessed. For UTF-8 text files, also includes is_text, char_count, line_count, encoding\nExample: {"paths": ["src/main.rs", "Cargo.toml"]}' },
                path_exists: { desc: 'Check if a path exists and get its type', usage: 'Usage: Check path existence.\nParameters: path\nReturns: exists (bool), path_type (file/dir/symlink/none)\nExample: {"path": "src/main.rs"}' },
                json_query: { desc: 'Query a JSON file using JSON Pointer syntax', usage: 'Usage: Query JSON file.\nParameters: path (JSON file), query (JSON Pointer like "/data/0/name"), optional max_chars (default: 15000)\nExample: {"path": "config.json", "query": "/database/host"}' },
                git_ops: { desc: 'Run git commands (status, diff, log, branch, show) in a repository', usage: 'Usage: Run git commands.\nParameters: action (status/diff/log/branch/show), optional repo_path (default: working_dir), optional options (array of extra args)\nExample: {"action": "status"} | {"action": "log", "options": ["--oneline", "-n", "10"]}' },
                calculator: { desc: 'Calculate mathematical expressions', usage: 'Usage: Calculate expressions.\nParameter: expression\nSupports: +, -, *, /, ^, sqrt, sin, cos, tan, log, ln, abs, pi, e\nExample: {"expression": "2 + 3 * 4"}' },
                http_request: { desc: 'Make HTTP requests with optional JSON extraction and response limiting', usage: 'Usage: Make HTTP requests.\nParameters: url, method (GET/POST), optional headers, optional body, optional extract_json_path (e.g. "/data/0/name"), optional include_response_headers (default: false), optional max_response_chars (default: 15000)\nExample: {"url": "https://api.example.com", "method": "GET"}' },
                datetime: { desc: 'Get current date and time', usage: 'Usage: Get current date and time.\nNo parameters required.\nExample: {}' },
                image_read: { desc: 'Read an image file and return base64 data or metadata only', usage: 'Usage: Read image file.\nParameters: path, optional mode (full/metadata, default: full)\nExample: {"path": "image.png", "mode": "metadata"}' },
                execute_command: { desc: 'Execute a shell command (disabled by default, dangerous)', usage: 'Usage: Execute shell command.\nParameters: command, optional cwd, optional timeout, optional shell (Windows: cmd/powershell/pwsh; Unix: sh/bash/zsh)\nExample: {"command": "ls -la", "cwd": "/home/user"}' },
                process_list: { desc: 'List system processes', usage: 'Usage: List system processes.\nNo parameters required.\nExample: {}' },
                base64_codec: { desc: 'Encode or decode base64 strings', usage: 'Usage: Base64 encode/decode.\nParameters: operation (encode/decode), input\nExample: {"operation": "encode", "input": "Hello, World!"}' },
                hash_compute: { desc: 'Compute hash of string or file (MD5, SHA1, SHA256)', usage: 'Usage: Compute hash.\nParameters: input, algorithm (MD5/SHA1/SHA256)\nFor files, prefix path with "file:"\nExample: {"input": "hello", "algorithm": "SHA256"}' },
                system_info: { desc: 'Get system information', usage: 'Usage: Get system information.\nNo parameters required.\nExample: {}' },
                env_get: { desc: 'Get the value of an environment variable', usage: 'Usage: Get environment variable.\nParameters: name\nExample: {"name": "PATH"}' },
                execute_python: { desc: 'Execute Python code. All Python standard library modules are available.', usage: 'Usage: Execute Python code.\nParameters: code (Python code), optional timeout_ms (default: 5000, max: 30000)\nAssign the return value to __result. If not set, the last line is automatically evaluated as an expression.\nAll Python standard library modules are available.\nEnable local filesystem access via the WebUI "Filesystem" toggle.\nExample: {"code": "import math\n__result = math.pi * 2"}' },
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
        this.render();
    }

    // ============================================================
    // THEME MANAGEMENT
    // ============================================================
    initTheme() {
        const saved = localStorage.getItem('cc-theme');
        this.theme = saved || 'system';
        this.applyTheme();

        if (this.theme === 'system') {
            window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => this.applyTheme());
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
    }

    getThemeIcon() {
        if (this.theme === 'dark') return '☀️';
        if (this.theme === 'light') return '🌙';
        return '💻';
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

        const resize = () => {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
        };
        resize();
        window.addEventListener('resize', resize);

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

        const getAccentColor = () => {
            const style = getComputedStyle(document.documentElement);
            const rgb = style.getPropertyValue('--canvas-accent').trim() || '0, 240, 255';
            return rgb;
        };

        const draw = () => {
            if (!this.bgAnimationPaused) {
                const accent = getAccentColor();
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

            this.sseSource.onerror = () => {
                this.addTerminalLog('error', this.t('errorSSE'));
                setTimeout(connect, 5000);
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
        try {
            const [toolsRes, configRes, fsRes] = await Promise.all([
                fetch('/api/tools'),
                fetch('/api/config'),
                fetch('/api/python-fs-access')
            ]);
            if (toolsRes.ok) {
                const toolsData = await toolsRes.json();
                this.tools = Array.isArray(toolsData) ? toolsData : (toolsData.tools || []);
                this.tools.forEach(t => {
                    if (!this.callHistory[t.name]) this.callHistory[t.name] = [];
                });
            }
            if (configRes.ok) {
                this.config = await configRes.json();
                this.renderConfig();
            }
            if (fsRes.ok) {
                const fsData = await fsRes.json();
                this.pythonFsAccessEnabled = fsData.enabled || false;
            }
            this.render();
        } catch (err) {
            this.showError(this.t('errorLoading'));
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
    // 3D CARD TILT
    // ============================================================
    bindCardTilt(card) {
        card.addEventListener('mousemove', (e) => {
            const rect = card.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;
            const cx = rect.width / 2;
            const cy = rect.height / 2;
            const dx = (x - cx) / cx;
            const dy = (y - cy) / cy;
            card.style.transform = `perspective(800px) rotateY(${dx * 5}deg) rotateX(${-dy * 5}deg) translateZ(8px)`;
            card.style.transition = 'transform 0.1s ease-out';
        });
        card.addEventListener('mouseleave', () => {
            card.style.transform = 'perspective(800px) rotateY(0) rotateX(0) translateZ(0)';
            card.style.transition = 'transform 0.3s ease-out';
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
        searchInput?.addEventListener('input', (e) => {
            this.searchQuery = e.target.value.toLowerCase();
            this.render();
        });

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

        canvas.width = canvas.offsetWidth;
        canvas.height = canvas.offsetHeight;
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
                <div class="config-value">${val !== undefined ? val : 'N/A'}</div>
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
        document.getElementById('about-desc').textContent = this.t('description');
        document.getElementById('about-author-label').textContent = this.t('author');
        document.getElementById('about-license-label').textContent = this.t('license');
        document.getElementById('about-github').querySelector('span').textContent = this.t('github');

        // Try to fetch version from API
        try {
            const res = await fetch('/api/version');
            if (res.ok) {
                const data = await res.json();
                document.getElementById('about-version').textContent = 'v' + (data.version || '0.2.0');
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
            case 'time':
                sorted.sort((a, b) => (b.last_call_time || 0) - (a.last_call_time || 0));
                break;
        }
        return sorted;
    }

    render() {
        this.renderAlphabetNav();
        this.renderTools();
        this.updateLangUI();
        this.updateCallsHud();
    }

    updateCallsHud() {
        const callsEl = document.querySelector('.hud-number[data-metric="calls"]');
        if (callsEl) {
            const total = this.tools.reduce((sum, t) => sum + (t.call_count || 0), 0);
            callsEl.textContent = total;
        }
        const concEl = document.querySelector('.hud-number[data-metric="concurrency"]');
        if (concEl && this.config) {
            concEl.textContent = `0/${this.config.max_concurrency}`;
        }
    }

    renderAlphabetNav() {
        const nav = document.getElementById('alphabet-nav');
        if (!nav) return;
        const letters = ['all', ...Array.from('abcdefghijklmnopqrstuvwxyz')];
        nav.innerHTML = letters.map(l =>
            `<button class="${this.currentAlphabet === l ? 'active' : ''}" data-letter="${l}" onclick="window.cc.setAlphabetFilter('${l}')">${l === 'all' ? (this.lang === 'zh' ? '全部' : 'All') : l.toUpperCase()}</button>`
        ).join('');
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

        // Re-bind tilt effects
        document.querySelectorAll('.tool-card').forEach(card => this.bindCardTilt(card));

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
        const fsToggle = tool.name === 'execute_python' ? `
            <div class="tool-fs-toggle">
                <span class="fs-toggle-label">${this.lang === 'zh' ? '文件系统' : 'Filesystem'}</span>
                <label class="neon-switch small">
                    <input type="checkbox" data-tool="execute_python" data-toggle="fs-access" ${this.pythonFsAccessEnabled ? 'checked' : ''}>
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
                <option value="time">${this.t('sortByTime')}</option>
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
    }

    showError(msg) {
        const el = document.getElementById('error-message');
        if (el) { el.textContent = msg; el.style.display = 'block'; setTimeout(() => el.style.display = 'none', 5000); }
    }

    showSuccess(msg) {
        const el = document.getElementById('success-message');
        if (el) { el.textContent = msg; el.style.display = 'block'; setTimeout(() => el.style.display = 'none', 3000); }
    }
}

// ============================================================
// BOOT
// ============================================================
document.addEventListener('DOMContentLoaded', () => {
    window.cc = new CommandCenter();
});
