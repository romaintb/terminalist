//! Non-blocking sync status toast for the Terminalist application.
//!
//! Replaces the old centered "Loading data" overlay that blocked interaction while a
//! sync ran. This toast anchors to the bottom-right corner of the task list area so the
//! user can keep navigating while a sync (initial, manual, or automatic) runs in the
//! background.
//!
//! The state machine is a pure function of `(event, now)`, so it can be unit tested
//! without sleeping or touching a real clock source beyond `Instant` values the test
//! constructs itself. See `should_auto_sync` for the companion auto-sync timer decision.

use crate::sync::SyncStatus;
use crate::theme::Theme;
use ratatui::{
    layout::{Margin, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

/// How long the "Synced" success toast stays visible before it hides itself.
const SUCCESS_TOAST_DURATION: Duration = Duration::from_secs(3);

const SYNCING_TEXT: &str = "⟳ Syncing…";
const SUCCEEDED_TEXT: &str = "✓ Synced";
const FAILED_TEXT: &str = "✗ Sync failed";

/// The toast's visual state, driven purely by explicit events and an `Instant` the
/// caller supplies (never by reading the wall clock itself).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastState {
    /// Nothing to show.
    Hidden,
    /// A sync is currently running.
    Syncing,
    /// A sync just finished successfully; stays visible until `until`.
    Succeeded { until: Instant },
    /// A sync failed. Stays visible until the user dismisses it (any keypress).
    Failed,
}

/// A small, non-blocking status indicator anchored to a corner of its host area.
pub struct SyncToast {
    state: ToastState,
    theme: Theme,
}

impl Default for SyncToast {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncToast {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ToastState::Hidden,
            theme: Theme::default(),
        }
    }

    pub fn update_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// A sync has started (or is still running): show the syncing indicator.
    ///
    /// Replaces whatever was showing before, including a `Failed` toast: the new sync
    /// attempt supersedes the stale failure notice.
    pub fn started(&mut self) {
        self.state = ToastState::Syncing;
    }

    /// A sync completed successfully at `now`. Visible for a short window, then hides
    /// itself the next time [`Self::tick`] observes it has expired.
    pub fn succeeded(&mut self, now: Instant) {
        self.state = ToastState::Succeeded {
            until: now + SUCCESS_TOAST_DURATION,
        };
    }

    /// A sync failed. Stays visible until [`Self::dismiss`] is called.
    pub fn failed(&mut self) {
        self.state = ToastState::Failed;
    }

    /// Clears a failure notice. No-op unless the toast is currently showing a failure,
    /// so it's safe to call unconditionally (e.g. on every keypress).
    pub fn dismiss(&mut self) {
        if matches!(self.state, ToastState::Failed) {
            self.state = ToastState::Hidden;
        }
    }

    /// Advances the state machine: expires a `Succeeded` toast whose window has passed.
    /// Does nothing to `Syncing`/`Failed`/`Hidden`.
    pub fn tick(&mut self, now: Instant) {
        if let ToastState::Succeeded { until } = self.state {
            if now >= until {
                self.state = ToastState::Hidden;
            }
        }
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        !matches!(self.state, ToastState::Hidden)
    }

    /// Whether a plain tick can change what this toast shows.
    ///
    /// Only `Succeeded` can: [`Self::tick`] is what expires it. `Syncing`'s text is static,
    /// `Failed` clears only on a keypress (which already forces a repaint), and `Hidden` shows
    /// nothing. The render loop asks this instead of [`Self::is_visible`] so that a `Failed`
    /// toast — which by design never expires — cannot pin the TUI at tick rate indefinitely.
    #[must_use]
    pub fn expires_on_tick(&self) -> bool {
        matches!(self.state, ToastState::Succeeded { .. })
    }

    #[must_use]
    pub fn text(&self) -> &str {
        match self.state {
            ToastState::Hidden => "",
            ToastState::Syncing => SYNCING_TEXT,
            ToastState::Succeeded { .. } => SUCCEEDED_TEXT,
            ToastState::Failed => FAILED_TEXT,
        }
    }

    /// Renders the toast anchored to the bottom-right corner, inset by 1 cell inside
    /// `task_list_area` so it never overlaps the task list's own border. Clamps so a
    /// narrow terminal never produces a negative or out-of-bounds `Rect`.
    pub fn render(&self, f: &mut Frame, task_list_area: Rect) {
        if !self.is_visible() {
            return;
        }

        let inner = task_list_area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let text = self.text();
        let width = (text.chars().count() as u16).saturating_add(2).min(inner.width);
        let height = 3u16.min(inner.height);
        if width == 0 || height == 0 {
            return;
        }

        let color = match self.state {
            ToastState::Syncing => self.theme.warning,
            ToastState::Succeeded { .. } => self.theme.success,
            ToastState::Failed => self.theme.danger,
            ToastState::Hidden => return,
        };

        let area = Rect {
            x: inner.right() - width,
            y: inner.bottom() - height,
            width,
            height,
        };

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(color))
            .block(Block::default().borders(Borders::ALL).style(Style::default().fg(color)));

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }
}

/// Whether a terminal `Action::SyncCompleted(status)` should be shown to the user as a
/// success.
///
/// Only `SyncStatus::Success` counts. `SyncStatus::Error` is an explicit failure, and
/// `Idle`/`InProgress` should never actually reach a "completed" handler in the first
/// place; both are treated defensively as a failure (never a panic) rather than risking
/// a false "Synced" toast.
#[must_use]
pub fn sync_completed_successfully(status: &SyncStatus) -> bool {
    matches!(status, SyncStatus::Success)
}

/// Decides whether the auto-sync timer should fire.
///
/// - `interval_minutes == 0` disables auto-sync entirely.
/// - A sync already in flight never triggers another one.
/// - No prior attempt (`last_sync_attempt_at == None`) never triggers the timer: the
///   startup sync is kicked off explicitly by `trigger_initial_sync`, not by this timer.
///
/// `last_sync_attempt_at` must be updated after *every* terminal sync outcome — success
/// **and** failure — not just success. Otherwise a sync that fails fast (e.g. a backend
/// resolution error, which fails well within one ~100ms tick) leaves the timestamp
/// stale, `duration_since` keeps reporting the interval has elapsed, and this fires
/// again on the very next tick: a retry storm with no backoff. Recording the attempt
/// unconditionally makes a failure wait a full interval before retrying, which is the
/// correct trade.
#[must_use]
pub fn should_auto_sync(
    last_sync_attempt_at: Option<Instant>,
    now: Instant,
    interval_minutes: u64,
    sync_in_flight: bool,
) -> bool {
    if interval_minutes == 0 || sync_in_flight {
        return false;
    }
    match last_sync_attempt_at {
        None => false,
        Some(last) => now.duration_since(last) >= Duration::from_secs(interval_minutes * 60),
    }
}
