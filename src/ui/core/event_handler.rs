use crossterm::event::{poll, Event, KeyEvent, MouseEvent};
use tokio::time::Duration;

/// Wait for the next terminal event, or return a `Tick` when the terminal is idle.
pub async fn next_event() -> anyhow::Result<EventType> {
    // Check for terminal events without blocking first
    if poll(Duration::from_millis(0))? {
        match crossterm::event::read()? {
            Event::Key(key) => {
                return Ok(EventType::Key(key));
            }
            Event::Mouse(mouse) => {
                return Ok(EventType::Mouse(mouse));
            }
            Event::Resize(w, h) => return Ok(EventType::Resize(w, h)),
            _ => return Ok(EventType::Other),
        }
    }

    // If no immediate event, wait a bit and return tick
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(EventType::Tick)
}

#[derive(Debug, Clone)]
pub enum EventType {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    Other,
}
