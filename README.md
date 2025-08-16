# Terminalist - Todoist Terminal Client

A terminal application for interacting with Todoist, built in Rust with a modern TUI interface.

## Features

- ✅ **Interactive TUI Interface** - Beautiful terminal user interface with ratatui
- ✅ **Local Data Caching** - Fast, responsive UI with in-memory SQLite storage
- ✅ **Smart Sync** - Automatic sync on startup and manual refresh with 'r'
- ✅ **Project Management** - Browse projects with hierarchical display
- ✅ **Task Management** - View, navigate, complete, and create tasks
- ✅ **Keyboard Navigation** - Efficient keyboard-only operation
- ✅ **Real-time Updates** - Create, complete, and delete tasks/projects
- ✅ **Label Support** - View task labels with colored badges
- ✅ **Responsive Layout** - Adapts to terminal size with smart scaling
- ✅ **Help System** - Built-in help panel with keyboard shortcuts

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
- **`Tab`** Switch focus between panes

### **Task Management**
- **`Space`** or **`Enter`** Toggle task completion (complete ↔ reopen)
- **`a`** Create new task
- **`d`** Delete selected task (with confirmation)

### **Project Management**
- **`A`** Create new project
- **`D`** Delete selected project (with confirmation)

### **System**
- **`r`** Force sync with Todoist
- **`?`** Toggle help panel
- **`q`** Quit the application
- **`Esc`** Cancel action or close dialogs
- **`Ctrl+C`** Quit application

### **Help Panel Scrolling**
- **`↑/↓`** Scroll help content up/down
- **`Home/End`** Jump to top/bottom of help

The interface consists of:

### **Layout Structure**
- **Top Area**: Projects list (1/3 width) | Tasks list (2/3 width) - side by side
- **Bottom Area**: Status bar (full width, 1 line height)

### **Components**
- **Projects List (Left)**: Hierarchical display of all Todoist projects
  - Responsive width: 1/3 of screen with a maximum of 30 characters
  - Long project names are automatically truncated with ellipsis (…)
  - Parent-child relationships clearly shown
- **Tasks List (Right)**: Shows tasks for the currently selected project
  - Takes remaining width after projects list
  - Displays task content, priority, labels, and status
- **Status Bar (Bottom)**: Full-width bar showing available shortcuts and current status
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
- **Sync Indicators**: Status bar shows sync progress and data freshness

### **Data Management**
- **Projects**: Hierarchical structure with parent-child relationships
- **Tasks**: Full task details including labels, priority, and status
- **Labels**: Colored badges for task categorization
- **Real-time Updates**: Create, modify, and delete tasks/projects immediately


## Dependencies

This project uses the following Rust crates:

- `todoist-api = "0.2.0"` - Unofficial Todoist API client
- `ratatui = "0.24"` - Terminal UI framework
- `crossterm = "0.27"` - Cross-platform terminal handling
- `tokio = "1.0"` - Async runtime
- `sqlx = "0.7"` - Database toolkit with SQLite support
- `serde` - Serialization/deserialization
- `chrono = "0.4"` - Date and time handling
- `anyhow = "1.0"` - Error handling

## Project Structure

```
src/
├── main.rs                    # Main application entry point
├── lib.rs                     # Library exports
├── todoist.rs                 # Todoist API models & display structs
├── sync.rs                    # Sync service
├── storage.rs                 # SQLite storage (in-memory)
├── badge.rs                   # Badge creation utilities
├── terminal_badge.rs          # Terminal-optimized badges
└── ui/                        # TUI components
    ├── app.rs                 # Application state management
    ├── events.rs              # Event handling & keyboard shortcuts
    ├── layout.rs              # Layout management
    ├── renderer.rs            # Main rendering loop
    └── components/            # UI components
        ├── projects_list.rs   # Projects display
        ├── tasks_list.rs      # Tasks display
        ├── status_bar.rs      # Status bar
        ├── help_panel.rs      # Help overlay
        └── dialogs/           # Dialog components
            ├── error_dialog.rs
            ├── delete_confirmation_dialog.rs
            ├── project_creation_dialog.rs
            ├── task_creation_dialog.rs
            └── project_delete_confirmation_dialog.rs
```

## Development Setup

This project is set up with modern Rust development tooling:

### Quick Start
```bash
# Install Rust components
rustup component add rustfmt clippy

# Development workflow
make dev          # Format + lint + check
make format       # Format code with rustfmt
make lint         # Run clippy linter
make fix          # Auto-fix clippy issues
```

### Available Commands
```bash
make help         # Show all available commands
make format       # Format code with rustfmt
make lint         # Run clippy linter  
make fix          # Auto-fix clippy issues
make check        # Check code without building
make test         # Run tests
make build        # Build the project
make run          # Run the main application
make clean        # Clean build artifacts
make all          # Run format, fix, check, test, and build
make dev          # Quick development check (format + fix + check)
make strict-lint  # Run all clippy lints (very strict)
make docs         # Generate and open documentation
```

### Configuration Files

- `rustfmt.toml` - Code formatting rules
- `clippy.toml` - Linting rules
- `Makefile` - Development commands

### CI/CD

GitHub Actions workflow is configured in `.github/workflows/ci.yml` with:
- Format checking with rustfmt
- Linting with clippy
- Testing on multiple Rust versions
- Security auditing

## Contributing

This is a fully-featured TUI application for Todoist. You can extend it by:

- Adding more keyboard shortcuts
- Implementing additional task filters
- Adding configuration file support
- Enhancing the badge system
- Adding more dialog types

### Development Workflow

1. `make format` - Format your code
2. `make fix` - Auto-fix linting issues
3. `make test` - Run tests
4. `make check` - Quick compile check
5. `git commit` - Commit your changes

## License

This project is open source. Feel free to modify and use as needed.