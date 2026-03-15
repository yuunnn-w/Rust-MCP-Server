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
│  └── Detects ; | & ` $ ( ) < > characters                   │
├─────────────────────────────────────────────────────────────┤
│  Layer 4: Two-Step Confirmation                             │
│  └── User must confirm dangerous operations                 │
├─────────────────────────────────────────────────────────────┤
│  Layer 5: Audit Logging                                     │
│  └── All commands logged with context                       │
├─────────────────────────────────────────────────────────────┤
│  Layer 6: Resource Limits                                   │
│  └── Concurrency limits, timeouts, output limits            │
└─────────────────────────────────────────────────────────────┘
```

## Security Features

### 1. Working Directory Restriction

All file operations are restricted to a configurable working directory to prevent unauthorized access to sensitive system files.

**How it works:**
1. All paths are canonicalized to absolute form
2. Symbolic links are resolved
3. Path traversal patterns (`../`) are detected and blocked
4. Paths outside working directory are rejected

**Configuration:**
```bash
# Restrict to specific directory
./rust-mcp-server --working-dir /var/mcp-safe

# Using environment variable
export MCP_WORKING_DIR=/var/mcp-safe
./rust-mcp-server
```

**Security Check:**
```rust
// Pseudo-code
fn validate_path(path: &Path, working_dir: &Path) -> bool {
    let canonical = path.canonicalize()?;
    canonical.starts_with(working_dir)
}
```

### 2. Dangerous Command Blacklist

The `execute_command` tool blocks 20 dangerous command patterns by default.

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
;  |  &  `  $  (  )  <  >
```

**How it works:**
- Analyzes command string before execution
- Excludes characters inside quoted strings (both single and double quotes)
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

If the user agrees, please call the execute_command tool 
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

### 8. Output Limits

Prevents memory exhaustion from large command outputs.

- **Limit:** 100KB per output (stdout + stderr)
- **Behavior:** Truncated with notification

## Tool Classification

### Safe Tools (11)
These tools are safe to enable by default:
- `calculator` - Mathematical calculations
- `dir_list` - Directory listing
- `file_read` - File reading
- `file_search` - File content search
- `datetime` - Date/time
- `base64_encode` - Base64 encoding
- `base64_decode` - Base64 decoding
- `hash_compute` - Hash calculation
- `http_request` - HTTP requests
- `image_read` - Image reading
- `system_info` - System information

### Dangerous Tools (7)
These tools require caution:
- `file_write` - File writing (can overwrite data)
- `file_copy` - File copying
- `file_move` - File moving
- `file_delete` - File deletion
- `file_rename` - File renaming
- `execute_command` - Shell command execution
- `process_list` - Process listing (information disclosure)

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
   # Start with minimal tools
   ./rust-mcp-server --disable-tools file_write,file_copy,file_move,file_delete,file_rename,execute_command,http_request
   ```
   - Only enable tools as needed
   - Enable `execute_command` only in trusted environments

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
   - Use `pwd` or `dir_list` to confirm location

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
