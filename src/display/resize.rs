use std::time::{Duration, Instant};

/// How long the terminal size must stay put before a new frame is drawn.
pub const SETTLE: Duration = Duration::from_millis(250);

/// Tracks terminal resizes.
///
/// A wider frame that the terminal reflowed into several rows cannot be
/// cleared by moving up a stored line count, so the old block is abandoned
/// instead of erased. Drawing is paused while the size keeps changing so a
/// drag does not leave one stale frame per pixel.
#[derive(Debug, Default)]
pub struct Resize {
    since: Option<Instant>,
}

impl Resize {
    /// Call when the size differs from the last frame. Returns true when the
    /// caller should hold off painting until the size settles.
    pub fn changed(&mut self, drawn: usize, now: Instant) -> bool {
        if drawn == 0 {
            return false;
        }
        self.since = Some(now);
        true
    }

    /// True once the size has been stable long enough to start a new block.
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
