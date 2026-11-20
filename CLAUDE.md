# OpenRef

Reference-image boards for artists: images and notes on an infinite canvas, in a window that can float
over other programs and be made see-through. One Windows program, written in Rust with eframe/egui.

This file holds facts about this repo only, each one checkable in the code. Update it in the same commit
as a change that makes it old.

## Layout

- `src/scene.rs`: the board's model, with no window code. Most tests are here.
- `src/app.rs`: the window's actions, menus and shortcuts. `src/canvas.rs`: gestures, selection, drawing.
- `src/board_file.rs`: the `.oref` file. `src/recovery.rs`: autosave. `src/export.rs`: exporting a board.
- `src/folders.rs`: where OpenRef keeps its files, and carrying its old folders over at start.
- `src/images.rs`: loading images and their textures. `src/history.rs`: undo and redo.
- `src/platform.rs`: every call to Windows but one. `src/window.rs`: the frameless window. `src/text.rs`:
  the English and Spanish texts. `src/open3d_settings.rs`: what every Open3D app shares, the language and
  the folders of each app, with the cases its tests read (`src/open3d_settings_cases.txt`).
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
  - `cargo test --workspace --locked` (72 tests; none needs the network or another program)
  - `ruff format --check .` and `ruff check .`, for the Python in `tools/`
- CI: `.github/workflows/check.yml` runs `tools\check.ps1`, without `-Full`, on every push and pull request
  to `main`, on GitHub's Windows runner with Python 3.14.
- `.editorconfig`, `.gitattributes`, `rustfmt.toml`, `clippy.toml`, `ruff.toml`, `LICENSE`,
  `tools/check.ps1`, `tools/repo.py`, `tools/repo_package.py`, `tools/repo_release.py` and
  `tools/repo_msi.py` are the same in every Open3D repo, and `src/open3d_settings.rs` with
  `src/open3d_settings_cases.txt` in every repo that has them. Do not change them here.

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
- The language is the one every Open3D app shares, in `settings.json` (`%APPDATA%\Open3D`, or the folder
  `OPEN3D_SETTINGS_DIR` names). At start `Settings::take_shared_language` takes its preference; with none,
  a Spanish chosen here before is carried over once (`language_shared`), and otherwise the language of
  Windows decides: Spanish if it is Spanish, else English. A saved English is not carried over: it cannot
  be told apart from the old default. The Language menu writes the file and changes at once.
- Its files, from `src/folders.rs`: eframe's `app.ron` (settings and window) in `%APPDATA%\Open3D\OpenRef`
  (`persistence_path`), autosaves in `%LOCALAPPDATA%\Open3D\OpenRef\recovery`; `OPEN3D_SETTINGS_DIR` and
  `OPEN3D_DATA_DIR` move them. In `run`, before eframe starts, `open3d_settings::carry_over` copies the old
  folders into them once: eframe's own `%APPDATA%\OpenRef\data` and `%LOCALAPPDATA%\OpenRef`, which are
  left as they were. If a copy fails, that run uses the old folder and the next start tries again; what
  was not copied is said in a notice. Only debug builds read `OPENREF_TEST_OLD_FOLDERS`, a folder whose
  `Roaming` and `Local` stand for the old ones in a test.
- Shortcuts live in the `SHORTCUTS` table of `src/app.rs`, which also builds the menu. Avoid the `-`, `+`
  and `=` keys: they are awkward on Spanish keyboards.
- `unsafe` is denied. It is allowed by name, with its reason, only where Windows or the graphics card is
  called: the `platform` module, `attach_console`, `user_interface_language` in `src/open3d_settings.rs`
  and one OpenGL query in `src/app.rs`.
- A `.oref` board is a zip with `board.json` (format `openref-board`, version 1) and `images/`. A file
  with a newer version is refused, never guessed at.
- Code from BeeRef (GPL-3.0-or-later) may be ported, with credit in `NOTICE`. Nothing from PureRef, its
  file format included.

## Traps

- The largest texture side is asked from OpenGL and capped at 8192; so is the size of an exported board.
- eframe's own clipboard shortcuts are switched off: the app handles copy and paste itself.
- The launcher keeps its own copy of `assets/icon-256.png`; when the icon changes, that copy must change
  with it.
