//! Event handling and key bindings

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crate::sync::SyncService;
use super::app::App;

/// Handle all user input events
pub async fn handle_events(
    event: Event,
    app: &mut App,
    sync_service: &SyncService,
) -> Result<bool, anyhow::Error> {
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press {
            // Handle delete confirmation dialog
            if app.delete_confirmation.is_some() {
                return handle_delete_confirmation(key.code, app, sync_service).await;
            }

            // Handle help panel - block all other shortcuts when help is open
            if app.show_help {
                return handle_help_panel(key.code, app);
            }

            // Handle normal navigation and actions
            return handle_normal_mode(key.code, app, sync_service).await;
        }
    }
    Ok(false)
}

/// Handle events when delete confirmation dialog is open
async fn handle_delete_confirmation(
    key_code: KeyCode,
    app: &mut App,
    sync_service: &SyncService,
) -> Result<bool, anyhow::Error> {
    match key_code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            // Confirm delete
            app.delete_selected_task(sync_service).await;
            Ok(true)
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            // Cancel delete
            app.delete_confirmation = None;
            Ok(true)
        }
        _ => Ok(false), // Ignore other keys during confirmation
    }
}

/// Handle events when help panel is open
fn handle_help_panel(key_code: KeyCode, app: &mut App) -> Result<bool, anyhow::Error> {
    match key_code {
        KeyCode::Char('?') | KeyCode::Esc => {
            app.show_help = false;
            app.help_scroll_offset = 0; // Reset scroll when closing
            Ok(true)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            // Scroll up in help panel
            if app.help_scroll_offset > 0 {
                app.help_scroll_offset = app.help_scroll_offset.saturating_sub(1);
            }
            Ok(true)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            // Scroll down in help panel
            app.help_scroll_offset = app.help_scroll_offset.saturating_add(1);
            Ok(true)
        }
        KeyCode::PageUp => {
            // Page up in help panel
            app.help_scroll_offset = app.help_scroll_offset.saturating_sub(10);
            Ok(true)
        }
        KeyCode::PageDown => {
            // Page down in help panel
            app.help_scroll_offset = app.help_scroll_offset.saturating_add(10);
            Ok(true)
        }
        KeyCode::Home => {
            // Go to top of help panel
            app.help_scroll_offset = 0;
            Ok(true)
        }
        KeyCode::End => {
            // Go to bottom of help panel (will be calculated in UI)
            app.help_scroll_offset = usize::MAX; // Will be clamped in UI
            Ok(true)
        }
        _ => Ok(false), // Ignore all other keys when help is open
    }
}

/// Handle events in normal mode
async fn handle_normal_mode(
    key_code: KeyCode,
    app: &mut App,
    sync_service: &SyncService,
) -> Result<bool, anyhow::Error> {
    match key_code {
        KeyCode::Char('q') => {
            app.should_quit = true;
            Ok(true)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.previous_task();
            Ok(true)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_task();
            Ok(true)
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.previous_project();
            if let Err(e) = app.load_tasks_for_selected_project(sync_service).await {
                app.error_message = Some(format!("Error loading tasks: {}", e));
            }
            Ok(true)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.next_project();
            if let Err(e) = app.load_tasks_for_selected_project(sync_service).await {
                app.error_message = Some(format!("Error loading tasks: {}", e));
            }
            Ok(true)
        }
        KeyCode::Char('r') => {
            // Set syncing state immediately so UI shows indicator
            app.syncing = true;
            // Now perform the sync
            app.force_clear_and_sync(sync_service).await;
            Ok(true)
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.toggle_selected_task(sync_service).await;
            Ok(true)
        }
        KeyCode::Char('d') => {
            // Trigger delete confirmation only if task is not already deleted
            if let Some(task) = app.tasks.get(app.selected_task_index) {
                if !task.is_deleted {
                    app.delete_confirmation = Some(task.id.clone());
                }
                // If task is already deleted, do nothing (silently ignore)
            }
            Ok(true)
        }
        KeyCode::Char('?') => {
            app.show_help = true;
            Ok(true)
        }
        _ => Ok(false),
    }
}
