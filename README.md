# OpenRef

Reference-image boards for artists: paste or drop images onto an infinite canvas, arrange, crop
and annotate them, and keep the board floating over 3ds Max or any other program. A single Windows
program, no install, no account. Part of the [Open3D](https://github.com/carlesllavata/Open3D)
suite of open-source tools for 3D work.

**Version 0.1.1** – the first release, packaged again.

## Install

Two ways, and nothing else is needed:

- **With the [Open3D launcher](https://github.com/carlesllavata/Open3D)**, version 0.3.0 or later:
  press *Install* on OpenRef's card. It goes in `%LOCALAPPDATA%\Programs\Open3D\OpenRef`, with
  shortcuts in the Start menu and on the desktop, and the launcher updates it and removes it.
- **By hand**: download `openref-<version>-windows-x64.zip` from the
  [releases page](https://github.com/carlesllavata/OpenRef/releases/latest), unzip it anywhere and
  run `OpenRef.exe`. To remove it, delete the folder.

Removing OpenRef either way keeps its settings in `%APPDATA%\Open3D\OpenRef` and any autosaved copy
in `%LOCALAPPDATA%\Open3D\OpenRef`; delete those folders too if you want them gone. Version 0.1.1 and
earlier kept them in `%APPDATA%\OpenRef` and `%LOCALAPPDATA%\OpenRef`: the first start of a newer
version copies them to the new folders and leaves the old ones as they were, so those can go too.

The program is not code-signed, so the first time Windows may show "Windows protected your PC":
choose *More info* and then *Run anyway*. `SHA256SUMS.txt` on the release page lets you check the
download.

Windows 10 or 11, 64-bit, with a graphics driver that supports OpenGL (any current one does).

## The board

- **Bring images in**: paste with **Ctrl+V** (a picture copied in a browser or an image editor,
  image files copied in Explorer, or the path of an image file), drop files on the window, add them
  with **Ctrl+I**, or open OpenRef with them (`OpenRef.exe a.jpg b.png`, or a `.oref` board).
  PNG, JPEG, BMP, GIF (its first frame), TGA and WebP; photos stand upright as the camera tagged them.
  Dragging an image straight out of a browser does not work yet: copy it and paste it instead. EXR
  and HDR images are not read yet.
- **Move around**: the mouse wheel zooms around the pointer, the middle button pans, **Space**
  frames the selection and **Ctrl+Space** everything.
- **Select** with a click, add or remove with Ctrl or Shift+click, or drag a box on an empty place.
- **Move** by dragging (Shift keeps it straight), **scale** from a corner (Alt scales from the
  middle), **rotate** by dragging just outside a corner (Shift snaps to 45°).
- **Crop** with **C** or a double-click on an image: drag the edges and corners, or drag inside to
  slide the picture under the frame; Enter, C, Esc or a click outside finishes. The original stays
  whole: **Ctrl+Shift+C** shows it again.
- **Notes**: **Ctrl+N** adds one under the mouse and you type straight away; pasting text makes one
  too. Double-click a note (or press Enter) to change it. The menu gives notes a dark, light, yellow
  or no background, and four text sizes.
- **Undo and redo** every change.
- **Right-click** for every command, with its shortcut. **Window ▸ Language** switches between
  English (the default) and Spanish.

## Boards

A board is one `.oref` file with the original images inside, so it can be moved or shared on its
own. It is a zip: `board.json` describes the board and `images/` holds the pictures, so any zip tool
can take them out. OpenRef asks before closing or replacing a board with unsaved changes.

While a board has unsaved changes, a copy is kept every minute in
`%LOCALAPPDATA%\Open3D\OpenRef\recovery`.
If OpenRef stops without closing, the next start offers the board back.

## Over another program

- **Always on top** keeps the board above 3ds Max or any other window.
- **No frame** hides the window's frame; a bar appears when the mouse nears the top to move,
  maximise, minimise or close it, and the edges still resize it.
- **See-through** lets the desktop show through the board, from 100 % down to 25 %, and
  **transparent background** leaves only the images floating.
- **Click-through** sends the mouse to the program behind, to trace over a reference. **Ctrl+Alt+T**,
  pressed anywhere, turns it off again. If another program already uses Ctrl+Alt+T, OpenRef says so
  and does not turn it on.
- **Colour picker**: hold **S** and click to copy the colour under the mouse as HEX (RGB and HSV are
  shown too). Keep the button down and leave the window to pick from any other program.

Window settings, the language and the last folder are remembered for next time.

## Export

- **Ctrl+E** writes the selected images (or all of them) into a folder. Images shown as they are keep
  their original files; cropped, flipped or grayscale ones are saved as PNG.
- **Ctrl+Shift+E** saves the selection (or the whole board) as one PNG, as sharp as its sharpest
  image allows, up to 8192 pixels a side, notes included.
- **Ctrl+C** copies the selected image to the clipboard as it looks on the board (or a note's text).

## Shortcuts

| Keys | Action |
|---|---|
| Ctrl+V / Ctrl+C | Paste / copy |
| Ctrl+I | Add images |
| Ctrl+N | Add a note |
| Enter | Edit the selected note |
| C / Ctrl+Shift+C | Crop / show whole images again |
| Ctrl+Z / Ctrl+Shift+Z or Ctrl+Y | Undo / redo |
| Ctrl+A / Esc | Select all / nothing |
| Ctrl+D | Duplicate |
| Del or Backspace | Delete |
| Alt+Shift+H / Alt+Shift+V | Flip horizontally / vertically |
| Alt+G | Grayscale |
| Ctrl+Shift+T | Reset scale, rotation and flips |
| Ctrl+PgUp / Ctrl+PgDn | Bring to front / send to back |
| Ctrl+P | Pack to the window's shape |
| Ctrl+Alt+N | Arrange by name |
| Space / Ctrl+Space | Frame the selection / everything |
| Ctrl+Shift+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S | New / open / save / save as |
| Ctrl+E / Ctrl+Shift+E | Export images / export as PNG |
| Ctrl+Shift+A | Always on top |
| Ctrl+Shift+F | No frame |
| Ctrl+Shift+↓ / ↑ | More / less see-through |
| Ctrl+Alt+B | Transparent background |
| Ctrl+Alt+T | Click-through on and off (works anywhere) |
| hold S + click | Colour picker |

Align, distribute and same size are in the right-click menu (Arrange).

## Build

Rust (stable) on Windows 10 or 11, 64-bit:

```
cargo build --release
```

`python -I tools/repo.py package` runs the checks, builds and writes the release zip and the Windows
installer (`.msi`, made with [WiX](https://wixtoolset.org) 5) to `dist/`; `tools/make_icon.ps1` draws
the icon.

## Credits

OpenRef is inspired by the way PureRef works; it is not affiliated with PureRef or Idyllic Pixel AB
and contains nothing of it. [BeeRef](https://github.com/rbreu/beeref) by Rebecca Breu
(GPL-3.0-or-later) is its reference; any code taken from it is listed in `NOTICE`.

## License

OpenRef is free software under the GNU General Public License, version 3 or later
(`GPL-3.0-or-later`). See [`LICENSE`](LICENSE). The libraries it contains and their licences are
listed in `THIRD-PARTY-LICENSES.txt` inside the release zip.
