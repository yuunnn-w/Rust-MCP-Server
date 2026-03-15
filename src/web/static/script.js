// Global state
const appState = {
    tools: [],
    config: {},
    mcpRunning: true,
    currentCalls: 0,
    maxConcurrency: 10,
    sortBy: 'name-asc',
    filterLetter: 'all',
    searchQuery: '',
    language: localStorage.getItem('language') || 'en'
};

// Tool descriptions in multiple languages
const toolDescriptions = {
    en: {
        'calculator': 'Calculate mathematical expressions (supports +, -, *, /, ^, sqrt, sin, cos, tan, log, ln, abs, pi, e)',
        'dir_list': 'List directory contents with tree structure (max depth 1)',
        'file_read': 'Read text file content with line range support (10KB limit)',
        'file_search': 'Search for keyword in file or directory (max depth 3)',
        'file_write': 'Write content to file (create/append/overwrite)',
        'file_copy': 'Copy a file to a new location',
        'file_move': 'Move a file to a new location',
        'file_delete': 'Delete a file',
        'file_rename': 'Rename a file',
        'http_request': 'Make HTTP GET or POST requests',
        'datetime': 'Get current date and time in China format',
        'image_read': 'Read image file and return base64 encoded data with MIME type',
        'execute_command': 'Execute shell command in specified directory (use with caution)',
        'process_list': 'List system processes with CPU and memory usage',
        'base64_encode': 'Encode string to base64',
        'base64_decode': 'Decode base64 to string',
        'hash_compute': 'Compute hash of string or file (MD5, SHA1, SHA256). Prefix file path with "file:" for files',
        'system_info': 'Get system information including OS, CPU, memory'
    },
    zh: {
        'calculator': '计算数学表达式（支持 +, -, *, /, ^, sqrt, sin, cos, tan, log, ln, abs, pi, e）',
        'dir_list': '列出目录内容，树形结构显示（最大深度1层）',
        'file_read': '读取文本文件内容（支持行数范围，10KB限制）',
        'file_search': '在文件或目录中搜索关键词（最大深度3层）',
        'file_write': '写入文件内容（新建/追加/覆盖）',
        'file_copy': '复制文件到新位置',
        'file_move': '移动文件到新位置',
        'file_delete': '删除文件',
        'file_rename': '重命名文件',
        'http_request': '发起 HTTP GET 或 POST 请求',
        'datetime': '获取中国格式的当前日期和时间',
        'image_read': '读取图片文件并返回 base64 编码数据及 MIME 类型',
        'execute_command': '在指定目录执行 shell 命令（谨慎使用）',
        'process_list': '列出系统进程及 CPU、内存使用情况',
        'base64_encode': '将字符串编码为 base64',
        'base64_decode': '将 base64 解码为字符串',
        'hash_compute': '计算字符串或文件的哈希值（MD5, SHA1, SHA256）。文件路径前加 "file:"',
        'system_info': '获取系统信息，包括操作系统、CPU、内存等'
    }
};

