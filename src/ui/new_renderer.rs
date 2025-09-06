use crate::sync::SyncService;
use crate::ui::app_component::AppComponent;
use crate::ui::core::{Component, EventHandler, EventType};
use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;
use tokio::time::{interval, Duration};

/// Enhanced async event loop with proper background task support
pub async fn run_new_app(sync_service: SyncService) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize application components
    let mut app = AppComponent::new(sync_service);
    let mut event_handler = EventHandler::new();

    // Start initial sync automatically
    app.trigger_initial_sync();

    // Create intervals for periodic tasks
    let mut cleanup_interval = interval(Duration::from_secs(5)); // Clean up finished tasks every 5 seconds
    let mut render_interval = interval(Duration::from_millis(16)); // ~60 FPS rendering
    let result = run_app_loop(
        &mut terminal,
        &mut app,
        &mut event_handler,
        &mut cleanup_interval,
        &mut render_interval,
    )
    .await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn run_app_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppComponent,
    event_handler: &mut EventHandler,
    _cleanup_interval: &mut tokio::time::Interval,
    _render_interval: &mut tokio::time::Interval,
) -> anyhow::Result<()> {
    let mut needs_render = true;

    loop {
        // Render when needed
        if needs_render {
            terminal.draw(|f| app.render(f, f.area()))?;
            needs_render = false;
        }

        // Simplified event loop to avoid deadlocks
        let event_result = event_handler.next_event().await?;

        match event_result {
            EventType::Key(_) | EventType::Resize(_, _) => {
                app.handle_event(event_result).await?;
                needs_render = true;
            }
            EventType::Tick => {
                // Process background actions on tick (less frequent)
                let background_actions = app.process_background_actions();

                for action in background_actions {
                    // Process action through component hierarchy first
                    let processed_action = app.update(action);

                    // Then handle app-level actions with async support
                    match app.handle_app_action(processed_action).await {
                        crate::ui::core::actions::Action::Quit => {
                            return Ok(());
                        }
                        _ => {
                            needs_render = true;
                        }
                    }
                }
                // Don't render on every tick - only when there are actual background actions
            }
            EventType::Render => {
                needs_render = true;
            }
            EventType::Other => {
                // Handle other event types if needed
            }
        }

        // Check if app wants to quit
        if app.should_quit() {
            break;
        }
    }

    Ok(())
}

/// Status information about the application
#[derive(Debug, Clone)]
pub struct AppStatus {
    pub active_tasks: usize,
    pub is_syncing: bool,
    pub last_sync: Option<std::time::SystemTime>,
    pub total_tasks: usize,
    pub total_projects: usize,
}

impl AppComponent {
    /// Get current application status for monitoring
    pub fn get_status(&self) -> AppStatus {
        AppStatus {
            active_tasks: self.active_task_count(),
            is_syncing: self.is_syncing(),
            last_sync: None, // TODO: Track last sync time
            total_tasks: self.total_tasks(),
            total_projects: self.total_projects(),
        }
    }

    /// Force a render on the next loop iteration
    pub fn request_render(&mut self) {
        // This could set a flag that the render loop checks
        // For now, the render loop handles this automatically
    }
}
