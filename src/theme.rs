//! Theme configuration for the Terminalist TUI.
//!
//! Defines the set of semantic colors used throughout the UI. Each field maps to a
//! meaning (e.g. "danger", "accent") rather than a specific widget, so a single change
//! propagates everywhere that meaning is used. Colors are stored as `ratatui::style::Color`
//! and serialized as plain strings (color names like `"blue"` or hex like `"#ff8000"`).
//!
//! Priority-flag colors (P1-P4) are intentionally not part of this theme: they're a fixed
//! visual language, so they stay hardcoded in `ui::components::badge`.

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Semantic color palette for the TUI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    /// Selection highlight color, used for the currently-selected sidebar entry.
    #[serde(with = "color_serde")]
    pub accent: Color,
    /// Completed-task checkmark icon, and the accent color for save/create actions in dialogs.
    #[serde(with = "color_serde")]
    pub success: Color,
    /// Deleted-task icon/text, delete/cancel shortcuts, and the error/delete-confirmation dialogs.
    #[serde(with = "color_serde")]
    pub danger: Color,
    /// Sync/loading banner, and the "Edit Project" dialog accent.
    #[serde(with = "color_serde")]
    pub warning: Color,
    /// Section/account headers, task & label dialog accent, and the Tab-select hint.
    #[serde(with = "color_serde")]
    pub info: Color,
    /// Border/title accent of the "Info" dialog.
    #[serde(with = "color_serde")]
    pub info_dialog: Color,
    /// Border/title accent of the "New Project" dialog.
    #[serde(with = "color_serde")]
    pub project_accent: Color,
    /// Color of the `#project` tag shown next to a task.
    #[serde(with = "color_serde")]
    pub project_tag: Color,
    /// Color of a task's due-date text.
    #[serde(with = "color_serde")]
    pub due_date: Color,
    /// Color of `@label` badges.
    #[serde(with = "color_serde")]
    pub label: Color,
    /// Default text color for unselected items and dialog content.
    #[serde(with = "color_serde")]
    pub text: Color,
    /// Muted text: descriptions, tree connectors, separators, completed-task text, list scrollbars.
    #[serde(with = "color_serde")]
    pub text_muted: Color,
    /// Color of the sidebar and task-list borders.
    #[serde(with = "color_serde")]
    pub border: Color,
    /// Color of dialog chrome borders/scrollbars, the child-count badge, and instruction separators.
    #[serde(with = "color_serde")]
    pub border_dim: Color,
    /// Background color of the currently-highlighted row in the task list.
    #[serde(with = "color_serde")]
    pub selection_bg: Color,
    /// Background color painted behind the whole UI. Defaults to `Color::Reset`, which
    /// keeps the terminal's own background; set it to override the terminal theme entirely.
    #[serde(with = "color_serde")]
    pub background: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Color::Yellow,
            success: Color::Green,
            danger: Color::Red,
            warning: Color::Yellow,
            info: Color::Cyan,
            info_dialog: Color::Blue,
            project_accent: Color::Magenta,
            project_tag: Color::Cyan,
            due_date: Color::Rgb(255, 165, 0),
            label: Color::Green,
            text: Color::White,
            text_muted: Color::DarkGray,
            border: Color::DarkGray,
            border_dim: Color::Gray,
            selection_bg: Color::DarkGray,
            background: Color::Reset,
        }
    }
}

/// A single theme field that couldn't be applied as written, so its built-in default was
/// used instead.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeWarning {
    /// The `[theme]` key that had a problem (e.g. `"danger"`).
    pub field: &'static str,
    /// What was actually written in the config file for that key.
    pub raw_value: String,
    /// 1-based line number in the config file where the value appears.
    pub line: usize,
}

impl fmt::Display for ThemeWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "theme.{} (line {}): '{}' is not a valid color, using default",
            self.field, self.line, self.raw_value
        )
    }
}

