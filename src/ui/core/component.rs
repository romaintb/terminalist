use super::actions::Action;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

pub trait Component {
    fn handle_key_events(&mut self, key: KeyEvent) -> Action;

    fn update(&mut self, action: Action) -> Action {
        // Default implementation passes action through
        action
    }

    fn render(&mut self, f: &mut Frame, rect: Rect);
}