// Internationalization
const i18n = {
    en: {
        'app.title': 'Rust MCP Server',
        'app.concurrency': 'Concurrency',
        'app.running': 'Running',
        'app.stopped': 'Stopped',
        'sidebar.title': 'Settings',
        'sidebar.currentConfig': 'Current Configuration',
        'sidebar.modifyConfig': 'Modify Config',
        'sidebar.restartMcp': 'Restart MCP Service',
        'config.webuiAddr': 'WebUI Address:',
        'config.mcpTransport': 'MCP Transport:',
        'config.mcpAddr': 'MCP Address:',
        'config.workingDir': 'Working Directory:',
        'config.maxConcurrency': 'Max Concurrency:',
        'config.logLevel': 'Log Level:',
        'config.modalTitle': 'Modify Configuration',
        'config.mcpTransportLabel': 'MCP Transport:',
        'config.mcpHost': 'MCP Host:',
        'config.mcpPort': 'MCP Port:',
        'config.webuiHost': 'WebUI Host:',
        'config.webuiPort': 'WebUI Port:',
        'config.restartNote': 'Note: Some configuration changes require server restart to take effect.',
        'tools.detailsTitle': 'Tool Details',
        'tools.description': 'Description',
        'tools.usage': 'Usage',
        'tools.totalCalls': 'Total Calls',
        'tools.last15Min': 'Last 15 Min',
        'tools.recentCalls': 'Recent Calls',
        'tools.all': 'All',
        'tools.safeTools': 'Safe Tools',
        'tools.dangerousTools': 'Dangerous Tools',
        'tools.searchPlaceholder': 'Search tools...',
        'sort.nameAsc': 'Name A-Z',
        'sort.nameDesc': 'Name Z-A',
        'sort.mostUsed': 'Most Used',
        'sort.leastUsed': 'Least Used',
        'common.cancel': 'Cancel',
        'common.save': 'Save',
        'footer.documentation': 'Documentation',
        'tool.status.idle': 'Idle',
        'tool.status.calling': 'Calling...',
        'tool.status.enabled': 'Enabled',
        'tool.status.disabled': 'Disabled',
        'tools.callFrequencyChart': 'Call Frequency (Last 2 Hours)',
        'message.configSaved': 'Configuration saved successfully',
        'message.restartConfirm': 'Configuration saved. Restart server to apply all changes?',
        'message.error': 'Error',
        'message.success': 'Success'
    },
    zh: {
        'app.title': 'Rust MCP 服务器',
        'app.concurrency': '并发数',
        'app.running': '运行中',
        'app.stopped': '已停止',
        'sidebar.title': '设置',
        'sidebar.currentConfig': '当前配置',
        'sidebar.modifyConfig': '修改配置',
        'sidebar.restartMcp': '重启 MCP 服务',
        'config.webuiAddr': 'WebUI 地址:',
        'config.mcpTransport': 'MCP 传输:',
        'config.mcpAddr': 'MCP 地址:',
        'config.workingDir': '工作目录:',
        'config.maxConcurrency': '最大并发:',
        'config.logLevel': '日志级别:',
        'config.modalTitle': '修改配置',
        'config.mcpTransportLabel': 'MCP 传输模式:',
        'config.mcpHost': 'MCP 主机:',
        'config.mcpPort': 'MCP 端口:',
        'config.webuiHost': 'WebUI 主机:',
        'config.webuiPort': 'WebUI 端口:',
        'config.restartNote': '注意：某些配置更改需要重启服务器才能生效。',
        'tools.detailsTitle': '工具详情',
        'tools.description': '描述',
        'tools.usage': '用法',
        'tools.totalCalls': '总调用次数',
        'tools.last15Min': '最近15分钟',
        'tools.recentCalls': '最近调用',
        'tools.all': '全部',
        'tools.safeTools': '安全工具',
        'tools.dangerousTools': '危险工具',
        'tools.searchPlaceholder': '搜索工具...',
        'sort.nameAsc': '名称 A-Z',
        'sort.nameDesc': '名称 Z-A',
        'sort.mostUsed': '使用最多',
        'sort.leastUsed': '使用最少',
        'common.cancel': '取消',
        'common.save': '保存',
        'footer.documentation': '文档',
        'tool.status.idle': '空闲',
        'tool.status.calling': '调用中...',
        'tool.status.enabled': '已启用',
        'tool.status.disabled': '已禁用',
        'tools.callFrequencyChart': '调用频数（最近2小时）',
        'message.configSaved': '配置保存成功',
        'message.restartConfirm': '配置已保存。是否重启服务器以应用所有更改？',
        'message.error': '错误',
        'message.success': '成功'
    }
};

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    initEventListeners();
    applyLanguage();
    loadInitialData();
    connectSSE();
});

