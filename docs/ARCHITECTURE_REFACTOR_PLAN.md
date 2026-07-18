# Architecture Refactor Plan

Status: In progress
Scope: UI operations, background loading, local storage, navigation counts, and Todoist transport

## Implementation Status

The correctness-focused work has been implemented and validated in a stacked branch series:

1. `codex/ui-navigation-improvements` establishes the user-visible baseline.
2. `codex/preserve-local-cache` preserves SQLite data across startup and synchronization
   failures and writes remote snapshots transactionally.
3. `codex/typed-operations` replaces delimiter-encoded commands with typed operations.
4. `codex/versioned-view-snapshots` adds stable UUID selections, load generations, stale-result
   rejection, and last-good-view retention.
5. `codex/test-full-stack` integrates those branches with `codex/search-input-footer` for
   end-to-end manual testing.

The integrated branch passes `cargo fmt --check`, strict Clippy, the full test suite, and
`cargo build`. It is a validation branch, not a replacement for the focused review history.

Phase 1 is mostly complete. Query-error propagation remains incomplete because task and count
queries can still convert errors to empty collections with `unwrap_or_default()`. Phase 2 has
not been implemented.

## Why This Work Is Needed

Recent UI work exposed several places where behavior was implemented for one command or view instead of at the shared architectural layer. The resulting duplication makes correctness depend on each new feature remembering to implement loading state, error handling, count refreshes, and data conversion consistently.

This plan consolidates those responsibilities while preserving current user-facing behavior.

## Goals

- Keep operation arguments typed from the UI action through backend execution.
- Prevent stale background results from replacing the active view.
- Preserve the last valid local snapshot when loading or synchronization fails.
- Use one canonical snapshot for task rows and navigation counts.
- Make component rebuilding proportional to actual data changes.
- Keep all Todoist HTTP behavior behind one transport abstraction.

## Non-Goals

- Adding another task backend.
- Redesigning the visible interface.
- Replacing SeaORM, Ratatui, or Tokio.
- Introducing speculative caching beyond the existing local database.

## Current Problems

### 1. Stringly Typed Operations — Completed

Typed actions are converted into delimiter-separated strings before execution. Task content or project names containing delimiters such as `|` can be parsed as operation metadata.

Target design:

- Introduce typed task, project, and label operation values.
- Pass typed values or typed closures directly to the background task manager.
- Remove the central string parser from `AppComponent`.

### 2. Silent Empty-State Failures — Partially Completed

Several data queries use `unwrap_or_default()`. A failed query therefore becomes a successful empty view and can reset navigation counts to zero.

Target design:

- Propagate query errors as explicit load failures.
- Retain and continue rendering the last valid snapshot.
- Show a non-destructive error state without replacing good data.

Current state:

- Top-level project, label, and section load failures are explicit and retain the last
  accepted view.
- Selected-view task loads, the all-task load, and per-label count loads still use
  `unwrap_or_default()` and must be changed to propagate errors.

### 3. Unversioned Navigation Loads — Completed

Every navigation change starts a background load, but results do not identify the request that produced them. An older request can arrive after a newer one.

Target design:

- Give each load a monotonically increasing generation ID.
- Include the requested stable selection identifier in every result.
- Apply a result only if it still matches the active generation and selection.
- Replace project and label vector indices in `SidebarSelection` with UUIDs.

### 4. Destructive Startup Cache Handling — Completed

Normal startup deletes the SQLite database before confirming that synchronization can succeed.

Target design:

- Open and render the existing database first.
- Refresh it transactionally.
- Preserve the previous snapshot when authentication, networking, or synchronization fails.
- If full replacement remains necessary, build a temporary database and swap it only after successful validation.

### 5. N+1 Navigation Count Queries — Not Started

Navigation loading performs one query per label in addition to the main task queries.

Target design:

- Add a repository query that groups active task counts by label in one pass.
- Derive project and date-view counts from the same canonical task snapshot or grouped query.
- Define count semantics once: visible actionable tasks, including matching subtasks.

### 6. Excessive Component Rebuilding — Not Started

Every input event clones the complete data model and rebuilds component item trees. The sidebar also rebuilds during rendering.

Target design:

- Update components only when their source data or selection changes.
- Keep cursor movement and focus changes local to the affected component.
- Build task hierarchy indices once per snapshot.
- Make rendering read-only with respect to derived item collections.

### 7. Split Todoist Transport — Not Started

Due-date clearing currently bypasses the Todoist wrapper with a separate HTTP client and hardcoded endpoint.

Target design:

- Extend, patch, or wrap the Todoist client so nullable due fields use the same transport.
- Keep authentication, base URL, timeout, serialization, and error mapping in one place.
- Preserve mock-server support for transport-level tests.

