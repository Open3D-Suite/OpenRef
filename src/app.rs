// SPDX-License-Identifier: GPL-3.0-or-later
//! The window: actions and their shortcuts, the right-click menu, the clipboard, dropped files,
//! opening and saving boards, and the question before unsaved changes are lost.

use crate::board_file::{self, EXTENSION};
use crate::canvas::{Editor, BACKGROUND};
use crate::images::{self, ImageStore, Source};
use crate::scene::{ItemKind, Scene};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Image files OpenRef reads.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "jfif", "bmp", "gif", "tga", "webp"];

/// Opens the main window and runs until it is closed. `files` are boards or images to open.
pub fn run(files: Vec<PathBuf>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("OpenRef")
            .with_title("OpenRef")
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([240.0, 160.0])
            .with_drag_and_drop(true),
        // Ctrl+C / Ctrl+V reach the board as keys: it reads images and files, not only text.
        clipboard_shortcuts: false,
        ..Default::default()
    };
    eframe::run_native("OpenRef", options, Box::new(move |cc| Ok(Box::new(OpenRefApp::new(cc, files)))))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    New,
    Open,
    Save,
    SaveAs,
    ImportImages,
    Paste,
    Copy,
    Undo,
    Redo,
    SelectAll,
    SelectNone,
    Delete,
    Duplicate,
    FlipHorizontal,
    FlipVertical,
    ResetTransform,
    Grayscale,
    BringToFront,
    SendToBack,
    Pack,
    Focus,
    FocusAll,
    Opacity(u8),
    AlwaysOnTop,
}

/// Every keyboard shortcut. Longer ones first, so Ctrl+Shift+Z is not read as Ctrl+Z.
const SHORTCUTS: &[(Modifiers, Key, Action)] = &[
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::S, Action::SaveAs),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Z, Action::Redo),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::T, Action::ResetTransform),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::A, Action::AlwaysOnTop),
    (Modifiers::ALT.plus(Modifiers::SHIFT), Key::H, Action::FlipHorizontal),
    (Modifiers::ALT.plus(Modifiers::SHIFT), Key::V, Action::FlipVertical),
    (Modifiers::CTRL, Key::N, Action::New),
    (Modifiers::CTRL, Key::O, Action::Open),
    (Modifiers::CTRL, Key::S, Action::Save),
    (Modifiers::CTRL, Key::I, Action::ImportImages),
    (Modifiers::CTRL, Key::V, Action::Paste),
    (Modifiers::CTRL, Key::C, Action::Copy),
    (Modifiers::CTRL, Key::Z, Action::Undo),
    (Modifiers::CTRL, Key::Y, Action::Redo),
    (Modifiers::CTRL, Key::A, Action::SelectAll),
    (Modifiers::CTRL, Key::D, Action::Duplicate),
    (Modifiers::CTRL, Key::P, Action::Pack),
    (Modifiers::CTRL, Key::CloseBracket, Action::BringToFront),
    (Modifiers::CTRL, Key::OpenBracket, Action::SendToBack),
    (Modifiers::CTRL, Key::Space, Action::FocusAll),
    (Modifiers::ALT, Key::G, Action::Grayscale),
    (Modifiers::NONE, Key::Delete, Action::Delete),
    (Modifiers::NONE, Key::Backspace, Action::Delete),
    (Modifiers::NONE, Key::Escape, Action::SelectNone),
    (Modifiers::NONE, Key::Space, Action::Focus),
];

fn shortcut_text(action: Action) -> String {
    SHORTCUTS
        .iter()
        .find(|(_, _, a)| *a == action)
        .map(|(m, k, _)| {
            let mut parts = Vec::new();
            if m.ctrl || m.command {
                parts.push("Ctrl");
            }
            if m.alt {
                parts.push("Alt");
            }
            if m.shift {
                parts.push("Shift");
            }
            let key = k.name();
            parts.push(key);
            parts.join("+")
        })
        .unwrap_or_default()
}

/// What to do once the user has answered about unsaved changes.
#[derive(Clone, Debug)]
enum Then {
    New,
    Open(Option<PathBuf>),
    Close,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    always_on_top: bool,
    /// The folder of the last board or image opened or saved.
    folder: Option<PathBuf>,
}

pub struct OpenRefApp {
    editor: Editor,
    path: Option<PathBuf>,
    settings: Settings,
    /// A short message at the bottom of the board, and when it was shown.
    notice: Option<(String, f64)>,
    asking: Option<Then>,
    /// Decided (answered, or nothing to lose): done at the start of the next frame.
    pending: Option<Then>,
    /// Closing after the user said so (the next close request goes through).
    closing: bool,
    title: String,
    clipboard: Option<arboard::Clipboard>,
    /// Files given on the command line, opened on the first frame.
    startup: Vec<PathBuf>,
}

