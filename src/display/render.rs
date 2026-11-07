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
    confirm_default(prompt, true)
}

pub fn confirm_no(prompt: &str) -> bool {
    confirm_default(prompt, false)
}

pub fn confirm_default(prompt: &str, default: bool) -> bool {
    if !std::io::stdin().is_terminal() {
        return false;
    }
    print!("{}", prompt);
    std::io::stdout().flush().ok();
    let mut line = String::new();
    let read = std::io::stdin().lock().read_line(&mut line);
    let line = line.trim();
    println!();
    if !matches!(read, Ok(n) if n > 0) {
        return false;
    }
    if line.is_empty() {
        return default;
    }
    line.eq_ignore_ascii_case("y") || line.eq_ignore_ascii_case("yes")
}

/// Numbered choice. Returns the zero-based index.
pub fn select(prompt: &str, options: &[String], default: usize) -> usize {
    if !std::io::stdin().is_terminal() || options.is_empty() {
        return default.min(options.len().saturating_sub(1));
    }
    println!("{prompt}");
    for (index, option) in options.iter().enumerate() {
        println!("  {}) {option}", index + 1);
    }
    print!("
 Enter a number (default={}): ", default + 1);
    std::io::stdout().flush().ok();
    let mut line = String::new();
    let read = std::io::stdin().lock().read_line(&mut line);
    println!();
    if !matches!(read, Ok(n) if n > 0) {
        return default.min(options.len().saturating_sub(1));
    }
    line.trim()
        .parse::<usize>()
        .ok()
        .filter(|n| *n >= 1 && *n <= options.len())
        .map(|n| n - 1)
        .unwrap_or(default.min(options.len().saturating_sub(1)))
}

/// True when a command would have to ask and stdin cannot answer.
pub fn confirmation_unavailable(assume_yes: bool, stdin_is_tty: bool) -> bool {
    !assume_yes && !stdin_is_tty
}

/// Fail before doing any work when a prompt is required but unavailable.
pub fn require_confirmation_possible(assume_yes: bool) -> anyhow::Result<()> {
    if confirmation_unavailable(assume_yes, std::io::stdin().is_terminal()) {
        return Err(anyhow::anyhow!("cannot prompt without a TTY; pass -y"));
    }
    Ok(())
}

/// Bytes charged to the transaction, capped at its size.
pub fn capped_bytes(raw: i64, cap: Option<i64>) -> i64 {
    let raw = raw.max(0);
    match cap {
        Some(cap) => raw.min(cap.max(0)),
        None => raw,
    }
}

#[derive(Debug)]
pub struct DownloadState {
    pub rendered: usize,
    pub last_size: Option<(usize, usize)>,
    pub resize: crate::display::resize::Resize,
    pub plain_log: usize,
    pub title: String,
    pub phase: bool,
    pub total_bytes: i64,
    pub completed_bytes: i64,
    pub total_files: usize,
    pub total_pkgs: usize,
    pub completed_files: usize,
    pub currently_downloading: Option<String>,
    pub completed_downloads: Vec<String>,
    pub file_to_pkg: HashMap<String, String>,
    pub start: Instant,
    pub last_render: Instant,
    pub log: Vec<String>,
    pub log_rows: usize,
    pub per_file: HashMap<String, FileProgress>,
    pub pkg_sizes: HashMap<String, i64>,
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
        if !std::io::stdout().is_terminal() {
            for entry in self.pending_log() {
                println!(" {entry}");
            }
            self.plain_log = self.log.len();
            return;
        }
        if std::io::stdout().is_terminal() {
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
                // A reflowed frame cannot be erased by row count. Start a new
                // block.
                println!();
                self.rendered = 0;
            }
        }
        if now.duration_since(self.last_render).as_millis() < 50 { return; }
        self.last_render = now;
        let tty = std::io::stdout().is_terminal();
        if tty {
            print!("{}", draw::clear_block(self.rendered));
        }
        self.rendered = 0;
        let out = self.build();
        if tty {
            for line in &out { print!("{line}\x1b[K\r\n"); }
            print!("\x1b[J");
        } else {
            for line in &out { println!("{line}"); }
        }
        self.rendered = out.len();
        let _ = std::io::stdout().flush();
    }

    /// Log lines a piped run has not printed yet.
    pub fn pending_log(&self) -> &[String] {
        let start = self.plain_log.min(self.log.len());
        &self.log[start..]
    }

    pub fn done(&mut self) {
        let r = self.rendered;
        if std::io::stdout().is_terminal() && r > 0 {
            print!("{}\x1b[J", draw::clear_block(r));
        }
        self.rendered = 0;
        let _ = std::io::stdout().flush();
    }

    pub fn next_phase(&mut self) {
        if self.total_files > 0 {
            // Show the completed bar before the install phase.
            self.completed_bytes = self.total_bytes;
            self.completed_files = self.total_pkgs.max(self.total_files);
            self.last_render = Instant::now() - std::time::Duration::from_millis(100);
            self.render();
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
        self.build_at(draw::term_width(), draw::term_rows())
    }

    pub fn build_at(&self, w: usize, rows: usize) -> Vec<String> {
        let body_cap = rows.saturating_sub(3);
        let mut out = Vec::new();

        if self.total_files > 0 && !self.phase {
            // The bar and its detail line take two rows.
            let body_cap = body_cap.saturating_sub(1);
            out.push(draw::box_top_w(&style::bold("Download"), w));
            let mut body: Vec<String> = Vec::new();
            let start = self.completed_downloads.len().saturating_sub(5);
            for name in &self.completed_downloads[start..] {
                body.push(draw::box_row_w(&format!("{} {}", style::green("Completed:"), name), w));
            }
            if let Some(fname) = &self.currently_downloading {
                let name = self.file_to_pkg.get(fname).map(String::as_str).unwrap_or(fname.as_str());
                body.push(draw::box_row_w(&format!("{} {}", style::cyan("Downloading:"), name), w));
            } else if self.completed_downloads.is_empty() {
                body.push(draw::box_row_w(&style::cyan("Starting Downloads..."), w));
            }
            let skip = body.len().saturating_sub(body_cap);
            out.extend(body.into_iter().skip(skip));
            out.extend(self.progress_rows(w));
        } else if !self.log.is_empty() {
            out.push(draw::box_top_w(&self.title, w));
            let start = self.log.len().saturating_sub(self.log_rows.min(body_cap));
            for entry in &self.log[start..] {
                out.push(draw::box_row_w(entry, w));
            }
        } else {
            out.push(draw::box_top_w(&self.title, w));
            out.push(draw::box_row_w(&format!("{}...", style::dim("Preparing")), w));
        }

        out.push(draw::box_bottom_w(w));
        out
    }

    /// Bar with the percentage on the left; ETA, size, and speed below.
    fn progress_rows(&self, w: usize) -> Vec<String> {
        let pct = if self.total_bytes > 0 {
            (self.completed_bytes as f64 / self.total_bytes as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        let content_w = w.saturating_sub(4);
        let pct_s = format!("{pct:.1}%");
        let pct_w = draw::display_width(&pct_s);
        let mut out = Vec::new();

        if content_w >= pct_w + 8 {
            let bar = draw::progress_bar(pct, content_w - pct_w - 3);
            out.push(draw::box_row_w(&format!("{} [{bar}]", style::bold(&pct_s)), w));
        } else {
            out.push(draw::box_row_w(&style::bold(&pct_s), w));
        }

        let elapsed = self.start.elapsed().as_secs_f64();
        let eta = style::cyan(&format!("ETA {}", self.eta(elapsed)));
        let packages = style::green(&format!(
            "{}/{} Packages",
            self.completed_files.min(self.total_pkgs.max(self.total_files)),
            self.total_pkgs.max(self.total_files)
        ));
        let size = style::green(&self.dl_status());
        let speed = style::cyan(&self.speed(elapsed));

        let left_full = format!("{eta} \u{2022} {packages}");
        let right_full = format!("{size} \u{2022} {speed}");
        let candidates = [
            (left_full.clone(), right_full),
            (left_full, size),
            (format!("{eta} \u{2022} {packages}"), String::new()),
            (eta, String::new()),
        ];
        let (left, right) = candidates
            .into_iter()
            .find(|(left, right)| {
                draw::display_width(left) + 2 + draw::display_width(right) <= content_w
            })
            .unwrap_or_default();

        if left.is_empty() && right.is_empty() {
            return out;
        }
        if right.is_empty() {
            out.push(draw::box_row_w(&left, w));
        } else {
            let gap = content_w.saturating_sub(draw::display_width(&left) + draw::display_width(&right));
            out.push(draw::box_row_w(&format!("{left}{}{right}", " ".repeat(gap)), w));
        }
        out
    }
}

