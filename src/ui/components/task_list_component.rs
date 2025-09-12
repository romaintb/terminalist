use crate::icons::IconService;
use crate::todoist::{LabelDisplay, ProjectDisplay, SectionDisplay, TaskDisplay};
use crate::ui::components::badge::{create_priority_badge, create_task_badges};
use crate::ui::core::SidebarSelection;
use crate::ui::core::{
    actions::{Action, DialogType},
    Component,
};
use chrono;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::collections::HashMap;

pub struct TaskListComponent {
    pub tasks: Vec<TaskDisplay>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub sidebar_selection: SidebarSelection,
    pub sections: Vec<SectionDisplay>,
    pub projects: Vec<ProjectDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub icons: IconService,
}

impl Default for TaskListComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskListComponent {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            selected_index: 0,
            list_state: ListState::default(),
            sidebar_selection: SidebarSelection::Today,
            sections: Vec::new(),
            projects: Vec::new(),
            labels: Vec::new(),
            icons: IconService::default(),
        }
    }

    pub fn update_data(
        &mut self,
        tasks: Vec<TaskDisplay>,
        sections: Vec<SectionDisplay>,
        projects: Vec<ProjectDisplay>,
        labels: Vec<LabelDisplay>,
        sidebar_selection: SidebarSelection,
    ) {
        self.tasks = tasks;
        self.sections = sections;
        self.projects = projects;
        self.labels = labels;
        self.sidebar_selection = sidebar_selection;
        self.update_list_state();
    }

    fn update_list_state(&mut self) {
        if self.tasks.is_empty() {
            self.selected_index = 0;
            self.list_state.select(None);
        } else {
            if self.selected_index >= self.tasks.len() {
                self.selected_index = self.tasks.len().saturating_sub(1);
            }

            // Calculate the rendered index accounting for section headers and separators
            let rendered_index = self.calculate_rendered_index();
            self.list_state.select(Some(rendered_index));
        }
    }

    /// Calculate the actual index in the rendered list for the selected task
    fn calculate_rendered_index(&self) -> usize {
        // This mirrors the logic in create_task_list_items to count items before the selected task
        match &self.sidebar_selection {
            SidebarSelection::Today => self.calculate_today_rendered_index(),
            SidebarSelection::Tomorrow => self.selected_index, // Tomorrow has simple 1:1 mapping
            SidebarSelection::Project(project_index) => {
                if let Some(project) = self.projects.get(*project_index) {
                    self.calculate_project_rendered_index(&project.id)
                } else {
                    self.selected_index // Fallback
                }
            }
            _ => self.selected_index, // Other views have simple 1:1 mapping
        }
    }

    fn create_task_list_items(&self, area: Rect) -> Vec<ListItem<'_>> {
        if self.tasks.is_empty() {
            return Vec::new();
        }

        // Handle Today view specially
        if matches!(self.sidebar_selection, SidebarSelection::Today) {
            return self.create_today_task_items(area);
        }

        // Handle Tomorrow view specially
        if matches!(self.sidebar_selection, SidebarSelection::Tomorrow) {
            return self.create_tomorrow_task_items(area);
        }

        // Handle different selection types
        match &self.sidebar_selection {
            SidebarSelection::Project(index) => {
                if let Some(project) = self.projects.get(*index) {
                    self.create_project_task_items(&project.id, area)
                } else {
                    self.create_simple_task_items(area)
                }
            }
            SidebarSelection::Label(index) => {
                if let Some(label) = self.labels.get(*index) {
                    self.create_label_task_items(&label.id, area)
                } else {
                    self.create_simple_task_items(area)
                }
            }
            _ => self.create_simple_task_items(area),
        }
    }

    fn create_today_task_items(&self, _area: Rect) -> Vec<ListItem<'_>> {
        let mut items = Vec::new();

        if self.tasks.is_empty() {
            return items;
        }

        // Since the database query already filters for today's and overdue tasks,
        // we just need to separate them for display purposes
        let now = chrono::Utc::now().date_naive();

        // Separate tasks into overdue and today
        let mut overdue_tasks = Vec::new();
        let mut today_tasks = Vec::new();

        for task in &self.tasks {
            if let Some(due_date_str) = &task.due {
                if let Ok(due_date) = chrono::NaiveDate::parse_from_str(due_date_str, "%Y-%m-%d") {
                    if due_date < now {
                        overdue_tasks.push(task);
                    } else if due_date == now {
                        today_tasks.push(task);
                    }
                }
            }
        }

        // Add overdue section if there are overdue tasks
        if !overdue_tasks.is_empty() {
            items.push(self.create_section_header("⏰ Overdue"));

            for task in &overdue_tasks {
                let item = self.create_task_item(task, 0);
                items.push(item);
            }

            // Add separator between sections if we have both
            if !today_tasks.is_empty() {
                items.push(ListItem::new(Line::from(vec![Span::styled("", Style::default())])));
            }
        }

        // Add today section if there are today tasks
        if !today_tasks.is_empty() {
            items.push(self.create_section_header("📅 Today"));

            for task in &today_tasks {
                let item = self.create_task_item(task, 0);
                items.push(item);
            }
        }

        // If no tasks match the date filtering, show all tasks (fallback for debugging)
        if overdue_tasks.is_empty() && today_tasks.is_empty() && !self.tasks.is_empty() {
            items.push(self.create_section_header("📋 All Tasks (Debug)"));

            for task in &self.tasks {
                let item = self.create_task_item(task, 0);
                items.push(item);
            }
        }

        items
    }

    fn create_tomorrow_task_items(&self, _area: Rect) -> Vec<ListItem<'_>> {
        let mut items = Vec::new();

        if self.tasks.is_empty() {
            return items;
        }

        // Add tomorrow section header
        items.push(self.create_section_header("📅 Tomorrow"));

        // Add all tasks (they're already filtered for tomorrow by the database query)
        for task in self.tasks.iter() {
            let item = self.create_task_item(task, 0);
            items.push(item);
        }

        items
    }

    fn create_project_task_items(&self, project_id: &str, area: Rect) -> Vec<ListItem<'_>> {
        let mut items = Vec::new();
        let area_width = area.width.saturating_sub(4);

        // Get sections for the current project
        let project_sections: Vec<_> = self
            .sections
            .iter()
            .filter(|section| section.project_id == *project_id)
            .collect();

        // Group tasks by section
        let mut tasks_by_section: HashMap<Option<String>, Vec<&TaskDisplay>> = HashMap::new();
        for task in &self.tasks {
            tasks_by_section
                .entry(task.section_id.clone())
                .or_default()
                .push(task);
        }

        // Add tasks without sections first
        if let Some(tasks_without_section) = tasks_by_section.get(&None) {
            for task in tasks_without_section {
                let item = self.create_task_item(task, area_width as usize);
                items.push(item);
            }
        }

        // Add sections with their tasks
        for (section_index, section) in project_sections.iter().enumerate() {
            if let Some(section_tasks) = tasks_by_section.get(&Some(section.id.clone())) {
                // Add blank line before section (except for first section with no tasks before)
                if !items.is_empty() || section_index > 0 {
                    items.push(ListItem::new(Line::from("")));
                }

                // Add section name
                items.push(ListItem::new(Line::from(Span::styled(
                    section.name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ))));

                // Add separator line
                items.push(ListItem::new(Line::from(Span::styled(
                    "─".repeat(area_width as usize),
                    Style::default().fg(Color::Gray),
                ))));

                // Add tasks in this section
                for task in section_tasks {
                    let item = self.create_task_item(task, area_width as usize);
                    items.push(item);
                }
            }
        }

        items
    }

    fn create_label_task_items(&self, label_id: &str, area: Rect) -> Vec<ListItem<'_>> {
        let mut items = Vec::new();
        let area_width = area.width.saturating_sub(4);

        // Filter tasks that have the specific label
        for task in self.tasks.iter() {
            if task.labels.iter().any(|label| label.id == label_id) {
                let item = self.create_task_item(task, area_width as usize);
                items.push(item);
            }
        }

        items
    }

    fn create_simple_task_items(&self, area: Rect) -> Vec<ListItem<'_>> {
        let mut items = Vec::new();
        let area_width = area.width.saturating_sub(4);

        for task in self.tasks.iter() {
            let item = self.create_task_item(task, area_width as usize);
            items.push(item);
        }

        items
    }

    fn format_due_date(&self, due_date: &str) -> String {
        if let Ok(task_date) = chrono::NaiveDate::parse_from_str(due_date, "%Y-%m-%d") {
            let today = chrono::Utc::now().date_naive();
            let tomorrow = today.succ_opt().unwrap_or(today);

            if task_date == today {
                "today".to_string()
            } else if task_date == tomorrow {
                "tomorrow".to_string()
            } else {
                due_date.to_string()
            }
        } else {
            due_date.to_string()
        }
    }

    fn create_task_item(&self, task: &TaskDisplay, _max_width: usize) -> ListItem<'_> {
        // Create status indicator
        let status_icon = if task.is_deleted {
            self.icons.task_deleted()
        } else if task.is_completed {
            self.icons.task_completed()
        } else {
            self.icons.task_pending()
        };

        // Build the line with multiple spans for proper color rendering
        let mut line_spans = Vec::new();

        // Status icon
        let status_style = if task.is_deleted {
            Style::default().fg(Color::Red)
        } else if task.is_completed {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };
        line_spans.push(Span::styled(format!("{} ", status_icon), status_style));

        // Priority badge (if any)
        if let Some(priority_badge) = create_priority_badge(task.priority) {
            line_spans.push(priority_badge);
            line_spans.push(Span::raw(" "));
        }

        // Task content with appropriate styling
        let content_style = if task.is_deleted {
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::CROSSED_OUT)
        } else if task.is_completed {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(Color::White)
        };
        line_spans.push(Span::styled(task.content.clone(), content_style));

        // Project display
        if let Some(project) = self.projects.iter().find(|p| p.id == task.project_id) {
            line_spans.push(Span::raw(" "));
            line_spans.push(Span::styled(
                format!("#{}", project.name),
                Style::default().fg(Color::Cyan),
            ));
        }

        // Due date display (after project)
        if let Some(due_date) = &task.due {
            line_spans.push(Span::raw(" "));
            line_spans.push(Span::styled(
                self.format_due_date(due_date),
                Style::default().fg(Color::Rgb(255, 165, 0)), // Orange color
            ));
        }

        // Metadata badges
        let metadata_badges = create_task_badges(
            task.is_recurring,
            task.due.is_some() || task.deadline.is_some(),
            task.duration.as_deref(),
            task.labels.as_slice(),
        );

        for badge in metadata_badges {
            line_spans.push(Span::raw(" "));
            line_spans.push(badge);
        }

        // Create the ListItem - selection highlighting handled by stateful widget
        ListItem::new(Line::from(line_spans))
    }

    pub fn get_selected_task(&self) -> Option<&TaskDisplay> {
        self.tasks.get(self.selected_index)
    }

    /// Create a section header item
    fn create_section_header(&self, name: &str) -> ListItem<'static> {
        ListItem::new(Line::from(Span::styled(
            name.to_string(),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        )))
    }
}

