# Current UI Work Baseline

Status: Ready for review
Branch: `codex/ui-navigation-improvements`
Base: `main` at `81c680e`

## Purpose

This branch checkpoints the current user-facing work before the architecture refactors in
[Architecture Refactor Plan](ARCHITECTURE_REFACTOR_PLAN.md). It keeps the behavior changes
together because they share the same action routing, task loading, sidebar, and task-list
components.

## Included Work

### Navigation and Pane Behavior

- Size the navigation pane from its longest item and count, within sensible minimum and
  maximum widths.
- Allow the divider between navigation and tasks to be resized.
- Show task counts at the right edge of navigation rows.
- Highlight the selected navigation row.
- Use left and right arrows to move focus between navigation and tasks.
- Highlight the border and title of the focused pane.

### Task Visibility and Counts

- Count active tasks consistently for Today, Tomorrow, Upcoming, projects, and labels.
- Refresh the selected row from the number of tasks actually shown in the current view.
- Parse Todoist date-only and datetime values consistently.
- Keep matching subtasks visible when their parent is outside the filtered result.
- Show parent context for a visible subtask whose parent is outside the current view.

### Bulk Task Commands

- Mark and unmark tasks with `x`.
- Apply completion/restoration and due-date commands to all marked tasks.
- Support unscheduling and the Today, Tomorrow, Next Week, and Weekend date shortcuts.
- Clear a task's Todoist due date and persist the returned remote date state locally.

### Processing Feedback

- Treat foreground mutations as blocking work while they run.
- Ignore additional commands during blocking work so operations cannot overlap
  unintentionally.
- Show the active operation in the task-pane title.
- Keep search and background synchronization non-blocking.
- Route operation descriptions through the shared task manager rather than implementing
  feedback for individual commands.

### Documentation and Tests

- Update configuration, keyboard shortcut, and README documentation.
- Add coverage for shortcuts, bulk actions, subtask rendering, navigation count refreshes,
  date parsing, and foreground-operation state.

## Commit Recommendation

Use two commits:

1. `feat: improve task navigation and bulk workflows`
   - All functional source changes, dependency updates, user documentation, and regression
     tests.
2. `docs: plan follow-up architecture refactors`
   - This baseline record, the architecture refactor plan, and their documentation index
     entries.

A finer split is not recommended for the existing worktree. The behaviors were developed
together and overlap in `AppComponent`, shared actions, task loading, the sidebar, and the
task list. Hunk-level separation would create intermediate commits that are harder to build,
test, and review than the complete behavior.

## Pull Request Recommendation

Open one draft pull request from `codex/ui-navigation-improvements` into `main`.

The pull request should:

- describe this as the behavioral baseline for later refactoring;
- call out the new `reqwest` path used to clear Todoist due dates;
- include the exact validation results;
- keep the architectural changes in the refactor plan out of this branch;
- remain draft until the UI behavior has been manually exercised against a real Todoist
  account.

After this pull request is accepted as the baseline, implement the refactor plan as separate,
stacked branches and pull requests. Do not add those refactors to this branch.

## Validation

Required automated checks:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo build`

Recommended manual checks:

1. Resize the navigation divider and restart to verify the intended width behavior.
2. Navigate between panes and lists with arrow keys and confirm focus and selection styling.
3. Compare sidebar counts with visible Today, Tomorrow, Upcoming, project, and label views.
4. Verify a matching subtask is visible with parent context when its parent is filtered out.
5. Mark several tasks and run complete, restore, schedule, and unschedule commands.
6. Confirm foreground commands show processing feedback and ignore additional input until
   completion.
7. Sync with Todoist and confirm that unscheduling clears the remote due date.

## Known Follow-Up Work

The current implementation intentionally establishes behavior before improving its internal
structure. The architecture refactor plan tracks:

- typed operations instead of delimiter-separated strings;
- versioned view snapshots and stale-result rejection;
- preservation of the last valid local cache on failures;
- grouped navigation-count queries;
- dependency-aware component updates;
- one shared Todoist transport, including due-date clearing.
