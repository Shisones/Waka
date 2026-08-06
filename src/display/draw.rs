pub fn term_width() -> usize {
    termsize::get().map(|s| s.cols).unwrap_or(80) as usize
}

pub fn box_top(title: &str) -> String {
    let w = term_width();
    let inner = w.saturating_sub(2);
    if title.is_empty() {
        format!("┌{}┐", "─".repeat(inner))
    } else {
        let head = format!("─ {} ", title);
        let head_w = display_width(&head);
        let tail = "─".repeat(inner.saturating_sub(head_w));
        format!("┌{}{}┐", head, tail)
    }
}

pub fn box_bottom() -> String {
    let w = term_width();
    let inner = w.saturating_sub(2);
    format!("└{}┘", "─".repeat(inner))
}

pub fn box_row(content: &str) -> String {
    let w = term_width();
    let inner = w.saturating_sub(2);
    let disp_len = display_width(content);
    if disp_len + 1 >= inner {
        let avail = inner.saturating_sub(2);
        let mut cropped = String::new();
        let mut cw = 0usize;
        for c in content.chars() {
            let dw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if cw + dw > avail.saturating_sub(1) { break; }
            cw += dw;
            cropped.push(c);
        }
        format!("│ {}\u{2026} │", cropped)
    } else {
        format!("│ {}{}│", content, " ".repeat(inner - disp_len - 1))
    }
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
    if !percent.is_finite() { return "─".repeat(width); }
    let full = percent / 100.0;
    let filled = (full * width as f64) as usize;
    if filled >= width {
        "━".repeat(width)
    } else {
        let done = "━".repeat(filled);
        let rest = "─".repeat(width - filled);
        format!("{done}{rest}")
    }
}
