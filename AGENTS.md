# Terminalist workspace

- Preserve the Todoist/local-cache distinction: remote deletion happens before local tombstoning.
- Run `cargo fmt --all -- --check` and `cargo test` after Rust changes.

## File map

- `src/sync/`: Todoist synchronization and local cache reconciliation.
- `src/repositories/`: SQLite queries.
- `src/ui/`: actions, navigation, dialogs, and TUI components.
- `tests/`: integration and UI behavior tests.
- `docs/KEYBOARD_SHORTCUTS.md`: user-facing controls and view behavior.
