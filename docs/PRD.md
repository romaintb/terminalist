# Terminalist - Product Requirements Document (PRD)

## 1. Executive Summary

### 1.1 Product Overview
Terminalist is a high-performance terminal user interface (TUI) application for Todoist, built in Rust. It provides a fast, keyboard-driven interface for managing tasks and projects directly from the terminal, targeting CLI users who want to replace the web application for daily task management. The project serves as a comprehensive Rust learning experience, implementing modern Rust patterns and best practices.

### 1.2 Key Value Propositions
- **Web App Replacement**: Complete daily task management without leaving the terminal
- **Speed**: Instant local data access with smart sync mechanisms
- **Efficiency**: Keyboard-only operation with vim-like navigation
- **Reliability**: Robust error handling and offline-first architecture
- **Rust Learning**: Comprehensive implementation of modern Rust patterns and best practices
- **Performance**: Native Rust implementation with minimal resource usage

### 1.3 Success Metrics
- **Performance**: Smooth operation without noticeable lags
- **Functionality**: Feature parity with Todoist web app for daily use
- **Code Quality**: 100% test coverage when codebase is stable
- **Rust Best Practices**: Full implementation of modern Rust patterns
- **User Experience**: CLI users can completely replace web app for daily task management

## 2. Current State Analysis

### 2.1 Existing Features ✅

#### Core Functionality
- **Interactive TUI Interface**: Modern terminal UI using ratatui framework
- **Local Data Caching**: On-disk SQLite cache for instant access
- **Smart Sync**: Automatic sync on startup, on a configurable interval, and manual refresh
- **Project Management**: Hierarchical project browsing and management
- **Task Management**: Full CRUD operations for tasks
- **Task Search**: Database-powered search across all tasks with '/' shortcut
- **Task Editing**: Edit task content and move a task to another project
- **Due Date Shortcuts**: Set a task due today, tomorrow, next Monday or Saturday
- **Time-Based Views**: Today, Tomorrow and Upcoming views across all projects
- **Sections**: Project sections are synced and tasks are grouped under them
- **Configurable Theme**: Interface colours, background included, come from the config file
- **Keyboard Navigation**: Efficient keyboard-only operation
- **Real-time Updates**: Immediate UI updates for all operations

#### Data Management
- **Projects**: Hierarchical display with parent-child relationships
- **Tasks**: Complete task details including priority, labels, due dates
- **Labels**: Colored badge system for task categorization
- **Sync Status**: Real-time sync progress and data freshness indicators

#### User Interface
- **Responsive Layout**: Adapts to terminal size with smart scaling
- **Status Indicators**: Visual task status (pending, completed, deleted)
- **Priority Badges**: P0/P1/P2/P3 priority system with visual indicators
- **Help System**: Built-in help panel with keyboard shortcuts
- **Error Handling**: Comprehensive error messages and recovery

#### Technical Architecture
- **Rust Implementation**: Native performance with memory safety
- **Async Runtime**: Tokio-based async operations
- **Database**: SQLite file in the XDG data directory, used as a disposable cache
- **API Integration**: todoist-api crate for all external communication
- **Modular Design**: Clean separation of concerns

### 2.2 Current Limitations & Technical Debt

#### Missing Core Features
- **Section Management**: Sections are displayed but cannot be created, renamed or deleted from the interface
- **Free-form Due Dates**: Only the today/tomorrow/next-week shortcuts, no arbitrary date entry
- **Advanced Filtering**: No filtering by priority, labels, or due dates (basic search implemented)
- **Task Descriptions**: Shown in the list, but not editable

#### User Experience
- Limited keyboard shortcuts compared to vim/emacs
- No undo functionality
- Key bindings are not remappable

#### Data Management
- No conflict resolution for concurrent edits
- No offline mode for task creation
- No data export/import capabilities
- Limited error recovery options

#### Technical
- ✅ ~~No configuration file support~~ (Implemented - TOML config with customizable UI, sync, display and theme settings)
- Partial test coverage (integration tests under `tests/`, full coverage planned for a stable codebase)
- No plugin system

## 3. Product Requirements

### 3.1 Functional Requirements