impl Component for TaskListComponent {
    fn handle_key_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => Action::PreviousTask,
            KeyCode::Down | KeyCode::Char('j') => Action::NextTask,
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    // Note: Detailed logging done when action is processed by AppComponent
                    Action::ToggleTask(task.id.clone())
                } else {
                    Action::None
                }
            }
            KeyCode::Char('d') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    // Note: Detailed logging done when action is processed by AppComponent
                    Action::ShowDialog(DialogType::DeleteConfirmation {
                        item_type: "task".to_string(),
                        item_id: task.id.clone(),
                    })
                } else {
                    Action::None
                }
            }
            KeyCode::Char('p') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    // Note: Detailed logging done when action is processed by AppComponent
                    Action::CyclePriority(task.id.clone())
                } else {
                    Action::None
                }
            }
            KeyCode::Char('e') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    // Note: Detailed logging done when action is processed by AppComponent
                    Action::ShowDialog(DialogType::TaskEdit {
                        task_id: task.id.clone(),
                        content: task.content.clone(),
                        project_id: task.project_id.clone(),
                    })
                } else {
                    Action::None
                }
            }
            KeyCode::Char('a') => {
                // Determine current project for task creation
                let default_project_id = match &self.sidebar_selection {
                    SidebarSelection::Project(index) => {
                        // User is viewing a specific project, create task in that project
                        self.projects.get(*index).map(|p| p.id.clone())
                    }
                    _ => {
                        // User is in Today/Tomorrow/Label view, create task in inbox (no project)
                        None
                    }
                };

                // Note: Detailed logging done when action is processed by AppComponent
                Action::ShowDialog(DialogType::TaskCreation { default_project_id })
            }
            KeyCode::Char('t') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    Action::SetTaskDueToday(task.id.clone())
                } else {
                    Action::None
                }
            }
            KeyCode::Char('T') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    Action::SetTaskDueTomorrow(task.id.clone())
                } else {
                    Action::None
                }
            }
            KeyCode::Char('w') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    Action::SetTaskDueNextWeek(task.id.clone())
                } else {
                    Action::None
                }
            }
            KeyCode::Char('W') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    Action::SetTaskDueWeekEnd(task.id.clone())
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }

    fn update(&mut self, action: Action) -> Action {
        match action {
            Action::NextTask => {
                if !self.tasks.is_empty() {
                    let _old_index = self.selected_index;
                    self.selected_index = (self.selected_index + 1) % self.tasks.len();
                    self.update_list_state();
                    // Note: Logging done via app-level logger when action is processed
                }
                Action::None
            }
            Action::PreviousTask => {
                if !self.tasks.is_empty() {
                    let _old_index = self.selected_index;
                    self.selected_index = if self.selected_index == 0 {
                        self.tasks.len() - 1
                    } else {
                        self.selected_index - 1
                    };
                    self.update_list_state();
                    // Note: Logging done via app-level logger when action is processed
                }
                Action::None
            }
            _ => action,
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        if self.tasks.is_empty() {
            // Show empty state message
            let empty_message = if self.projects.is_empty() {
                "No projects available. Press 'r' to sync or 'A' to create a project."
            } else if matches!(self.sidebar_selection, SidebarSelection::Today) {
                "No tasks due today. Press 'a' to create a task or 'r' to sync."
            } else if matches!(self.sidebar_selection, SidebarSelection::Tomorrow) {
                "No tasks due tomorrow. Press 'a' to create a task or 'r' to sync."
            } else {
                "No tasks in this project. Press 'a' to create a task."
            };

            let empty_list = List::new(vec![ListItem::new(empty_message)])
                .block(Block::default().borders(Borders::ALL).title("Tasks"));

            f.render_stateful_widget(empty_list, rect, &mut self.list_state);
        } else {
            // Create list items with sections and tasks
            let items = self.create_task_list_items(rect);
            let mut list_state = self.list_state.clone();

            let tasks_list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Tasks"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(tasks_list, rect, &mut list_state);
            self.list_state = list_state;
        }
    }
}

