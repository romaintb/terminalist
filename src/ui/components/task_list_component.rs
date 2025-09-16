use crate::config::DisplayConfig;
use crate::icons::IconService;
use crate::todoist::{LabelDisplay, ProjectDisplay, SectionDisplay, TaskDisplay};
use crate::ui::components::task_list_item_component::{ListItem, TaskItem, TaskListItemType};
use crate::ui::core::SidebarSelection;
use crate::ui::core::{
    actions::{Action, DialogType},
    Component,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem as RatatuiListItem, ListState},
    Frame,
};

pub struct TaskListComponent {
    pub items: Vec<TaskListItemType>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub sidebar_selection: SidebarSelection,
    pub sections: Vec<SectionDisplay>,
    pub projects: Vec<ProjectDisplay>,
    pub labels: Vec<LabelDisplay>,
    pub icons: IconService,
    // Keep raw task data for building items
    pub tasks: Vec<TaskDisplay>,
    pub display_config: DisplayConfig,
}

impl Default for TaskListComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskListComponent {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            tasks: Vec::new(),
            selected_index: 0,
            list_state: ListState::default(),
            sidebar_selection: SidebarSelection::Today,
            sections: Vec::new(),
            projects: Vec::new(),
            labels: Vec::new(),
            icons: IconService::default(),
            display_config: DisplayConfig::default(),
        }
    }

    pub fn update_display_config(&mut self, display_config: DisplayConfig) {
        self.display_config = display_config;
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

        // Build the flat list of items from the hierarchical task data
        self.build_item_list();
        self.update_list_state();
    }

    /// Build the flat list of items from task data
    fn build_item_list(&mut self) {
        self.items.clear();

        if self.tasks.is_empty() {
            return;
        }

        // Handle different sidebar selections with appropriate sectioning
        match &self.sidebar_selection {
            SidebarSelection::Today => self.build_today_items(),
            SidebarSelection::Tomorrow => self.build_tomorrow_items(),
            SidebarSelection::Upcoming => self.build_upcoming_items(),
            SidebarSelection::Project(index) => {
                if let Some(project) = self.projects.get(*index) {
                    let project_id = project.id.clone();
                    self.build_project_items(&project_id);
                } else {
                    self.build_simple_items();
                }
            }
            SidebarSelection::Label(index) => {
                if let Some(label) = self.labels.get(*index) {
                    let label_id = label.id.clone();
                    self.build_label_items(&label_id);
                } else {
                    self.build_simple_items();
                }
            }
        }
    }

    /// Build items for Today view (with Overdue and Today sections)
    fn build_today_items(&mut self) {
        use crate::ui::components::task_list_item_component::{HeaderItem, SeparatorItem};

        let now = chrono::Utc::now().date_naive();
        let mut overdue_tasks = Vec::new();
        let mut today_tasks = Vec::new();

        // Separate tasks by date
        for task in &self.tasks {
            if let Some(due_date_str) = &task.due {
                if let Ok(due_date) = chrono::NaiveDate::parse_from_str(due_date_str, "%Y-%m-%d") {
                    if due_date < now {
                        overdue_tasks.push(task.clone());
                    } else if due_date == now {
                        today_tasks.push(task.clone());
                    }
                }
            }
        }

        // Add overdue section if there are overdue tasks
        if !overdue_tasks.is_empty() {
            self.items
                .push(TaskListItemType::Header(HeaderItem::new("⏰ Overdue".to_string(), 0)));

            for task in overdue_tasks {
                self.add_task_and_children_to_items(task, 0);
            }

            // Add separator between sections if we have both
            if !today_tasks.is_empty() {
                self.items.push(TaskListItemType::Separator(SeparatorItem::new(0)));
            }
        }

        // Add today section if there are today tasks
        if !today_tasks.is_empty() {
            self.items
                .push(TaskListItemType::Header(HeaderItem::new("📅 Today".to_string(), 0)));

            for task in today_tasks {
                self.add_task_and_children_to_items(task, 0);
            }
        }
    }

    /// Build items for Tomorrow view
    fn build_tomorrow_items(&mut self) {
        use crate::ui::components::task_list_item_component::HeaderItem;

        self.items
            .push(TaskListItemType::Header(HeaderItem::new("📅 Tomorrow".to_string(), 0)));

        // Sort tasks by priority
        let mut tasks = self.tasks.clone();
        tasks.sort_by(|a, b| a.priority.cmp(&b.priority));

        for task in tasks {
            self.add_task_and_children_to_items(task, 0);
        }
    }

    /// Build items for Upcoming view (with date sections)
    fn build_upcoming_items(&mut self) {
        use crate::ui::components::task_list_item_component::{HeaderItem, SeparatorItem};
        use std::collections::BTreeMap;

        let today = chrono::Utc::now().date_naive();
        let mut overdue_tasks = Vec::new();
        let mut future_tasks_by_date: BTreeMap<chrono::NaiveDate, Vec<TaskDisplay>> = BTreeMap::new();

        // Group tasks by date
        for task in &self.tasks {
            if let Some(due_date_str) = &task.due {
                if let Ok(due_date) = chrono::NaiveDate::parse_from_str(due_date_str, "%Y-%m-%d") {
                    if due_date < today {
                        overdue_tasks.push(task.clone());
                    } else {
                        future_tasks_by_date.entry(due_date).or_default().push(task.clone());
                    }
                }
            }
        }

        // Add overdue section first
        if !overdue_tasks.is_empty() {
            self.items
                .push(TaskListItemType::Header(HeaderItem::new("⏰ Overdue".to_string(), 0)));

            for task in overdue_tasks {
                self.add_task_and_children_to_items(task, 0);
            }
        }

        // Add future date sections
        for (due_date, tasks) in future_tasks_by_date {
            // Add separator before each new section
            if !self.items.is_empty() {
                self.items.push(TaskListItemType::Separator(SeparatorItem::new(0)));
            }

            // Format the date header
            let date_header = if due_date == today {
                "📅 Today".to_string()
            } else if due_date == today + chrono::Duration::days(1) {
                "📅 Tomorrow".to_string()
            } else {
                let weekday = due_date.format("%A").to_string();
                let formatted_date = due_date.format("%b %d").to_string();
                format!("📊 {} - {}", weekday, formatted_date)
            };

            self.items.push(TaskListItemType::Header(HeaderItem::new(date_header, 0)));

            for task in tasks {
                self.add_task_and_children_to_items(task, 0);
            }
        }
    }

    /// Build items for Project view (with section headers)
    fn build_project_items(&mut self, project_id: &str) {
        use crate::ui::components::task_list_item_component::{HeaderItem, SeparatorItem};
        use std::collections::HashMap;

        // Get sections for the current project
        let project_sections: Vec<_> = self
            .sections
            .iter()
            .filter(|section| section.project_id == *project_id)
            .cloned()
            .collect();

        // Group tasks by section
        let mut tasks_by_section: HashMap<Option<String>, Vec<TaskDisplay>> = HashMap::new();
        for task in &self.tasks {
            if task.project_id == *project_id {
                tasks_by_section.entry(task.section_id.clone()).or_default().push(task.clone());
            }
        }

        // Add tasks without sections first
        if let Some(tasks_without_section) = tasks_by_section.get(&None) {
            for task in tasks_without_section {
                self.add_task_and_children_to_items(task.clone(), 0);
            }
        }

        // Add sections with their tasks
        for section in project_sections {
            if let Some(section_tasks) = tasks_by_section.get(&Some(section.id.clone())) {
                // Add separator before section
                if !self.items.is_empty() {
                    self.items.push(TaskListItemType::Separator(SeparatorItem::new(0)));
                }

                // Add section header
                self.items
                    .push(TaskListItemType::Header(HeaderItem::new(section.name.clone(), 0)));

                for task in section_tasks {
                    self.add_task_and_children_to_items(task.clone(), 0);
                }
            }
        }
    }

    /// Build items for Label view
    fn build_label_items(&mut self, label_id: &str) {
        // Filter tasks that have the specific label
        let filtered_tasks: Vec<TaskDisplay> = self
            .tasks
            .iter()
            .filter(|task| task.labels.iter().any(|label| label.id == *label_id))
            .cloned()
            .collect();

        for task in filtered_tasks {
            self.add_task_and_children_to_items(task, 0);
        }
    }

    /// Build simple items (no sectioning)
    fn build_simple_items(&mut self) {
        let mut root_tasks: Vec<TaskDisplay> = self.tasks.iter().filter(|t| t.parent_id.is_none()).cloned().collect();

        // Sort by priority
        root_tasks.sort_by(|a, b| a.priority.cmp(&b.priority));

        // Add each root task and its children recursively
        for task in root_tasks {
            self.add_task_and_children_to_items(task, 0);
        }
    }

    /// Recursively add a task and its children to the items list
    fn add_task_and_children_to_items(&mut self, task: TaskDisplay, depth: usize) {
        // Calculate child count
        let child_count = self.get_child_task_count(&task.id);

        // Create and add the task item
        let task_item = TaskItem::new(
            task.clone(),
            depth,
            child_count,
            self.icons.clone(),
            self.projects.clone(),
        );
        self.items.push(TaskListItemType::Task(Box::new(task_item)));

        // Find and add children
        let task_id = task.id.clone();
        let mut children: Vec<TaskDisplay> = self
            .tasks
            .iter()
            .filter(|t| t.parent_id.as_ref() == Some(&task_id))
            .cloned()
            .collect();

        children.sort_by(|a, b| a.priority.cmp(&b.priority));

        // Recursively add each child and their descendants
        for child in children {
            self.add_task_and_children_to_items(child, depth + 1);
        }
    }

    fn update_list_state(&mut self) {
        // Count only selectable items
        let selectable_count = self.items.iter().filter(|item| item.is_selectable()).count();

        if selectable_count == 0 {
            self.selected_index = 0;
            self.list_state.select(None);
        } else {
            if self.selected_index >= selectable_count {
                self.selected_index = selectable_count.saturating_sub(1);
            }

            // Map logical selection to physical list index
            let physical_index = self.logical_to_physical_index(self.selected_index);
            self.list_state.select(physical_index);
        }
    }

    /// Convert logical selection index (among selectable items) to physical list index
    fn logical_to_physical_index(&self, logical_index: usize) -> Option<usize> {
        let mut selectable_count = 0;
        for (i, item) in self.items.iter().enumerate() {
            if item.is_selectable() {
                if selectable_count == logical_index {
                    return Some(i);
                }
                selectable_count += 1;
            }
        }
        None
    }

    pub fn get_selected_task(&self) -> Option<&TaskDisplay> {
        // Find the currently selected task item
        if let Some(physical_index) = self.logical_to_physical_index(self.selected_index) {
            if let Some(TaskListItemType::Task(task_item)) = self.items.get(physical_index) {
                return Some(&task_item.task);
            }
        }
        None
    }

    /// Get child task count for a parent task
    fn get_child_task_count(&self, parent_id: &str) -> usize {
        self.tasks.iter().filter(|t| t.parent_id.as_deref() == Some(parent_id)).count()
    }

    /// Create the list items for rendering
    fn create_list_items(&self, _rect: Rect) -> Vec<RatatuiListItem<'static>> {
        self.items
            .iter()
            .map(|item| {
                item.render(false, &self.display_config) // Selection styling handled by List widget
            })
            .collect()
    }

    /// Navigate to the next selectable item
    fn next_task(&mut self) {
        let selectable_count = self.items.iter().filter(|item| item.is_selectable()).count();
        if selectable_count > 0 {
            self.selected_index = (self.selected_index + 1) % selectable_count;
            self.update_list_state();
        }
    }

    /// Navigate to the previous selectable item
    fn previous_task(&mut self) {
        let selectable_count = self.items.iter().filter(|item| item.is_selectable()).count();
        if selectable_count > 0 {
            self.selected_index = if self.selected_index == 0 {
                selectable_count - 1
            } else {
                self.selected_index - 1
            };
            self.update_list_state();
        }
    }
}

