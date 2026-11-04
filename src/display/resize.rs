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

#[cfg(test)]
mod tests {
    use super::{Resize, SETTLE};
    use std::time::{Duration, Instant};

    #[test]
    fn holds_until_the_size_settles() {
        let start = Instant::now();
        let mut resize = Resize::default();
        assert!(resize.changed(3, start));
        assert!(!resize.settled(start + Duration::from_millis(100)));
        assert!(resize.is_waiting());
        assert!(resize.settled(start + SETTLE));
        assert!(!resize.is_waiting());
    }

    #[test]
    fn a_new_size_restarts_the_timer() {
        let start = Instant::now();
        let mut resize = Resize::default();
        resize.changed(3, start);
        let later = start + Duration::from_millis(200);
        assert!(resize.changed(3, later));
        assert!(!resize.settled(start + Duration::from_millis(300)));
        assert!(resize.settled(later + SETTLE));
    }

    #[test]
    fn nothing_on_screen_means_nothing_to_wait_for() {
        let mut resize = Resize::default();
        assert!(!resize.changed(0, Instant::now()));
        assert!(!resize.is_waiting());
    }
}
