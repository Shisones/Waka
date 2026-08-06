use crate::display::style;

const KIB: i64 = 1024;
const MIB: i64 = KIB * 1024;
const GIB: i64 = MIB * 1024;

fn unit_for(bytes: i64) -> (i64, &'static str) {
    if bytes >= GIB { (GIB, "GiB") }
    else if bytes >= MIB { (MIB, "MiB") }
    else if bytes >= KIB { (KIB, "KiB") }
    else { (1, "B") }
}

pub fn format_size(bytes: i64) -> String {
    if bytes == 0 { return "0 B".to_string(); }
    let prefix = if bytes < 0 { "-" } else { "" };
    let abs_bytes = bytes.checked_abs().unwrap_or(i64::MAX);
    let (unit, suffix) = unit_for(abs_bytes);
    let val = abs_bytes as f64 / unit as f64;
    if val < 10.0 { format!("{}{:.1} {suffix}", prefix, val) }
    else { format!("{}{:.0} {suffix}", prefix, val) }
}

pub fn format_speed(bps: f64) -> String {
    if bps >= GIB as f64 { format!("{:.1} GiB/s", bps / GIB as f64) }
    else if bps >= MIB as f64 { format!("{:.1} MiB/s", bps / MIB as f64) }
    else if bps >= KIB as f64 { format!("{:.1} KiB/s", bps / KIB as f64) }
    else { format!("{:.0} B/s", bps) }
}

pub fn formatted_version(v: &alpm::Ver) -> String {
    format!("({})", style::color(v.as_str(), style::Color::Blue))
}

pub fn pick_unit(total: i64) -> (i64, &'static str) {
    unit_for(total)
}
