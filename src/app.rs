// SPDX-License-Identifier: GPL-3.0-or-later
//! The window. For now an empty board that tells the user what it will do.

use eframe::egui;

/// Opens the main window and runs until it is closed.
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenRef")
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([320.0, 240.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("OpenRef", options, Box::new(|_cc| Ok(Box::new(Board::default()))))
}

/// An empty board: what is on it comes with the canvas in 0.1.0.
#[derive(Default)]
struct Board;

impl eframe::App for Board {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(32));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Paste or drop images here",
            egui::FontId::proportional(18.0),
            egui::Color32::from_gray(140),
        );
    }
}
