# 安全指南

## 概述

Rust MCP Server 实现了多层安全机制，在为 AI 助手提供强大功能的同时防止恶意使用。

## 安全架构

```
┌─────────────────────────────────────────────────────────────┐
│                      安全层架构                              │
├─────────────────────────────────────────────────────────────┤
│  第1层：工作目录限制                                          │
│  └── 所有文件操作限制在配置目录内                               │
├─────────────────────────────────────────────────────────────┤
│  第2层：危险命令黑名单（20种模式）                              │
│  └── 阻止 rm、format、dd、fork 炸弹等                         │
├─────────────────────────────────────────────────────────────┤
│  第3层：命令注入检测                                           │
│  └── 检测 ; | & ` $ ( ) < > \n \r 等特殊字符                  │
├─────────────────────────────────────────────────────────────┤
│  第4层：两步确认机制                                           │
│  └── 用户必须确认危险操作                                      │
├─────────────────────────────────────────────────────────────┤
│  第5层：审计日志                                              │
│  └── 记录所有命令执行的上下文                                   │
├─────────────────────────────────────────────────────────────┤
│  第6层：资源限制                                              │
│  └── 并发限制、超时、输出/文件大小限制                           │
├─────────────────────────────────────────────────────────────┤
│  第7层：Python 沙箱                                           │
│  └── 模块黑名单、阻塞 open()、路径限制                          │
├─────────────────────────────────────────────────────────────┤
│  第8层：HTTP SSRF 防护                                        │
│  └── 私有 IP 拦截、禁止重定向、连接限制                          │
└─────────────────────────────────────────────────────────────┘
```

## 安全特性

### 1. 工作目录限制

写操作工具（file_write、file_ops、file_edit、execute_command、execute_python）被限制在可配置的工作目录内，防止未授权修改敏感系统文件。只读工具（dir_list、file_read、file_search、file_stat、path_exists、json_query、git_ops、image_read、hash_compute）可以访问文件系统中的任意路径。

**工作原理：**
1. 写操作的所有路径被规范化为绝对路径
2. 解析符号链接
3. 检测并阻止路径穿越模式（`../`）
4. 工作目录外的路径对写操作被拒绝

**配置：**
```bash
# 限制到特定目录
./rust-mcp-server --working-dir /var/mcp-safe

