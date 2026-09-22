# Development Guide

This document contains information for developers contributing to Terminalist.

## Development Setup

This project is set up with modern Rust development tooling.

### Quick Start

```bash
# Install Rust components
rustup component add rustfmt clippy

# Development workflow
cargo fmt && cargo clippy --fix --allow-dirty && cargo check  # Format + lint + check
```

### Available Commands

```bash
cargo fmt         # Format code with rustfmt
cargo clippy      # Run clippy linter
cargo clippy --fix --allow-dirty  # Auto-fix clippy issues
cargo check       # Check code without building
cargo test        # Run tests
cargo build       # Build the project
cargo run         # Run the main application
cargo clean       # Clean build artifacts
cargo clippy -- -W clippy::all -W clippy::pedantic  # Run all clippy lints (strict)
cargo doc --open --no-deps  # Generate and open documentation
```

### Configuration Files

- `rustfmt.toml` - Code formatting rules
- `clippy.toml` - Linting rules

### Development Workflow

1. `cargo fmt` - Format your code
2. `cargo clippy --fix --allow-dirty` - Auto-fix linting issues
3. `cargo test` - Run tests
4. `cargo check` - Quick compile check
5. `git commit` - Commit your changes

## CI/CD

GitHub Actions workflow is configured in `.github/workflows/ci.yml` with:
- Format checking with rustfmt
- Linting with clippy
- Testing on multiple Rust versions and OSes
- MSRV 1.94 build job
- Smoke tests for `--help` and `--version`
- Security auditing

## Contributing

This is a fully-featured TUI application for Todoist. You can extend it by:

- Adding more keyboard shortcuts
- Implementing additional task filters
- Extending the configuration system
- Enhancing the badge system
- Adding more dialog types

## Dependencies

The crates this project uses and what each one is for. `Cargo.toml` is the source of truth for
versions, so none are repeated here.

- `todoist-api` - Unofficial Todoist API client
- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal handling
- `tokio` - Async runtime (only the `rt-multi-thread`, `macros`, `time` and `sync` features)
- `sea-orm` - ORM with SQLite support (via its `sqlx-sqlite` feature, default features off)
- `serde` / `serde_json` - Serialization/deserialization
- `chrono` - Date and time handling
- `uuid` - Entity primary keys
- `anyhow` / `thiserror` - Error handling
- `async-trait` - Async methods in the backend trait
- `toml` - Configuration file parsing
- `dirs` - Platform-specific directory paths
- `log` - Logging facade
