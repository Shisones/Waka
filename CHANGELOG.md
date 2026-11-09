# Changelog

## v0.1.7

### Fixed

- `-y` and `--assume-yes` now work on upgrade, install, remove, autoremove,
  and clean. The undocumented `-a` short flag is gone.
- Ctrl-C is no longer swallowed during prepare. The transaction handle is
  published only at commit, so prepare errors cannot leave a stale pointer.
- Local package signatures are staged beside the archive, so
  `LocalFileSigLevel` can verify them.
- `clean` no longer deletes files that are not package archives, signatures,
  partial archives, or database files.
- `history -i` prints operations logged in the same second as the transaction
  header.
- Package metadata is sanitized before display in info, search, list, summary,
  remove, autoremove, callbacks, download names, history, and clean warnings.
- The remove summary and prompt survive the commit-log redraw.
- The upgrade screen is separated from the repository bars by one blank line.
- `SigLevel` processes tokens in order. `Never` no longer discards a later
  `DatabaseRequired` or `PackageRequired`. `PackageNever` and `DatabaseNever`
  are supported.
- The update row is right-aligned with the size left of the bar and one
  trailing space of padding.

Package checksums are refreshed after the v0.1.7 tag and release asset exist.

## v0.1.6

### Fixed

- Preserve the transaction prompt and show remaining download size.
- Clean up the download display.
- Tighten comments and messages.
- Update checksums for 0.1.5.

The published 0.1.6 asset was built before the right-aligned update row and
before the v0.1.7 audit fixes. Use 0.1.7 for those changes.

## v0.1.5

- Reset the mainline history to 0.1.0 through 0.1.5.
- Moved the AUR work to the local `aur` branch.
- Updated checksums for 0.1.4.

## v0.1.4

- Cap download bytes at the transaction size.
- Update checksums for 0.1.3.

## v0.1.3

- Require consent before importing a PGP key.
- Confirm autoremove after an upgrade.
- Fail instead of reporting success when a prompt is impossible.
- Only remove the ALPM lock this process created.
- Count packages, not in-flight files, in the download details.
- Show the finished package count in the download details.
- Pad box rows on both sides.
- Bold the progress figures and restore the size/speed dot.
- Move the percentage onto the bar and split the details.
- Paint the download details with a black foreground.
- Left-align the download details and dim them.
- Give the download bar its own line.
- Show what clean will free and align search results.
- Sharpen the download row and transaction summaries.
- Stop redraw drift when the terminal is resized.
- Report commit metadata in `--version`.
- Clear the previous frame before repainting.
- Restore the `README.md` extension.
- Ignore makepkg build artifacts.
- Update checksums for 0.1.2.

## v0.1.2

- Install the license under its new name.
- Stop tracking generated release tarballs.
- Rename the license and readme to conventional casing.
- Keep output inside narrow terminals.
- Update checksums for 0.1.1.

## v0.1.1

- Animate database sync progress in real time.
- Update the prebuilt binary checksum.
- Fix the dubious ownership error.
- Install git in the release workflow.
- Add a CI workflow for the prebuilt binary.
- Add the proper Arch build process.

## v0.1.0

- Initial release: a libalpm front-end inspired by nala.
