use std::time::{Duration, Instant};

/// How long the terminal size must stay put before a new frame is drawn.
pub const SETTLE: Duration = Duration::from_millis(250);

/// Delay redraws while the terminal is resizing.
#[derive(Debug, Default)]
pub struct Resize {
    since: Option<Instant>,
}

impl Resize {
    /// Returns true when the caller should wait for the size to settle.
    pub fn changed(&mut self, drawn: usize, now: Instant) -> bool {
        if drawn == 0 {
            return false;
        }
        self.since = Some(now);
        true
    }

    /// True once the size has been stable long enough.
    pub fn settled(&mut self, now: Instant) -> bool {
        match self.since {
            Some(since) if now.duration_since(since) >= SETTLE => {
                self.since = None;
                true
            }
            _ => false,
        }
    }

    pub fn is_waiting(&self) -> bool {
        self.since.is_some()
    }
}
