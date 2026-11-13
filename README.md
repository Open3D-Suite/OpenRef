# OpenRef

Reference-image boards for artists: paste or drop images onto an infinite canvas, arrange, crop
and annotate them, and keep the board floating over 3ds Max or any other program. A single Windows
program, no install, no account. Part of the [Open3D](https://github.com/carlesllavata/Open3D)
suite of open-source tools for 3D work.

**Version 0.1.0** – in development, not released yet. The window opens, but the board does not do
anything yet.

## Planned for 0.1.0

- An infinite canvas: zoom, pan, select, undo and redo.
- Images in by pasting (Ctrl+V), dragging from Explorer or a browser, or opening files.
- Move, scale, rotate, flip, crop (without losing the original), grayscale and opacity.
- Pack the board to the window, arrange by name, align and distribute.
- Text notes and a colour picker.
- A window that stays on top, without a frame, see-through, or transparent to the mouse so you can
  trace over it.
- Boards saved as one `.oref` file with the images inside, with autosave.
- Export images or the whole board, in English and Spanish.

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
