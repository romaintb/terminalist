use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use std::time::{Duration, Instant};
use terminalist::sync::SyncStatus;
use terminalist::ui::components::sync_toast::sync_completed_successfully;
use terminalist::ui::components::{should_auto_sync, SyncToast};

// --- SyncToast state machine -----------------------------------------------------

#[test]
fn started_is_visible_with_syncing_text() {
    let mut toast = SyncToast::new();
    toast.started();

    assert!(toast.is_visible());
    assert!(toast.text().contains("Syncing"), "text was: {:?}", toast.text());
}

#[test]
fn succeeded_stays_visible_briefly_then_expires() {
    let now = Instant::now();
    let mut toast = SyncToast::new();
    toast.succeeded(now);
    assert!(toast.is_visible(), "should be visible immediately after succeeding");

    toast.tick(now + Duration::from_secs(2));
    assert!(toast.is_visible(), "should still be visible 2s after succeeding");

    toast.tick(now + Duration::from_secs(4));
    assert!(!toast.is_visible(), "should be hidden 4s after succeeding");
}

#[test]
fn failed_stays_visible_until_dismissed() {
    let now = Instant::now();
    let mut toast = SyncToast::new();
    toast.failed();

    assert!(toast.is_visible());
    toast.tick(now + Duration::from_secs(60));
    assert!(
        toast.is_visible(),
        "a failure must not auto-expire, even after a long time"
    );
}

// The render loop keys its tick repaint off this, not off `is_visible`. `Failed` never expires
// by design, so repainting whenever the toast is visible pins the TUI at tick rate (10 Hz)
// forever after any failed sync.
#[test]
fn only_a_succeeded_toast_expires_on_tick() {
    let now = Instant::now();

    let hidden = SyncToast::new();
    assert!(!hidden.expires_on_tick(), "a hidden toast has nothing to expire");

    let mut syncing = SyncToast::new();
    syncing.started();
    assert!(
        !syncing.expires_on_tick(),
        "the syncing text is static; a tick cannot change it"
    );

    let mut succeeded = SyncToast::new();
    succeeded.succeeded(now);
    assert!(succeeded.expires_on_tick(), "a success toast expires on its own");

    let mut failed = SyncToast::new();
    failed.failed();
    assert!(
        !failed.expires_on_tick(),
        "a failure clears only on a keypress, which already forces a repaint"
    );
}

#[test]
fn a_failed_toast_never_asks_for_tick_repaints_however_long_it_stays_up() {
    let now = Instant::now();
    let mut toast = SyncToast::new();
    toast.failed();

    for minutes in [0, 1, 5, 60] {
        toast.tick(now + Duration::from_secs(minutes * 60));
        assert!(toast.is_visible(), "the failure notice must stay up");
        assert!(
            !toast.expires_on_tick(),
            "a sticky failure must never force a repaint at {minutes} minutes in"
        );
    }
}

#[test]
fn a_succeeded_toast_stops_asking_for_repaints_once_it_has_expired() {
    let now = Instant::now();
    let mut toast = SyncToast::new();
    toast.succeeded(now);

    // Still counting down: the loop must keep drawing so the toast can disappear on time.
    toast.tick(now + Duration::from_secs(1));
    assert!(toast.expires_on_tick());

    // Expired: the erase frame is driven by the value sampled *before* this tick, and from
    // here on the loop goes back to idling.
    toast.tick(now + Duration::from_secs(4));
    assert!(!toast.is_visible());
    assert!(!toast.expires_on_tick());
}

#[test]
fn dismiss_clears_a_failure() {
    let mut toast = SyncToast::new();
    toast.failed();
    assert!(toast.is_visible());

    toast.dismiss();
    assert!(!toast.is_visible());
}

#[test]
fn dismiss_is_a_no_op_when_not_failed() {
    let now = Instant::now();
    let mut toast = SyncToast::new();
    toast.succeeded(now);

    toast.dismiss();
    assert!(toast.is_visible(), "dismiss() should only clear a Failed toast");
}

#[test]
fn started_while_failed_replaces_it_with_syncing() {
    let mut toast = SyncToast::new();
    toast.failed();
    assert!(toast.is_visible());

    toast.started();
    assert!(toast.is_visible());
    assert!(toast.text().contains("Syncing"), "text was: {:?}", toast.text());
}

// --- should_auto_sync --------------------------------------------------------------

#[test]
fn should_auto_sync_never_when_interval_is_zero() {
    let now = Instant::now();
    let last = now - Duration::from_secs(3600);

    assert!(!should_auto_sync(Some(last), now, 0, false));
}

#[test]
fn should_auto_sync_never_while_a_sync_is_in_flight() {
    let now = Instant::now();
    let last = now - Duration::from_secs(3600);

    assert!(!should_auto_sync(Some(last), now, 5, true));
}

#[test]
fn should_auto_sync_false_when_never_synced() {
    let now = Instant::now();

    assert!(
        !should_auto_sync(None, now, 5, false),
        "the startup sync is triggered explicitly, not by the timer"
    );
}