/// Builds a single user-facing message summarizing a list of [`ThemeWarning`]s, suitable
/// for showing in an info dialog at startup. Returns `None` if there are no warnings.
#[must_use]
pub fn format_warnings(warnings: &[ThemeWarning]) -> Option<String> {
    if warnings.is_empty() {
        return None;
    }

    let noun = if warnings.len() == 1 { "color" } else { "colors" };
    let mut message = format!(
        "⚠ {} theme {noun} in your config could not be applied and fell back to defaults:\n",
        warnings.len()
    );

    for warning in warnings {
        message.push_str(&format!("\n• {warning}"));
    }

    message.push_str("\n\nFix these in your config file, then restart Terminalist.");

    Some(message)
}

/// Raw (unvalidated) counterpart of [`Theme`] used only for lenient loading from a config
/// file. Each field is optional (absent = "use the default") and keeps its source span so a
/// bad value can be reported with a line number, instead of failing the whole config load.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RawTheme {
    accent: Option<toml::Spanned<toml::Value>>,
    success: Option<toml::Spanned<toml::Value>>,
    danger: Option<toml::Spanned<toml::Value>>,
    warning: Option<toml::Spanned<toml::Value>>,
    info: Option<toml::Spanned<toml::Value>>,
    info_dialog: Option<toml::Spanned<toml::Value>>,
    project_accent: Option<toml::Spanned<toml::Value>>,
    project_tag: Option<toml::Spanned<toml::Value>>,
    due_date: Option<toml::Spanned<toml::Value>>,
    label: Option<toml::Spanned<toml::Value>>,
    text: Option<toml::Spanned<toml::Value>>,
    text_muted: Option<toml::Spanned<toml::Value>>,
    border: Option<toml::Spanned<toml::Value>>,
    border_dim: Option<toml::Spanned<toml::Value>>,
    selection_bg: Option<toml::Spanned<toml::Value>>,
    background: Option<toml::Spanned<toml::Value>>,
}

/// 1-based line number containing the given byte offset into `source`.
fn line_number(source: &str, byte_offset: usize) -> usize {
    source.get(..byte_offset).unwrap_or(source).matches('\n').count() + 1
}

impl Theme {
    /// Builds a [`Theme`] from a [`RawTheme`], falling back to the default for any field
    /// that is missing, the wrong type, or not a recognized color. Every fallback is
    /// reported as a [`ThemeWarning`] so the caller can surface it to the user.
    pub(crate) fn from_raw(raw: RawTheme, source: &str) -> (Theme, Vec<ThemeWarning>) {
        let defaults = Theme::default();
        let mut warnings = Vec::new();

        macro_rules! resolve {
            ($field:ident) => {
                match raw.$field {
                    None => defaults.$field,
                    Some(spanned) => {
                        let line = line_number(source, spanned.span().start);
                        match spanned.get_ref().as_str() {
                            Some(s) => match Color::from_str(s) {
                                Ok(color) => color,
                                Err(_) => {
                                    warnings.push(ThemeWarning {
                                        field: stringify!($field),
                                        raw_value: s.to_string(),
                                        line,
                                    });
                                    defaults.$field
                                }
                            },
                            None => {
                                warnings.push(ThemeWarning {
                                    field: stringify!($field),
                                    raw_value: spanned.get_ref().to_string(),
                                    line,
                                });
                                defaults.$field
                            }
                        }
                    }
                }
            };
        }

        let theme = Theme {
            accent: resolve!(accent),
            success: resolve!(success),
            danger: resolve!(danger),
            warning: resolve!(warning),
            info: resolve!(info),
            info_dialog: resolve!(info_dialog),
            project_accent: resolve!(project_accent),
            project_tag: resolve!(project_tag),
            due_date: resolve!(due_date),
            label: resolve!(label),
            text: resolve!(text),
            text_muted: resolve!(text_muted),
            border: resolve!(border),
            border_dim: resolve!(border_dim),
            selection_bg: resolve!(selection_bg),
            background: resolve!(background),
        };

        (theme, warnings)
    }
}

/// Serializes/deserializes `ratatui::style::Color` as a plain string, using the color's
/// own `Display`/`FromStr` impls (which already support names like `"red"` and hex like
/// `"#ff8000"`).
mod color_serde {
    use ratatui::style::Color;
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&color.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Color::from_str(&s).map_err(|_| D::Error::custom(format!("invalid color: '{s}'")))
    }
}