#### 3.1.1 Core Task Management
- **FR-001**: Users can view all tasks in a project with status indicators
- **FR-002**: Users can create new tasks with content, project assignment, and priority
- **FR-003**: Users can complete tasks
- **FR-004**: Users can delete tasks with confirmation dialog
- **FR-005**: Users can navigate tasks using j/k keys (down/up)
- **FR-006**: Tasks display priority badges (P0-P3) and label badges
- **FR-007**: Tasks are sorted by status (pending → completed → deleted)

#### 3.1.2 Project Management
- **FR-008**: Users can view hierarchical project structure
- **FR-009**: Users can create new projects with optional parent assignment
- **FR-010**: Users can delete projects with confirmation dialog
- **FR-011**: Users can navigate projects using J/K keys (down/up)
- **FR-012**: Projects display favorites and hierarchical relationships
- **FR-013**: Users can switch between project and label views
- **FR-014**: Users can create and manage project sections
- **FR-015**: Users can assign tasks to specific sections within projects

#### 3.1.3 Data Synchronization
- **FR-016**: Application syncs with Todoist API on startup and every `auto_sync_interval_minutes`
- **FR-017**: Users can force sync with 'r' key
- **FR-018**: Sync status is displayed during operations
- **FR-019**: Local data is cached for instant access
- **FR-020**: Sync errors are displayed with retry options

#### 3.1.4 User Interface
- **FR-021**: Two-pane layout: projects (left) and tasks (right)
- **FR-022**: Help panel shows available shortcuts
- **FR-023**: Help panel accessible with '?' key
- **FR-024**: Responsive design adapts to terminal size
- **FR-025**: Error messages are displayed prominently
- **FR-026**: Loading states are shown for async operations

#### 3.1.5 Time-Based Views
- **FR-027**: Users can view "Today" tasks across all projects
- **FR-028**: Users can view "Tomorrow" tasks across all projects
- **FR-029**: Users can view "Upcoming" tasks with due dates
- **FR-030**: Time-based views show tasks sorted by due date and priority

#### 3.1.6 Task Management Enhancements
- **FR-031**: ✅ Users can edit existing task content, and move a task to another project (Implemented)
- **FR-032**: Users can set and modify task due dates (shortcuts only: today, tomorrow, next Monday, Saturday)
- **FR-033**: Users can assign tasks to sections
- **FR-034**: ✅ Users can search tasks by content (Implemented)
- **FR-035**: Users can filter tasks by priority, labels, or due dates

### 3.2 Non-Functional Requirements

#### 3.2.1 Performance
- **NFR-001**: Application startup time < 500ms
- **NFR-002**: Task operations complete < 200ms
- **NFR-003**: Sync operations complete < 5 seconds
- **NFR-004**: Memory usage < 50MB for typical datasets
- **NFR-005**: CPU usage < 5% during idle state

#### 3.2.2 Reliability
- **NFR-006**: Zero data loss during normal operations
- **NFR-007**: Graceful handling of network failures
- **NFR-008**: Recovery from API rate limiting
- **NFR-009**: Consistent state after sync operations
- **NFR-010**: Proper cleanup on application exit

#### 3.2.3 Usability
- **NFR-011**: All operations accessible via keyboard
- **NFR-012**: Consistent key bindings throughout application
- **NFR-013**: Clear visual feedback for all actions
- **NFR-014**: Intuitive navigation patterns
- **NFR-015**: Help system covers all functionality

#### 3.2.4 Compatibility
- **NFR-016**: Works on Linux, macOS, and Windows
- **NFR-017**: Supports terminals with 80x24 minimum size
- **NFR-018**: Compatible with major terminal emulators
- **NFR-019**: Works with Todoist API v1
- **NFR-020**: Rust 1.94+ compatibility

## 4. User Stories

### 4.1 Primary User Personas

#### 4.1.1 CLI Daily User (Primary)
- **Background**: Developer or technical professional who uses CLI tools daily
- **Goals**: Replace Todoist web app completely for daily task management
- **Pain Points**: Web interface is slow, context switching is disruptive
- **Success Criteria**: Complete daily task management without leaving terminal

#### 4.1.2 Rust Learning Developer (Secondary)
- **Background**: Developer learning Rust through practical projects
- **Goals**: Implement modern Rust patterns and best practices
- **Pain Points**: Need real-world project to apply Rust concepts
- **Success Criteria**: Comprehensive Rust implementation with full test coverage

### 4.2 User Stories