impl OpenRefApp {
    fn new(cc: &eframe::CreationContext<'_>, startup: Vec<PathBuf>) -> Self {
        let max_side = cc
            .wgpu_render_state
            .as_ref()
            .map(|rs| rs.device.limits().max_texture_dimension_2d)
            .unwrap_or(8192)
            .min(8192);
        let settings: Settings = cc.storage.and_then(|s| eframe::get_value(s, "settings")).unwrap_or_default();
        if settings.always_on_top {
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        }
        Self {
            editor: Editor::new(ImageStore::new(cc.egui_ctx.clone(), max_side)),
            path: None,
            settings,
            notice: None,
            asking: None,
            pending: None,
            closing: false,
            title: String::new(),
            clipboard: arboard::Clipboard::new().ok(),
            startup,
        }
    }

    fn notify(&mut self, ctx: &egui::Context, text: impl Into<String>) {
        self.notice = Some((text.into(), ctx.input(|i| i.time)));
    }

    fn remember_folder(&mut self, path: &Path) {
        let folder = if path.is_dir() { Some(path.to_path_buf()) } else { path.parent().map(Path::to_path_buf) };
        if folder.is_some() {
            self.settings.folder = folder;
        }
    }

    fn dialog_folder(&self) -> Option<PathBuf> {
        self.settings.folder.clone().filter(|f| f.is_dir())
    }

