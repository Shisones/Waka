use clap::{CommandFactory, FromArgMatches};
use waka::cli::Cli;

fn main() {
    let lock_path = waka::constants::lock_file();
    ctrlc::set_handler(move || {
        if waka::alpm::transaction::interrupt_active() {
            return;
        }
        if waka::owns_lockfile() && lock_path.exists() {
            let _ = std::fs::remove_file(&lock_path);
        }
        std::process::exit(130);
    }).expect("Error setting Ctrl-C handler");

    let mut cmd = Cli::command();
    cmd = cmd.version(waka::version::full());
    let matches = cmd.get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

    if let Err(e) = waka::run(cli) {
        exit_with(e);
    }
}

fn exit_with(e: anyhow::Error) -> ! {
    if e.downcast_ref::<waka::Aborted>().is_some() {
        let code = if waka::alpm::transaction::was_interrupted() { 130 } else { 1 };
        std::process::exit(code);
    }
    if waka::alpm::transaction::was_interrupted() {
        std::process::exit(130);
    }
    eprintln!(
        "{}",
        waka::display::style::error(&waka::display::draw::sanitize(&e.to_string()))
    );
    std::process::exit(1);
}