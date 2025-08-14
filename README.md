# Terminalist - Todoist Terminal Client

A terminal application for interacting with Todoist, built in Rust.

## Features

- ✅ **Interactive TUI Interface** - Beautiful terminal user interface
- ✅ **Local Data Caching** - Fast, responsive UI with SQLite local storage
- ✅ **Smart Sync** - Automatic sync on startup and manual refresh with 'r'
- ✅ **Offline Support** - Browse cached data when offline
- ✅ **Project Sidebar** - Browse projects in a dedicated left sidebar
- ✅ **Task Management** - View, navigate, and complete tasks in the main pane
- ✅ **Interactive Task Toggle** - Complete and reopen tasks with Space/Enter
- ✅ **Keyboard Navigation** - Efficient keyboard-only operation
- ✅ **Sync Status** - Real-time sync indicators and data freshness info
- ✅ List all your Todoist projects
- ✅ View your tasks
- ✅ Filter tasks by project
- ✅ Create new tasks (API ready)
- ✅ Complete tasks (API ready)
- ✅ Delete tasks (API ready)
- ✅ Update task content (API ready)
- ✅ View labels (API ready)

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

- **← →** Navigate between projects (left sidebar)
- **↑ ↓** Navigate between tasks (main pane)
- **Space/Enter** Toggle task completion (complete ↔ reopen)
- **r** Refresh data from Todoist
- **q** Quit the application

The interface consists of:
- **Left Sidebar**: Lists all your Todoist projects with favorites marked with ⭐
  - Responsive width: 30% of screen with a maximum of 25 characters
  - Long project names are automatically truncated with ellipsis (…)
- **Main Pane**: Shows tasks for the currently selected project
- **Status Bar**: Displays available keyboard shortcuts and current status

Tasks are displayed with:
- **Status Icons**: ✅ (completed) or ⏳ (pending)
- **Priority Indicators**: 🔴 (urgent), 🟠 (high), 🟡 (medium), ⚪ (normal)
- **Task Content**: Truncated to fit the display
- **Rich Metadata Badges**:
  - `🔄REC` Recurring tasks (blue badge)
  - `⏰DUE` Tasks with deadlines (red badge)
  - `2h`, `30m` Duration estimates (yellow badge)
  - `3L` Tasks with labels (green badge)
  - `P1`, `P2`, `P3` Priority levels (colored badges)
- **Completion Visual**: Completed tasks appear dimmed and green
- **Interactive**: Press Space or Enter to toggle task completion (complete/reopen)

## Sync Mechanism

Terminalist uses a smart sync mechanism for optimal performance:

### **Local Storage**
- All data is cached locally in a SQLite database
- Located in your system's data directory (`~/.local/share/terminalist/` on Linux)
- Provides instant loading and offline browsing capabilities

### **Sync Behavior**
- **First Run**: Automatically syncs all data from Todoist
- **Startup**: Loads local data instantly, then syncs in background if data is older than 5 minutes
- **Manual Sync**: Press `r` to force refresh from Todoist API
- **Sync Indicators**: Status bar shows sync progress and data freshness

### **Offline Support**
- Browse cached projects and tasks when offline
- Sync status clearly indicates when data is stale
- Graceful error handling for network issues

## Usage Examples

### Using the TodoistWrapper in your code

```rust
use terminalist::todoist::TodoistWrapper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let todoist = TodoistWrapper::new("your_api_token".to_string());
    
    // Get all projects
    let projects = todoist.get_projects().await?;
    println!("Found {} projects", projects.len());
    
    // Get all tasks
    let tasks = todoist.get_tasks().await?;
    println!("Found {} tasks", tasks.len());
    
    // Create a new task
    let new_task = todoist.create_task("Learn Rust", None).await?;
    println!("Created task: {}", new_task.content);
    
    // Complete a task
    todoist.complete_task(&new_task.id).await?;
    println!("Task completed!");
    
    Ok(())
}
```

### Available Methods

The `TodoistWrapper` provides these methods:

- `get_projects()` - Get all projects
- `get_tasks()` - Get all tasks
- `get_tasks_for_project(project_id)` - Get tasks for a specific project
- `create_task(content, project_id)` - Create a new task
- `complete_task(task_id)` - Mark a task as complete
- `delete_task(task_id)` - Delete a task
- `update_task(task_id, content)` - Update task content
- `get_labels()` - Get all labels


## Dependencies

This project uses the following Rust crates:

- `reqwest` - HTTP client for making API requests
- `tokio` - Async runtime
- `serde` - Serialization/deserialization
- `serde_json` - JSON serialization
- `anyhow` - Error handling

## Project Structure

```
src/
├── main.rs        # Main application entry point
└── todoist.rs     # Todoist API wrapper module
```

## Error Handling

All API calls return `Result<T>` types. Make sure to handle errors appropriately:

```rust
match todoist.get_tasks().await {
    Ok(tasks) => {
        // Handle successful response
        for task in tasks {
            println!("{}", task.content);
        }
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Development Setup (Ruby Developer Friendly! 🚀)

This project is set up with Rust tooling that works similarly to RuboCop and other Ruby development tools:

### Quick Start
```bash
# Install Rust components (like installing gems)
rustup component add rustfmt clippy

# Development workflow (like running rubocop)
make dev          # Format + lint + check
make format       # Format code (like rubocop --auto-correct)
make lint         # Run linter (like rubocop)
make fix          # Auto-fix issues (like rubocop --auto-correct)
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
make example      # Run the basic usage example
make clean        # Clean build artifacts
make all          # Run format, fix, check, test, and build
make dev          # Quick development check (format + fix + check)
make strict-lint  # Run all clippy lints (very strict)
make docs         # Generate and open documentation
```

### IDE Setup (VS Code)

The project includes VS Code configuration for:
- **Format on save** (like RuboCop auto-correct)
- **Inline linting** (like RuboCop in your editor)
- **Auto-imports** and helpful hints
- **Debugging support**

Recommended extensions are configured in `.vscode/extensions.json`.

### Configuration Files

- `rustfmt.toml` - Code formatting rules (like `.rubocop.yml`)
- `clippy.toml` - Linting rules (like RuboCop cops configuration)
- `.vscode/settings.json` - IDE settings for format-on-save
- `Makefile` - Development commands (like Rake tasks)

### Rust Tools vs Ruby Tools

| Ruby Tool | Rust Equivalent | Purpose |
|-----------|----------------|---------|
| RuboCop | clippy | Linting and style checking |
| RuboCop --auto-correct | rustfmt | Code formatting |
| bundle exec | cargo | Running tools and commands |
| Rake | make (Makefile) | Task runner |
| RSpec | cargo test | Testing framework |
| bundler-audit | cargo audit | Security auditing |

### CI/CD

GitHub Actions workflow is configured in `.github/workflows/ci.yml` with:
- Format checking (like RuboCop in CI)
- Linting with clippy
- Testing on multiple Rust versions
- Security auditing

## Contributing

This is a basic wrapper to get you started. You can extend it by:

- Adding more API endpoints
- Implementing better error handling
- Adding command-line argument parsing
- Creating a proper TUI interface
- Adding configuration file support

### Development Workflow

1. `make format` - Format your code
2. `make fix` - Auto-fix linting issues
3. `make test` - Run tests
4. `make check` - Quick compile check
5. `git commit` - Commit your changes

## License

This project is open source. Feel free to modify and use as needed.
