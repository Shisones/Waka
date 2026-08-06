mod alpm;
mod cli;
mod config;
mod constants;
mod display;
mod pkg;

use clap::Parser;
use std::sync::Mutex;

static LOCKFILE: Mutex<Option<String>> = Mutex::new(None);

fn main() {
    let lock_path = constants::lock_file();
    ctrlc::set_handler(move || {
        if lock_path.exists() {
            let _ = std::fs::remove_file(&lock_path);
        }
        std::process::exit(130);
    }).expect("Error setting Ctrl-C handler");

    let cli = cli::Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("{}", display::style::error(&e.to_string()));
        std::process::exit(1);
    }
}

const CAT: &str = r#"
   |\---/|
   | ,_, |
    \_`_/-..----.
 ___/ `   ' ,""+ \  
(__...'   __\    |`.___.';
  (_,...'(_,.`__)/'.....+

"#;

fn run(cli: cli::Cli) -> anyhow::Result<()> {
    match cli.command {
        cli::Command::Update => alpm::operations::run_update(),
        cli::Command::Upgrade { assume_yes, autoremove } => alpm::operations::run_upgrade(assume_yes, autoremove),
        cli::Command::Install { packages, assume_yes } => alpm::operations::run_install(&packages, assume_yes),
        cli::Command::Remove { packages, assume_yes } => alpm::operations::run_remove(&packages, assume_yes),
        cli::Command::Autoremove { assume_yes } => alpm::operations::run_autoremove(assume_yes),
        cli::Command::Info { packages } => {
            let handle = alpm::handle::AlpmHandle::new()?;
            pkg::show::show_packages(&handle, &packages)
        }
        cli::Command::Search { terms } => {
            let handle = alpm::handle::AlpmHandle::new()?;
            pkg::search::search_packages(&handle, &terms.join(" "))
        }
        cli::Command::List { installed, upgradable, packages } => {
            let handle = alpm::handle::AlpmHandle::new()?;
            pkg::list::run_list(&handle, installed, upgradable, &packages)
        }
        cli::Command::Clean { all, assume_yes } => alpm::operations::run_clean(all, assume_yes),
        cli::Command::History { id } => alpm::operations::run_history(id),
        cli::Command::Fetch => alpm::operations::run_fetch(),
        cli::Command::Meow => {
            println!("{CAT}");
            println!("\"I got kidnapped from the source code of nala\"");
            Ok(())
        }
    }
}

pub fn set_lockfile(path: &str) {
    if let Ok(mut guard) = LOCKFILE.lock() {
        *guard = Some(path.to_string());
    }
}

pub fn clear_lockfile() {
    if let Ok(mut guard) = LOCKFILE.lock() {
        *guard = None;
    }
}