    fn run_action(&mut self, ctx: &egui::Context, frame: &eframe::Frame, action: Action) {
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let editor = &mut self.editor;
        match action {
            Action::New => self.ask_then(Then::New),
            Action::Open => self.ask_then(Then::Open(None)),
            Action::Save => {
                self.save(ctx, frame, false);
            }
            Action::SaveAs => {
                self.save(ctx, frame, true);
            }
            Action::ImportImages => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Add images")
                    .add_filter("Images", IMAGE_EXTENSIONS)
                    .set_parent(frame);
                if let Some(folder) = self.dialog_folder() {
                    dialog = dialog.set_directory(folder);
                }
                if let Some(files) = dialog.pick_files() {
                    if let Some(first) = files.first() {
                        self.remember_folder(first);
                    }
                    let at = self.editor.drop_point(None);
                    self.import_paths(ctx, &files, at);
                }
            }
            Action::Paste => self.paste(ctx, pointer),
            Action::Copy => self.copy(ctx),
            Action::Undo => editor.undo(),
            Action::Redo => editor.redo(),
            Action::SelectAll => editor.selection = editor.scene.ids(),
            Action::SelectNone => editor.selection.clear(),
            Action::Delete => editor.change(|scene, selection| scene.remove(selection)),
            Action::Duplicate => {
                let mut copies = Default::default();
                let offset = egui::vec2(20.0, 20.0) / editor.camera.zoom;
                editor.change(|scene, selection| copies = scene.duplicate(selection, offset));
                editor.selection = copies;
            }
            Action::FlipHorizontal | Action::FlipVertical => {
                let targets = editor.targets();
                if let Some(bounds) = editor.scene.bounds_of(&targets) {
                    let horizontal = action == Action::FlipHorizontal;
                    editor.change(|scene, _| scene.flip_about(&targets, bounds.center(), horizontal));
                }
            }
            Action::ResetTransform => {
                let targets = editor.targets();
                editor.change(|scene, _| scene.reset_transform(&targets));
            }
            Action::Grayscale => {
                let targets = editor.targets();
                // Turn it on unless every target already has it.
                let all_gray = editor.scene.items.iter().filter(|i| targets.contains(&i.id)).all(|i| {
                    let ItemKind::Image { grayscale, .. } = &i.kind;
                    *grayscale
                });
                editor.change(|scene, _| scene.set_grayscale(&targets, !all_gray));
            }
            Action::BringToFront => editor.change(|scene, selection| scene.restack(selection, true)),
            Action::SendToBack => editor.change(|scene, selection| scene.restack(selection, false)),
            Action::Pack => {
                let targets = editor.targets();
                let aspect = editor.viewport.width() / editor.viewport.height().max(1.0);
                // The gap is 8 screen points at the zoom the items will be seen at, roughly.
                let gap = 8.0 / editor.camera.zoom.max(0.01);
                editor.change(|scene, _| scene.pack(&targets, aspect, gap));
                editor.focus(editor.selection.is_empty());
            }
            Action::Focus => editor.focus(false),
            Action::FocusAll => editor.focus(true),
            Action::Opacity(percent) => {
                let targets = editor.targets();
                editor.change(|scene, _| scene.set_opacity(&targets, percent as f32 / 100.0));
            }
            Action::AlwaysOnTop => {
                self.settings.always_on_top = !self.settings.always_on_top;
                let level = if self.settings.always_on_top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                };
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                let text = if self.settings.always_on_top { "Always on top" } else { "Not on top" };
                self.notify(ctx, text);
            }
        }
    }

    /// Asks about unsaved changes first when there are any.
    fn ask_then(&mut self, then: Then) {
        if self.editor.dirty && !self.editor.scene.is_empty() {
            self.asking = Some(then);
        } else {
            self.asking = None;
            self.pending = Some(then);
        }
    }

    fn import_paths(&mut self, ctx: &egui::Context, paths: &[PathBuf], at: egui::Pos2) {
        let mut sources = Vec::new();
        let mut problems = Vec::new();
        for path in paths {
            if path.is_dir() {
                continue;
            }
            match Source::from_path(path) {
                Ok(source) => sources.push(source),
                Err(error) => problems.push(format!("{}: {error}", path.file_name().unwrap_or_default().to_string_lossy())),
            }
        }
        let count = sources.len();
        self.editor.add_images(sources, at);
        match (count, problems.first()) {
            (_, Some(problem)) if problems.len() == 1 => self.notify(ctx, format!("Could not add {problem}")),
            (_, Some(_)) => self.notify(ctx, format!("Could not add {} files", problems.len())),
            (0, None) => {}
            (1, None) => self.notify(ctx, "1 image added"),
            (n, None) => self.notify(ctx, format!("{n} images added")),
        }
    }

    /// Pastes files copied in Explorer, an image, or the path of an image file.
    fn paste(&mut self, ctx: &egui::Context, pointer: Option<egui::Pos2>) {
        let at = self.editor.drop_point(pointer);
        let Some(clipboard) = self.clipboard.as_mut() else {
            self.notify(ctx, "The clipboard is not available");
            return;
        };
        if let Ok(files) = clipboard.get().file_list() {
            if !files.is_empty() {
                let boards: Vec<&PathBuf> = files.iter().filter(|f| is_board(f)).collect();
                if let (Some(board), true) = (boards.first(), files.len() == 1) {
                    let board = (*board).clone();
                    self.ask_then(Then::Open(Some(board)));
                    return;
                }
                self.import_paths(ctx, &files, at);
                return;
            }
        }
        if let Ok(image) = clipboard.get_image() {
            match Source::from_rgba(image.width as u32, image.height as u32, &image.bytes, "Pasted image") {
                Ok(source) => {
                    self.editor.add_images(vec![source], at);
                    self.notify(ctx, "Image pasted");
                }
                Err(error) => self.notify(ctx, format!("Could not paste the image: {error}")),
            }
            return;
        }
        if let Ok(text) = clipboard.get_text() {
            let text = text.trim().trim_matches('"');
            let path = PathBuf::from(text);
            if path.is_file() {
                self.import_paths(ctx, &[path], at);
                return;
            }
        }
        self.notify(ctx, "There is no image on the clipboard");
    }

    /// Copies the selected image (as seen: cropped, flipped, grayscale) to the clipboard.
    fn copy(&mut self, ctx: &egui::Context) {
        let editor = &self.editor;
        let mut chosen = editor.scene.items.iter().filter(|i| editor.selection.contains(&i.id));
        let (Some(item), None) = (chosen.next(), chosen.next()) else {
            self.notify(ctx, "Select one image to copy");
            return;
        };
        let ItemKind::Image { image, crop, grayscale } = &item.kind;
        let Some(source) = editor.images.source(*image) else { return };
        let result = images::decode_full(&source.bytes, *grayscale).map(|full| {
            let r = crop.intersect(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(full.width() as f32, full.height() as f32)));
            let mut cut = image::imageops::crop_imm(
                &full,
                r.min.x.round() as u32,
                r.min.y.round() as u32,
                r.width().round().max(1.0) as u32,
                r.height().round().max(1.0) as u32,
            )
            .to_image();
            if item.flip_x {
                image::imageops::flip_horizontal_in_place(&mut cut);
            }
            if item.flip_y {
                image::imageops::flip_vertical_in_place(&mut cut);
            }
            cut
        });
        let copied = match (result, self.clipboard.as_mut()) {
            (Ok(cut), Some(clipboard)) => clipboard
                .set_image(arboard::ImageData {
                    width: cut.width() as usize,
                    height: cut.height() as usize,
                    bytes: cut.into_raw().into(),
                })
                .map_err(|e| e.to_string()),
            (Ok(_), None) => Err("the clipboard is not available".into()),
            (Err(error), _) => Err(error),
        };
        match copied {
            Ok(()) => self.notify(ctx, "Image copied"),
            Err(error) => self.notify(ctx, format!("Could not copy: {error}")),
        }
    }

    fn save(&mut self, ctx: &egui::Context, frame: &eframe::Frame, choose: bool) -> bool {
        let path = match (&self.path, choose) {
            (Some(path), false) => path.clone(),
            _ => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Save board")
                    .add_filter("OpenRef board", &[EXTENSION])
                    .set_file_name(self.board_name() + "." + EXTENSION)
                    .set_parent(frame);
                if let Some(folder) = self.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).or(self.dialog_folder()) {
                    dialog = dialog.set_directory(folder);
                }
                match dialog.save_file() {
                    Some(path) if path.extension().is_none() => path.with_extension(EXTENSION),
                    Some(path) => path,
                    None => return false,
                }
            }
        };
        let images = &self.editor.images;
        match board_file::save(&path, &self.editor.scene, Some(self.editor.camera.view()), |key| images.source(key).cloned()) {
            Ok(()) => {
                self.remember_folder(&path);
                self.path = Some(path);
                self.editor.dirty = false;
                self.notify(ctx, "Saved");
                true
            }
            Err(error) => {
                self.notify(ctx, format!("Could not save: {error}"));
                false
            }
        }
    }

    fn open(&mut self, ctx: &egui::Context, frame: &eframe::Frame, path: Option<PathBuf>) {
        let path = match path {
            Some(path) => path,
            None => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Open board")
                    .add_filter("OpenRef board", &[EXTENSION])
                    .set_parent(frame);
                if let Some(folder) = self.dialog_folder() {
                    dialog = dialog.set_directory(folder);
                }
                match dialog.pick_file() {
                    Some(path) => path,
                    None => return,
                }
            }
        };
        match board_file::load(&path) {
            Ok(board) => {
                self.editor.replace(board.scene, board.images, board.view);
                self.remember_folder(&path);
                self.path = Some(path);
            }
            Err(error) => self.notify(ctx, format!("Could not open the board: {error}")),
        }
    }

    fn board_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let mark = if self.editor.dirty && !self.editor.scene.is_empty() { "*" } else { "" };
        let title = format!("{}{mark} – OpenRef", self.board_name());
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if ctx.egui_wants_keyboard_input() || self.asking.is_some() {
            return;
        }
        let mut actions = Vec::new();
        ctx.input_mut(|input| {
            for (modifiers, key, action) in SHORTCUTS {
                if input.consume_shortcut(&KeyboardShortcut::new(*modifiers, *key)) {
                    actions.push(*action);
                }
            }
        });
        for action in actions {
            self.run_action(ctx, frame, action);
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let at = self.editor.drop_point(pointer);
        let paths: Vec<PathBuf> = dropped.iter().map(|f| f.path().to_path_buf()).collect();
        if let [board] = paths.as_slice() {
            if is_board(board) {
                self.ask_then(Then::Open(Some(board.clone())));
                return;
            }
        }
        if let Some(first) = paths.first() {
            self.remember_folder(first);
        }
        self.import_paths(ctx, &paths, at);
    }

    fn menu(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame) {
        let has_items = !self.editor.scene.is_empty();
        let has_selection = !self.editor.selection.is_empty();
        let ctx = ui.ctx().clone();
        let mut chosen = None;
        let mut item = |ui: &mut egui::Ui, label: &str, action: Action, enabled: bool| {
            let button = egui::Button::new(label).shortcut_text(shortcut_text(action));
            if ui.add_enabled(enabled, button).clicked() {
                chosen = Some(action);
                ui.close();
            }
        };
        item(ui, "Paste", Action::Paste, true);
        item(ui, "Add images…", Action::ImportImages, true);
        item(ui, "Copy image", Action::Copy, self.editor.selection.len() == 1);
        ui.separator();
        item(ui, "Undo", Action::Undo, self.editor.history.can_undo());
        item(ui, "Redo", Action::Redo, self.editor.history.can_redo());
        ui.separator();
        item(ui, "Select all", Action::SelectAll, has_items);
        item(ui, "Duplicate", Action::Duplicate, has_selection);
        item(ui, "Delete", Action::Delete, has_selection);
        ui.separator();
        item(ui, "Flip horizontally", Action::FlipHorizontal, has_items);
        item(ui, "Flip vertically", Action::FlipVertical, has_items);
        item(ui, "Grayscale", Action::Grayscale, has_items);
        item(ui, "Reset transform", Action::ResetTransform, has_items);
        ui.menu_button("Opacity", |ui| {
            for percent in [100u8, 75, 50, 25] {
                item(ui, &format!("{percent} %"), Action::Opacity(percent), has_items);
            }
        });
        ui.menu_button("Order", |ui| {
            item(ui, "Bring to front", Action::BringToFront, has_selection);
            item(ui, "Send to back", Action::SendToBack, has_selection);
        });
        ui.separator();
        item(ui, "Pack to window", Action::Pack, has_items);
        item(ui, "Frame selection", Action::Focus, has_items);
        item(ui, "Frame everything", Action::FocusAll, has_items);
        ui.separator();
        item(ui, "New board", Action::New, true);
        item(ui, "Open board…", Action::Open, true);
        item(ui, "Save board", Action::Save, has_items);
        item(ui, "Save board as…", Action::SaveAs, has_items);
        ui.separator();
        let on_top = if self.settings.always_on_top { "✔ Always on top" } else { "Always on top" };
        item(ui, on_top, Action::AlwaysOnTop, true);
        if let Some(action) = chosen {
            self.run_action(&ctx, frame, action);
        }
    }

    fn ask_dialog(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let Some(then) = self.asking.clone() else { return };
        let mut answer: Option<Option<bool>> = None; // Some(Some(save?)) or Some(None) = cancel
        let response = egui::Modal::new(egui::Id::unique("unsaved")).show(ctx, |ui| {
            ui.set_width(320.0);
            ui.heading("Save the changes?");
            ui.label(format!("“{}” has changes that are not saved.", self.board_name()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    answer = Some(Some(true));
                }
                if ui.button("Don't save").clicked() {
                    answer = Some(Some(false));
                }
                if ui.button("Cancel").clicked() {
                    answer = Some(None);
                }
            });
        });
        if response.should_close() && answer.is_none() {
            answer = Some(None);
        }
        match answer {
            Some(Some(save)) => {
                self.asking = None;
                if save && !self.save(ctx, frame, false) {
                    return;
                }
                self.pending = Some(then);
            }
            Some(None) => self.asking = None,
            None => {}
        }
    }
}