#### 4.2.1 Task Management
- **US-001**: As a CLI user, I want to quickly view my tasks so I can see what needs to be done
- **US-002**: As a CLI user, I want to complete tasks with a single keystroke so I can maintain flow
- **US-003**: As a CLI user, I want to create tasks quickly so I can capture ideas without interruption
- **US-004**: As a CLI user, I want to edit existing tasks so I can update them without recreating
- **US-005**: As a CLI user, I want to set due dates so I can manage deadlines effectively

#### 4.2.2 Project Organization
- **US-006**: As a CLI user, I want to navigate between projects quickly so I can focus on different areas
- **US-007**: As a CLI user, I want to create new projects so I can organize my work
- **US-008**: As a CLI user, I want to see project hierarchy so I can understand organization
- **US-009**: As a CLI user, I want to use sections within projects so I can organize tasks like in the web app

#### 4.2.3 Time-Based Views
- **US-010**: As a CLI user, I want to see "Today" tasks so I can focus on immediate priorities
- **US-011**: As a CLI user, I want to see "Tomorrow" tasks so I can plan ahead
- **US-012**: As a CLI user, I want to see upcoming tasks so I can manage my schedule

#### 4.2.4 Search and Discovery
- **US-013**: ✅ As a CLI user, I want to search all my tasks quickly so I can find specific items (Implemented)
- **US-014**: As a CLI user, I want to filter tasks by different criteria so I can focus on what matters
- **US-015**: As a CLI user, I want search to be fast and responsive so I don't lose my flow

#### 4.2.5 Data Synchronization
- **US-016**: As a CLI user, I want my changes to sync automatically so I don't lose work
- **US-017**: As a CLI user, I want to force sync when needed so I can get latest data
- **US-018**: As a CLI user, I want to see sync status so I know when data is fresh

## 5. Technical Architecture

### 5.1 Current Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   UI Layer      │    │  Business Logic │    │   Data Layer    │
│                 │    │                 │    │                 │
│ • App State     │◄──►│ • Sync Service  │◄──►│ • Local Storage │
│ • Components    │    │ • Event Handler │    │ • Todoist API   │
│ • Renderer      │    │ • Task Manager  │    │ • SQLite DB     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### 5.2 Component Overview

See [Architecture Overview](ARCHITECTURE.md) for the modules and what each one holds.

### 5.3 Data Flow

1. **Startup**: Load local data → Check sync need → Sync if required
2. **User Action**: Event → App State Update → API Call → Local Update → UI Refresh
3. **Sync**: Fetch from API → Transform Data → Store Locally → Update UI

## 6. Implementation Roadmap

### 6.1 Phase 1: Foundation (Current State)
- ✅ Core TUI interface with ratatui
- ✅ Basic task and project management
- ✅ Todoist API integration
- ✅ Local SQLite caching
- ✅ Keyboard navigation
- ✅ Help system

### 6.2 Phase 2: Core Features
- [ ] **Sections Support**
  - ✅ Sections synced and displayed
  - [ ] Section creation and management
  - [ ] Task assignment to sections
  - [ ] Section-based task filtering
- [x] **Today/Tomorrow Views**
  - ✅ "Today" view showing all due tasks
  - ✅ "Tomorrow" view for planning
  - ✅ "Upcoming" view for later tasks
- [ ] **Task Editing**
  - ✅ Edit existing task content, and move a task to another project
  - ✅ Due date shortcuts (today, tomorrow, next Monday, Saturday)
  - [ ] Arbitrary due date entry
  - [ ] Task description editing
- [ ] **Enhanced Navigation**
  - ✅ Search functionality (Implemented)
  - [ ] Quick switching between views
  - [ ] Filter by priority, labels, due dates

### 6.3 Phase 3: Polish & Performance (6 months)
- [ ] **Themes & Customization**
  - ✅ Configurable colors, background included, via `[theme]`
  - ✅ Configuration file support (Implemented - TOML config with generate option)
  - [ ] Shipped color themes to pick from
  - [ ] Customizable key bindings
- [ ] **Performance Optimization**
  - ✅ Persistent local storage, kept between runs
  - ✅ Background sync, on startup and on an interval
  - ✅ Concurrent fetch of projects, tasks, labels and sections
  - [ ] Incremental sync
- [ ] **Advanced Features**
  - Bulk operations
  - Undo functionality
  - Recurring tasks
  - Task dependencies
- [ ] **Testing & Quality**
  - Comprehensive test coverage
  - Performance benchmarks
  - Error handling improvements

