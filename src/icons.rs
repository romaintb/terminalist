//! Icon service for managing different icon themes
//!
//! This module provides a centralized way to manage icons throughout the application,
//! supporting different themes like emoji, Unicode, and ASCII fallbacks.

use serde::{Deserialize, Serialize};

/// Icon theme variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconTheme {
    /// Emoji icons (colorful, modern look)
    Emoji,
    /// Unicode symbols (clean, native look)
    Unicode,
    /// ASCII characters (maximum compatibility)
    Ascii,
}

impl Default for IconTheme {
    fn default() -> Self {
        Self::Ascii
    }
}

/// Task status icons
#[derive(Debug, Clone)]
pub struct TaskStatusIcons {
    pub pending: &'static str,
    pub completed: &'static str,
    pub deleted: &'static str,
}

/// UI element icons
#[derive(Debug, Clone)]
pub struct UiIcons {
    pub tasks_title: &'static str,
    pub projects_title: &'static str,
    pub error: &'static str,
    pub info: &'static str,
    pub warning: &'static str,
    pub success: &'static str,
}

/// Priority indicators
#[derive(Debug, Clone)]
pub struct PriorityIcons {
    pub urgent: &'static str,
    pub high: &'static str,
    pub medium: &'static str,
    pub low: &'static str,
}

/// Status and metadata icons
#[derive(Debug, Clone)]
pub struct StatusIcons {
    pub recurring: &'static str,
    pub due_date: &'static str,
    pub duration: &'static str,
    pub sync_in_progress: &'static str,
    pub sync_success: &'static str,
    pub sync_error: &'static str,
}

/// Complete icon set for a specific theme
#[derive(Debug, Clone)]
pub struct IconSet {
    pub task_status: TaskStatusIcons,
    pub ui: UiIcons,
    pub priority: PriorityIcons,
    pub status: StatusIcons,
}

/// Icon service for managing themes and providing icons
#[derive(Debug, Clone)]
pub struct IconService {
    current_theme: IconTheme,
}

impl Default for IconService {
    fn default() -> Self {
        Self::new(IconTheme::default())
    }
}

impl IconService {
    /// Create a new icon service with the specified theme
    #[must_use]
    pub fn new(theme: IconTheme) -> Self {
        Self { current_theme: theme }
    }

    /// Get the current theme
    #[must_use]
    pub fn theme(&self) -> IconTheme {
        self.current_theme
    }

    /// Set the current theme
    pub fn set_theme(&mut self, theme: IconTheme) {
        self.current_theme = theme;
    }

    /// Cycle to the next icon theme in the sequence: Ascii -> Unicode -> Emoji -> Ascii
    pub fn cycle_icon_theme(&mut self) {
        self.current_theme = match self.current_theme {
            IconTheme::Ascii => IconTheme::Unicode,
            IconTheme::Unicode => IconTheme::Emoji,
            IconTheme::Emoji => IconTheme::Ascii,
        };
    }

    /// Get the complete icon set for the current theme
    #[must_use]
    pub fn icons(&self) -> IconSet {
        match self.current_theme {
            IconTheme::Emoji => Self::emoji_icons(),
            IconTheme::Unicode => Self::unicode_icons(),
            IconTheme::Ascii => Self::ascii_icons(),
        }
    }

    /// Get emoji icon set
    fn emoji_icons() -> IconSet {
        IconSet {
            task_status: TaskStatusIcons {
                pending: "🔳",
                completed: "✅",
                deleted: "❌",
            },
            ui: UiIcons {
                tasks_title: "📝",
                projects_title: "📁",
                error: "❌",
                info: "💡",
                warning: "⚠️",
                success: "✅",
            },
            priority: PriorityIcons {
                urgent: "🔴",
                high: "🟡",
                medium: "🟢",
                low: "🔵",
            },
            status: StatusIcons {
                recurring: "🔄",
                due_date: "📅",
                duration: "⏱️",
                sync_in_progress: "🔄",
                sync_success: "✅",
                sync_error: "❌",
            },
        }
    }

    /// Get Unicode icon set
    fn unicode_icons() -> IconSet {
        IconSet {
            task_status: TaskStatusIcons {
                pending: "□",
                completed: "✓",
                deleted: "✗",
            },
            ui: UiIcons {
                tasks_title: "▶",
                projects_title: "◆",
                error: "✗",
                info: "ⓘ",
                warning: "⚠",
                success: "✓",
            },
            priority: PriorityIcons {
                urgent: "●",
                high: "◉",
                medium: "○",
                low: "◯",
            },
            status: StatusIcons {
                recurring: "↻",
                due_date: "◷",
                duration: "⧖",
                sync_in_progress: "⟳",
                sync_success: "✓",
                sync_error: "✗",
            },
        }
    }

    /// Get ASCII icon set
    fn ascii_icons() -> IconSet {
        IconSet {
            task_status: TaskStatusIcons {
                pending: "[ ]",
                completed: "[X]",
                deleted: "[D]",
            },
            ui: UiIcons {
                tasks_title: ">",
                projects_title: "#",
                error: "X",
                info: "i",
                warning: "!",
                success: "+",
            },
            priority: PriorityIcons {
                urgent: "!!",
                high: "!",
                medium: "+",
                low: "-",
            },
            status: StatusIcons {
                recurring: "~",
                due_date: "@",
                duration: "T",
                sync_in_progress: "...",
                sync_success: "+",
                sync_error: "X",
            },
        }
    }

    /// Convenience methods for commonly used icons
    #[must_use]
    pub fn task_pending(&self) -> &'static str {
        self.icons().task_status.pending
    }

    #[must_use]
    pub fn task_completed(&self) -> &'static str {
        self.icons().task_status.completed
    }

    #[must_use]
    pub fn task_deleted(&self) -> &'static str {
        self.icons().task_status.deleted
    }

    #[must_use]
    pub fn info(&self) -> &'static str {
        self.icons().ui.info
    }

    #[must_use]
    pub fn warning(&self) -> &'static str {
        self.icons().ui.warning
    }

    /// Convenience methods for project and label icons
    #[must_use]
    pub fn project_regular(&self) -> &'static str {
        match self.current_theme {
            IconTheme::Emoji => "📁",
            IconTheme::Unicode => "◆",
            IconTheme::Ascii => "#",
        }
    }

    #[must_use]
    pub fn project_favorite(&self) -> &'static str {
        match self.current_theme {
            IconTheme::Emoji => "⭐",
            IconTheme::Unicode => "★",
            IconTheme::Ascii => "*",
        }
    }

    #[must_use]
    pub fn label(&self) -> &'static str {
        match self.current_theme {
            IconTheme::Emoji => "🏷️",
            IconTheme::Unicode => "◉",
            IconTheme::Ascii => "@",
        }
    }

    #[must_use]
    pub fn today(&self) -> &'static str {
        match self.current_theme {
            IconTheme::Emoji => "📅",
            IconTheme::Unicode => "◷",
            IconTheme::Ascii => "@",
        }
    }

    #[must_use]
    pub fn tomorrow(&self) -> &'static str {
        match self.current_theme {
            IconTheme::Emoji => "🗓️",
            IconTheme::Unicode => "◶",
            IconTheme::Ascii => "+",
        }
    }
}
