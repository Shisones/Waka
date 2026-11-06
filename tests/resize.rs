use std::time::{Duration, Instant};
use waka::display::resize::{Resize, SETTLE};

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
