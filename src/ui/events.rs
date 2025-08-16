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
            // Handle project deletion confirmation dialog
            if app.delete_project_confirmation.is_some() {
                return handle_project_delete_confirmation(key, app, sync_service).await;
            }

            // Handle project creation dialog
            if app.creating_project {
                return handle_project_creation(key, app, sync_service).await;
            }

            // Handle delete confirmation dialog
            if app.delete_confirmation.is_some() {
                return handle_delete_confirmation(key, app, sync_service).await;
            }

            // Handle help panel - block all other shortcuts when help is open
            if app.show_help {
                return Ok(handle_help_panel(key, app));
            }

            // Handle normal navigation and actions
            return handle_normal_mode(key, app, sync_service).await;
        }
    }
    Ok(false)
}

/// Handle events when delete confirmation dialog is open
async fn handle_delete_confirmation(
    key: crossterm::event::KeyEvent,
    app: &mut App,
    sync_service: &SyncService,
) -> Result<bool, anyhow::Error> {
    match key.code {
        KeyCode::Char('y' | 'Y') => {
            // Confirm delete
            app.delete_selected_task(sync_service).await;
            Ok(true)
        }
        KeyCode::Char('n' | 'N') | KeyCode::Esc => {
            // Cancel delete
            app.delete_confirmation = None;
            Ok(true)
        }
        _ => Ok(false), // Ignore other keys during confirmation
    }
}

/// Handle events when project deletion confirmation dialog is open
async fn handle_project_delete_confirmation(
    key: crossterm::event::KeyEvent,
    app: &mut App,
    sync_service: &SyncService,
) -> Result<bool, anyhow::Error> {
    match key.code {
        KeyCode::Char('y' | 'Y') => {
            // Confirm delete
            app.delete_project(sync_service).await;
            Ok(true)
        }
        KeyCode::Char('n' | 'N') | KeyCode::Esc => {
            // Cancel delete
            app.cancel_delete_project();
            Ok(true)
        }
        _ => Ok(false), // Ignore other keys during confirmation
    }
}

/// Handle events when project creation dialog is open
async fn handle_project_creation(
    key: crossterm::event::KeyEvent,
    app: &mut App,
    sync_service: &SyncService,
) -> Result<bool, anyhow::Error> {
    match key.code {
        KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
            // Add character to project name
            app.add_char_to_project_name(c);
            Ok(true)
        }
        KeyCode::Backspace => {
            // Remove last character from project name
            app.remove_char_from_project_name();
            Ok(true)
        }
        KeyCode::Char('p' | 'P') => {
            // Cycle through parent project options
            app.cycle_parent_project();
            Ok(true)
        }
        KeyCode::Enter => {
            // Create the project
            app.create_project(sync_service).await;
            Ok(true)
        }
        KeyCode::Esc => {
            // Cancel project creation
            app.cancel_create_project();
            Ok(true)
        }
        _ => Ok(false), // Ignore other keys during creation
    }
}

/// Handle events when help panel is open
fn handle_help_panel(key: crossterm::event::KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc => {
            app.show_help = false;
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            // Scroll up in help panel
            app.help_scroll_offset = app.help_scroll_offset.saturating_sub(1);
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            // Scroll down in help panel
            app.help_scroll_offset = app.help_scroll_offset.saturating_add(1);
            true
        }
        KeyCode::PageUp => {
            // Page up in help panel
            app.help_scroll_offset = app.help_scroll_offset.saturating_sub(10);
            true
        }
        KeyCode::PageDown => {
            // Page down in help panel
            app.help_scroll_offset = app.help_scroll_offset.saturating_add(10);
            true
        }
        KeyCode::Home => {
            // Go to top of help panel
            app.help_scroll_offset = 0;
            true
        }
        KeyCode::End => {
            // Go to bottom of help panel (will be clamped in UI)
            app.help_scroll_offset = usize::MAX; // Will be clamped in UI
            true
        }
        _ => false, // Ignore all other keys when help is open
    }
}

/// Handle events in normal mode
async fn handle_normal_mode(
    key: crossterm::event::KeyEvent,
    app: &mut App,
    sync_service: &SyncService,
) -> Result<bool, anyhow::Error> {
    // Check for Ctrl+C first
    if key.code == KeyCode::Char('c') && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
        app.should_quit = true;
        return Ok(true);
    }
    
    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
            Ok(true)
        }
        KeyCode::Char('c') => {
            // Normal 'c' key (not Ctrl+C)
            Ok(false)
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
                app.error_message = Some(format!("Error loading tasks: {e}"));
            }
            Ok(true)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.next_project();
            if let Err(e) = app.load_tasks_for_selected_project(sync_service).await {
                app.error_message = Some(format!("Error loading tasks: {e}"));
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
        KeyCode::Char('N') => {
            // Create new project
            app.start_create_project();
            Ok(true)
        }
        KeyCode::Char('D') => {
            // Delete selected project (capital D to distinguish from task deletion)
            app.start_delete_project();
            Ok(true)
        }
        _ => Ok(false),
    }
}