impl TaskListComponent {
    /// Calculate rendered index for Today view (with overdue/today sections)
    fn calculate_today_rendered_index(&self) -> usize {
        if self.tasks.is_empty() {
            return 0;
        }

        let now = chrono::Utc::now().date_naive();
        let mut overdue_tasks = Vec::new();
        let mut today_tasks = Vec::new();

        for task in &self.tasks {
            if let Some(due_str) = &task.due {
                if let Ok(due_date) = chrono::NaiveDate::parse_from_str(due_str, "%Y-%m-%d") {
                    if due_date < now {
                        overdue_tasks.push(task);
                    } else if due_date == now {
                        today_tasks.push(task);
                    }
                }
            }
        }

        let mut rendered_index = 0;
        let mut task_index = 0;

        // Count overdue section
        if !overdue_tasks.is_empty() {
            rendered_index += 1; // Section header
            for _ in &overdue_tasks {
                if task_index == self.selected_index {
                    return rendered_index;
                }
                rendered_index += 1;
                task_index += 1;
            }

            // Add separator if both sections exist
            if !today_tasks.is_empty() {
                rendered_index += 1;
            }
        }

        // Count today section
        if !today_tasks.is_empty() {
            rendered_index += 1; // Section header
            for _ in &today_tasks {
                if task_index == self.selected_index {
                    return rendered_index;
                }
                rendered_index += 1;
                task_index += 1;
            }
        }

        rendered_index
    }

