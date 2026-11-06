pub mod alpm;
pub mod cli;
pub mod commands;
pub mod config;
pub mod constants;
pub mod display;
pub mod pkg;
pub mod sys;
pub mod version;

use std::sync::Mutex;

static LOCKFILE: Mutex<Option<String>> = Mutex::new(None);

/// The user declined a prompt. The caller should exit non-zero without
/// printing another error.
#[derive(Debug)]
pub struct Aborted;

impl std::fmt::Display for Aborted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("aborted")
    }
}

impl std::error::Error for Aborted {}

pub fn aborted() -> anyhow::Error {
    anyhow::Error::new(Aborted)
}

pub const CAT: &str = r#"
   |\---/|
   | ,_, |
    \_`_/-..----.
 ___/ `   ' ,""+ \  
(__...'   __\    |`.___.';
  (_,...'(_,.`__)/'.....+

"#;

pub fn run(cli: cli::Cli) -> anyhow::Result<()> {
    match cli.command {
        cli::Command::Update => commands::update::run(),
        cli::Command::Upgrade { assume_yes, autoremove } => commands::upgrade::run(assume_yes, autoremove),
        cli::Command::Install { packages, assume_yes } => {
            commands::install::run(&packages, assume_yes)
        }
        cli::Command::Remove { packages, assume_yes } => commands::remove::run(&packages, assume_yes),
        cli::Command::Autoremove { assume_yes } => commands::autoremove::run(assume_yes),
        cli::Command::Info { packages } => {
            let handle = alpm::handle::AlpmHandle::new()?;
            pkg::info::print_packages(&handle, &packages)
        }
        cli::Command::Search { terms } => {
            let handle = alpm::handle::AlpmHandle::new()?;
            pkg::search::search_packages(&handle, &terms)
        }
        cli::Command::List { installed, upgradable, packages } => {
            let handle = alpm::handle::AlpmHandle::new()?;
            pkg::list::run_list(&handle, installed, upgradable, &packages)
        }
        cli::Command::Clean { all, assume_yes } => commands::clean::run(all, assume_yes),
        cli::Command::History { id } => commands::history::run(id),
        cli::Command::Fetch => commands::fetch::run(),
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

/// True only when this process created the ALPM lock during a successful
/// transaction init, so an interrupt never removes another process's lock.
pub fn owns_lockfile() -> bool {
    LOCKFILE.lock().map(|guard| guard.is_some()).unwrap_or(false)
}