impl Component for TaskListComponent {
    fn handle_key_events(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_task();
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_task();
                Action::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(task) = self.get_selected_task() {
                    Action::CompleteTask(task.id.clone())
                } else {
                    Action::None
                }
            }
            KeyCode::Char('a') => {
                // When viewing a specific project, preselect it as the default project
                let default_project_id = match &self.sidebar_selection {
                    SidebarSelection::Project(index) => self.projects.get(*index).map(|p| p.id.clone()),
                    _ => None,
                };
                Action::ShowDialog(DialogType::TaskCreation { default_project_id })
            }
            KeyCode::Char('e') => {
                if let Some(task) = self.get_selected_task() {
                    Action::ShowDialog(DialogType::TaskEdit {
                        task_id: task.id.clone(),
                        content: task.content.clone(),
                        project_id: task.project_id.clone(),
                    })
                } else {
                    Action::None
                }
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                if let Some(task) = self.get_selected_task() {
                    Action::ShowDialog(DialogType::DeleteConfirmation {
                        item_type: "task".to_string(),
                        item_id: task.id.clone(),
                    })
                } else {
                    Action::None
                }
            }
            KeyCode::Char('p') => {
                if let Some(task) = self.get_selected_task() {
                    Action::CyclePriority(task.id.clone())
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
                self.next_task();
                Action::None
            }
            Action::PreviousTask => {
                self.previous_task();
                Action::None
            }
            _ => action,
        }
    }

    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let tasks_list = if self.items.is_empty() {
            // Show contextual empty state message
            let empty_message = match &self.sidebar_selection {
                SidebarSelection::Today => "No tasks due today. Press 'a' to create a task or 'r' to sync.",
                SidebarSelection::Tomorrow => "No tasks due tomorrow. Press 'a' to create a task or 'r' to sync.",
                _ if self.projects.is_empty() => "No projects available. Press 'r' to sync or 'A' to create a project.",
                _ => "No tasks in this view. Press 'a' to create a task.",
            };

            List::new(vec![RatatuiListItem::new(empty_message)])
        } else {
            List::new(self.create_list_items(rect))
                .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        }
        .block(Block::default().borders(Borders::ALL).title("Tasks"));

        f.render_stateful_widget(tasks_list, rect, &mut self.list_state);
    }
}
