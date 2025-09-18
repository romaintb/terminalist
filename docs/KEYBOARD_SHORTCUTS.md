# Keyboard Shortcuts

This document lists all available keyboard shortcuts and TUI controls.

## Navigation

- **`j/k`** Navigate between tasks (down/up)
- **`J/K`** Navigate between projects (down/up)
- **Mouse** Click on sidebar items to navigate

## Task Management

- **`Space`** or **`Enter`** Complete task
- **`a`** Create new task
- **`d`** Delete selected task (with confirmation)
- **`p`** Cycle task priority
- **`t`** Set task due date to today
- **`T`** Set task due date to tomorrow
- **`w`** Set task due date to next week (Monday)
- **`W`** Set task due date to next week end (Saturday)

## Project Management

- **`A`** Create new project
- **`D`** Delete selected project (with confirmation)

## System

- **`/`** Open task search dialog (search across all tasks)
- **`r`** Force sync with Todoist
- **`i`** Cycle through icon themes
- **`?`** Toggle help panel
- **`q`** Quit the application
- **`Esc`** Cancel action or close dialogs
- **`Ctrl+C`** Quit application

## Task Search

- **`/`** Open search dialog
- **Type** Search across all tasks by content
- **`Enter`** Close search dialog
- **`Esc`** Close search dialog
- **`Backspace/Delete`** Edit search query
- **`Left/Right`** Move cursor in search box

## Help Panel Scrolling

- **`↑/↓`** Scroll help content up/down
- **`Home/End`** Jump to top/bottom of help

## Interface Layout

### Layout Structure
- **Main Area**: Projects list (sidebar) | Tasks list (main area) - side by side

### Components
- **Projects List (Left)**: Hierarchical display of all Todoist projects
  - Configurable width via `sidebar_width` in config
  - Long project names are automatically truncated with ellipsis (…)
  - Parent-child relationships clearly shown
- **Tasks List (Right)**: Shows tasks for the currently selected project
  - Takes remaining width after projects list
  - Displays task content, priority, labels, and status
- **Help Panel**: Modal overlay accessible with `?` key

### Task Display Features
Tasks are displayed with:
- **Status Icons**: ☐ (pending), ☒ (completed), ✗ (deleted)
- **Priority Badges**: [P0] (urgent), [P1] (high), [P2] (medium), [P3] (low), no badge (normal)
- **Label Badges**: Colored badges showing task labels
- **Task Content**: Truncated to fit the display width
- **Completion Visual**: Completed tasks appear dimmed
- **Interactive**: Press Space or Enter to toggle completion