# 更新日志

本文件记录项目的所有重要更新。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本控制](https://semver.org/lang/zh-CN/)。

## [0.1.0] - 2024-03-15

### 新增功能
- Rust MCP Server 初始版本发布
- 18 个内置工具：文件操作、系统信息、HTTP 请求等
- WebUI 控制面板，支持实时更新
- 多传输协议支持（SSE 和 HTTP）
- 危险命令黑名单，支持配置 ID（19 个命令）
- 命令注入检测
- 危险操作两步确认机制
- 文件操作工作目录限制
- 所有命令执行的审计日志
- 国际化支持（中文和英文）
- 完整的文档体系

### 安全特性
- 所有文件操作的工作目录限制
- 危险命令黑名单（19 种命令模式）
- 命令注入模式检测
- 可疑命令两步确认
- 自动清理待确认命令（5 分钟超时）

---

For English version, see [CHANGELOG.md](CHANGELOG.md)
