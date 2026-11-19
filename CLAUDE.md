# OpenRef

Reference-image boards for artists: images and notes on an infinite canvas, in a window that can float
over other programs and be made see-through. One Windows program, written in Rust with eframe/egui.

This file holds facts about this repo only, each one checkable in the code. Update it in the same commit
as a change that makes it old.

## Layout

- `src/scene.rs`: the board's model, with no window code. Most tests are here.
- `src/app.rs`: the window's actions, menus and shortcuts. `src/canvas.rs`: gestures, selection, drawing.
- `src/board_file.rs`: the `.oref` file. `src/recovery.rs`: autosave. `src/export.rs`: exporting a board.
- `src/images.rs`: loading images and their textures. `src/history.rs`: undo and redo.
- `src/platform.rs`: every call to Windows. `src/window.rs`: the frameless window. `src/text.rs`: the
  English and Spanish texts.
- `tools/make_icon.ps1`: draws the icons in `assets/`.

## Build and check

- Build: `cargo build --release --locked`.
- Checks: `tools\check.ps1` runs them all, and all must pass. It needs Python 3.11 or newer and the
  ruff that `ruff.toml` names. First what `tools/repo.py` has built in, then the commands that
  `tools/repo.toml` lists:
  - the version agrees in `Cargo.toml`, `Cargo.lock` and the README; every source has its licence
    line; `Cargo.toml` has the common lints; no file holds a path of somebody's computer
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo test --workspace --locked` (50 tests; none needs the network or another program)
  - `ruff format --check .` and `ruff check .`, for the Python in `tools/`
- CI: `.github/workflows/check.yml` runs `tools\check.ps1`, without `-Full`, on every push and pull request
  to `main`, on GitHub's Windows runner with Python 3.14.
- `.editorconfig`, `.gitattributes`, `rustfmt.toml`, `clippy.toml`, `ruff.toml`, `LICENSE`,
  `tools/check.ps1`, `tools/repo.py`, `tools/repo_package.py` and `tools/repo_release.py` are the same in
  every Open3D repo. Do not change them here.

## Version and release

- The version is in `Cargo.toml`. `python -I tools/repo.py version set X.Y.Z` writes it there, in
  `Cargo.lock` and in the README's `**Version X.Y.Z**` line; `tools\check.ps1` fails if they disagree.
- Changes a user can notice go into `CHANGELOG.md` under `## In development`, in the same commit.
- `python -I tools/repo.py package` makes the release package as the `[package]` table of
  `tools/repo.toml` says: the version and its `## X.Y.Z` section in the CHANGELOG, the quick checks,
  `cargo build --release --locked` without this computer's folders in the exe, the licences of the
  dependencies in `THIRD-PARTY-LICENSES.txt`, then `dist/openref-<version>-windows-x64.zip` (`OpenRef.exe`,
  README, LICENSE, NOTICE, CHANGELOG, `LICENSES/*.txt`) and `dist/SHA256SUMS.txt`. It refuses an exe that
  holds a path of the computer that built it, or whose file version or `--version` is not the version.
- `python -I tools/repo.py release X.Y.Z` turns `## In development` into `## X.Y.Z — <date>`, sets the
  version, commits `Release X.Y.Z`, packages, tags `vX.Y.Z` and prints the pushes to approve; run again
  once they are done, it makes the GitHub release as a draft. It never pushes. `--dry-run` tries it all
  and writes only to `dist/`.
- The Open3D launcher needs `OpenRef.exe` at the root of the zip, with the release's version as its file
  version.

## Rules of this code

- The OpenGL renderer (`glow`) only: the transparent window and the see-through mode need it.
- Every text of the interface is written with `tr`, `tf`, `t` or `say`, with the English text as its key.
  A test scans the sources and fails when a text has no Spanish.
- Shortcuts live in the `SHORTCUTS` table of `src/app.rs`, which also builds the menu. Avoid the `-`, `+`
  and `=` keys: they are awkward on Spanish keyboards.
- `unsafe` is denied. It is allowed by name, with its reason, only where Windows or the graphics card is
  called: the `platform` module, `attach_console` and one OpenGL query in `src/app.rs`.
- A `.oref` board is a zip with `board.json` (format `openref-board`, version 1) and `images/`. A file
  with a newer version is refused, never guessed at.
- Code from BeeRef (GPL-3.0-or-later) may be ported, with credit in `NOTICE`. Nothing from PureRef, its
  file format included.

## Traps

- The largest texture side is asked from OpenGL and capped at 8192; so is the size of an exported board.
- eframe's own clipboard shortcuts are switched off: the app handles copy and paste itself.
- The launcher keeps its own copy of `assets/icon-256.png`; when the icon changes, that copy must change
  with it.
