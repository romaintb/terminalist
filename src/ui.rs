use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use tokio::time::Duration;

use crate::sync::{SyncService, SyncStats, SyncStatus};
use crate::terminal_badge::{create_terminal_priority_badge, create_terminal_task_badges};
use crate::todoist::{ProjectDisplay, TaskDisplay};

/// Application state
pub struct App {
    pub should_quit: bool,
    pub projects: Vec<ProjectDisplay>,
    pub tasks: Vec<TaskDisplay>,
    pub selected_project_index: usize,
    pub selected_task_index: usize,
    pub project_list_state: ListState,
    pub task_list_state: ListState,
    pub loading: bool,
    pub syncing: bool,
    pub completing_task: bool,
    pub deleting_task: bool,
    pub delete_confirmation: Option<String>, // Task ID to delete if confirmed
    pub error_message: Option<String>,
    pub sync_stats: Option<SyncStats>,
    pub last_sync_status: SyncStatus,
}

impl App {
    pub fn new() -> Self {
        let mut project_list_state = ListState::default();
        project_list_state.select(Some(0));

        let mut task_list_state = ListState::default();
        task_list_state.select(Some(0));

        Self {
            should_quit: false,
            projects: Vec::new(),
            tasks: Vec::new(),
            selected_project_index: 0,
            selected_task_index: 0,
            project_list_state,
            task_list_state,
            loading: true,
            syncing: false,
            completing_task: false,
            deleting_task: false,
            delete_confirmation: None,
            error_message: None,
            sync_stats: None,
            last_sync_status: SyncStatus::Idle,
        }
    }

    pub async fn load_local_data(&mut self, sync_service: &SyncService) {
        self.loading = true;
        self.error_message = None;

        // Remember the current project selection
        let current_project_id = self
            .projects
            .get(self.selected_project_index)
            .map(|p| p.id.clone());

        // Load projects from local storage (fast)
        match sync_service.get_projects().await {
            Ok(projects) => {
                self.projects = projects;
                if !self.projects.is_empty() {
                    // Try to maintain the current project selection
                    if let Some(current_id) = current_project_id {
                        // Find the previously selected project in the new list
                        if let Some(index) = self.projects.iter().position(|p| p.id == current_id) {
                            self.selected_project_index = index;
                            self.project_list_state.select(Some(index));
                        } else {
                            // If the project no longer exists, default to first project
                            self.selected_project_index = 0;
                            self.project_list_state.select(Some(0));
                        }
                    } else {
                        // No previous selection, default to first project
                        self.selected_project_index = 0;
                        self.project_list_state.select(Some(0));
                    }

                    // Load tasks for the selected project
                    if let Err(e) = self.load_tasks_for_selected_project(sync_service).await {
                        self.error_message = Some(format!("Error loading tasks: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Error loading projects: {}", e));
            }
        }

        // Update sync stats
        if let Ok(stats) = sync_service.get_sync_stats().await {
            self.sync_stats = Some(stats);
        }

        self.loading = false;
    }

    pub async fn load_tasks_for_selected_project(&mut self, sync_service: &SyncService) -> Result<()> {
        if let Some(project) = self.projects.get(self.selected_project_index) {
            let tasks = sync_service.get_tasks_for_project(&project.id).await?;
            self.tasks = tasks;
            self.selected_task_index = 0;
            self.task_list_state.select(Some(0));
        }
        Ok(())
    }

    pub async fn force_clear_and_sync(&mut self, sync_service: &SyncService) {
        // Note: syncing state should already be set by caller
        self.error_message = None;

        // First clear all local data
        match sync_service.clear_local_data().await {
            Ok(()) => {
                // Then perform a fresh sync
                match sync_service.force_sync().await {
                    Ok(status) => {
                        self.last_sync_status = status;
                        // Reload local data after sync
                        self.load_local_data(sync_service).await;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Sync failed after clearing: {}", e));
                        self.last_sync_status = SyncStatus::Error { message: e.to_string() };
                    }
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to clear database: {}", e));
            }
        }

        self.syncing = false;
    }

    pub async fn delete_selected_task(&mut self, sync_service: &SyncService) {
        if let Some(task) = self.tasks.get(self.selected_task_index) {
            self.deleting_task = true;

            match sync_service.delete_task(&task.id).await {
                Ok(()) => {
                    // Mark the task as deleted in local UI immediately
                    if let Some(task_mut) = self.tasks.get_mut(self.selected_task_index) {
                        task_mut.is_deleted = true;
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to delete task: {}", e));
                }
            }

            self.deleting_task = false;
            self.delete_confirmation = None;
        }
    }

    pub async fn toggle_selected_task(&mut self, sync_service: &SyncService) {
        if let Some(task) = self.tasks.get(self.selected_task_index) {
            self.completing_task = true;

            if task.is_completed {
                // Reopen the task
                match sync_service.reopen_task(&task.id).await {
                    Ok(()) => {
                        // Update local task status immediately for responsive UI
                        if let Some(task_mut) = self.tasks.get_mut(self.selected_task_index) {
                            task_mut.is_completed = false;
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to reopen task: {}", e));
                    }
                }
            } else {
                // Complete the task
                match sync_service.complete_task(&task.id).await {
                    Ok(()) => {
                        // Update local task status immediately for responsive UI
                        if let Some(task_mut) = self.tasks.get_mut(self.selected_task_index) {
                            task_mut.is_completed = true;
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to complete task: {}", e));
                    }
                }
            }

            self.completing_task = false;
        }
    }

    pub fn next_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project_index = (self.selected_project_index + 1) % self.projects.len();
            self.project_list_state
                .select(Some(self.selected_project_index));
        }
    }

    pub fn previous_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project_index = if self.selected_project_index == 0 {
                self.projects.len() - 1
            } else {
                self.selected_project_index - 1
            };
            self.project_list_state
                .select(Some(self.selected_project_index));
        }
    }

    pub fn next_task(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_task_index = (self.selected_task_index + 1) % self.tasks.len();
            self.task_list_state.select(Some(self.selected_task_index));
        }
    }

    pub fn previous_task(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_task_index = if self.selected_task_index == 0 {
                self.tasks.len() - 1
            } else {
                self.selected_task_index - 1
            };
            self.task_list_state.select(Some(self.selected_task_index));
        }
    }
}

