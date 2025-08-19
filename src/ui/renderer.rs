//! Main UI rendering and coordination

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::time::Duration;

use super::app::App;
use super::components::{
    dialogs::{
        DeleteConfirmationDialog, ErrorDialog, InfoDialog, ProjectCreationDialog, ProjectDeleteConfirmationDialog,
        ProjectEditDialog, SyncingDialog, TaskCreationDialog, TaskEditDialog,
    },
    HelpPanel, Sidebar, StatusBar, TasksList,
};
use super::events::handle_events;
use super::layout::LayoutManager;
use crate::sync::SyncService;

/// Run the main TUI application
pub async fn run_app() -> Result<()> {
    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create application state
    let mut app = App::new();
    let api_token = std::env::var("TODOIST_API_TOKEN").expect("TODOIST_API_TOKEN environment variable must be set");
    let sync_service = SyncService::new(api_token).await?;

    // Load initial local data so we can render the UI immediately
    app.load_local_data(&sync_service).await;

    // If local DB is empty, start an initial sync in the background
    match sync_service.has_local_data().await {
        Ok(false) => {
            app.syncing = true;
            let svc = sync_service.clone();
            app.sync_task = Some(tokio::spawn(async move { svc.force_sync().await }));
        }
        Ok(true) => {}
        Err(e) => {
            app.error_message = Some(format!("Failed to check local data: {e}"));
        }
    }

    // Main application loop
    let res = run_ui(&mut terminal, &mut app, &sync_service).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    res
}

/// Main UI loop
async fn run_ui(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    sync_service: &SyncService,
) -> Result<()> {
    loop {
        terminal.draw(|f| render_ui(f, app))?;

        // Handle events with a timeout to allow for async operations
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        // Handle the event
                        let _handled = handle_events(Event::Key(key), app, sync_service).await?;
                    }
                }
                Event::Resize(_, _) => {
                    // Handle terminal resize events
                }
                _ => {
                    // Handle other event types if needed
                }
            }
        }

        // If a background sync task finished, process its result and reload data
        if let Some(handle_ref) = app.sync_task.as_ref() {
            if handle_ref.is_finished() {
                if let Some(handle) = app.sync_task.take() {
                    match handle.await {
                        Ok(Ok(status)) => {
                            app.last_sync_status = status;
                            app.load_local_data(sync_service).await;
                        }
                        Ok(Err(e)) => {
                            app.error_message = Some(format!("Sync failed: {e}"));
                        }
                        Err(join_err) => {
                            app.error_message = Some(format!("Sync task error: {join_err}"));
                        }
                    }
                    app.syncing = false;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Main UI rendering function
fn render_ui(f: &mut ratatui::Frame, app: &mut App) {
    // Calculate layouts
    let chunks = LayoutManager::main_layout(f.area());
    let top_chunks = LayoutManager::top_pane_layout(chunks[0]);

    // Render components
    Sidebar::render(f, top_chunks[0], app);
    TasksList::render(f, top_chunks[1], app);
    StatusBar::render(f, chunks[1], app);
    // Render syncing dialog if loading or syncing
    if app.loading || app.syncing {
        SyncingDialog::render(f, app);
    }

    // Render overlays - error messages have priority over info messages
    if app.error_message.is_some() {
        ErrorDialog::render(f, app);
    } else if app.info_message.is_some() {
        InfoDialog::render(f, app);
    }

    if app.delete_confirmation.is_some() {
        DeleteConfirmationDialog::render(f, app);
    }

    if app.creating_project {
        ProjectCreationDialog::render(f, app);
    }

    if app.creating_task {
        TaskCreationDialog::render(f, app);
    }

    if app.editing_task {
        TaskEditDialog::render(f, app);
    }

    if app.editing_project {
        ProjectEditDialog::render(f, app);
    }

    if app.delete_project_confirmation.is_some() {
        ProjectDeleteConfirmationDialog::render(f, app);
    }

    // Render help panel last to ensure it's on top of everything
    if app.show_help {
        HelpPanel::render(f, app);
    }
}