### 6.4 Phase 4: Advanced Features (12 months)
- [ ] **Advanced Task Management**
  - Comments support
  - File attachments
  - Task templates
  - Advanced filtering
- [ ] **Integration & Export**
  - Calendar integration
  - Time tracking
  - Data export/import
  - API for third-party tools
- [ ] **Collaboration Features**
  - Team project support
  - Shared projects
  - User mentions
- [ ] **Plugin System**
  - Custom commands
  - Extensible architecture
  - Third-party integrations

## 7. Success Criteria

### 7.1 Technical Metrics
- **Performance**: Smooth operation without noticeable lags
- **Reliability**: < 0.1% error rate for normal operations
- **Code Quality**: 100% test coverage when codebase is stable
- **Rust Best Practices**: Full implementation of modern Rust patterns

### 7.2 User Experience Metrics
- **Web App Replacement**: CLI users can completely replace Todoist web app for daily use
- **Feature Parity**: Core features match Todoist web app functionality
- **Efficiency**: Users report faster task management than web interface
- **Satisfaction**: Positive feedback from CLI user community

### 7.3 Learning & Development Metrics
- **Rust Mastery**: Comprehensive implementation of Rust concepts
- **Code Quality**: Zero clippy warnings, full test coverage
- **Architecture**: Clean, maintainable, and extensible codebase
- **Documentation**: Complete technical and user documentation

## 8. Risk Assessment

### 8.1 Technical Risks
- **API Changes**: Todoist API modifications could break functionality
  - *Mitigation*: Use stable API version, implement graceful degradation
- **Performance**: Large datasets could impact performance
  - *Mitigation*: Implement pagination, lazy loading, data archiving
- **Platform Compatibility**: Terminal differences across platforms
  - *Mitigation*: Extensive testing, fallback mechanisms

### 8.2 User Experience Risks
- **Learning Curve**: Terminal interface may be intimidating
  - *Mitigation*: Comprehensive help system, intuitive defaults
- **Feature Gaps**: Missing features compared to web/mobile apps
  - *Mitigation*: Focus on core workflows, clear feature boundaries

### 8.3 Business Risks
- **Maintenance Burden**: Ongoing API and dependency updates
  - *Mitigation*: Automated testing, dependency monitoring
- **User Base**: Limited to terminal users
  - *Mitigation*: Target specific user segments, focus on quality

## 9. Rust Learning Objectives

### 9.1 Core Rust Concepts
- **Ownership & Borrowing**: Proper memory management patterns
- **Error Handling**: Comprehensive use of Result<T, E> and Option<T>
- **Async Programming**: Tokio runtime and async/await patterns
- **Traits & Generics**: Generic programming and trait bounds
- **Pattern Matching**: Exhaustive match expressions and destructuring

### 9.2 Advanced Rust Features
- **Macros**: Custom derive macros and procedural macros
- **Unsafe Rust**: Safe abstractions over unsafe code when needed
- **Concurrency**: Thread-safe data structures and synchronization
- **Performance**: Zero-cost abstractions and optimization techniques
- **Testing**: Unit tests, integration tests, and property-based testing

### 9.3 Rust Ecosystem
- **Crates**: Effective use of external dependencies
- **Cargo**: Build system, dependency management, and workspace organization
- **Documentation**: Comprehensive rustdoc comments and examples
- **Clippy**: Static analysis and linting for code quality
- **Rustfmt**: Consistent code formatting and style

### 9.4 Project-Specific Learning
- **TUI Development**: Terminal user interface patterns with ratatui
- **Database Integration**: SQLite with sqlx for data persistence
- **API Integration**: HTTP clients and JSON serialization
- **Event Handling**: Input processing and state management
- **Error Recovery**: Graceful error handling and user feedback

## 10. Appendices

### 10.1 Current Keyboard Shortcuts

See [Keyboard Shortcuts](KEYBOARD_SHORTCUTS.md). It is kept there so the list is maintained in
one place only.

### 10.2 Dependencies

See [Development Guide](DEVELOPMENT.md) for the crates and their roles, and `Cargo.toml` for the
versions.

### 10.3 File Structure

See [Architecture Overview](ARCHITECTURE.md) for the project structure. It is kept there so the
tree is maintained in one place only.

---

*This PRD is a living document and should be updated as the product evolves and new requirements emerge.*
