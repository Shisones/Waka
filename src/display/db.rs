use crate::display::{draw, format as fmt};
use std::collections::{HashMap, HashSet};
use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

pub struct DbSyncState {
    pub bytes: HashMap<String, (i64, i64)>,
    active: HashSet<String>,
    rendered: usize,
    last_size: Option<(usize, usize)>,
    resize: crate::display::resize::Resize,
    last_render: Instant,
}

impl Default for DbSyncState {
    fn default() -> Self {
        DbSyncState {
            bytes: HashMap::new(),
            active: HashSet::new(),
            rendered: 0,
            last_size: None,
            resize: crate::display::resize::Resize::default(),
            last_render: Instant::now() - Duration::from_millis(200),
        }
    }
}

impl DbSyncState {
    pub fn update(&mut self, fname: &str, downloaded: i64, total: i64) {
        let fname = fname.rsplit('/').next().unwrap_or(fname);
        let repo = fname.strip_suffix(".db.part").or_else(|| fname.strip_suffix(".db")).unwrap_or(fname);
        if !self.bytes.contains_key(repo) { return; }
        self.bytes.insert(repo.to_string(), (downloaded, total));
        self.active.insert(repo.to_string());
        self.render();
    }

    pub fn active_repos(&self) -> Vec<String> {
        let mut repos: Vec<String> = self.active.iter().cloned().collect();
        repos.sort();
        repos
    }

    pub fn render(&mut self) {
        let now = Instant::now();
        if !std::io::stdout().is_terminal() { return; }
        let size = (draw::term_width(), draw::term_rows());
        if self.last_size != Some(size) {
            self.last_size = Some(size);
            if self.resize.changed(self.rendered, now) {
                return;
            }
        }
        if self.resize.is_waiting() {
            if !self.resize.settled(now) {
                return;
            }
            println!();
            self.rendered = 0;
        }
        if now.duration_since(self.last_render).as_millis() < 50 { return; }
        self.last_render = now;
        print!("{}", draw::clear_block(self.rendered));
        self.rendered = 0;
        let lines = self.live_lines();
        let cap = draw::term_rows().saturating_sub(1).max(1);
        let shown = lines.len().min(cap);
        for line in lines.iter().take(shown) { print!("{line}\x1b[K\r\n"); }
        print!("\x1b[J");
        self.rendered = shown;
        let _ = std::io::stdout().flush();
    }

    pub fn done(&mut self) {
        if self.rendered > 0 && std::io::stdout().is_terminal() {
            print!("{}\x1b[J", draw::clear_block(self.rendered));
            let _ = std::io::stdout().flush();
        }
        self.rendered = 0;
    }

    fn live_lines(&self) -> Vec<String> {
        let w = draw::term_width();
        let mut names: Vec<String> = self.active.iter().cloned().collect();
        names.sort();
        let max_nw = names.iter().map(|n| draw::display_width(n)).max().unwrap_or(1).max(1);
        let max_sw = names.iter()
            .map(|n| {
                let (dl, _) = self.bytes.get(n).copied().unwrap_or((0, 0));
                if dl > 0 { draw::display_width(&fmt::format_size(dl)) } else { 3 }
            })
            .max()
            .unwrap_or(3);
        names.iter().map(|n| {
            let (dl, tot) = self.bytes.get(n).copied().unwrap_or((0, 0));
            bar_line(n, dl, tot, max_nw, max_sw, w, 0.0)
        }).collect()
    }
}

pub fn bar_line(name: &str, dl: i64, tot: i64, max_nw: usize, max_sw: usize, w: usize, unknown_pct: f64) -> String {
    let pct = if tot > 0 { (dl as f64 / tot as f64 * 100.0).min(100.0) } else { unknown_pct };
    let size_str = if dl > 0 { fmt::format_size(dl) } else { "0 B".to_string() };
    let padded_size = format!("{:>width$}", size_str, width = max_sw);
    let npad = " ".repeat(max_nw.saturating_sub(draw::display_width(name)));
    let name_prefix = format!(" {}{} ", name, npad);

    if w < 3 {
        return draw::truncate("[━]", w);
    }

    // The bar and its size stay together on the right edge, with one free
    // column of padding after the size.
    let size_w = draw::display_width(&padded_size);
    let show_size = w >= size_w + 5;
    let reserved = if show_size {
        size_w + 5
    } else if w >= 4 {
        4
    } else {
        3
    };
    let name_room = w.saturating_sub(reserved);
    let name_prefix = if draw::display_width(&name_prefix) > name_room {
        draw::truncate(&name_prefix, name_room)
    } else {
        name_prefix
    };
    let name_w = draw::display_width(&name_prefix);
    let max_bar = ((w as f64 * 0.25) as usize).min(50).max(1);

    if show_size {
        let bar_space = w.saturating_sub(name_w + size_w + 4);
        let bar_w = bar_space.min(max_bar).max(1);
        let filled = (((pct / 100.0) * bar_w as f64) as usize).min(bar_w);
        let bar = format!("{}{}", "━".repeat(filled), "─".repeat(bar_w - filled));
        let gap = w.saturating_sub(name_w + bar_w + size_w + 4);
        format!("{name_prefix}{}{padded_size} [{bar}] ", " ".repeat(gap))
    } else if w >= 4 {
        let bar_space = w.saturating_sub(name_w + 3);
        let bar_w = bar_space.min(max_bar).max(1);
        let filled = (((pct / 100.0) * bar_w as f64) as usize).min(bar_w);
        let bar = format!("{}{}", "━".repeat(filled), "─".repeat(bar_w - filled));
        let gap = w.saturating_sub(name_w + bar_w + 3);
        format!("{name_prefix}{}[{bar}] ", " ".repeat(gap))
    } else {
        let bar_w = max_bar.min(w.saturating_sub(2)).max(1);
        let filled = (((pct / 100.0) * bar_w as f64) as usize).min(bar_w);
        let bar = format!("{}{}", "━".repeat(filled), "─".repeat(bar_w - filled));
        format!("[{bar}]")
    }
}

pub fn db_sync_lines(bytes: &HashMap<String, (i64, i64)>, names: &[String]) -> Vec<String> {
    let w = draw::term_width();
    let max_nw = names.iter().map(|n| draw::display_width(n)).max().unwrap_or(1).max(1);
    let max_sw = names.iter()
        .map(|n| {
            let (dl, _) = bytes.get(n).copied().unwrap_or((0, 0));
            if dl > 0 { draw::display_width(&fmt::format_size(dl)) } else { 3 }
        })
        .max()
        .unwrap_or(3);
    names.iter().map(|n| {
        let (dl, tot) = bytes.get(n).copied().unwrap_or((0, 0));
        bar_line(n, dl, tot, max_nw, max_sw, w, 100.0)
    }).collect()
}