    /// Calculate rendered index for Project view (with sections)
    fn calculate_project_rendered_index(&self, project_id: &str) -> usize {
        use std::collections::HashMap;

        if self.tasks.is_empty() {
            return 0;
        }

        // Get sections for the current project
        let project_sections: Vec<_> = self
            .sections
            .iter()
            .filter(|section| section.project_id == *project_id)
            .collect();

        // Group tasks by section
        let mut tasks_by_section: HashMap<Option<String>, Vec<&TaskDisplay>> = HashMap::new();
        for task in &self.tasks {
            tasks_by_section
                .entry(task.section_id.clone())
                .or_default()
                .push(task);
        }

        let mut rendered_index = 0;
        let mut task_index = 0;

        // Tasks without sections first
        if let Some(tasks_without_section) = tasks_by_section.get(&None) {
            for _ in tasks_without_section {
                if task_index == self.selected_index {
                    return rendered_index;
                }
                rendered_index += 1;
                task_index += 1;
            }
        }

        // Sections with their tasks
        for (section_index, section) in project_sections.iter().enumerate() {
            if let Some(section_tasks) = tasks_by_section.get(&Some(section.id.clone())) {
                // Add blank line before section (except for first section with no tasks before)
                if rendered_index > 0 || section_index > 0 {
                    rendered_index += 1;
                }

                // Add section name
                rendered_index += 1;

                // Add separator line
                rendered_index += 1;

                // Add tasks in this section
                for _ in section_tasks {
                    if task_index == self.selected_index {
                        return rendered_index;
                    }
                    rendered_index += 1;
                    task_index += 1;
                }
            }
        }

        rendered_index
    }
}