pub async fn run_app() -> Result<()> {
    // Get API token from environment variable
    let api_token = std::env::var("TODOIST_API_TOKEN").expect("Please set TODOIST_API_TOKEN environment variable");

    // Create the sync service
    let sync_service = SyncService::new(api_token).await?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    let res = run_ui(&mut terminal, &mut app, &sync_service).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

async fn run_ui<B: Backend>(terminal: &mut Terminal<B>, app: &mut App, sync_service: &SyncService) -> Result<()> {
    // Check if we have local data, if not perform initial sync
    if !sync_service.has_local_data().await? {
        app.syncing = true;
        app.force_clear_and_sync(sync_service).await;
    } else {
        // Load local data first (fast)
        app.load_local_data(sync_service).await;

        // Then sync in background if needed
        if sync_service.should_sync().await? {
            tokio::spawn({
                let sync_service = sync_service.clone();
                async move {
                    let _ = sync_service.sync_if_needed().await;
                }
            });
        }
    }

    loop {
        terminal.draw(|f| ui(f, app))?;

        // Handle events with a timeout to allow for async operations
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Handle delete confirmation dialog
                    if app.delete_confirmation.is_some() {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                // Confirm delete
                                app.delete_selected_task(sync_service).await;
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                // Cancel delete
                                app.delete_confirmation = None;
                            }
                            _ => {} // Ignore other keys during confirmation
                        }
                        continue; // Don't process other keys during confirmation
                    }

                    match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.previous_task();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next_task();
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            app.previous_project();
                            if let Err(e) = app.load_tasks_for_selected_project(sync_service).await {
                                app.error_message = Some(format!("Error loading tasks: {}", e));
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            app.next_project();
                            if let Err(e) = app.load_tasks_for_selected_project(sync_service).await {
                                app.error_message = Some(format!("Error loading tasks: {}", e));
                            }
                        }
                        KeyCode::Char('r') => {
                            // Set syncing state immediately so UI shows indicator
                            app.syncing = true;
                            // Force a redraw to show the syncing indicator
                            terminal.draw(|f| ui(f, app))?;
                            // Now perform the sync
                            app.force_clear_and_sync(sync_service).await;
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            app.toggle_selected_task(sync_service).await;
                        }
                        KeyCode::Char('d') => {
                            // Trigger delete confirmation
                            if let Some(task) = app.tasks.get(app.selected_task_index) {
                                app.delete_confirmation = Some(task.id.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    // Calculate sidebar width: 30% of screen but max 25 characters
    let screen_width = f.size().width;
    let sidebar_width = std::cmp::min(screen_width * 30 / 100, 25);
    let main_width = screen_width.saturating_sub(sidebar_width);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(sidebar_width),
                Constraint::Length(main_width),
            ]
            .as_ref(),
        )
        .split(f.size());

    // Left sidebar - Projects
    let max_name_width = sidebar_width.saturating_sub(4); // Account for icon, space, and borders
    let project_items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, project)| {
            let icon = if project.is_favorite { "⭐" } else { "📁" };
            let style = if i == app.selected_project_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Truncate project name to fit sidebar
            let display_name = if project.name.len() > max_name_width as usize {
                format!("{}…", &project.name[..max_name_width.saturating_sub(1) as usize])
            } else {
                project.name.clone()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", icon), style),
                Span::styled(display_name, style),
            ]))
        })
        .collect();

    let projects_list = List::new(project_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📁 Projects")
                .title_alignment(Alignment::Center),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("→ ");

    f.render_stateful_widget(projects_list, chunks[0], &mut app.project_list_state.clone());

    // Right pane - Tasks
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(chunks[1]);

    if app.loading {
        let loading_text = Paragraph::new("Loading...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📝 Tasks")
                    .title_alignment(Alignment::Center),
            )
            .alignment(Alignment::Center);
        f.render_widget(loading_text, right_chunks[0]);
    } else {
        let task_items: Vec<ListItem> = app
            .tasks
            .iter()
            .enumerate()
            .map(|(i, task)| {
                let status_icon = if task.is_deleted {
                    "❌" // Red X mark for deleted tasks
                } else if task.is_completed {
                    "✅" // Check mark for completed tasks
                } else {
                    "🔳" // Empty box for pending tasks
                };
                // Base style for icons and badges (no strikethrough)
                let base_style = if i == app.selected_task_index {
                    if task.is_completed {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    }
                } else if task.is_completed {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(Color::White)
                };

                // Content style (with strikethrough for deleted tasks)
                let content_style = if task.is_deleted {
                    // Deleted tasks: red with strikethrough for content only
                    if i == app.selected_task_index {
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::CROSSED_OUT | Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::CROSSED_OUT | Modifier::DIM)
                    }
                } else {
                    base_style
                };

                // Create badges using the terminal_badge module
                let mut badge_spans = Vec::new();

                // Add priority badge first if not normal priority
                if let Some(priority_badge) = create_terminal_priority_badge(task.priority) {
                    badge_spans.push(priority_badge);
                }

                // Add other task badges
                badge_spans.extend(create_terminal_task_badges(
                    task.is_recurring,
                    task.deadline.is_some(),
                    task.duration.as_deref(),
                    task.labels.len(),
                ));

                // Calculate available width for content based on actual task box width
                let task_box_width = main_width.saturating_sub(4); // Account for borders and padding
                let icon_width = 3; // Status icon + space
                let badge_length: usize = badge_spans.iter().map(|span| span.content.len()).sum();
                let badge_spacing = if badge_spans.is_empty() { 0 } else { 1 }; // Space before badges

                let max_content_len = (task_box_width as usize)
                    .saturating_sub(icon_width)
                    .saturating_sub(badge_length)
                    .saturating_sub(badge_spacing);

                let content = if task.content.len() > max_content_len && max_content_len > 3 {
                    format!("{}...", &task.content[..max_content_len.saturating_sub(3)])
                } else {
                    task.content.clone()
                };

                // Build the complete line with badges
                let mut line_spans = vec![Span::styled(format!("{} ", status_icon), base_style)];

                // Add badges if any exist (using base style, no strikethrough)
                if !badge_spans.is_empty() {
                    line_spans.extend(badge_spans);
                    line_spans.push(Span::raw(" "));
                }

                // Add content with appropriate style (strikethrough for deleted)
                line_spans.push(Span::styled(content, content_style));

                ListItem::new(Line::from(line_spans))
            })
            .collect();

        let tasks_title = if let Some(project) = app.projects.get(app.selected_project_index) {
            format!("📝 Tasks - {}", project.name)
        } else {
            "📝 Tasks".to_string()
        };

        let tasks_list = List::new(task_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(tasks_title)
                    .title_alignment(Alignment::Center),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("→ ");

        f.render_stateful_widget(tasks_list, right_chunks[0], &mut app.task_list_state.clone());
    }

    // Status bar
    let status_text = if app.loading {
        "Loading local data...".to_string()
    } else if app.syncing {
        "🔄 Syncing with Todoist...".to_string()
    } else if app.completing_task {
        "🔄 Toggling task status...".to_string()
    } else if app.deleting_task {
        "🔄 Deleting task...".to_string()
    } else {
        let sync_info = match &app.sync_stats {
            Some(stats) => format!(" | {} | {}", stats.data_summary(), stats.status_description()),
            None => String::new(),
        };
        format!(
            "← → h l Projects | ↑ ↓ k j Tasks | Space/Enter Toggle | 'd' Delete | 'r' Sync | 'q' Quit{}",
            sync_info
        )
    };

    let status_color = if app.syncing || app.completing_task {
        Color::Yellow
    } else if app.error_message.is_some() {
        Color::Red
    } else {
        Color::Gray
    };

    let status_bar = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(status_color));

    f.render_widget(status_bar, right_chunks[1]);

    // Error message overlay
    if let Some(error_msg) = &app.error_message {
        let error_area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, error_area);
        let error_paragraph = Paragraph::new(error_msg.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Error")
                    .title_alignment(Alignment::Center),
            )
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(error_paragraph, error_area);
    }

    // Delete confirmation dialog
    if let Some(_task_id) = &app.delete_confirmation {
        if let Some(task) = app.tasks.get(app.selected_task_index) {
            let confirm_area = centered_rect(60, 25, f.size());
            f.render_widget(Clear, confirm_area);

            let task_preview = if task.content.len() > 40 {
                format!("{}...", &task.content[..37])
            } else {
                task.content.clone()
            };

            let confirm_text = format!(
                "Delete task?\n\n\"{}\"\n\nThis action cannot be undone!\n\nPress 'y' to confirm or 'n'/Esc to cancel",
                task_preview
            );

            let confirm_paragraph = Paragraph::new(confirm_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("⚠️  Confirm Delete")
                        .title_alignment(Alignment::Center),
                )
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(confirm_paragraph, confirm_area);
        }
    }
}

/// Helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
