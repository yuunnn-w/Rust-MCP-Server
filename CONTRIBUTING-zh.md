# 贡献指南

感谢您对 Rust MCP Server 项目的关注！本文档提供了参与贡献的指南和说明。

## 行为准则

在所有互动中保持尊重和建设性。

## 如何贡献

### 报告 Bug

1. 首先在 [GitHub Issues](https://github.com/yuunnn-w/Rust-MCP-Server/issues) 中检查问题是否已存在
2. 如果不存在，创建新 issue 并包含：
   - 清晰的标题和描述
   - 复现步骤
   - 预期行为与实际行为
   - 系统信息（操作系统、Rust 版本）
   - 相关日志或截图

### 建议新功能

1. 新建 issue 并添加 "feature request" 标签
2. 描述功能及其使用场景
3. 解释为什么这个功能有用

### 提交 Pull Request

1. Fork 本仓库
2. 创建新分支（`git checkout -b feature/your-feature`）
3. 进行您的修改
4. 运行测试（`cargo test`）
5. 确保代码无警告编译（`cargo build`）
6. 格式化代码（`cargo fmt`）
7. 运行 clippy（`cargo clippy`）
8. 提交清晰的提交信息
9. 推送到您的 fork
10. 创建 Pull Request

## 开发环境设置

```bash
# 克隆您的 fork
git clone https://github.com/YOUR_USERNAME/Rust-MCP-Server.git
cd Rust-MCP-Server

# 构建
cargo build

# 运行测试
cargo test

# 带日志运行
RUST_LOG=debug cargo run
```

## 编码规范

- 遵循 Rust 命名约定
- 为公共 API 编写文档注释
- 为新功能添加测试
- 保持函数专注且精简
- 使用有意义的变量名
- 正确处理错误

## 提交信息格式

```
type(scope): subject

body (可选)

footer (可选)
```

类型：`feat`、`fix`、`docs`、`style`、`refactor`、`test`、`chore`

示例：
```
feat(tools): 添加 Grep 工具

添加用于在文件和目录中搜索关键词的新工具。
实现了带深度限制的递归搜索。
```

## 测试

- 为工具函数编写单元测试
- 为工具编写集成测试
- 提交 PR 前确保所有测试通过

## 文档

- 添加新功能时更新 README
- 更新 `docs/` 目录中的相关文档
- 为复杂代码添加内联文档

## 有疑问？

随时创建 issue 或发起讨论！

---

For English version, see [CONTRIBUTING.md](CONTRIBUTING.md)
