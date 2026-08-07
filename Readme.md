# Waka

A front-end for libalpm, inspired by nala.

## Features

- Parses /etc/pacman.conf directly (SigLevel, Include with glob, Server with $repo/$arch, Architecture, CacheDir, ParallelDownloads, DownloadUser, DisableSandbox)
- Own config at ~/.config/waka/waka.conf (TOML, searched XDG to system paths)
- Live download progress with ETA, speed, per-file status (50ms throttle)
- Real-time commit log showing package operations and hook output
- Unicode box drawing with proportional columns
- NO_COLOR support (disables colours when env var is set or piped)
- Non-TTY confirm returns false; --assume-yes / config option for automation
- Cleans up ALPM lock file on Ctrl+C
- Auto-answers ALPM questions (conflicts, providers, key import, corrupted packages)

## Installation

```
git clone https://github.com/Shisones/waka
cd waka
cargo build --release
sudo install -m755 target/release/waka /usr/local/bin/
```

Or from the AUR:

```
paru -S waka        # builds from source
paru -S waka-bin    # prebuilt binary from GitHub releases
```

Both PKGBUILDs are in this repo: `PKGBUILD` (source build) and `dist/PKGBUILD` (binary).

## Configuration

### pacman.conf

Waka reads /etc/pacman.conf directly.

[options] directives:
- SigLevel, LocalFileSigLevel, RemoteFileSigLevel
- CacheDir, Architecture (additive, auto becomes native)
- Include (reads SigLevel from another file)
- ParallelDownloads, DownloadUser, DisableSandbox

[repo] directives:
- Server (supports $repo, $arch)
- SigLevel (per-repo override)
- Include (reads Server lines, glob supported, nested Include works)

### waka.conf

Search order: $XDG_CONFIG_HOME/waka/waka.conf, $HOME/.config/waka/waka.conf, $HOME/.waka.conf, /etc/waka/waka.conf, /usr/etc/waka/waka.conf.

```toml
[waka]
assume_yes = true
```

## Commands

| Command | Description | Pacman | Flags |
|---|---|---|---|
| update | Sync databases | -Sy | |
| upgrade | Refresh + sysupgrade | -Syu | --autoremove, -y |
| install <pkg> | Install packages | -S | -y |
| remove <pkg> | Remove + deps + config | -Rns | -y |
| autoremove | Remove orphans | | -y |
| clean | Clear cache | | --all, -y |
| history | View transaction log | | -i <id> |
| info <pkg> | Package details | -Qi/-Si | |
| search <term> | Substring search | -Ss | |
| list | List packages | -Qe | -i, -u |
| fetch | Download mirrorlist | | |
| meow | Mew mew :3 | | |

## Dependencies

- libalpm (Arch Linux Package Manager)
- curl (for waka fetch)
- Rust toolchain for building

Runtime: clap, alpm, anyhow, serde, toml, chrono, termsize, ctrlc, unicode-width, tempfile.

## License

GPL-3.0-only
