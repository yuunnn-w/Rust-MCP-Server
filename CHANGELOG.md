# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-03-15

### Added
- Initial release of Rust MCP Server
- 18 built-in tools for file operations, system info, HTTP requests, and more
- WebUI control panel with real-time updates
- Multi-transport support (SSE and HTTP)
- Dangerous command blacklist with configurable IDs (19 commands)
- Command injection detection
- Two-step confirmation for dangerous operations
- Working directory restriction for file operations
- Audit logging for all command executions
- Internationalization support (English and Chinese)
- Comprehensive documentation

### Security
- Working directory restriction for all file operations
- Dangerous command blacklist (19 command patterns)
- Command injection pattern detection
- Two-step confirmation for suspicious commands
- Automatic pending command cleanup (5-minute timeout)

---

中文版本请查看 [CHANGELOG-zh.md](CHANGELOG-zh.md)
