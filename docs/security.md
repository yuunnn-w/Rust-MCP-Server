# Security Guide

## Overview

Rust MCP Server implements multiple layers of security to protect against malicious use while providing powerful capabilities for AI assistants.

## Security Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Security Layers                          │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Working Directory Restriction                     │
│  └── All file ops restricted to configured directory        │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: Dangerous Command Blacklist (20 patterns)         │
│  └── Blocks rm, format, dd, fork bomb, etc.                 │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: Command Injection Detection                       │
│  └── Detects ; | & ` $ ( ) < > \n \r characters             │
├─────────────────────────────────────────────────────────────┤
│  Layer 4: Two-Step Confirmation                             │
│  └── User must confirm dangerous operations                 │
├─────────────────────────────────────────────────────────────┤
│  Layer 5: Audit Logging                                     │
│  └── All commands logged with context                       │
├─────────────────────────────────────────────────────────────┤
│  Layer 6: Resource Limits                                   │
│  └── Concurrency limits, timeouts, output/file size limits  │
├─────────────────────────────────────────────────────────────┤
│  Layer 7: Python Sandbox                                    │
│  └── Filesystem function interception, blocked open(), path restriction │
├─────────────────────────────────────────────────────────────┤
│  Layer 8: HTTP SSRF Protection                              │
│  └── Private IP blocking, no redirects, connection limits   │
└─────────────────────────────────────────────────────────────┘
```

## Security Features

### 1. Working Directory Restriction

Write operations (Write, Edit, FileOps, Bash, ExecutePython, Archive, Diff) are restricted to a configurable working directory to prevent unauthorized modification of sensitive system files. Read-only tools (Glob, Read, Grep, FileStat, Git, Clipboard, NoteStorage) can access any path on the filesystem.

**Note on `Archive`**: The archive tool validates all source paths, destination paths, and archive paths against the working directory. Extracted files cannot escape the working directory.

**Note on `NoteStorage`**: Notes are stored purely in memory and automatically cleared after 30 minutes of inactivity. They are not persisted to disk and cannot survive server restarts. This is designed as a short-term scratchpad for AI reasoning, not long-term storage.

**How it works:**
1. All paths for write operations are canonicalized to absolute form
2. Symbolic links are resolved
3. Path traversal patterns (`../`) are detected and blocked
4. Paths outside working directory are rejected for write operations

**Configuration:**
```bash
# Restrict to specific directory
./rust-mcp-server --working-dir /var/mcp-safe

# Using environment variable
export MCP_WORKING_DIR=/var/mcp-safe
./rust-mcp-server
```

**Security Check (write operations only):**
```rust
// Pseudo-code
fn validate_path(path: &Path, working_dir: &Path) -> bool {
    let canonical = path.canonicalize()?;
    canonical.starts_with(working_dir)
}
```

### 2. Dangerous Command Blacklist

The `Bash` tool blocks 20 dangerous command patterns by default.

**Blocked Commands:**

