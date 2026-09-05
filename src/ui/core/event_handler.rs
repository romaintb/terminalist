use crossterm::event::{poll, Event, KeyEvent, MouseEvent};
use tokio::time::{interval, Duration};

pub struct EventHandler {
    #[allow(dead_code)]
    tick_interval: tokio::time::Interval,
    #[allow(dead_code)]
    render_interval: tokio::time::Interval,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            tick_interval: interval(Duration::from_millis(100)), // 10 Hz for application ticks
            render_interval: interval(Duration::from_millis(16)), // ~60 FPS render rate
        }
    }

    pub async fn next_event(&mut self) -> anyhow::Result<EventType> {
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
}

#[derive(Debug, Clone)]
pub enum EventType {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    Render,
    Other,
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
