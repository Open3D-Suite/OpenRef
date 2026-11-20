// SPDX-License-Identifier: GPL-3.0-or-later
//! OpenRef: reference-image boards on an infinite canvas.

pub mod app;
pub mod board_file;
pub mod canvas;
pub mod export;
pub mod folders;
pub mod history;
pub mod images;
// The language shared by every Open3D app: a copy of a common file. Its one call to Windows, for
// the language of Windows, allows `unsafe` by name.
pub mod open3d_settings;
// `unsafe` is allowed here because this module is where OpenRef calls Windows itself.
#[allow(unsafe_code)]
pub mod platform;
pub mod recovery;
pub mod scene;
pub mod text;
pub mod window;

/// The program's version, as in Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