impl eframe::App for OpenRefApp {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.editor.images.poll();
        self.handle_drops(ctx);
        self.handle_keys(ctx, frame);

        if let Some(then) = self.pending.take() {
            match then {
                Then::New => {
                    self.editor.replace(Scene::default(), Vec::new(), Some(crate::canvas::Camera::default().view()));
                    self.path = None;
                }
                Then::Open(path) => self.open(ctx, frame, path),
                Then::Close => {
                    self.closing = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.closing && self.editor.dirty && !self.editor.scene.is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.asking = Some(Then::Close);
        }
        self.update_title(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let response = self.editor.show(ui);
        response.context_menu(|ui| self.menu(ui, frame));

        // Files from the command line, once the board area is known.
        if !self.startup.is_empty() {
            let files = std::mem::take(&mut self.startup);
            let ctx = ui.ctx().clone();
            match files.as_slice() {
                [board] if is_board(board) => self.open(&ctx, frame, Some(board.clone())),
                _ => {
                    let at = self.editor.drop_point(None);
                    self.import_paths(&ctx, &files, at);
                }
            }
        }

        if let Some((text, shown)) = &self.notice {
            let age = ui.input(|i| i.time) - shown;
            if age < 3.0 {
                let alpha = (1.0 - ((age - 2.5) / 0.5).clamp(0.0, 1.0)) as f32;
                let painter = ui.painter();
                let galley = painter.layout_no_wrap(text.clone(), egui::FontId::proportional(14.0), egui::Color32::WHITE.gamma_multiply(alpha));
                let at = ui.max_rect().left_bottom() + egui::vec2(14.0, -14.0 - galley.size().y);
                let back = egui::Rect::from_min_size(at, galley.size()).expand2(egui::vec2(10.0, 6.0));
                painter.rect_filled(back, 6.0, egui::Color32::from_black_alpha((200.0 * alpha) as u8));
                painter.galley(at, galley, egui::Color32::WHITE);
                ui.ctx().request_repaint();
            } else {
                self.notice = None;
            }
        }

        let ctx = ui.ctx().clone();
        self.ask_dialog(&ctx, frame);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.settings);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        BACKGROUND.to_normalized_gamma_f32()
    }
}

fn is_board(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}
