use crate::config::DisplayConfig;
use crate::entities::task;
use crate::icons::{TASK_COMPLETED, TASK_DELETED, TASK_PENDING};
use crate::theme::Theme;
use crate::ui::components::badge::{create_priority_badge, create_task_badges};
use crate::utils::datetime::{format_human_date, format_human_datetime};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::ListItem as RatatuiListItem,
};

/// Width of indentation per depth level in characters
const INDENT_WIDTH: usize = 2;

/// Trait for items that can be displayed in a task list
pub trait ListItem {
    /// Render this item as a ratatui ListItem
    fn render(&self, selected: bool, display_config: &DisplayConfig, theme: &Theme) -> RatatuiListItem<'static>;

    /// Whether this item can be selected by the user
    fn is_selectable(&self) -> bool;
}

/// Enum representing different types of items that can appear in the task list
#[derive(Debug, Clone)]
pub enum TaskListItemType {
    Task(Box<TaskItem>),
    Header(HeaderItem),
    Separator(SeparatorItem),
}

impl ListItem for TaskListItemType {
    fn render(&self, selected: bool, display_config: &DisplayConfig, theme: &Theme) -> RatatuiListItem<'static> {
        match self {
            Self::Task(item) => item.render(selected, display_config, theme),
            Self::Header(item) => item.render(selected, display_config, theme),
            Self::Separator(item) => item.render(selected, display_config, theme),
        }
    }

    fn is_selectable(&self) -> bool {
        match self {
            Self::Task(item) => item.is_selectable(),
            Self::Header(item) => item.is_selectable(),
            Self::Separator(item) => item.is_selectable(),
        }
    }
}

/// A task item component
#[derive(Debug, Clone)]
pub struct TaskItem {
    pub task: task::Model,
    pub depth: usize,
    pub child_count: usize,
    pub project_name: Option<String>,
    pub labels: Vec<crate::entities::label::Model>,
}

impl TaskItem {
    pub fn new(
        task: task::Model,
        depth: usize,
        child_count: usize,
        project_name: Option<String>,
        labels: Vec<crate::entities::label::Model>,
    ) -> Self {
        Self {
            task,
            depth,
            child_count,
            project_name,
            labels,
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
    fn render(&self, selected: bool, display_config: &DisplayConfig, theme: &Theme) -> RatatuiListItem<'static> {
        // Choose the appropriate icon based on task state
        let status_icon = if self.task.is_deleted {
            TASK_DELETED
        } else if self.task.is_completed {
            TASK_COMPLETED
        } else {
            TASK_PENDING
        };
        let mut line_spans = Vec::new();

        // Add hierarchical indentation for subtasks
        if self.depth > 0 {
            let mut indent_str = String::new();

            // Add spaces for each level
            let total_indent = (self.depth - 1) * INDENT_WIDTH;
            indent_str.push_str(&" ".repeat(total_indent));

            // Add tree connector for the current level
            indent_str.push_str("└─");

            line_spans.push(Span::styled(indent_str, Style::default().fg(theme.text_muted)));
        }

        // Status icon with state-based styling
        let status_style = if self.task.is_deleted {
            // Deleted tasks: danger color
            Style::default().fg(theme.danger)
        } else if self.task.is_completed {
            // Completed tasks: success color for the tick mark
            Style::default().fg(theme.success)
        } else if selected {
            // Selected active tasks: accent and bold
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            // Normal active tasks: default text color
            Style::default().fg(theme.text)
        };
        line_spans.push(Span::styled(format!("{} ", status_icon), status_style));

        // Priority badge (if any)
        if let Some(priority_badge) = create_priority_badge(self.task.priority) {
            line_spans.push(priority_badge);
            line_spans.push(Span::raw(" "));
        }

        // Task content with selection styling and deleted/completed styling
        let content_style = if self.task.is_deleted {
            // Deleted tasks: danger color with strikethrough
            Style::default().fg(theme.danger).add_modifier(Modifier::CROSSED_OUT)
        } else if self.task.is_completed {
            // Completed tasks: muted with strikethrough
            Style::default().fg(theme.text_muted).add_modifier(Modifier::CROSSED_OUT)
        } else if selected {
            // Selected active tasks: accent and bold
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            // Normal active tasks: default text color
            Style::default().fg(theme.text)
        };
        line_spans.push(Span::styled(self.task.content.clone(), content_style));

        // Child task count (for tasks with children)
        if self.child_count > 0 {
            let progress_text = format!(" ({})", self.child_count);
            let progress_style = Style::default().fg(theme.border_dim);
            line_spans.push(Span::styled(progress_text, progress_style));
        }

        // Project tag (resolved at item build time)
        if let Some(name) = &self.project_name {
            line_spans.push(Span::raw(" "));
            line_spans.push(Span::styled(format!("#{name}"), Style::default().fg(theme.project_tag)));
        }

        // Due date/datetime display
        if let Some(due_date) = &self.task.due_date {
            line_spans.push(Span::raw(" "));

            // Use datetime formatting if available, otherwise use date formatting
            let formatted_date = if let Some(due_datetime) = &self.task.due_datetime {
                self.format_due_datetime(due_datetime)
            } else {
                self.format_due_date(due_date)
            };

            line_spans.push(Span::styled(formatted_date, Style::default().fg(theme.due_date)));
        }

        // Metadata badges (only if configured to show)
        if display_config.show_durations || display_config.show_labels {
            let metadata_badges = create_task_badges(
                self.task.is_recurring,
                if display_config.show_durations {
                    self.task.duration.as_deref()
                } else {
                    None
                },
                if display_config.show_labels { &self.labels } else { &[] },
                theme,
            );

            for badge in metadata_badges {
                line_spans.push(Span::raw(" "));
                line_spans.push(badge);
            }
        }

        // Add description excerpt if available and configured to show
        if display_config.show_descriptions {
            if let Some(desc) = &self.task.description {
                if !desc.is_empty() {
                    // Get first line of description
                    let description_line = desc.lines().next().unwrap_or("");

                    // Add the description with separator and muted styling
                    line_spans.push(Span::raw(" - "));
                    line_spans.push(Span::styled(
                        description_line.to_string(),
                        Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC),
                    ));
                }
            }
        }

        RatatuiListItem::new(Line::from(line_spans))
    }

    fn is_selectable(&self) -> bool {
        true
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
    fn render(&self, _selected: bool, _display_config: &DisplayConfig, theme: &Theme) -> RatatuiListItem<'static> {
        let indent_str = " ".repeat(self.indent * INDENT_WIDTH);
        RatatuiListItem::new(Line::from(Span::styled(
            format!("{}{}", indent_str, self.text),
            Style::default().add_modifier(Modifier::BOLD).fg(theme.info),
        )))
    }

    fn is_selectable(&self) -> bool {
        false
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
    fn render(&self, _selected: bool, _display_config: &DisplayConfig, theme: &Theme) -> RatatuiListItem<'static> {
        let indent_str = " ".repeat(self.indent * INDENT_WIDTH);
        let separator = " ";

        RatatuiListItem::new(Line::from(Span::styled(
            format!("{}{}", indent_str, separator),
            Style::default().fg(theme.text_muted),
        )))
    }

    fn is_selectable(&self) -> bool {
        false
    }
}
