use std::io::IsTerminal;
use std::sync::OnceLock;

static NO_COLOR: OnceLock<bool> = OnceLock::new();

pub fn colors_enabled(stdout_tty: bool, no_color_env: bool) -> bool {
    stdout_tty && !no_color_env
}

fn no_color() -> bool {
    *NO_COLOR.get_or_init(|| {
        !colors_enabled(
            std::io::stdout().is_terminal(),
            std::env::var("NO_COLOR").is_ok(),
        )
    })
}

const RESET: &str = "\x1b[0m";

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Cyan,
}

impl Color {
    pub fn code(&self) -> &'static str {
        match self {
            Color::Red => "1;31",
            Color::Green => "1;32",
            Color::Yellow => "1;33",
            Color::Blue => "1;34",
            Color::Cyan => "1;36",
        }
    }
}

pub fn color(text: &str, c: Color) -> String {
    if no_color() { text.to_string() } else { format!("\x1b[{}m{text}{RESET}", c.code()) }
}

pub fn green(s: &str) -> String { color(s, Color::Green) }
pub fn red(s: &str) -> String { color(s, Color::Red) }
pub fn yellow(s: &str) -> String { color(s, Color::Yellow) }
pub fn cyan(s: &str) -> String { color(s, Color::Cyan) }

pub fn bold(s: &str) -> String {
    if no_color() { s.to_string() } else { format!("\x1b[1m{s}{RESET}") }
}

pub fn dim(s: &str) -> String {
    if no_color() { s.to_string() } else { format!("\x1b[2m{s}{RESET}") }
}

pub fn error(s: &str) -> String {
    format!("{} {s}", color("error:", Color::Red))
}
