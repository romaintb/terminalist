//! Transient notices parked in the bottom-right corner.
//!
//! A toast reports the outcome of background work without stealing focus the way a
//! modal dialog does. It carries its own expiry, so the owner only has to drop it
//! once [`Toast::expired`] says so.

use crate::constants::{TOAST_ERROR_TTL_SECS, TOAST_TTL_SECS};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, Instant};

/// A notice in the corner. `expires` is `None` for one that stays until whatever it
/// reports on finishes, like an in-flight sync.
pub struct Toast {
    text: String,
    color: Color,
    expires: Option<Instant>,
}

impl Toast {
    #[must_use]
    pub fn success(text: &str, theme: &Theme) -> Self {
        Self::timed(text.to_string(), theme.success, TOAST_TTL_SECS)
    }

    /// Failures linger: they carry information the user has to actually read.
    #[must_use]
    pub fn error(message: &str, theme: &Theme) -> Self {
        Self::timed(format!("❌ {message}"), theme.danger, TOAST_ERROR_TTL_SECS)
    }

    /// Work in progress. Sticks around until the caller stops rendering it.
    #[must_use]
    pub fn spinner(title: &str, theme: &Theme) -> Self {
        Self {
            text: format!("⟳ {title}…"),
            color: theme.warning,
            expires: None,
        }
    }

    fn timed(text: String, color: Color, ttl_secs: u64) -> Self {
        Self {
            text,
            color,
            expires: Some(Instant::now() + Duration::from_secs(ttl_secs)),
        }
    }

    /// Whether this toast has outlived its TTL. Always false for a spinner.
    #[must_use]
    pub fn expired(&self) -> bool {
        self.expires.is_some_and(|e| e <= Instant::now())
    }

    /// Draw into the bottom-right of `area`, one column clear of its border.
    pub fn render(&self, f: &mut Frame, area: Rect) {
        // Size from display width, not char count: the status emoji are two columns each.
        let line = Line::from(Span::styled(self.text.as_str(), Style::default().fg(self.color)));
        let Some(bounds) = rect(area, line.width()) else {
            return;
        };

        f.render_widget(Clear, bounds);
        f.render_widget(
            Paragraph::new(line)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).style(Style::default().fg(self.color))),
            bounds,
        );
    }
}

/// Bottom-right box for `text_width` display columns, inset one column off the border.
/// `None` when `area` is too cramped to be worth it.
#[must_use]
pub fn rect(area: Rect, text_width: usize) -> Option<Rect> {
    if area.width < 12 || area.height < 5 {
        return None;
    }
    let len = u16::try_from(text_width).unwrap_or(u16::MAX);
    let width = len.saturating_add(4).min(area.width - 2);
    let lines = len.div_ceil(width - 2).max(1);
    let height = (lines + 2).min(area.height - 2);
    Some(Rect {
        x: area.right() - width - 1,
        y: area.bottom() - height - 1,
        width,
        height,
    })
}
