use super::actions::Action;
use crossterm::event::{Event, KeyEvent};
use ratatui::{layout::Rect, Frame};

pub trait Component {
    fn init(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn handle_events(&mut self, event: Option<Event>) -> Action {
        if let Some(Event::Key(key)) = event {
            self.handle_key_events(key)
        } else {
            Action::None
        }
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Action;

    fn update(&mut self, action: Action) -> Action {
        // Default implementation passes action through
        action
    }

    fn render(&mut self, f: &mut Frame, rect: Rect);

    // Optional lifecycle methods
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}
}
