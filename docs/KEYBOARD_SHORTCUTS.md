# Keyboard Shortcuts

This document lists all available keyboard shortcuts and TUI controls.

## Navigation

- **`j/k`** or **`↓/↑`** Navigate between tasks (down/up)
- **`J/K`** Navigate between sidebar entries (down/up)
- **`H`** Collapse the folder under the sidebar cursor
- **`L`** Expand the folder under the sidebar cursor
- **Mouse** Click on sidebar items or tasks to select them, scroll wheel to scroll

## Task Management

- **`Space`** or **`Enter`** Toggle the selected task. A pending task is completed, a completed or deleted one is restored.
- **`a`** Create new task
- **`e`** Edit selected task
- **`d`** or **`Delete`** Delete selected task (with confirmation). A deleted task is restored instead.
- **`p`** Cycle task priority
- **`t`** Set task due date to today
- **`T`** Set task due date to tomorrow
- **`w`** Set task due date to next week (Monday)
- **`W`** Set task due date to next week end (Saturday)

## Project and Label Management

- **`A`** Create new project
- **`E`** Edit the selected project or label
- **`D`** Delete the selected project or label (with confirmation)

## System

- **`b`** Toggle sidebar visibility
- **`/`** Open task search dialog (search across all tasks)
- **`r`** Force sync with Todoist
- **`G`** Open the log dialog
- **`?`** or **`h`** Toggle help panel
- **`q`** Quit the application
- **`Esc`** Close the dialog on screen. With no dialog open, quit the application.
- **`Ctrl+C`** Quit application
- **`R`** Reload from the local cache. Available with `--debug` only.

## Dialogs

- **`Enter`** Confirm. In the search dialog, close it.
- **`Esc`** Cancel and close
- **`Tab`** Cycle the project of a task in the task create and edit dialogs, or the parent project in the project creation dialog
- **Type** In the search dialog, filter all tasks by content as you type
- **`Left/Right`** Move the cursor in the input field
- **`Backspace/Delete`** Edit the input field

## Help Panel Scrolling

- **`↑/↓`** Scroll help content up/down
- **`Home/End`** Jump to top/bottom of help

## Interface Layout

### Layout Structure
- **Main Area**: Sidebar (views, projects, labels) | Tasks list (main area) - side by side

### Components
- **Sidebar (Left)**: Today, Tomorrow and Upcoming views, then a hierarchical display of all Todoist projects and labels
  - Configurable width via `sidebar_width` in config
  - Hidden and shown with `b`, and `sidebar_visible` sets the state at startup
  - Long project names are automatically truncated with ellipsis (…)
  - Parent-child relationships clearly shown
- **Tasks List (Right)**: Shows tasks for the currently selected sidebar entry
  - Takes remaining width after the sidebar
  - Displays task content, priority, labels, and status
- **Help Panel**: Modal overlay accessible with `?` or `h`
- **Sync Toast**: Sync progress is shown in a corner toast, it does not block the interface

### Task Display Features
Tasks are displayed with:
- **Status Icons**: ☐ (pending), ☒ (completed), ✗ (deleted)
- **Priority Flags**: a coloured flag before the content - ⚑ red (P1, urgent), ⚑ orange (P2, high), ⚑ blue (P3, medium), ⚐ white (P4, normal).
- **Label Badges**: Colored badges showing task labels
- **Task Content**: Truncated to fit the display width
- **Completion Visual**: Completed tasks appear dimmed
- **Interactive**: Press Space or Enter to toggle completion
