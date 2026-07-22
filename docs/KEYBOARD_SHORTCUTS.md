# Keyboard Shortcuts

This document lists all available keyboard shortcuts and TUI controls.

## Navigation

- **`j/k`** Navigate between tasks (down/up)
- **`J/K`** Navigate between projects (down/up)
- **Mouse** Click on sidebar items to navigate

## Task Management

- **`x`** Mark or unmark a task for bulk actions
- **`Esc`** Clear all marked tasks
- **`Space`** or **`Enter`** Toggle completion for marked tasks, or the current task when none are marked
- **`a`** Create new task
- **`d`** Delete selected task (with confirmation)
- **`p`** Cycle task priority
- **`u`** Remove the due date from marked tasks, or the current task when none are marked
- **`t`** Set marked tasks due today
- **`T`** Set marked tasks due tomorrow
- **`w`** Set marked tasks due next week (Monday)
- **`W`** Set marked tasks due next week end (Saturday)

When no tasks are marked, due-date and completion shortcuts operate on the
currently highlighted task.

## Project Management

- **`A`** Create new project
- **`D`** Delete selected project (with confirmation)

## System

- **`b`** Toggle sidebar visibility
- **`/`** Open task search dialog (search across all tasks)
- **`r`** Force sync with Todoist
- **`D`** Empty Trash while the Trash view is selected (with confirmation)
- **`i`** Cycle through icon themes
- **`?`** Toggle help panel
- **`q`** Quit the application
- **`Esc`** Cancel action or close dialogs
- **`Ctrl+C`** Quit application

## Task Search

- **`/`** Open search dialog
- **Type** Search across all tasks by content
- **`↓`** Move focus from the query to search results
- **`j/k`** or **`↑/↓`** Navigate focused search results
- **`Space`** Complete or reopen the focused result
- **`t`** Set the focused result's due date to today
- **`Enter`** No action
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
- **Shortcut Bar**: Common controls shown along the bottom (configurable with `shortcut_bar_visible`)

## Agenda

- **`s`** Set the selected task's Todoist due time (for example, `2pm` or `14:30`)
- Timed tasks use their saved Todoist time. Untimed Today tasks receive dimmed, local-only one-hour suggestions beginning at the next whole hour.

### Task Display Features
Tasks are displayed with:
- **Trash**: Appears only while locally restorable deleted tasks exist; deleted tasks expire after 30 days
- **Restore**: Press `d` on a task in Trash to recreate it in Todoist
- **Status Icons**: ☐ (pending), ☒ (completed), ✗ (deleted)
- **Priority Badges**: [P0] (urgent), [P1] (high), [P2] (medium), [P3] (low), no badge (normal)
- **Label Badges**: Colored badges showing task labels
- **Task Content**: Truncated to fit the display width
- **Completion Visual**: Completed tasks appear dimmed
- **Interactive**: Press Space or Enter to toggle completion
