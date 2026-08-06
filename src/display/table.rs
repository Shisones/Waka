use crate::display;

pub fn pad(text: &str, target_width: usize, right_align: bool) -> String {
    let text_width = display::draw::display_width(text);
    if text_width >= target_width {
        let stripped = display::draw::strip_ansi(text);
        let mut cropped = String::new();
        let mut cw = 0usize;
        let limit = target_width.saturating_sub(1);
        for c in stripped.chars() {
            let dw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if cw + dw > limit { break; }
            cw += dw;
            cropped.push(c);
        }
        return cropped + "\u{2026}";
    }
    let padding = " ".repeat(target_width - text_width);
    if right_align { padding + text } else { text.to_string() + &padding }
}

fn term_width() -> usize {
    display::draw::term_width()
}

pub fn write_separator() {
    let w = term_width();
    println!(" {} ", "\u{2500}".repeat(w.saturating_sub(2)));
}

pub fn write_summary_row(name: &str, old_ver: &str, new_ver: &str, net_change: &str, size: &str) {
    let w = term_width();
    let avail = w.saturating_sub(6);
    let name_col = (avail as f64 * 0.26) as usize;
    let ver_col = (avail as f64 * 0.16) as usize;
    let net_col = (avail as f64 * 0.26) as usize;
    let size_col = avail.saturating_sub(name_col + ver_col + ver_col + net_col);
    println!(" {} {} {} {} {} ",
        pad(name, name_col, false),
        pad(old_ver, ver_col, false),
        pad(new_ver, ver_col, false),
        pad(net_change, net_col, true),
        pad(size, size_col, true));
}

pub fn write_summary_footer(items: &[(&str, &str)]) {
    let max_label_w = items.iter().map(|(l, _)| display::draw::display_width(l) + 1).max().unwrap_or(0);
    let max_value_w = items.iter().map(|(_, v)| display::draw::display_width(v)).max().unwrap_or(0);
    for (label, value) in items {
        let label_colon = format!("{}:", label);
        let label_w = display::draw::display_width(&label_colon);
        let label_pad = " ".repeat(max_label_w.saturating_sub(label_w));
        let value_pad = " ".repeat(max_value_w.saturating_sub(display::draw::display_width(value)));
        println!(" {}{} {} ",
            crate::display::style::bold(&label_colon),
            label_pad, value_pad + value);
    }
}

pub fn plural(word: &str, count: usize) -> String {
    if count == 1 { word.to_string() } else { format!("{}s", word) }
}
