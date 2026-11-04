pub fn term_width() -> usize {
    termsize::get().map(|s| s.cols as usize).unwrap_or(80)
}

pub fn term_rows() -> usize {
    termsize::get().map(|s| s.rows as usize).unwrap_or(24)
}

/// Fit `s` into `max` columns. Escape sequences are copied whole so a cut
/// string never leaves half an ANSI code behind.
pub fn truncate(s: &str, max: usize) -> String {
    if display_width(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let limit = max - 1;
    let mut out = String::new();
    let mut width = 0;
    let mut styled = false;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            out.push(c);
            styled = true;
            for n in chars.by_ref() {
                out.push(n);
                if n as u8 >= 0x40 && n as u8 <= 0x7e && n != '[' {
                    break;
                }
            }
            continue;
        }
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if width + cw > limit {
            break;
        }
        width += cw;
        out.push(c);
    }
    out.push('\u{2026}');
    if styled {
        out.push_str("\x1b[0m");
    }
    out
}

/// Move up over `lines` rows and clear each one.
///
/// The cursor must start on the row directly below the block. Moving before
/// clearing is what erases the top row; the old order cleared the row being
/// left, so the first line of every frame survived.
pub fn clear_block(lines: usize) -> String {
    let mut out = String::new();
    for _ in 0..lines {
        out.push_str("\x1b[1A\r\x1b[2K");
    }
    out
}

pub fn box_top(title: &str) -> String {
    box_top_w(title, term_width())
}

pub fn box_top_w(title: &str, w: usize) -> String {
    if w < 2 {
        return String::new();
    }
    let inner = w - 2;
    let title = truncate(title, inner.saturating_sub(4));
    if title.is_empty() {
        return format!("┌{}┐", "─".repeat(inner));
    }
    let head = format!("─ {} ", title);
    let tail = "─".repeat(inner.saturating_sub(display_width(&head)));
    format!("┌{head}{tail}┐")
}

pub fn box_bottom() -> String {
    box_bottom_w(term_width())
}

pub fn box_bottom_w(w: usize) -> String {
    if w < 2 {
        return String::new();
    }
    format!("└{}┘", "─".repeat(w - 2))
}

pub fn box_row(content: &str) -> String {
    box_row_w(content, term_width())
}

pub fn box_row_w(content: &str, w: usize) -> String {
    if w < 4 {
        return truncate(content, w);
    }
    // One space between each border and the content.
    let content_w = w - 4;
    let content = truncate(content, content_w);
    let pad = content_w.saturating_sub(display_width(&content));
    format!("│ {content}{} │", " ".repeat(pad))
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for n in chars.by_ref() {
                if n == '\x1b' { break; }
                if n as u8 >= 0x40 && n as u8 <= 0x7e && n != '[' { break; }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn display_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(strip_ansi(s).as_str())
}

pub fn progress_bar(percent: f64, width: usize) -> String {
    if width == 0 || !percent.is_finite() {
        return "─".repeat(width);
    }
    let full = (percent / 100.0).clamp(0.0, 1.0);
    let filled = (full * width as f64) as usize;
    if filled >= width {
        "━".repeat(width)
    } else {
        let done = "━".repeat(filled);
        let rest = "─".repeat(width - filled);
        format!("{done}{rest}")
    }
}
