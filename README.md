# OpenRef

Reference-image boards for artists: paste or drop images onto an infinite canvas, arrange, crop
and annotate them, and keep the board floating over 3ds Max or any other program. A single Windows
program, no install, no account. Part of the [Open3D](https://github.com/carlesllavata/Open3D)
suite of open-source tools for 3D work.

**Version 0.1.0** – in development, not released yet.

## What works today

- An infinite board: zoom with the wheel, pan with the middle button, frame the selection (Space)
  or everything (Ctrl+Space).
- Images in by pasting (Ctrl+V: a copied picture or files copied in Explorer), dropping files on
  the window, adding them (Ctrl+I) or passing them on the command line. PNG, JPEG, BMP, GIF (first
  frame), TGA and WebP.
- Select (click, Ctrl/Shift+click, drag a box), move, scale from a corner, rotate from just outside
  a corner, flip, grayscale, opacity, reset, duplicate, delete, stacking order, and pack to the
  window's shape (Ctrl+P). Undo and redo everything.
- Boards saved as one `.oref` file with the original images inside; a question before unsaved
  changes are lost.
- Copy an image back to the clipboard as it looks on the board.
- Always on top (Ctrl+Shift+A).
- Everything in the right-click menu, with its shortcut.

## Still to come for 0.1.0

- Crop, text notes and a colour picker.
- A window without a frame, see-through, or transparent to the mouse so you can trace over it.
- Arrange by name, align, distribute and match sizes.
- Export images or the whole board; autosave and recovery; a Spanish interface.

`.oref` boards are zip files: `board.json` describes the board and `images/` holds the original
files, so any zip tool can take the pictures out.

## Install

Not released yet. It will come as a zip with `OpenRef.exe`, and through the
[Open3D launcher](https://github.com/carlesllavata/Open3D).

## Build

Rust (stable) on Windows 10 or 11, 64-bit:

```
cargo build --release
```

`python -I tools/package.py` builds, tests and writes the release zip to `dist/`.

## Credits

OpenRef is inspired by the way PureRef works; it is not affiliated with PureRef or Idyllic Pixel AB
and contains nothing of it. [BeeRef](https://github.com/rbreu/beeref) by Rebecca Breu
(GPL-3.0-or-later) is its reference; any code taken from it is listed in `NOTICE`.

## License

OpenRef is free software under the GNU General Public License, version 3 or later
(`GPL-3.0-or-later`). See [`LICENSE`](LICENSE).