## Delivery Plan

### Branch and Pull Request Strategy

Use a stable baseline branch followed by stacked, independently reviewable branches:

1. `codex/ui-navigation-improvements`
   - Checkpoint the current user-facing UI, task-count, subtask, date-handling, focus, and processing-state work.
2. `codex/preserve-local-cache`
   - Add transactional startup refresh and offline/failure tests.
3. `codex/typed-operations`
   - Remove delimiter-based operation parsing and add regression tests.
4. `codex/versioned-view-snapshots`
   - Add UUID selections, request generations, stale-result rejection, and explicit load errors.
5. `codex/navigation-query-performance`
   - Add grouped counts, hierarchy indexing, and dependency-aware component updates.
6. `codex/unified-todoist-transport`
   - Remove the second HTTP client and hardcoded endpoint.

Branches 2–4 have been implemented as stacked pull requests on the fork. Branches 5–6 remain
planned. Rebase each branch onto `main` after its predecessor merges. Keep each pull request
focused on one architectural boundary and include its acceptance criteria in the PR
description.

#### Current Worktree Checkpoint

The current worktree checkpoint is documented in
[Current UI Work Baseline](CURRENT_UI_WORK_BASELINE.md). Before architectural refactoring begins:

1. Create `codex/ui-navigation-improvements` at the current `main` commit without discarding the worktree.
2. Review the complete diff and confirm that it belongs to the behavioral baseline.
3. Keep the intertwined functional changes and tests in one buildable commit, followed by a
   planning-documentation commit.
4. Run the full validation suite after the final commit.
5. Push the branch and open a draft pull request describing the user-visible behavior and known follow-up refactors.
6. Merge or otherwise establish this branch as the baseline before starting Phase 1.

Do not mix the architectural phases into the current UI pull request. The baseline PR should remain reviewable as a behavior change, while later PRs can focus on internal structure with smaller user-visible diffs.

### Phase 1: Correctness and State Ownership

1. [x] Add typed operation enums and remove delimiter parsing.
2. [x] Replace index-based sidebar selections with stable UUIDs.
3. [ ] Partially complete: introduce a versioned `ViewSnapshot` containing:
   - generation ID;
   - requested selection;
   - projects, labels, sections, and tasks;
   - navigation counts;
   - load status or error.

   Snapshot data is grouped and versioned. Load errors currently use a separate typed action
   rather than being stored in `ViewSnapshot`.
4. [x] Reject stale load results.
5. [x] Preserve the last good snapshot on load failure.
6. [x] Replace destructive startup deletion with transactional cache refresh.
7. [ ] Propagate selected-view, all-task, and count-query failures instead of converting them
   to empty results.

Phase 1 acceptance criteria:

- Content containing `|` or `: ` works in task and project operations.
- Rapid navigation cannot display tasks from a previously selected view.
- A forced database or network failure leaves the last valid data visible.
- Startup without network access does not delete usable cached data.

### Phase 2: Performance and Transport Consolidation

1. [ ] Add grouped repository queries for navigation counts.
2. [ ] Remove per-label count queries.
3. [ ] Add task hierarchy indices keyed by task and parent UUID.
4. [ ] Stop rebuilding components after unrelated input events.
5. [ ] Move nullable due-date updates into the shared Todoist transport.
6. [ ] Remove the extra HTTP client and hardcoded Todoist endpoint.

Phase 2 acceptance criteria:

- Data loading uses a bounded number of database queries regardless of label count.
- Cursor navigation does not clone task/project/label collections.
- Render methods do not rebuild item trees.
- All Todoist task updates use one configurable transport.

## Validation

Each phase should include:

- Unit tests for typed operations and delimiter-containing content.
- Deterministic tests for out-of-order load completion.
- Failure tests proving that last-good snapshots are retained.
- Startup tests with an existing cache and unavailable backend.
- Query-count or integration tests for grouped navigation counts.
- Rendering tests for stable counts, subtasks, focus, and processing state.
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

## Sequencing and Rollback

- Keep phases in separate commits or pull requests.
- Land typed operations before changing load-state ownership.
- Land versioned snapshots before optimizing component updates.
- Keep schema changes backward compatible until transactional startup behavior is verified.
- Do not remove the old Todoist update path until nullable-field tests pass against a mock server.

## Risks

- Changing selection identity affects navigation, dialogs, configuration, and tests.
- Snapshot versioning can expose previously hidden assumptions about action ordering.
- Cache preservation requires clear conflict rules between local mutations and remote refreshes.
- Transport changes can affect authentication and error behavior even when request payloads are unchanged.
