use crate::display;

pub fn pad(text: &str, target_width: usize, right_align: bool) -> String {
    let text_width = display::draw::display_width(text);
    if text_width > target_width {
        return display::draw::truncate(text, target_width);
    }
    let padding = " ".repeat(target_width - text_width);
    if right_align { padding + text } else { text.to_string() + &padding }
}

fn term_width() -> usize {
    display::draw::term_width()
}

pub fn write_separator() {
    let w = term_width();
    if w < 3 {
        println!("{}", "\u{2500}".repeat(w));
        return;
    }
    println!(" {} ", "\u{2500}".repeat(w - 2));
}

/// Column set shrinks with the terminal: old version goes first, then net
/// change, then size. Name and new version always stay.
pub fn summary_row_line(name: &str, old_ver: &str, new_ver: &str, net_change: &str, size: &str, w: usize) -> String {
    if w < 3 {
        return display::draw::truncate(name, w);
    }
    let (count, widths): (usize, Vec<usize>) = if w >= 40 {
        let avail = w - 6;
        let name_w = avail * 26 / 100;
        let ver_w = avail * 16 / 100;
        let net_w = avail * 26 / 100;
        let size_w = avail.saturating_sub(name_w + ver_w * 2 + net_w);
        (5, vec![name_w, ver_w, ver_w, net_w, size_w])
    } else if w >= 33 {
        let avail = w - 5;
        let name_w = avail * 30 / 100;
        let ver_w = avail * 20 / 100;
        let net_w = avail * 28 / 100;
        (4, vec![name_w, ver_w, net_w, avail.saturating_sub(name_w + ver_w + net_w)])
    } else if w >= 25 {
        let avail = w - 4;
        let name_w = avail * 34 / 100;
        let ver_w = avail * 22 / 100;
        (3, vec![name_w, ver_w, avail.saturating_sub(name_w + ver_w)])
    } else if w >= 17 {
        let avail = w - 3;
        let name_w = avail * 62 / 100;
        (2, vec![name_w, avail.saturating_sub(name_w)])
    } else {
        (1, vec![w - 2])
    };

    let all = [name, old_ver, new_ver, net_change, size];
    let idx: &[usize] = match count {
        5 => &[0, 1, 2, 3, 4],
        4 => &[0, 2, 3, 4],
        3 => &[0, 2, 4],
        2 => &[0, 2],
        _ => &[0],
    };
    let cells: Vec<String> = idx
        .iter()
        .zip(widths)
        .map(|(&i, width)| pad(all[i], width, i == 3 || i == 4))
        .collect();
    format!(" {} ", cells.join(" "))
}

pub fn write_summary_row(name: &str, old_ver: &str, new_ver: &str, net_change: &str, size: &str) {
    println!("{}", summary_row_line(name, old_ver, new_ver, net_change, size, term_width()));
}

pub fn write_summary_footer(items: &[(&str, &str)]) {
    let w = term_width();
    let max_label_w = items.iter().map(|(l, _)| display::draw::display_width(l) + 1).max().unwrap_or(0);
    let max_value_w = items.iter().map(|(_, v)| display::draw::display_width(v)).max().unwrap_or(0);
    for (label, value) in items {
        let label_colon = format!("{}:", label);
        let label_w = display::draw::display_width(&label_colon);
        let label_pad = " ".repeat(max_label_w.saturating_sub(label_w));
        let value_pad = " ".repeat(max_value_w.saturating_sub(display::draw::display_width(value)));
        let line = format!(" {}{} {} ",
            crate::display::style::bold(&label_colon),
            label_pad, value_pad + value);
        println!("{}", display::draw::truncate(&line, w));
    }
}

pub fn plural(word: &str, count: usize) -> String {
    if count == 1 { word.to_string() } else { format!("{}s", word) }
}