// Event Listeners
function initEventListeners() {
    // Sidebar toggle
    document.getElementById('menu-btn').addEventListener('click', toggleSidebar);
    document.getElementById('close-sidebar').addEventListener('click', toggleSidebar);
    
    // Language switch
    document.getElementById('lang-switch').addEventListener('click', switchLanguage);
    
    // Config modal
    document.getElementById('edit-config-btn').addEventListener('click', () => {
        openConfigModal();
    });
    document.getElementById('save-config-btn').addEventListener('click', saveConfig);
    
    // MCP toggle
    document.getElementById('mcp-toggle').addEventListener('change', toggleMcpService);
    
    // Restart MCP
    document.getElementById('restart-mcp-btn').addEventListener('click', restartMcpService);
    
    // Alphabet navigation
    document.querySelectorAll('.alphabet-nav button').forEach(btn => {
        btn.addEventListener('click', (e) => {
            document.querySelectorAll('.alphabet-nav button').forEach(b => b.classList.remove('active'));
            e.target.classList.add('active');
            appState.filterLetter = e.target.dataset.letter;
            renderTools();
        });
    });
    
    // Search
    document.getElementById('search-input').addEventListener('input', (e) => {
        appState.searchQuery = e.target.value.toLowerCase();
        renderTools();
    });
    
    // Sort
    document.getElementById('sort-select').addEventListener('change', (e) => {
        appState.sortBy = e.target.value;
        renderTools();
    });
    
    // Close modals on outside click
    document.querySelectorAll('.modal').forEach(modal => {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                closeModal(modal.id);
            }
        });
    });
}

// Language functions
function switchLanguage() {
    appState.language = appState.language === 'en' ? 'zh' : 'en';
    localStorage.setItem('language', appState.language);
    applyLanguage();
}

function applyLanguage() {
    const lang = i18n[appState.language];
    
    // Update all elements with data-i18n attribute
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        if (lang[key]) {
            el.textContent = lang[key];
        }
    });
    
    // Update elements with data-i18n-attr attribute
    document.querySelectorAll('[data-i18n-attr]').forEach(el => {
        const attrMap = el.getAttribute('data-i18n-attr');
        // Format: "attr1:key1,attr2:key2"
        attrMap.split(',').forEach(pair => {
            const [attr, key] = pair.split(':');
            if (lang[key]) {
                el.setAttribute(attr, lang[key]);
            }
        });
    });
    
    // Update document language
    document.documentElement.lang = appState.language === 'en' ? 'en' : 'zh-CN';
}

function t(key) {
    return i18n[appState.language][key] || key;
}

// Load initial data
async function loadInitialData() {
    try {
        // Load tools
        const toolsResponse = await fetch('/api/tools');
        const toolsData = await toolsResponse.json();
        appState.tools = toolsData.tools || [];
        
        // Load config
        const configResponse = await fetch('/api/config');
        appState.config = await configResponse.json();
        
        // Load server status (concurrency, etc.)
        const serverStatusResponse = await fetch('/api/server-status');
        const serverStatus = await serverStatusResponse.json();
        appState.currentCalls = serverStatus.current_calls || 0;
        appState.maxConcurrency = serverStatus.max_concurrency || 10;
        
        // Update UI
        updateConfigUI();
        renderTools();
        updateConcurrencyDisplay();
    } catch (error) {
        console.error('Failed to load initial data:', error);
        showError(t('message.error') + ': ' + 'Failed to load data from server');
    }
}

// Connect to SSE for real-time updates
function connectSSE() {
    console.log('Connecting to SSE...');
    const eventSource = new EventSource('/api/events');
    
    eventSource.onmessage = (event) => {
        console.log('SSE raw message:', event.data);
        try {
            const update = JSON.parse(event.data);
            console.log('SSE parsed update:', update);
            handleSSEUpdate(update);
        } catch (error) {
            console.error('Failed to parse SSE message:', error, event.data);
        }
    };
    
    eventSource.onerror = (error) => {
        console.error('SSE connection error:', error);
        // Attempt to reconnect after 5 seconds
        setTimeout(connectSSE, 5000);
    };
    
    eventSource.onopen = () => {
        console.log('SSE connection established');
    };
}

// Handle SSE updates
function handleSSEUpdate(update) {
    console.log('SSE update:', update);
    switch (update.type) {
        case 'ToolCallCount':
            updateToolCallCount(update.tool, update.count, update.isCalling, update.isBusy);
            break;
        case 'ToolEnabled':
            updateToolEnabled(update.tool, update.enabled);
            break;
        case 'McpServiceStatus':
            updateMcpStatus(update.running);
            break;
        case 'ConcurrentCalls':
            appState.currentCalls = update.current;
            appState.maxConcurrency = update.max;
            updateConcurrencyDisplay();
            break;
    }
}

