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
- MSRV 1.78 build job
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

This project uses the following Rust crates (see `Cargo.toml` for exact versions):

- `todoist-api = "0.3.0"` - Unofficial Todoist API client
- `ratatui = "0.29"` - Terminal UI framework
- `crossterm = "0.29"` - Cross-platform terminal handling
- `tokio = "1.x"` - Async runtime
- `sqlx = "0.8"` - Database toolkit with SQLite support
- `serde` - Serialization/deserialization
- `chrono = "0.4"` - Date and time handling
- `anyhow = "1.0"` - Error handling
- `toml = "0.8"` - Configuration file parsing
- `dirs = "5.0"` - Platform-specific directory paths