# Terminalist - Todoist Terminal Client

[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://github.com/romaintb/terminalist/workflows/CI/badge.svg)](https://github.com/romaintb/terminalist/actions)
[![Crates.io](https://img.shields.io/crates/v/terminalist.svg)](https://crates.io/crates/terminalist)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Terminal](https://img.shields.io/badge/terminal-TUI-brightgreen.svg)](https://github.com/romaintb/terminalist)
[![Todoist](https://img.shields.io/badge/Todoist-API-red.svg)](https://developer.todoist.com)

A terminal application for interacting with Todoist, built in Rust with a modern TUI interface.

## Features

- ✅ **Interactive TUI Interface** - Beautiful terminal user interface with ratatui
- ✅ **Local Data Caching** - Fast, responsive UI with in-memory SQLite storage
- ✅ **Smart Sync** - Automatic sync on startup and manual refresh with 'r'
- ✅ **Project Management** - Browse projects with hierarchical display
- ✅ **Task Management** - View, navigate, complete, and create tasks
- ✅ **Task Search** - Fast database-powered search across all tasks with '/' shortcut
- ✅ **Keyboard & Mouse Navigation** - Efficient keyboard operation with mouse support
- ✅ **Real-time Updates** - Create, complete, and delete tasks/projects
- ✅ **Label Support** - View task labels with colored badges
- ✅ **Responsive Layout** - Adapts to terminal size with smart scaling
- ✅ **Help System** - Built-in help panel with keyboard shortcuts

## Installation

### Option 1: Install from Crates.io (Recommended)

```bash
cargo install terminalist
```

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/romaintb/terminalist.git
cd terminalist

# Build the project
cargo build --release

# Run the application
cargo run --release
```

The binary will be available at `target/release/terminalist` after building.

## Setup

### 1. Get your Todoist API Token

1. Go to [Todoist Integrations Settings](https://todoist.com/prefs/integrations)
2. Find the "API token" section
3. Copy your API token

### 2. Set Environment Variable

```bash
export TODOIST_API_TOKEN=your_token_here
```

### 3. Build and Run

```bash
cargo build
cargo run
```

## TUI Controls

Once the application is running, you can use these keyboard shortcuts:

### **Navigation**
- **`j/k`** Navigate between tasks (down/up)
- **`J/K`** Navigate between projects (down/up)
- **Mouse** Click on sidebar items to navigate

### **Task Management**
- **`Space`** or **`Enter`** Complete task
- **`a`** Create new task
- **`d`** Delete selected task (with confirmation)
- **`p`** Cycle task priority
- **`t`** Set task due date to today
- **`T`** Set task due date to tomorrow
- **`w`** Set task due date to next week (Monday)
- **`W`** Set task due date to next week end (Saturday)

### **Project Management**
- **`A`** Create new project
- **`D`** Delete selected project (with confirmation)

### **System**
- **`/`** Open task search dialog (search across all tasks)
- **`r`** Force sync with Todoist
- **`i`** Cycle through icon themes
- **`?`** Toggle help panel
- **`q`** Quit the application
- **`Esc`** Cancel action or close dialogs
- **`Ctrl+C`** Quit application

### **Task Search**
- **`/`** Open search dialog
- **Type** Search across all tasks by content
- **`Enter`** Close search dialog
- **`Esc`** Close search dialog
- **`Backspace/Delete`** Edit search query
- **`Left/Right`** Move cursor in search box

### **Help Panel Scrolling**
- **`↑/↓`** Scroll help content up/down
- **`Home/End`** Jump to top/bottom of help

The interface consists of:

### **Layout Structure**
- **Main Area**: Projects list (1/3 width) | Tasks list (2/3 width) - side by side

### **Components**
- **Projects List (Left)**: Hierarchical display of all Todoist projects
  - Responsive width: 1/3 of screen with a maximum of 30 characters
  - Long project names are automatically truncated with ellipsis (…)
  - Parent-child relationships clearly shown
- **Tasks List (Right)**: Shows tasks for the currently selected project
  - Takes remaining width after projects list
  - Displays task content, priority, labels, and status
- **Help Panel**: Modal overlay accessible with `?` key

### **Task Display Features**
Tasks are displayed with:
- **Status Icons**: 🔳 (pending), ✅ (completed), ❌ (deleted)
- **Priority Badges**: [P0] (urgent), [P1] (high), [P2] (medium), [P3] (low), no badge (normal)
- **Label Badges**: Colored badges showing task labels
- **Task Content**: Truncated to fit the display width
- **Completion Visual**: Completed tasks appear dimmed
- **Interactive**: Press Space or Enter to toggle completion

## Sync Mechanism

Terminalist uses a smart sync mechanism for optimal performance:

### **Local Storage**
- All data is cached locally in an **in-memory SQLite database**
- **No persistence between sessions** - data is fresh on each launch
- Provides instant loading and fast response times

### **Sync Behavior**
- **First Run**: Automatically syncs all data from Todoist
- **Startup**: Loads local data instantly, then syncs in background if data is older than 5 minutes
- **Manual Sync**: Press `r` to force refresh from Todoist API
- **Sync Indicators**: Sync progress is shown during operations

### **Data Management**
- **Projects**: Hierarchical structure with parent-child relationships
- **Tasks**: Full task details including labels, priority, and status
- **Labels**: Colored badges for task categorization
- **Search**: Fast database-level search across all tasks with live results
- **Real-time Updates**: Create, modify, and delete tasks/projects immediately


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

## Project Structure

```
src/
├── main.rs                    # Main application entry point
├── lib.rs                     # Library exports
├── todoist.rs                 # Todoist API models & display structs
├── sync.rs                    # Sync service with API integration
├── storage/                   # SQLite storage (in-memory)
│   ├── db.rs
│   ├── labels.rs
│   ├── projects.rs
│   ├── sections.rs
│   ├── tasks.rs
│   └── mod.rs
├── icons.rs                   # Icon service for terminal compatibility
├── logger.rs                  # Debug logging system
├── utils/                     # Utility modules
│   ├── mod.rs
│   └── datetime.rs            # Date/time utilities
└── ui/                        # Modern Component-Based Architecture
    ├── app_component.rs       # Main application orchestrator
    ├── renderer.rs            # Modern rendering system
    ├── core/                  # Core architecture components
    │   ├── actions.rs         # Action system for component communication
    │   ├── component.rs       # Component trait and lifecycle
    │   ├── context.rs         # App context
    │   ├── event_handler.rs   # Event processing system
    │   └── task_manager.rs    # Background async task management
    └── components/            # UI Components
        ├── badge.rs
        ├── dialog_component.rs    # Unified modal dialog system
        ├── sidebar_component.rs   # Project/label navigation
        ├── task_list_component.rs # Task management and display
        └── task_list_item_component.rs
```

## Development Setup

This project is set up with modern Rust development tooling:

### Quick Start
```bash
# Install Rust components
rustup component add rustfmt clippy

# Development workflow
cargo fmt && cargo clippy --fix --allow-dirty && cargo check  # Format + lint + check
cargo fmt         # Format code with rustfmt
cargo clippy      # Run clippy linter
cargo clippy --fix --allow-dirty  # Auto-fix clippy issues
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

### CI/CD

GitHub Actions workflow is configured in `.github/workflows/ci.yml` with:
- Format checking with rustfmt
- Linting with clippy
- Testing on multiple Rust versions and OSes
- MSRV 1.78 build job
- Smoke tests for `--help` and `--version`
- Security auditing

See the roadmap and product details in `docs/PRD.md`.

## Contributing

This is a fully-featured TUI application for Todoist. You can extend it by:

- Adding more keyboard shortcuts
- Implementing additional task filters
- Adding configuration file support
- Enhancing the badge system
- Adding more dialog types

### Development Workflow

1. `cargo fmt` - Format your code
2. `cargo clippy --fix --allow-dirty` - Auto-fix linting issues
3. `cargo test` - Run tests
4. `cargo check` - Quick compile check
5. `git commit` - Commit your changes

## License

This project is open source. Feel free to modify and use as needed.