// Update tool call count in UI
function updateToolCallCount(toolName, count, isCalling, isBusy) {
    const tool = appState.tools.find(t => t.name === toolName);
    if (!tool) return;
    
    // Update tool state
    tool.call_count = count;
    tool.is_calling = isCalling;
    tool.is_busy = isBusy;
    
    // Update card if visible
    const card = document.querySelector(`[data-tool="${toolName}"]`);
    if (!card) return;
    
    // Update call count
    const countEl = card.querySelector('.call-count');
    if (countEl) countEl.textContent = count;
    
    // Update status text and styling
    const statusEl = card.querySelector('.tool-status');
    if (statusEl) {
        statusEl.textContent = isBusy ? t('tool.status.calling') : t('tool.status.idle');
        statusEl.className = `tool-status ${isBusy ? 'calling' : 'idle'}`;
    }
    
    // Update status dot in header
    const statusDot = card.querySelector('.tool-status-dot');
    if (statusDot) {
        if (isBusy) {
            statusDot.className = 'tool-status-dot calling';
        } else if (!tool.enabled) {
            statusDot.className = 'tool-status-dot disabled';
        } else {
            statusDot.className = 'tool-status-dot';
        }
    }
}

// Update tool enabled state in UI
function updateToolEnabled(toolName, enabled) {
    const tool = appState.tools.find(t => t.name === toolName);
    if (tool) {
        tool.enabled = enabled;
        
        // Update card
        const card = document.querySelector(`[data-tool="${toolName}"]`);
        if (card) {
            card.classList.toggle('disabled', !enabled);
            
            const toggle = card.querySelector('.tool-toggle');
            if (toggle) toggle.checked = enabled;
        }
    }
}

// Update MCP status in UI
function updateMcpStatus(running) {
    appState.mcpRunning = running;
    const toggle = document.getElementById('mcp-toggle');
    const status = document.getElementById('mcp-status');
    
    toggle.checked = running;
    status.textContent = running ? t('app.running') : t('app.stopped');
    status.style.color = running ? 'var(--success-color)' : 'var(--danger-color)';
}

// Update concurrency display
function updateConcurrencyDisplay() {
    document.getElementById('current-calls').textContent = appState.currentCalls;
    document.getElementById('max-concurrency').textContent = appState.maxConcurrency;
}

// Update config UI
function updateConfigUI() {
    if (appState.config) {
        document.getElementById('config-webui-addr').textContent = 
            `${appState.config.webui_host}:${appState.config.webui_port}`;
        document.getElementById('config-transport').textContent = 
            appState.config.mcp_transport === 'http' ? 'HTTP (JSON)' : 'SSE (Stream)';
        document.getElementById('config-mcp-addr').textContent = 
            `${appState.config.mcp_host}:${appState.config.mcp_port}`;
        document.getElementById('config-working-dir').textContent = appState.config.working_dir;
        document.getElementById('config-max-concurrency').textContent = appState.config.max_concurrency;
        document.getElementById('config-log-level').textContent = appState.config.log_level;
    }
}

// Open config modal and populate values
function openConfigModal() {
    if (appState.config) {
        document.getElementById('new-mcp-transport').value = appState.config.mcp_transport || 'http';
        document.getElementById('new-mcp-host').value = appState.config.mcp_host || '';
        document.getElementById('new-mcp-port').value = appState.config.mcp_port || '';
        document.getElementById('new-webui-host').value = appState.config.webui_host || '';
        document.getElementById('new-webui-port').value = appState.config.webui_port || '';
        document.getElementById('new-max-concurrency').value = appState.config.max_concurrency || 10;
        document.getElementById('new-log-level').value = appState.config.log_level || 'info';
        document.getElementById('new-working-dir').value = appState.config.working_dir || '';
    }
    openModal('config-modal');
}

