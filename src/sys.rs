use anyhow::Result;

pub fn root_hint(args: &[String]) -> String {
    if args.is_empty() {
        "this command needs root; run it with sudo".to_string()
    } else {
        format!("this command needs root; run `sudo waka {}`", args.join(" "))
    }
}


pub fn require_root() -> Result<()> {
    if unsafe { libc::geteuid() } == 0 {
        return Ok(());
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    Err(anyhow::anyhow!("{}", root_hint(&args)))
}


/// Shell-style match for HoldPkg entries.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    use std::ffi::CString;
    let Ok(pattern) = CString::new(pattern) else { return false; };
    let Ok(text) = CString::new(text) else { return false; };
    unsafe { libc::fnmatch(pattern.as_ptr(), text.as_ptr(), 0) == 0 }
}