# 使用环境变量
export MCP_WORKING_DIR=/var/mcp-safe
./rust-mcp-server
```

**安全检查（仅写操作）：**
```rust
// 伪代码
fn validate_path(path: &Path, working_dir: &Path) -> bool {
    let canonical = path.canonicalize()?;
    canonical.starts_with(working_dir)
}
```

### 2. 危险命令黑名单

`execute_command` 工具默认阻止 20 种危险命令模式。

**被阻止的命令：**

| ID | 命令 | 匹配模式 | 平台 | 风险 |
|----|------|----------|------|------|
| 1 | rm | `rm -rf /`, `rm -rf /*` | Linux | 数据销毁 |
| 2 | del | `del /`, `del C:\`, `del *.* /s/q` | Windows | 数据销毁 |
| 3 | format | `format /`, `format C:` | 两者 | 磁盘擦除 |
| 4 | mkfs | `mkfs.`, `mkfs /dev/` | Linux | 文件系统销毁 |
| 5 | dd | `dd if=/dev/zero of=/` | Linux | 磁盘覆盖 |
| 6 | fork 炸弹 | `:(){:|:&};:` | Linux | 拒绝服务攻击 |
| 7 | eval | `eval $(`, ``eval ``` | Linux | 代码注入 |
| 8 | exec | `exec `, `exec(` | Linux | 进程替换 |
| 9 | system | `system(`, `system (` | 两者 | 系统调用 |
| 10 | shred | `shred -`, `shred /` | Linux | 安全删除 |
| 11 | rd | `rd /s /q`, `rmdir /s /q` | Windows | 目录删除 |
| 13 | diskpart | `diskpart` | Windows | 磁盘操作 |
| 14 | reg | `reg delete`, `reg add` | Windows | 注册表更改 |
| 15 | net | `net user`, `net stop` | Windows | 网络/账户 |
| 16 | sc | `sc delete`, `sc config` | Windows | 服务控制 |
| 17 | schtasks | `schtasks /create` | Windows | 计划任务 |
| 18 | powercfg | `powercfg /` | Windows | 电源设置 |
| 19 | bcdedit | `bcdedit /` | Windows | 启动配置 |
| 20 | wevtutil | `wevtutil cl` | Windows | 事件日志清除 |

**允许特定命令：**
```bash
# 允许 rm (ID 1) 和 format (ID 3)
./rust-mcp-server --allow-dangerous-commands 1,3
```

### 3. 命令注入检测

检测可用于注入攻击的 shell 元字符。

**检测字符：**
```
;  |  &  `  $  (  )  <  >  \n  \r
```

**工作原理：**
- 执行前分析命令字符串
- 引号内的字符被排除（单引号和双引号）
- 正确处理引号内的反斜杠转义（如 `echo "hello\;world"`）
- 包含特殊字符的命令需要确认
- 5分钟内第二次相同执行则通过

**示例：**
```bash
# 这会触发确认：
ls -la; rm -rf /

# 这些不会触发（在引号内）：
echo "hello; world"
cat 'file with | in name'
```

### 4. 两步确认机制

危险命令和可疑模式需要通过 AI 助手进行明确的用户确认。

**流程：**
```
1. 首次调用
   └── 返回安全警告
   └── 命令存入待确认列表
   └── 5分钟超时开始
   
2. 用户确认
   └── 用户与 AI 助手查看警告
   └── 用户明确批准执行
   
3. 第二次调用（5分钟内）
   └── 命令匹配待确认条目
   └── 从待确认列表移除
   └── 命令执行
```

**确认响应：**
```
安全警告：检测到危险命令 'rm (delete files)'。

命令：rm -rf /home/user/temp

此命令可能对系统或数据造成损害。
请与用户确认是否执行此命令。

如果用户同意，请使用相同参数再次调用 
execute_command 工具以确认执行。
```

### 5. 审计日志

所有命令执行尝试都会被记录以供审查。

**日志格式：**
```
[AUDIT] Execute command attempt: cwd=/path, command=ls -la
[AUDIT] Dangerous command pending confirmation: id=1, command=rm -rf /
[AUDIT] Command with injection patterns pending confirmation: command=ls; cat /etc/passwd
[AUDIT] Command executed after confirmation: command=rm -rf /tmp/old
[AUDIT] Command executed: exit_code=0, cwd=/path, command=ls -la
[AUDIT] Command execution failed: error=..., cwd=/path, command=ls -la
[AUDIT] Command timed out: timeout=30, cwd=/path, command=sleep 100
```

**启用调试日志：**
```bash
RUST_LOG=debug ./rust-mcp-server
```

### 6. 并发限制

通过可配置的并发限制防止资源耗尽。

- **默认：** 10 个并发调用
- **最大：** 可通过 `--max-concurrency` 配置
- **每工具：** 所有工具共享同一个并发池

### 7. 超时保护

防止长时间运行的命令挂起。

- **默认：** 30 秒
- **最大：** 300 秒（5 分钟）
- **配置：** 每个命令的超时参数
- **行为：** 超时后终止子进程，而非仅断开等待

### 8. 输出与内容大小限制

防止大输出或文件导致内存耗尽。

- **命令输出：** 每次输出 100KB（stdout + stderr），UTF-8 安全截断
- **文件写入：** 最大内容大小 100MB
- **图片读取：** 最大文件大小 50MB（超大图片仅返回元数据）
- **Python 代码：** 最大 10,000 字符
- **Shell 命令：** 最大 10,000 字符

### 9. Python 沙箱

`execute_python` 工具在 RustPython 解释器中运行用户代码，并启用沙箱隔离。

**沙箱特性：**
- 禁用文件系统访问时，`builtins.open` 和 `_io.open` / `_io.FileIO` 被替换为阻塞存根
- 模块导入黑名单：`os`、`nt`、`posix`、`subprocess`、`socket`、`urllib`、`http.client`、`ctypes`、`platform`、`importlib`
- 通过 WebUI 启用文件系统访问后，`open()` 被包装为仅限工作目录内路径
- 执行超时通过 `sys.settrace` 在 VM 内部注入自终止检查

### 10. HTTP SSRF 防护

`http_request` 工具包含服务器端请求伪造防护。

**防护措施：**
- 拦截私有 IP 段：`127.0.0.0/8`、`10.0.0.0/8`、`172.16.0.0/12`、`192.168.0.0/16`、`169.254.0.0/16`、`::1`、`::`、`fc00::/7`、`fe80::/10`
- 拦截 IPv4 映射 IPv6 地址（`::ffff:127.0.0.1`）
- 拦截 `localhost` 和 `.localhost` 域名
- 禁用自动 HTTP 重定向
- 连接超时：10 秒；请求超时：60 秒
- 每主机最大 10 条空闲连接

## 工具分类

### 安全工具（17个）
这些工具默认启用是安全的。只读文件工具不受工作目录限制：
- `calculator` - 数学计算
- `dir_list` - 目录列表（不受工作目录限制）
- `file_read` - 文件读取（不受工作目录限制）
- `file_search` - 文件内容搜索（不受工作目录限制）
- `datetime` - 日期/时间
- `base64_codec` - Base64 编码/解码
- `hash_compute` - 哈希计算（不受工作目录限制）
- `http_request` - HTTP 请求（具备 SSRF 防护）
- `image_read` - 图像读取（不受工作目录限制）
- `system_info` - 系统信息
- `file_stat` - 文件/目录元数据（不受工作目录限制）
- `path_exists` - 路径存在性检查（不受工作目录限制）
- `json_query` - JSON 文件查询（不受工作目录限制）
- `env_get` - 环境变量读取（过滤敏感变量）
- `git_ops` - Git 仓库只读操作（不受工作目录限制）
- `process_list` - 系统进程列表
- `execute_python` - Python 代码执行（默认沙箱模式；文件系统访问可通过 WebUI 切换）

### 危险工具（4个）
这些工具默认禁用，需要谨慎使用：
- `file_write` - 文件写入（可能覆盖数据，限制在工作目录内，100MB 限制）
- `file_ops` - 复制、移动、删除或重命名文件（限制在工作目录内）
- `file_edit` - 多模式文件编辑（可能修改文件，限制在工作目录内）
- `execute_command` - Shell 命令执行（具备注入检测和两步确认）

## 最佳实践

### 管理员指南

1. **设置受限工作目录**
   ```bash
   ./rust-mcp-server --working-dir /var/mcp-safe
   ```
   - 为 MCP 操作创建专用目录
   - 设置适当的文件系统权限
   - 定期审计目录内容

2. **默认工具策略**
   ```bash
   # 从最小工具集开始
   ./rust-mcp-server --disable-tools file_write,file_ops,file_edit,execute_command,http_request
   ```
   - 按需启用工具
   - 仅在受信任环境中启用 `execute_command`

3. **定期审查审计日志**
   ```bash
   # 过滤审计日志
   grep "\[AUDIT\]" /var/log/mcp-server.log
   
   # 实时监控
   tail -f /var/log/mcp-server.log | grep "\[AUDIT\]"
   ```

4. **网络安全**
   - 仅绑定到本地主机（默认：127.0.0.1）
   - 如需外部暴露使用防火墙规则
   - 远程访问考虑使用 VPN

5. **定期更新**
   - 保持服务器更新到最新安全补丁
   - 查看更新日志了解安全更新

### 用户指南

1. **确认前验证**
   - 确认前始终查看完整命令
   - 理解每个参数和标志的作用
   - 不确定时不要确认

2. **使用具体路径**
   - 尽可能避免使用通配符
   - 使用工作目录内的绝对路径
   - 破坏性操作前仔细检查目标路径

3. **检查工作目录**
   - 确认您在正确的目录中操作
   - 使用 `pwd` 或 `dir_list` 确认位置

4. **报告可疑活动**
   - 监控意外的工具调用
   - 报告任何可疑的 AI 行为

## 安全检查清单

生产环境部署前：

- [ ] 工作目录已正确配置和限制
- [ ] 仅启用了必要的工具
- [ ] 危险命令已正确配置
- [ ] 审计日志已启用并正在监控
- [ ] 服务器绑定到本地主机或安全网络
- [ ] 并发限制适合您的硬件
- [ ] 建立了定期日志审查流程
- [ ] 备份策略已就绪
- [ ] 服务器运行最新版本

## 事件响应

### 如果检测到可疑活动

1. **立即行动**
   ```bash
   # 通过 WebUI 或 API 停止 MCP 服务
   curl -X POST http://127.0.0.1:2233/api/mcp/stop
   ```

2. **审查日志**
   ```bash
   # 检查最近的审计日志
   grep "\[AUDIT\]" /var/log/mcp-server.log | tail -100
   ```

3. **检查文件系统**
   - 审查工作目录中的文件
   - 检查未授权的更改
   - 验证系统完整性

## 报告安全问题

请在公开披露前私下报告安全漏洞：

1. 在 GitHub 上创建私人安全公告
2. 包含详细的复现步骤
3. 提供影响评估
4. 给予合理的修复时间

## 参考

- [OWASP 命令注入](https://owasp.org/www-community/attacks/Command_Injection)
- [OWASP 路径遍历](https://owasp.org/www-community/attacks/Path_Traversal)
- [MCP 安全模型](https://modelcontextprotocol.io/)

---

For English version, see [security.md](security.md)