// Render tools grid - split into safe and dangerous sections
function renderTools() {
    const safeGrid = document.getElementById('tools-grid-safe');
    const dangerousGrid = document.getElementById('tools-grid-dangerous');
    
    // Filter tools
    let filtered = appState.tools.filter(tool => {
        // Filter by letter
        if (appState.filterLetter !== 'all') {
            if (!tool.name.toLowerCase().startsWith(appState.filterLetter)) {
                return false;
            }
        }
        
        // Filter by search
        if (appState.searchQuery) {
            const searchLower = appState.searchQuery;
            if (!tool.name.toLowerCase().includes(searchLower) &&
                !tool.description.toLowerCase().includes(searchLower)) {
                return false;
            }
        }
        
        return true;
    });
    
    // Split into safe and dangerous
    let safeTools = filtered.filter(t => !t.is_dangerous);
    let dangerousTools = filtered.filter(t => t.is_dangerous);
    
    // Sort each group
    const sortFn = (a, b) => {
        switch (appState.sortBy) {
            case 'name-asc':
                return a.name.localeCompare(b.name);
            case 'name-desc':
                return b.name.localeCompare(a.name);
            case 'calls-desc':
                return (b.call_count || 0) - (a.call_count || 0);
            case 'calls-asc':
                return (a.call_count || 0) - (b.call_count || 0);
            default:
                return a.name.localeCompare(b.name);
        }
    };
    
    safeTools.sort(sortFn);
    dangerousTools.sort(sortFn);
    
    // Render safe tools
    if (safeTools.length > 0) {
        safeGrid.innerHTML = safeTools.map(tool => createToolCard(tool)).join('');
        document.getElementById('safe-tools-section').style.display = 'block';
    } else {
        safeGrid.innerHTML = '<div class="no-tools">No safe tools match the filter</div>';
    }
    
    // Render dangerous tools
    if (dangerousTools.length > 0) {
        dangerousGrid.innerHTML = dangerousTools.map(tool => createToolCard(tool)).join('');
        document.getElementById('dangerous-tools-section').style.display = 'block';
    } else {
        dangerousGrid.innerHTML = '<div class="no-tools">No dangerous tools match the filter</div>';
    }
    
    // Add toggle listeners
    document.querySelectorAll('.tool-toggle').forEach(toggle => {
        toggle.addEventListener('change', (e) => {
            const toolName = e.target.dataset.tool;
            const enabled = e.target.checked;
            toggleTool(toolName, enabled);
        });
    });
    
    // Add detail button listeners
    document.querySelectorAll('.tool-detail-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const toolName = e.target.dataset.tool;
            showToolDetails(toolName);
        });
    });
}

// Get localized tool description
function getToolDescription(toolName) {
    const lang = appState.language;
    return toolDescriptions[lang][toolName] || toolDescriptions['en'][toolName] || toolName;
}

// Create tool card HTML
function createToolCard(tool) {
    const isDisabled = !tool.enabled;
    const isDangerous = tool.is_dangerous;
    const isBusy = tool.is_busy || tool.is_calling || false;  // Use is_busy field
    const callCount = tool.call_count || 0;
    const description = getToolDescription(tool.name);
    
    return `
        <div class="tool-card ${isDisabled ? 'disabled' : ''} ${isDangerous ? 'dangerous' : ''}" 
             data-tool="${tool.name}">
            <div class="tool-header">
                <div class="tool-name">
                    <span class="tool-status-dot ${isDisabled ? 'disabled' : ''}"></span>
                    ${tool.name}
                </div>
                <label class="switch">
                    <input type="checkbox" class="tool-toggle" data-tool="${tool.name}" 
                           ${tool.enabled ? 'checked' : ''}>
                    <span class="slider round"></span>
                </label>
            </div>
            <div class="tool-description">${description}</div>
            <div class="tool-stats-summary">
                <div class="tool-stat">
                    <span class="tool-stat-label">${t('tools.totalCalls')}:</span>
                    <span class="tool-stat-value call-count">${callCount}</span>
                </div>
            </div>
            <div class="tool-footer">
                <span class="tool-status ${isBusy ? 'calling' : 'idle'}">
                    ${isBusy ? t('tool.status.calling') : t('tool.status.idle')}
                </span>
                <button class="btn btn-info tool-detail-btn" data-tool="${tool.name}">
                    ${t('tools.detailsTitle')}
                </button>
            </div>
        </div>
    `;
}

