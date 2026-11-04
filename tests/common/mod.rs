use std::collections::HashMap;
use std::time::Instant;
use waka::display::render::DownloadState;

pub fn download_state(completed_bytes: i64, total_bytes: i64) -> DownloadState {
    DownloadState {
        rendered: 0,
        last_size: None,
        resize: Default::default(),
        title: String::new(),
        phase: false,
        total_bytes,
        completed_bytes,
        total_files: 0,
        completed_files: 0,
        currently_downloading: None,
        completed_downloads: Vec::new(),
        file_to_pkg: HashMap::new(),
        start: Instant::now(),
        last_render: Instant::now(),
        log: Vec::new(),
        per_file: HashMap::new(),
    }
}