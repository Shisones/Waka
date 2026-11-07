use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "waka", about = "Waka - an alpm front-end inspired by nala")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Update package databases (pacman -Sy)
    Update,

    /// Upgrade all packages (pacman -Syu)
    #[command()]
    Upgrade {
        /// Remove orphaned packages after upgrade
        #[arg(long)]
        autoremove: bool,

        /// Assume yes to all prompts
        #[arg(short, long)]
        assume_yes: bool,
    },

    /// Install packages (pacman -S)
    #[command()]
    Install {
        /// Package names or local .pkg.tar.* archives to install
        packages: Vec<String>,

        /// Assume yes to all prompts
        #[arg(short, long)]
        assume_yes: bool,
    },

    /// Remove packages, reverse dependencies, unused deps and config files (pacman -Rcns)
    #[command()]
    Remove {
        /// Package names to remove
        packages: Vec<String>,

        /// Assume yes to all prompts
        #[arg(short, long)]
        assume_yes: bool,
    },

    /// Remove orphaned packages
    #[command()]
    Autoremove {
        /// Assume yes to all prompts
        #[arg(short, long)]
        assume_yes: bool,
    },

    /// Show package details (pacman -Qi/Si)
    #[command()]
    Info {
        /// Package names
        packages: Vec<String>,
    },

    /// Search package names and descriptions (pacman -Ss)
    #[command()]
    Search {
        /// Search terms (case-insensitive substring matching)
        terms: Vec<String>,
    },

    /// List packages (pacman -Qe/Q/Qu)
    #[command()]
    List {
        /// Include dependency packages
        #[arg(short, long)]
        installed: bool,

        /// Show only upgradable packages
        #[arg(short, long, conflicts_with = "installed")]
        upgradable: bool,

        /// Package patterns to filter
        packages: Vec<String>,
    },

    /// Clear package cache
    #[command()]
    Clean {
        /// Also clean unused sync databases
        #[arg(long)]
        all: bool,

        /// Assume yes to all prompts
        #[arg(short, long)]
        assume_yes: bool,
    },

    /// View transaction history
    #[command()]
    History {
        /// Show full details for a specific transaction
        #[arg(short, long)]
        id: Option<String>,
    },

    /// Fetch the fastest mirrors
    #[command()]
    Fetch,

    /// I beg, pls meow
    #[command()]
    Meow,
}