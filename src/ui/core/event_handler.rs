use crossterm::event::{poll, Event, KeyEvent};
use tokio::time::{interval, Duration, Instant};

pub struct EventHandler {
    #[allow(dead_code)]
    tick_interval: tokio::time::Interval,
    #[allow(dead_code)]
    render_interval: tokio::time::Interval,
    last_render_time: Instant,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            tick_interval: interval(Duration::from_millis(100)), // 10 Hz for application ticks
            render_interval: interval(Duration::from_millis(16)), // ~60 FPS render rate
            last_render_time: Instant::now(),
        }
    }

    pub async fn next_event(&mut self) -> anyhow::Result<EventType> {
        // Check for terminal events without blocking first
        if poll(Duration::from_millis(0))? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    return Ok(EventType::Key(key));
                }
                Event::Resize(w, h) => return Ok(EventType::Resize(w, h)),
                _ => return Ok(EventType::Other),
            }
        }

        // If no immediate event, wait a bit and return tick
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(EventType::Tick)
    }

    /// Get the time since last render for frame timing
    pub fn time_since_last_render(&self) -> Duration {
        self.last_render_time.elapsed()
    }

    /// Check if we should render based on timing
    pub fn should_render(&self) -> bool {
        self.time_since_last_render() >= Duration::from_millis(16) // Cap at ~60 FPS
    }
}

#[derive(Debug, Clone)]
pub enum EventType {
    Key(KeyEvent),
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
