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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_round_trips_through_toml() {
        let theme = Theme::default();
        let toml_str = toml::to_string(&theme).expect("serialize theme");
        let parsed: Theme = toml::from_str(&toml_str).expect("deserialize theme");
        assert_eq!(theme, parsed);
    }

    #[test]
    fn accepts_hex_colors() {
        let toml_str = r##"due_date = "#ff8000""##;
        let theme: Theme = toml::from_str(toml_str).expect("deserialize theme");
        assert_eq!(theme.due_date, Color::Rgb(0xff, 0x80, 0x00));
    }

    #[test]
    fn rejects_invalid_colors() {
        let toml_str = r#"accent = "not-a-color""#;
        let result: Result<Theme, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn from_raw_defaults_missing_fields_silently() {
        let source = "";
        let raw: RawTheme = toml::from_str(source).unwrap();
        let (theme, warnings) = Theme::from_raw(raw, source);
        assert_eq!(theme, Theme::default());
        assert!(warnings.is_empty());
    }

    #[test]
    fn from_raw_keeps_valid_overrides_and_defaults_the_rest() {
        let source = "accent = \"Magenta\"\n";
        let raw: RawTheme = toml::from_str(source).unwrap();
        let (theme, warnings) = Theme::from_raw(raw, source);
        assert_eq!(theme.accent, Color::Magenta);
        assert_eq!(theme.danger, Theme::default().danger);
        assert!(warnings.is_empty());
    }

    #[test]
    fn from_raw_falls_back_and_warns_on_invalid_color_string() {
        let source = "accent = \"Blue\"\ndanger = \"notacolor\"\n";
        let raw: RawTheme = toml::from_str(source).unwrap();
        let (theme, warnings) = Theme::from_raw(raw, source);

        assert_eq!(theme.accent, Color::Blue);
        assert_eq!(
            theme.danger,
            Theme::default().danger,
            "invalid value should fall back to default"
        );

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].field, "danger");
        assert_eq!(warnings[0].raw_value, "notacolor");
        assert_eq!(warnings[0].line, 2);
    }

    #[test]
    fn from_raw_falls_back_and_warns_on_wrong_type() {
        let source = "accent = 5\n";
        let raw: RawTheme = toml::from_str(source).unwrap();
        let (theme, warnings) = Theme::from_raw(raw, source);

        assert_eq!(theme.accent, Theme::default().accent);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].field, "accent");
        assert_eq!(warnings[0].line, 1);
    }

    #[test]
    fn from_raw_reports_multiple_warnings_with_correct_lines() {
        let source = "accent = \"Blue\"\ndanger = \"bogus1\"\nborder = \"bogus2\"\n";
        let raw: RawTheme = toml::from_str(source).unwrap();
        let (_, warnings) = Theme::from_raw(raw, source);

        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].field, "danger");
        assert_eq!(warnings[0].line, 2);
        assert_eq!(warnings[1].field, "border");
        assert_eq!(warnings[1].line, 3);
    }

    #[test]
    fn format_warnings_returns_none_when_empty() {
        assert_eq!(format_warnings(&[]), None);
    }

    #[test]
    fn format_warnings_summarizes_all_entries() {
        let warnings = vec![
            ThemeWarning {
                field: "danger",
                raw_value: "notacolor".to_string(),
                line: 2,
            },
            ThemeWarning {
                field: "border",
                raw_value: "bogus".to_string(),
                line: 3,
            },
        ];
        let message = format_warnings(&warnings).unwrap();
        assert!(message.contains("2 theme colors"));
        assert!(message.contains("theme.danger (line 2)"));
        assert!(message.contains("theme.border (line 3)"));
    }

    #[test]
    fn theme_warning_display_is_readable() {
        let warning = ThemeWarning {
            field: "danger",
            raw_value: "notacolor".to_string(),
            line: 2,
        };
        assert_eq!(
            warning.to_string(),
            "theme.danger (line 2): 'notacolor' is not a valid color, using default"
        );
    }
}
