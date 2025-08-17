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
    DeleteConfirmationDialog, ErrorDialog, HelpPanel, ProjectCreationDialog, ProjectDeleteConfirmationDialog,
    Sidebar, StatusBar, TaskCreationDialog, TasksList,
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

    // Load initial data
    app.load_local_data(&sync_service).await;

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

    // Render overlays
    if app.error_message.is_some() {
        ErrorDialog::render(f, app);
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

    if app.delete_project_confirmation.is_some() {
        ProjectDeleteConfirmationDialog::render(f, app);
    }

    // Render help panel last to ensure it's on top of everything
    if app.show_help {
        HelpPanel::render(f, app);
    }
}
