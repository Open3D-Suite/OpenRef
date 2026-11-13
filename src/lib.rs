// SPDX-License-Identifier: GPL-3.0-or-later
//! OpenRef: reference-image boards on an infinite canvas.

pub mod app;
pub mod board_file;
pub mod canvas;
pub mod history;
pub mod images;
pub mod platform;
pub mod scene;
pub mod window;

/// The program's version, as in Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
