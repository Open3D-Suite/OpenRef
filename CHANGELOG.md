# Changelog

What changed for users in each version of OpenRef, newest first.

## In development

- **The same language as the Open3D launcher and the other Open3D apps.** Choosing English or Español in
  the Language menu now changes it for the launcher and the apps that share it too, the next time they
  start; a language chosen there shows here the next time OpenRef starts.
- **If you never chose a language, OpenRef now follows Windows:** Spanish if Windows is in Spanish, English
  otherwise. Before, it always started in English. If you had chosen Spanish, it stays. If you had chosen
  English on a Windows in Spanish, choose English once more.
- **OpenRef's files move to the Open3D folders:** its settings to `%APPDATA%\Open3D\OpenRef`, the copies
  of unsaved boards to `%LOCALAPPDATA%\Open3D\OpenRef\recovery`. The first time the new version starts it
  copies what it finds in the old folders, `%APPDATA%\OpenRef` and `%LOCALAPPDATA%\OpenRef`, so nothing
  has to be set up again: the window, the language and a board left unsaved come back as they were. The
  old folders are left as they were, and an older OpenRef still finds its settings there.
- **Nothing else to install.** The Visual C++ runtime is now inside `OpenRef.exe`, so it starts on a
  Windows that has no Visual C++ Redistributable.
- **An installer for everyone on the computer.** Each release also carries
  `openref-<version>-windows-x64.msi`, which installs OpenRef in `C:\Program Files\Open3D\OpenRef`, with a
  shortcut in the Start menu for every user. Windows asks for permission first and names no publisher
  ("Unknown publisher"): the package is not signed yet. It is removed from Windows' Installed apps. The
  Open3D launcher uses it from its version 0.4.0; the zip stays as it is, to unzip anywhere and for a
  portable launcher.

## 0.1.1 — 16 November 2026

- **The program no longer names folders of the computer it was built on.** Earlier builds
  carried the full paths of their source files inside `OpenRef.exe`. Nothing else changes.

## 0.1.0 — 13 November 2026

The first release: a reference-image board for Windows that floats over 3ds Max or any other
program.

- **A board you can fill.** Paste images with Ctrl+V (a copied picture, image files copied in
  Explorer, or an image path), drop image files onto the window, add them with Ctrl+I, or open them
  with OpenRef from the command line. PNG, JPEG, BMP, GIF (first frame), TGA and WebP; photos stand
  upright as their camera tagged them.
- **Move around freely.** Zoom with the mouse wheel around the pointer, pan with the middle button,
  Space frames the selection and Ctrl+Space everything. Large images stay smooth when zoomed out.
- **Arrange images.** Click, Ctrl/Shift+click or drag a box to select; drag to move (Shift keeps
  it straight); drag a corner to scale (Alt scales from the middle); drag just outside a corner
  to rotate (Shift snaps to 45°). Flip (Alt+Shift+H / V), grayscale (Alt+G), opacity, reset
  (Ctrl+Shift+T), duplicate (Ctrl+D), delete, bring to front or send to back (Ctrl+PgUp / PgDn),
  and pack everything to the window's shape (Ctrl+P).
- **Crop without losing anything.** Press C (or double-click an image): drag the edges or corners
  to keep only part of it, or drag inside to slide the picture under the frame. Enter, C, Esc or a
  click outside finishes. Ctrl+Shift+C shows whole images again.
- **Text notes.** Ctrl+N adds a note under the mouse and you type straight away; pasting text that
  is not an image path makes a note too. Double-click a note (or press Enter) to change it; notes
  move, scale and rotate like images, with a dark, light, yellow or no background and four text
  sizes.
- **Arrange.** Arrange by name (Ctrl+Alt+N) in reading order; align edges or centres; distribute
  evenly; give images the same height, width or area.
- **Undo and redo** every change (Ctrl+Z, Ctrl+Shift+Z or Ctrl+Y).
- **Boards in one file.** Save and open `.oref` boards (Ctrl+S, Ctrl+Shift+S, Ctrl+O; a new board
  is Ctrl+Shift+N): the original images go inside, so a board can be moved or shared on its own.
  OpenRef asks before closing or replacing a board with unsaved changes.
- **Autosave.** While a board has unsaved changes, a copy is kept every minute. If OpenRef stops
  without closing, the next start offers the board back.
- **Export.** Ctrl+E writes the selected images (or all) into a folder: untouched ones as their
  original files, cropped, flipped or grayscale ones as PNG. Ctrl+Shift+E saves the selection or the
  whole board as one PNG, as sharp as its sharpest image allows (up to 8192 pixels a side), notes
  included. Ctrl+C copies an image as it looks on the board, or a note's text.
- **A window that stays out of the way**, all remembered for next time:
  - always on top (Ctrl+Shift+A);
  - no frame (Ctrl+Shift+F): a bar appears when the mouse nears the top to move, maximise,
    minimise or close the window, and the edges still resize it;
  - see-through: Ctrl+Shift+↓ / ↑ lets the desktop show through, from 100 % down to 25 %;
  - transparent background (Ctrl+Alt+B): only the images float over what is behind;
  - click-through (Ctrl+Alt+T): the mouse goes through OpenRef to the program behind, to trace over
    a reference; Ctrl+Alt+T, pressed anywhere, brings it back.
- **Colour picker.** Hold S and click to copy the colour under the mouse as HEX (RGB and HSV are
  shown too); keep the button down and leave the window to pick from any other program.
- **English and Spanish.** OpenRef starts in English; Window ▸ Language switches to Spanish (menus,
  notices, dialogs and shortcut names) and it is remembered.
- **Its own icon**: a red push pin.
- Everything is also in the right-click menu, with its shortcut.

Known limits: dragging an image straight out of a browser does not work yet (copy and paste it);
animated GIFs show their first frame; EXR and HDR images are not read yet.