| ID | Command | Pattern | Platform | Risk |
|----|---------|---------|----------|------|
| 1 | rm | `rm -rf /`, `rm -rf /*` | Linux | Data destruction |
| 2 | del | `del /`, `del C:\`, `del *.* /s/q` | Windows | Data destruction |
| 3 | format | `format /`, `format C:` | Both | Disk erasure |
| 4 | mkfs | `mkfs.`, `mkfs /dev/` | Linux | Filesystem destruction |
| 5 | dd | `dd if=/dev/zero of=/` | Linux | Disk overwrite |
| 6 | fork bomb | `:(){:|:&};:` | Linux | DoS attack |
| 7 | eval | `eval $(`, ``eval ``` | Linux | Code injection |
| 8 | exec | `exec `, `exec(` | Linux | Process replacement |
| 9 | system | `system(`, `system (` | Both | System calls |
| 10 | shred | `shred -`, `shred /` | Linux | Secure delete |
| 11 | rd | `rd /s /q`, `rmdir /s /q` | Windows | Directory deletion |
| 12 | format | `format` | Windows | Disk erasure |
| 13 | diskpart | `diskpart` | Windows | Disk manipulation |
| 14 | reg | `reg delete`, `reg add` | Windows | Registry changes |
| 15 | net | `net user`, `net stop` | Windows | Network/accounts |
| 16 | sc | `sc delete`, `sc config` | Windows | Service control |
| 17 | schtasks | `schtasks /create` | Windows | Scheduled tasks |
| 18 | powercfg | `powercfg /` | Windows | Power settings |
| 19 | bcdedit | `bcdedit /` | Windows | Boot config |
| 20 | wevtutil | `wevtutil cl` | Windows | Event log clearing |

**Allow Specific Commands:**
```bash
# Allow rm (ID 1) and format (ID 3)
./rust-mcp-server --allow-dangerous-commands 1,3
```

### 3. Command Injection Detection

Detects shell metacharacters that could be used for injection attacks.

**Detected Characters:**
```
;  |  &  `  $  (  )  <  >  \n  \r
```

**How it works:**
- Analyzes command string before execution
- Excludes characters inside quoted strings (both single and double quotes)
- Handles backslash escapes inside quotes (e.g., `echo "hello\;world"`)
- Requires confirmation for commands containing special characters
- Second identical execution within 5 minutes proceeds

**Example:**
```bash
# This would trigger confirmation:
ls -la; rm -rf /

# These would NOT trigger (inside quotes):
echo "hello; world"
cat 'file with | in name'
```

### 4. Two-Step Confirmation

Dangerous commands and suspicious patterns require explicit user confirmation through AI assistant.

**Process:**
```
1. First Call
   └── Returns security warning
   └── Command stored in pending list
   └── 5-minute timeout starts
   
2. User Confirmation
   └── User reviews warning with AI assistant
   └── User explicitly approves execution
   
3. Second Call (within 5 minutes)
   └── Command matches pending entry
   └── Command removed from pending
   └── Command executes
```

**Confirmation Response:**
```
Security Warning: Dangerous command 'rm (delete files)' detected.

Command: rm -rf /home/user/temp

This command may cause damage to the system or data. 
Please confirm with the user whether to execute this command.

If the user agrees, please call the Bash tool 
again with the same parameters to confirm execution.
```

### 5. Audit Logging

All command execution attempts are logged for review.

**Log Format:**
```
[AUDIT] Execute command attempt: cwd=/path, command=ls -la
[AUDIT] Dangerous command pending confirmation: id=1, command=rm -rf /
[AUDIT] Command with injection patterns pending confirmation: command=ls; cat /etc/passwd
[AUDIT] Command executed after confirmation: command=rm -rf /tmp/old
[AUDIT] Command executed: exit_code=0, cwd=/path, command=ls -la
[AUDIT] Command execution failed: error=..., cwd=/path, command=ls -la
[AUDIT] Command timed out: timeout=30, cwd=/path, command=sleep 100
```

**Enable Debug Logging:**
```bash
RUST_LOG=debug ./rust-mcp-server
```

### 6. Concurrency Limits

Prevents resource exhaustion through configurable concurrency limits.

- **Default:** 10 concurrent calls
- **Maximum:** Configurable via `--max-concurrency`
- **Per-tool:** All tools share the same concurrency pool

### 7. Timeout Protection

Prevents long-running commands from hanging.

- **Default:** 30 seconds
- **Maximum:** 300 seconds (5 minutes)
- **Configuration:** Per-command timeout parameter
- **Behavior:** Child process is killed on timeout, not just detached

### 8. Output & Content Size Limits

Prevents memory exhaustion from large outputs or files.

- **Command output:** 100KB per output (stdout + stderr), UTF-8 safe truncation
- **File write:** 100MB maximum content size
- **Image read:** 50MB maximum file size (metadata-only mode for oversized images)
- **Python code:** 10,000 characters maximum
- **Shell command:** 10,000 characters maximum

### 9. Python Sandbox

The `ExecutePython` tool runs user code in a RustPython interpreter with sandboxing.

**Sandbox features:**
- `builtins.open` and `_io.open` / `_io.FileIO` are replaced with blocked stubs when filesystem access is disabled
- `os` module filesystem functions (`listdir`, `mkdir`, `remove`, `rename`, `stat`, `walk`, etc.) are replaced with blocked stubs in sandbox mode
- Network standard library modules (`socket`, `urllib`, `http`, `ssl`) remain fully functional in sandbox mode
- `subprocess` and `ctypes` are blocked as a security baseline
- When filesystem access is enabled via WebUI, `open()` and `os` filesystem functions are wrapped to restrict paths to the working directory
- Execution timeout uses `sys.settrace` to inject a self-terminating check inside the VM

### 10. HTTP SSRF Protection

The `WebFetch` tool includes server-side request forgery protections.

**Protections:**
- Blocks private IP ranges: `127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`, `::1`, `::`, `fc00::/7`, `fe80::/10`
- Blocks IPv4-mapped IPv6 addresses (`::ffff:127.0.0.1`)
- Blocks `localhost` and `.localhost` domains
- Disables automatic HTTP redirects
- Connection timeout: 10 seconds; request timeout: 60 seconds
- Maximum 10 idle connections per host

## Tool Classification

### Safe Tools
These tools are generally safe (read-only or non-destructive). The `minimal` preset enables 9 of them by default. Read-only file tools are not restricted to the working directory:
- `Glob` - Directory listing (no working directory restriction)
- `Read` - File reading (no working directory restriction)
- `Grep` - File content search (no working directory restriction)
- `WebFetch` - URL content fetching (with SSRF protection)
- `FileStat` - File/directory metadata and path existence check (no working directory restriction)
- `Git` - Git repository read-only operations (no working directory restriction)
- `SystemInfo` - Comprehensive system information (OS, CPU, memory, disks, network interfaces, temperature); on legacy Windows versions prior to Windows 10, disk, network, and temperature data are omitted
- `ExecutePython` - Python code execution. All standard library modules are available. Filesystem access is toggleable via WebUI.
- `Clipboard` - Read/write system clipboard content (text or image)
- `Diff` - Compare text, files, or directories (read-only, file/dir modes restricted to working directory)
- `NoteStorage` - In-memory temporary scratchpad for AI short-term memory (auto-clears after 30min)
- `Task` - Create, list, update, and delete tasks with title, description, priority, and tags
- `WebSearch` - Search the web using configurable search engine (uses external network, results may vary)
- `WebFetch` - Fetch and parse content from a URL (fetches external content, data may be untrusted)
- `AskUser` - Prompt the user for input or confirmation

### Dangerous Tools (6)
These tools require caution and are disabled by default:
- `Write` - File writing (can overwrite data, restricted to working directory, 100MB limit)
- `FileOps` - Copy, move, delete, or rename files (restricted to working directory)
- `Edit` - Multi-mode file editing (can modify files, restricted to working directory)
- `Bash` - Shell command execution (with injection detection, two-step confirmation, and custom shell path support)
- `Archive` - ZIP archive creation/extraction (restricted to working directory)
- `NotebookEdit` - Read, write, and edit Jupyter .ipynb notebook files (restricted to working directory)

## Best Practices

### For Administrators

1. **Set Restrictive Working Directory**
   ```bash
   ./rust-mcp-server --working-dir /var/mcp-safe
   ```
   - Create dedicated directory for MCP operations
   - Set appropriate filesystem permissions
   - Regularly audit directory contents

2. **Default Tool Policy**
   ```bash
   # Start with minimal preset (default)
   ./rust-mcp-server --preset minimal
   
   # Use a more restrictive preset or none
   ./rust-mcp-server --preset none
   ```
    - The `minimal` preset enables only safe, read-only tools (9 tools)
    - Only switch to higher presets (`coding`, `data_analysis`, `system_admin`, `research`, `full_power`) as needed
   - Enable `Bash` only in trusted environments

3. **Review Audit Logs Regularly**
   ```bash
   # Filter audit logs
   grep "\[AUDIT\]" /var/log/mcp-server.log
   
   # Monitor in real-time
   tail -f /var/log/mcp-server.log | grep "\[AUDIT\]"
   ```

4. **Network Security**
   - Bind to localhost only (default: 127.0.0.1)
   - Use firewall rules if exposing externally
   - Consider VPN for remote access

5. **Update Regularly**
   - Keep server updated with latest security patches
   - Review changelog for security updates

### For Users

1. **Verify Before Confirming**
   - Always review the full command before confirming
   - Understand what each flag and parameter does
   - Don't confirm if unsure

2. **Use Specific Paths**
   - Avoid wildcards when possible
   - Use absolute paths within working directory
   - Double-check target paths for destructive operations

3. **Check Working Directory**
   - Verify you're operating in the correct directory
   - Use `pwd` or `Glob` to confirm location

4. **Report Suspicious Activity**
   - Monitor for unexpected tool calls
   - Report any suspicious AI behavior

## Security Checklist

Before deploying to production:

- [ ] Working directory is properly configured and restricted
- [ ] Only necessary tools are enabled
- [ ] Dangerous commands are properly configured
- [ ] Audit logging is enabled and logs are monitored
- [ ] Server is bound to localhost or secure network
- [ ] Concurrency limits are appropriate for your hardware
- [ ] Regular log review process is established
- [ ] Backup strategy is in place
- [ ] Server is running latest version

## Incident Response

### If Suspicious Activity Detected

1. **Immediate Actions**
   ```bash
   # Stop MCP service via WebUI or API
  curl -X POST http://127.0.0.1:2233/api/mcp/stop
   ```

2. **Review Logs**
   ```bash
   # Check recent audit logs
   grep "\[AUDIT\]" /var/log/mcp-server.log | tail -100
   ```

3. **Check File System**
   - Review files in working directory
   - Check for unauthorized changes
   - Verify system integrity

## Reporting Security Issues

Please report security vulnerabilities privately before public disclosure:

1. Create a private security advisory on GitHub
2. Include detailed reproduction steps
3. Provide impact assessment
4. Allow reasonable time for fix before disclosure

## References

- [OWASP Command Injection](https://owasp.org/www-community/attacks/Command_Injection)
- [OWASP Path Traversal](https://owasp.org/www-community/attacks/Path_Traversal)
- [MCP Security Model](https://modelcontextprotocol.io/)

---

中文版本请查看 [security-zh.md](security-zh.md)
