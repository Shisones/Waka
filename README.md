# Waka

A libalpm front-end for Arch Linux.

## Installation

```
git clone https://github.com/Shisones/waka
cd waka
cargo build --release
sudo install -m755 target/release/waka /usr/local/bin/
```

Or build the package:

```
makepkg -si
```

Requires Rust, libalpm, and curl.

## Commands

| Command | Description | pacman | Flags |
|---|---|---|---|
| update | Sync databases | -Sy | |
| upgrade | Refresh and upgrade all packages | -Syu | --autoremove, -y |
| install <pkg> | Install packages or local .pkg.tar.* files | -S | -y |
| remove <pkg> | Remove packages, reverse dependencies, unused deps, and config files | -Rcns | -y |
| autoremove | Remove orphaned packages | | -y |
| clean | Clear the package cache | | --all, -y |
| history | View transaction history | | -i <id> |
| info <pkg> | Show package details | -Qi/-Si | |
| search <term> | Search package names and descriptions | -Ss | |
| list | List packages | -Qe | -i, -u |
| fetch | Download a mirrorlist | | |
| meow | I beg, pls meow | | |

## Configuration

Waka reads `/etc/pacman.conf` directly.

`[options]`:

- SigLevel, LocalFileSigLevel, RemoteFileSigLevel
- CacheDir, Architecture
- Include
- ParallelDownloads, DownloadUser, DisableSandbox
- HoldPkg, IgnorePkg, IgnoreGroup
- NoUpgrade, NoExtract

`[repo]`:

- Server, SigLevel, Usage, Include

Waka config, first match wins:

1. `$XDG_CONFIG_HOME/waka/waka.conf`
2. `$HOME/.config/waka/waka.conf`
3. `$HOME/.waka.conf`
4. `/etc/waka/waka.conf`
5. `/usr/etc/waka/waka.conf`

```toml
[waka]
assume_yes = true
```

## License

GPL-3.0-only.
