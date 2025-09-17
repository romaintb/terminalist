use crate::config::DisplayConfig;
use crate::icons::IconService;
use crate::todoist::{ProjectDisplay, TaskDisplay};
use crate::ui::components::badge::{create_priority_badge, create_task_badges};
use crate::utils::datetime::{format_human_date, format_human_datetime};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::ListItem as RatatuiListItem,
};

/// Trait for items that can be displayed in a task list
pub trait ListItem {
    /// Render this item as a ratatui ListItem
    fn render(&self, selected: bool, display_config: &DisplayConfig) -> RatatuiListItem<'static>;

    /// Whether this item can be selected by the user
    fn is_selectable(&self) -> bool;

    /// Indentation level for hierarchical display (0 = root level)
    fn indent_level(&self) -> usize;
}

/// Enum representing different types of items that can appear in the task list
#[derive(Debug, Clone)]
pub enum TaskListItemType {
    Task(Box<TaskItem>),
    Header(HeaderItem),
    Separator(SeparatorItem),
}

impl ListItem for TaskListItemType {
    fn render(&self, selected: bool, display_config: &DisplayConfig) -> RatatuiListItem<'static> {
        match self {
            Self::Task(item) => item.render(selected, display_config),
            Self::Header(item) => item.render(selected, display_config),
            Self::Separator(item) => item.render(selected, display_config),
        }
    }

    fn is_selectable(&self) -> bool {
        match self {
            Self::Task(item) => item.is_selectable(),
            Self::Header(item) => item.is_selectable(),
            Self::Separator(item) => item.is_selectable(),
        }
    }

    fn indent_level(&self) -> usize {
        match self {
            Self::Task(item) => item.indent_level(),
            Self::Header(item) => item.indent_level(),
            Self::Separator(item) => item.indent_level(),
        }
    }
}

/// A task item component
#[derive(Debug, Clone)]
pub struct TaskItem {
    pub task: TaskDisplay,
    pub depth: usize,
    pub child_count: usize,
    pub icons: IconService,
    pub projects: Vec<ProjectDisplay>,
}

impl TaskItem {
    pub fn new(
        task: TaskDisplay,
        depth: usize,
        child_count: usize,
        icons: IconService,
        projects: Vec<ProjectDisplay>,
    ) -> Self {
        Self {
            task,
            depth,
            child_count,
            icons,
            projects,
        }
    }

    fn format_due_date(&self, due_date: &str) -> String {
        // Use human-readable date formatting similar to Todoist
        format_human_date(due_date)
    }

    /// Format due datetime with time information if available
    fn format_due_datetime(&self, due_datetime: &str) -> String {
        format_human_datetime(due_datetime)
    }
}

impl ListItem for TaskItem {
    fn render(&self, selected: bool, display_config: &DisplayConfig) -> RatatuiListItem<'static> {
        let status_icon = self.icons.task_pending();
        let mut line_spans = Vec::new();

        // Add hierarchical indentation for subtasks
        if self.depth > 0 {
            let mut indent_str = String::new();

            // Add spaces for each level (2 spaces per level)
            for _ in 0..(self.depth - 1) {
                indent_str.push_str("  ");
            }

            // Add tree connector for the current level
            indent_str.push_str("└─");

            line_spans.push(Span::styled(indent_str, Style::default().fg(Color::DarkGray)));
        }

        // Status icon
        let status_style = if selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        line_spans.push(Span::styled(format!("{} ", status_icon), status_style));

        // Priority badge (if any)
        if let Some(priority_badge) = create_priority_badge(self.task.priority) {
            line_spans.push(priority_badge);
            line_spans.push(Span::raw(" "));
        }

        // Task content with selection styling
        let content_style = if selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        line_spans.push(Span::styled(self.task.content.clone(), content_style));

        // Child task count (for tasks with children)
        if self.child_count > 0 {
            let progress_text = format!(" ({})", self.child_count);
            let progress_style = Style::default().fg(Color::Gray);
            line_spans.push(Span::styled(progress_text, progress_style));
        }

        // Project display (with optional colors)
        if let Some(project) = self.projects.iter().find(|p| p.id == self.task.project_id) {
            line_spans.push(Span::raw(" "));
            let project_style = if display_config.show_project_colors {
                // Use project color if available, otherwise cyan
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Cyan)
            };
            line_spans.push(Span::styled(format!("#{}", project.name), project_style));
        }

        // Due date/datetime display
        if let Some(due_date) = &self.task.due {
            line_spans.push(Span::raw(" "));

            // Use datetime formatting if available, otherwise use date formatting
            let formatted_date = if let Some(due_datetime) = &self.task.due_datetime {
                self.format_due_datetime(due_datetime)
            } else {
                self.format_due_date(due_date)
            };

            line_spans.push(Span::styled(
                formatted_date,
                Style::default().fg(Color::Rgb(255, 165, 0)), // Orange color
            ));
        }

        // Metadata badges (only if configured to show)
        if display_config.show_durations || display_config.show_labels {
            let metadata_badges = create_task_badges(
                self.task.is_recurring,
                self.task.due.is_some() || self.task.deadline.is_some(),
                if display_config.show_durations {
                    self.task.duration.as_deref()
                } else {
                    None
                },
                if display_config.show_labels {
                    self.task.labels.as_slice()
                } else {
                    &[]
                },
            );

            for badge in metadata_badges {
                line_spans.push(Span::raw(" "));
                line_spans.push(badge);
            }
        }

        // Add description excerpt if available and configured to show
        if display_config.show_descriptions && !self.task.description.is_empty() {
            // Get first line of description
            let description_line = self.task.description.lines().next().unwrap_or("");

            // Add the description with separator and grey styling
            line_spans.push(Span::raw(" - "));
            line_spans.push(Span::styled(
                description_line.to_string(),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ));
        }

        RatatuiListItem::new(Line::from(line_spans))
    }

    fn is_selectable(&self) -> bool {
        true
    }

    fn indent_level(&self) -> usize {
        self.depth
    }
}

/// A header item component (for sections, projects, etc.)
#[derive(Debug, Clone)]
pub struct HeaderItem {
    pub text: String,
    pub indent: usize,
}

impl HeaderItem {
    pub fn new(text: String, indent: usize) -> Self {
        Self { text, indent }
    }
}

impl ListItem for HeaderItem {
    fn render(&self, _selected: bool, _display_config: &DisplayConfig) -> RatatuiListItem<'static> {
        let indent_str = " ".repeat(self.indent * 2);
        RatatuiListItem::new(Line::from(Span::styled(
            format!("{}{}", indent_str, self.text),
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
        )))
    }

    fn is_selectable(&self) -> bool {
        false
    }

    fn indent_level(&self) -> usize {
        self.indent
    }
}

/// A separator item component
#[derive(Debug, Clone)]
pub struct SeparatorItem {
    pub indent: usize,
}

impl SeparatorItem {
    pub fn new(indent: usize) -> Self {
        Self { indent }
    }
}

impl ListItem for SeparatorItem {
    fn render(&self, _selected: bool, _display_config: &DisplayConfig) -> RatatuiListItem<'static> {
        let indent_str = " ".repeat(self.indent * 2);
        let separator = " ";

        RatatuiListItem::new(Line::from(Span::styled(
            format!("{}{}", indent_str, separator),
            Style::default().fg(Color::DarkGray),
        )))
    }

    fn is_selectable(&self) -> bool {
        false
    }

    fn indent_level(&self) -> usize {
        self.indent
    }
}