#[test]
fn should_auto_sync_false_before_the_interval_elapses() {
    let now = Instant::now();
    let interval_minutes = 5;
    let last = now - Duration::from_secs(interval_minutes * 60 - 1);

    assert!(!should_auto_sync(Some(last), now, interval_minutes, false));
}

#[test]
fn should_auto_sync_true_once_the_interval_elapses() {
    let now = Instant::now();
    let interval_minutes = 5;
    let last = now - Duration::from_secs(interval_minutes * 60);

    assert!(should_auto_sync(Some(last), now, interval_minutes, false));
}

/// Regression test for a retry-storm bug: a sync that fails fast (e.g. a backend
/// resolution error, which resolves well within one ~100ms tick) must not cause the
/// auto-sync timer to fire again on the very next tick. The fix is that `AppComponent`
/// updates `last_sync_attempt_at` on *every* terminal outcome, not just success — so by
/// the time the failed attempt is recorded, `now` and the recorded attempt are the same
/// instant (the worst case: zero elapsed time), and the timer must still treat that as
/// "not yet".
#[test]
fn should_auto_sync_does_not_immediately_refire_after_a_same_instant_failed_attempt() {
    let now = Instant::now();
    let interval_minutes = 5;

    // The failed attempt is recorded at the same instant it's checked (elapsed == 0):
    // the worst case for a fast-failing sync (e.g. get_backend() resolution failure).
    assert!(
        !should_auto_sync(Some(now), now, interval_minutes, false),
        "a just-failed attempt must not immediately refire the timer"
    );

    // It also must not refire on ticks shortly after, only once a full interval has
    // actually elapsed since that failed attempt.
    let almost_there = now + Duration::from_secs(interval_minutes * 60 - 1);
    assert!(!should_auto_sync(Some(now), almost_there, interval_minutes, false));

    let interval_elapsed = now + Duration::from_secs(interval_minutes * 60);
    assert!(
        should_auto_sync(Some(now), interval_elapsed, interval_minutes, false),
        "a failed attempt should retry once, and only once, a full interval has passed"
    );
}

// --- sync_completed_successfully ----------------------------------------------------

#[test]
fn sync_completed_successfully_is_true_only_for_success() {
    assert!(sync_completed_successfully(&SyncStatus::Success));
}

#[test]
fn sync_completed_successfully_is_false_for_an_error_status() {
    assert!(!sync_completed_successfully(&SyncStatus::Error {
        message: "boom".to_string(),
    }));
}

#[test]
fn sync_completed_successfully_is_false_and_does_not_panic_for_unexpected_statuses() {
    // `Idle`/`InProgress` should never actually reach a "completed" handler, but this
    // must degrade defensively (treated as failure) rather than panicking.
    assert!(!sync_completed_successfully(&SyncStatus::Idle));
    assert!(!sync_completed_successfully(&SyncStatus::InProgress));
}

// --- render bounds -------------------------------------------------------------------

/// Renders a visible toast into a `task_list_area` covering the whole given terminal
/// size and asserts it doesn't panic. Returns the terminal so the caller can inspect
/// the buffer if needed.
fn render_into(width: u16, height: u16, toast: &SyncToast) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal should construct");
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, width, height);
            toast.render(f, area);
        })
        .expect("render should not panic on a small terminal");
    terminal
}

#[test]
fn render_does_not_panic_on_a_very_small_terminal() {
    let mut toast = SyncToast::new();
    toast.started();

    // Small enough that the 1-cell border inset leaves almost nothing to work with.
    for (width, height) in [(20, 5), (3, 3), (2, 2), (1, 1), (0, 0)] {
        render_into(width, height, &toast);
    }
}

#[test]
fn render_respects_the_1_cell_border_inset_on_a_small_terminal() {
    let mut toast = SyncToast::new();
    toast.started();

    let width = 20u16;
    let height = 5u16;
    let terminal = render_into(width, height, &toast);
    let buffer = terminal.backend().buffer();

    // The toast must be inset by 1 cell from the edge of `task_list_area` (that's
    // where the task list's own border lives), so the outermost ring of cells must
    // stay untouched: top row, bottom row, left column, right column.
    for x in 0..width {
        assert_eq!(buffer[(x, 0)].symbol(), " ", "top row must stay blank");
        assert_eq!(buffer[(x, height - 1)].symbol(), " ", "bottom row must stay blank");
    }
    for y in 0..height {
        assert_eq!(buffer[(0, y)].symbol(), " ", "left column must stay blank");
        assert_eq!(buffer[(width - 1, y)].symbol(), " ", "right column must stay blank");
    }
}

#[test]
fn render_is_a_no_op_when_hidden() {
    let toast = SyncToast::new();
    assert!(!toast.is_visible());

    let terminal = render_into(20, 5, &toast);
    let buffer = terminal.backend().buffer();
    for cell in buffer.content() {
        assert_eq!(cell.symbol(), " ", "a hidden toast should paint nothing");
    }
}
