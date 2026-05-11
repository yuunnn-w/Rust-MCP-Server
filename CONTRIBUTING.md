# Contributing to Rust MCP Server

Thank you for your interest in contributing to Rust MCP Server! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful and constructive in all interactions.

## How to Contribute

### Reporting Bugs

1. Check if the issue already exists in [GitHub Issues](https://github.com/yuunnn-w/Rust-MCP-Server/issues)
2. If not, create a new issue with:
   - Clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - System information (OS, Rust version)
   - Relevant logs or screenshots

### Suggesting Features

1. Open a new issue with the "feature request" label
2. Describe the feature and its use case
3. Explain why it would be useful

### Pull Requests

1. Fork the repository
2. Create a new branch (`git checkout -b feature/your-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Ensure code compiles without warnings (`cargo build`)
6. Format code (`cargo fmt`)
7. Run clippy (`cargo clippy`)
8. Commit with clear messages
9. Push to your fork
10. Create a Pull Request

## Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/Rust-MCP-Server.git
cd Rust-MCP-Server

# Build
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

## Coding Standards

- Follow Rust naming conventions
- Write documentation comments for public APIs
- Add tests for new features
- Keep functions focused and small
- Use meaningful variable names
- Handle errors properly

## Commit Message Format

```
type(scope): subject

body (optional)

footer (optional)
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Example:
```
feat(tools): add Grep tool

Add a new tool to search for keywords in files and directories.
Implements recursive search with depth limit.
```

## Testing

- Write unit tests for utility functions
- Write integration tests for tools
- Ensure all tests pass before submitting PR

## Documentation

- Update README if adding new features
- Update relevant docs in `docs/` directory
- Add inline documentation for complex code

## Questions?

Feel free to open an issue or start a discussion!
