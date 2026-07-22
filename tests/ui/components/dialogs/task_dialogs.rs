use ratatui::{backend::Backend, backend::TestBackend, Terminal};
use terminalist::ui::components::dialogs::task_dialogs::render_task_time_dialog;

#[test]
fn time_dialog_renders_input_and_places_cursor_after_it() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_task_time_dialog(frame, frame.area(), "2pm", 3))
        .unwrap();

    let cursor = terminal.backend_mut().get_cursor_position().unwrap();
    let buffer = terminal.backend().buffer();
    assert!(cursor.x >= 3, "cursor must leave room for the typed value");
    let before_cursor = (cursor.x - 3..cursor.x)
        .map(|x| buffer[(x, cursor.y)].symbol())
        .collect::<String>();

    assert_eq!(
        before_cursor, "2pm",
        "typed value must be visible immediately before the cursor"
    );
}
