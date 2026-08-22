use clap::Parser;
use waka::cli::Cli;

fn main() {
    let lock_path = waka::constants::lock_file();
    ctrlc::set_handler(move || {
        if lock_path.exists() {
            let _ = std::fs::remove_file(&lock_path);
        }
        std::process::exit(130);
    }).expect("Error setting Ctrl-C handler");

    let cli = Cli::parse();

    if let Err(e) = waka::run(cli) {
        eprintln!("{}", waka::display::style::error(&e.to_string()));
        std::process::exit(1);
    }
}