// Toggle tool enabled state
async function toggleTool(toolName, enabled) {
    try {
        const response = await fetch(`/api/tool/${toolName}/enable`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ enabled })
        });
        
        if (!response.ok) {
            throw new Error('Failed to toggle tool');
        }
        
        // Optimistic update
        updateToolEnabled(toolName, enabled);
    } catch (error) {
        console.error('Failed to toggle tool:', error);
        showError(`${t('message.error')}: ${enabled ? 'enable' : 'disable'} tool ${toolName}`);
        
        // Revert toggle
        const tool = appState.tools.find(t => t.name === toolName);
        if (tool) {
            const card = document.querySelector(`[data-tool="${toolName}"]`);
            const toggle = card?.querySelector('.tool-toggle');
            if (toggle) toggle.checked = !enabled;
        }
    }
}

// Show tool details modal
async function showToolDetails(toolName) {
    try {
        // Fetch tool stats
        const statsResponse = await fetch(`/api/tool/${toolName}/stats`);
        const stats = await statsResponse.json();
        
        // Fetch tool detail
        const detailResponse = await fetch(`/api/tool/${toolName}/detail`);
        const detail = await detailResponse.json();
        
        document.getElementById('tool-modal-title').textContent = `${toolName} ${t('tools.detailsTitle')}`;
        document.getElementById('tool-description').textContent = getToolDescription(toolName) || detail.description || '';
        document.getElementById('tool-usage').textContent = detail.usage || '';
        document.getElementById('tool-total-calls').textContent = stats.total_calls || 0;
        document.getElementById('tool-recent-calls').textContent = stats.recent_calls_15min || 0;
        
        // Render recent calls
        const recentList = document.getElementById('recent-calls-list');
        if (stats.recent_call_times && stats.recent_call_times.length > 0) {
            recentList.innerHTML = stats.recent_call_times
                .map(time => `<li>${time}</li>`)
                .join('');
        } else {
            recentList.innerHTML = '<li>No recent calls</li>';
        }
        
        // Render chart
        renderToolChart(stats.stats_history || []);
        
        openModal('tool-modal');
    } catch (error) {
        console.error('Failed to load tool details:', error);
        showError('Failed to load tool statistics');
    }
}

// Render tool stats chart
function renderToolChart(history) {
    const canvas = document.getElementById('tool-chart');
    const ctx = canvas.getContext('2d');
    
    // Set fixed canvas size for high DPI displays
    const dpr = window.devicePixelRatio || 1;
    const displayWidth = canvas.clientWidth || 600;
    const displayHeight = canvas.clientHeight || 200;
    
    canvas.width = displayWidth * dpr;
    canvas.height = displayHeight * dpr;
    
    // Scale context for high DPI
    ctx.scale(dpr, dpr);
    
    // Store display dimensions for drawing
    canvas.displayWidth = displayWidth;
    canvas.displayHeight = displayHeight;
    
    const width = canvas.displayWidth;
    const height = canvas.displayHeight;
    
    if (!history.length) {
        ctx.fillStyle = '#999';
        ctx.font = '14px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText('No data available', width / 2, height / 2);
        return;
    }
    
    // Simple bar chart
    const padding = 30;
    const barWidth = (width - padding * 2) / history.length;
    const maxValue = Math.max(...history, 1);
    const chartHeight = height - padding * 2;
    
    // Draw bars
    ctx.fillStyle = '#3498db';
    history.forEach((value, index) => {
        const barHeight = (value / maxValue) * chartHeight;
        const x = padding + index * barWidth + barWidth * 0.1;
        const y = height - padding - barHeight;
        const w = barWidth * 0.8;
        
        ctx.fillRect(x, y, w, barHeight);
        
        // Draw value label if bar is tall enough
        if (barHeight > 15) {
            ctx.fillStyle = '#fff';
            ctx.font = '10px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText(value.toString(), x + w / 2, y + 12);
            ctx.fillStyle = '#3498db';
        }
    });
    
    // Draw axes
    ctx.strokeStyle = '#ccc';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(padding, padding);
    ctx.lineTo(padding, height - padding);
    ctx.lineTo(width - padding, height - padding);
    ctx.stroke();
    
    // Draw X-axis tick marks and labels (every 6 bars = 30 minutes)
    ctx.strokeStyle = '#ccc';
    ctx.lineWidth = 1;
    ctx.fillStyle = '#666';
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'center';
    
    const tickInterval = 6; // 6 bars = 30 minutes (5 min per bar)
    const tickLength = 5;
    const minutesPerBar = 5; // Each bar represents 5 minutes
    
    for (let i = 0; i <= history.length; i += tickInterval) {
        const x = padding + i * barWidth;
        
        // Draw tick mark
        ctx.beginPath();
        ctx.moveTo(x, height - padding);
        ctx.lineTo(x, height - padding + tickLength);
        ctx.stroke();
        
        // Draw minute label (minutes ago)
        const minutesAgo = i * minutesPerBar;
        ctx.fillText(minutesAgo.toString(), x, height - padding + 18);
    }
    
    // Draw unit label
    ctx.font = '11px sans-serif';
    ctx.fillStyle = '#888';
    ctx.fillText('(minutes ago)', width / 2, height - 2);
}

