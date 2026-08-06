use crate::display::{draw, format as fmt, style};
use std::collections::HashMap;
use std::io::{BufRead, IsTerminal, Write};
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct FileProgress {
    pub downloaded: i64,
    pub total: i64,
}

pub fn pkg_name_from_filename(fname: &str) -> String {
    let base = fname.split(".pkg.tar.").next().unwrap_or(fname);
    let mut parts: Vec<&str> = base.rsplitn(4, '-').collect();
    parts.reverse();
    if parts.is_empty() { return base.to_string(); }
    parts[0].to_string()
}

pub fn confirm(prompt: &str) -> bool {
    if !std::io::stdin().is_terminal() {
        return false;
    }
    print!("{}", prompt);
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).ok();
    let line = line.trim();
    println!();
    line.is_empty() || line.eq_ignore_ascii_case("y") || line.eq_ignore_ascii_case("yes")
}

#[derive(Debug)]
pub struct DownloadState {
    pub rendered: usize,
    pub title: String,
    pub phase: bool,
    pub total_bytes: i64,
    pub completed_bytes: i64,
    pub total_files: usize,
    pub completed_files: usize,
    pub currently_downloading: Option<String>,
    pub completed_downloads: Vec<String>,
    pub file_to_pkg: HashMap<String, String>,
    pub start: Instant,
    pub last_render: Instant,
    pub log: Vec<String>,
    pub per_file: HashMap<String, FileProgress>,
}

impl DownloadState {
    pub fn best_progress(&self) -> Option<String> {
        self.file_to_pkg.keys()
            .filter_map(|fname| {
                let prog = self.per_file.get(fname).copied().unwrap_or(FileProgress { downloaded: 0, total: 0 });
                if prog.total > 0 && prog.downloaded < prog.total {
                    Some((fname, prog.downloaded as f64 / prog.total as f64))
                } else {
                    None
                }
            })
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(fname, _)| fname.clone())
    }

    pub fn render(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_render).as_millis() < 50 { return; }
        self.last_render = now;
        if std::io::stdout().is_terminal() {
            for _ in 0..self.rendered { print!("\r\x1b[K\x1b[1A"); }
        }
        self.rendered = 0;
        let out = self.build();
        for line in &out { println!("{line}"); }
        self.rendered = out.len();
        let _ = std::io::stdout().flush();
    }

    pub fn done(&mut self) {
        let r = self.rendered;
        if std::io::stdout().is_terminal() && r > 0 {
            for _ in 0..r { print!("\r\x1b[K\x1b[1A"); }
            print!("\r\x1b[K");
        }
        self.rendered = 0;
        let _ = std::io::stdout().flush();
    }

    pub fn next_phase(&mut self) {
        if self.total_files > 0 {
            self.rendered = 0;
            println!();
            println!(" {}", style::bold("Done downloading, proceeding with installation"));
            println!();
        } else {
            self.done();
        }
        let _ = std::io::stdout().flush();
    }

    pub fn eta(&self, elapsed: f64) -> String {
        if self.completed_bytes == 0 { return "-:--:--".to_string(); }
        let speed = self.completed_bytes as f64 / elapsed;
        if speed <= 0.0 { return "-:--:--".to_string(); }
        let remaining = (self.total_bytes - self.completed_bytes) as f64 / speed;
        let secs = remaining as u64;
        format!("{}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
    }

    pub fn speed(&self, elapsed: f64) -> String {
        if self.completed_bytes == 0 { return "0 B/s".to_string(); }
        let bps = self.completed_bytes as f64 / elapsed;
        fmt::format_speed(bps)
    }

    pub fn dl_status(&self) -> String {
        let total = self.total_bytes.max(1);
        let (unit, suffix) = fmt::pick_unit(total);
        let cv = self.completed_bytes as f64 / unit as f64;
        let tv = total as f64 / unit as f64;
        let prec = if unit == 1 { 0 } else { 1 };
        format!("{cv:.prec$}/{tv:.prec$} {suffix}", prec = prec)
    }

    pub fn build(&self) -> Vec<String> {
        let w = draw::term_width();
        let mut out = Vec::new();

        if self.total_files > 0 && !self.phase {
            out.push(draw::box_top(&style::bold("Download")));
            let start = self.completed_downloads.len().saturating_sub(5);
            for name in &self.completed_downloads[start..] {
                out.push(draw::box_row(&format!("{} {}", style::green("Completed:"), name)));
            }
            if let Some(fname) = &self.currently_downloading {
                out.push(draw::box_row(&format!("{} {}", style::cyan("Downloading:"), fname)));
            } else if self.completed_downloads.is_empty() {
                out.push(draw::box_row(&style::cyan("Starting Downloads...")));
            }
            let pct = if self.total_bytes > 0 { self.completed_bytes as f64 / self.total_bytes as f64 * 100.0 } else { 0.0 };
            let inner = w.saturating_sub(4);
            let elapsed = self.start.elapsed().as_secs_f64();
            let prefix = format!("{} {} [", style::cyan("Time Remaining:"), self.eta(elapsed));
            let suffix = format!("] {} {} {} {} {}",
                style::color(&format!("{:>6.1}%", pct), style::Color::Blue),
                style::bold("\u{2022}"), style::green(&self.dl_status()),
                style::bold("\u{2022}"), style::color(&self.speed(elapsed), style::Color::Blue));
            let prefix_w = draw::display_width(&prefix);
            let suffix_w = draw::display_width(&suffix);
            let bar_w = inner.saturating_sub(prefix_w + suffix_w).max(4);
            let bar = draw::progress_bar(pct, bar_w);
            out.push(draw::box_row(&format!("{}{}{}", prefix, bar, suffix)));
        } else if !self.log.is_empty() {
            out.push(draw::box_top(&self.title));
            let start = self.log.len().saturating_sub(5);
            for entry in &self.log[start..] { out.push(draw::box_row(entry)); }
        } else {
            out.push(draw::box_top(&self.title));
            out.push(draw::box_row(&format!("{}...", style::dim("Preparing"))));
        }

        out.push(draw::box_bottom());
        out
    }
}