// Toggle MCP service
async function toggleMcpService() {
    const enabled = document.getElementById('mcp-toggle').checked;
    
    try {
        const endpoint = enabled ? '/api/mcp/start' : '/api/mcp/stop';
        const response = await fetch(endpoint, { method: 'POST' });
        
        if (!response.ok) {
            throw new Error('Failed to toggle MCP service');
        }
        
        updateMcpStatus(enabled);
    } catch (error) {
        console.error('Failed to toggle MCP service:', error);
        showError(`${t('message.error')}: ${enabled ? 'start' : 'stop'} MCP service`);
        document.getElementById('mcp-toggle').checked = !enabled;
    }
}

// Restart MCP service
async function restartMcpService() {
    try {
        const response = await fetch('/api/mcp/restart', { method: 'POST' });
        
        if (!response.ok) {
            throw new Error('Failed to restart MCP service');
        }
        
        showSuccess('MCP service restarted successfully');
        updateMcpStatus(true);
    } catch (error) {
        console.error('Failed to restart MCP service:', error);
        showError('Failed to restart MCP service');
    }
}

// Save configuration
async function saveConfig() {
    const config = {
        mcp_transport: document.getElementById('new-mcp-transport').value || undefined,
        mcp_host: document.getElementById('new-mcp-host').value || undefined,
        mcp_port: parseInt(document.getElementById('new-mcp-port').value) || undefined,
        webui_host: document.getElementById('new-webui-host').value || undefined,
        webui_port: parseInt(document.getElementById('new-webui-port').value) || undefined,
        max_concurrency: parseInt(document.getElementById('new-max-concurrency').value) || undefined,
        log_level: document.getElementById('new-log-level').value || undefined,
        working_dir: document.getElementById('new-working-dir').value || undefined,
    };
    
    // Remove undefined values
    Object.keys(config).forEach(key => {
        if (config[key] === undefined) delete config[key];
    });
    
    try {
        const response = await fetch('/api/config', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(config)
        });
        
        if (!response.ok) {
            const error = await response.text();
            throw new Error(error);
        }
        
        const result = await response.json();
        
        // Update local config
        Object.assign(appState.config, config);
        updateConfigUI();
        closeModal('config-modal');
        
        showSuccess(t('message.configSaved'));
        
        // Show restart prompt if needed
        if (result.restart_required) {
            if (confirm(t('message.restartConfirm'))) {
                // Note: Full server restart would require external mechanism
                // For now, we just restart MCP service
                restartMcpService();
            }
        }
    } catch (error) {
        console.error('Failed to save configuration:', error);
        showError('Failed to save configuration: ' + error.message);
    }
}

// Toggle sidebar
function toggleSidebar() {
    const sidebar = document.getElementById('sidebar');
    sidebar.classList.toggle('open');
    
    // Add/remove overlay
    let overlay = document.querySelector('.overlay');
    if (sidebar.classList.contains('open')) {
        if (!overlay) {
            overlay = document.createElement('div');
            overlay.className = 'overlay';
            overlay.addEventListener('click', toggleSidebar);
            document.body.appendChild(overlay);
        }
        overlay.classList.add('show');
    } else if (overlay) {
        overlay.classList.remove('show');
    }
}

// Open modal
function openModal(modalId) {
    document.getElementById(modalId).classList.add('show');
}

// Close modal
function closeModal(modalId) {
    document.getElementById(modalId).classList.remove('show');
}

// Show error message
function showError(message) {
    alert(`${t('message.error')}: ${message}`);
}

// Show success message
function showSuccess(message) {
    alert(`${t('message.success')}: ${message}`);
}
