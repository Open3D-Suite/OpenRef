// SPDX-License-Identifier: GPL-3.0-or-later
//! The words OpenRef shows, in English (the default) and Spanish.
//!
//! The English text is the key: code writes `tr(lang, "Paste")` and gets the Spanish from the table
//! below when the language is Spanish. Templates take `{name}` fields filled by [`tf`]. A test reads
//! the sources and fails when an English text has no Spanish.

use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// The language shown. Which one is decided by the language every Open3D app shares, or by the
/// language of Windows when there is no preference (`app.rs`, `open3d_settings`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    /// What settings saved before there was a language field read as.
    #[default]
    En,
    Es,
}

impl From<crate::open3d_settings::Language> for Lang {
    fn from(language: crate::open3d_settings::Language) -> Self {
        match language {
            crate::open3d_settings::Language::English => Lang::En,
            crate::open3d_settings::Language::Spanish => Lang::Es,
        }
    }
}

impl From<Lang> for crate::open3d_settings::Language {
    fn from(lang: Lang) -> Self {
        match lang {
            Lang::En => crate::open3d_settings::Language::English,
            Lang::Es => crate::open3d_settings::Language::Spanish,
        }
    }
}

/// The text in the language (English when a translation is missing).
pub fn tr(lang: Lang, en: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Es => SPANISH.iter().find(|(e, _)| *e == en).map_or(en, |(_, es)| es),
    }
}

/// A template in the language, with its `{name}` fields filled.
pub fn tf(lang: Lang, en: &'static str, fields: &[(&str, &dyn Display)]) -> String {
    let mut text = tr(lang, en).to_string();
    for (name, value) in fields {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

/// English → Spanish.
const SPANISH: &[(&str, &str)] = &[
    // Right-click menu.
    ("Paste", "Pegar"),
    ("Copy", "Copiar"),
    ("Add images…", "Añadir imágenes…"),
    ("Add note", "Añadir nota"),
    ("Undo", "Deshacer"),
    ("Redo", "Rehacer"),
    ("Duplicate", "Duplicar"),
    ("Delete", "Eliminar"),
    ("Image", "Imagen"),
    ("Crop", "Recortar"),
    ("Reset crop", "Quitar el recorte"),
    ("Flip horizontally", "Voltear en horizontal"),
    ("Flip vertically", "Voltear en vertical"),
    ("Grayscale", "Escala de grises"),
    ("Reset transform", "Restablecer la transformación"),
    ("Opacity", "Opacidad"),
    ("Note", "Nota"),
    ("Edit note", "Editar la nota"),
    ("Dark", "Oscura"),
    ("Light", "Clara"),
    ("Yellow", "Amarilla"),
    ("No background", "Sin fondo"),
    ("Small text", "Texto pequeño"),
    ("Medium text", "Texto mediano"),
    ("Large text", "Texto grande"),
    ("Huge text", "Texto enorme"),
    ("Arrange", "Ordenar"),
    ("Select all", "Seleccionar todo"),
    ("Pack to window", "Empaquetar en la ventana"),
    ("Bring to front", "Traer al frente"),
    ("Send to back", "Enviar al fondo"),
    ("Arrange by name", "Ordenar por nombre"),
    ("Align", "Alinear"),
    ("Left edges", "Bordes izquierdos"),
    ("Right edges", "Bordes derechos"),
    ("Top edges", "Bordes superiores"),
    ("Bottom edges", "Bordes inferiores"),
    ("Centres in a column", "Centros en columna"),
    ("Centres in a row", "Centros en fila"),
    ("Distribute", "Distribuir"),
    ("Horizontally", "En horizontal"),
    ("Vertically", "En vertical"),
    ("Same size", "Mismo tamaño"),
    ("Same height", "Misma altura"),
    ("Same width", "Misma anchura"),
    ("Same area", "Misma área"),
    ("Frame selection", "Encuadrar la selección"),
    ("Frame everything", "Encuadrar todo"),
    ("Pick a colour", "Cuentagotas"),
    ("Board", "Tablero"),
    ("New board", "Nuevo tablero"),
    ("Open board…", "Abrir tablero…"),
    ("Save board", "Guardar tablero"),
    ("Save board as…", "Guardar tablero como…"),
    ("Export images…", "Exportar imágenes…"),
    ("Export as PNG…", "Exportar como PNG…"),
    ("Window", "Ventana"),
    ("Always on top", "Siempre encima"),
    ("No frame", "Sin marco"),
    ("Transparent background", "Fondo transparente"),
    ("Click-through", "Dejar pasar el ratón"),
    ("Visible", "Visible"),
    ("Language", "Idioma"),
    // Shortcut names.
    ("Shift", "Mayús"),
    ("Del", "Supr"),
    ("Space", "Espacio"),
    ("Enter", "Intro"),
    ("Escape", "Esc"),
    ("Backspace", "Retroceso"),
    ("Up", "Arriba"),
    ("Down", "Abajo"),
    ("PgUp", "RePág"),
    ("PgDn", "AvPág"),
    // Dialogs.
    ("Add images", "Añadir imágenes"),
    ("Images", "Imágenes"),
    ("Open board", "Abrir tablero"),
    ("OpenRef board", "Tablero de OpenRef"),
    ("Export images to…", "Exportar imágenes a…"),
    ("Export board as PNG", "Exportar el tablero como PNG"),
    ("PNG image", "Imagen PNG"),
    ("Save the changes?", "¿Guardar los cambios?"),
    ("“{name}” has changes that are not saved.", "«{name}» tiene cambios sin guardar."),
    ("Save", "Guardar"),
    ("Don't save", "No guardar"),
    ("Cancel", "Cancelar"),
    ("Recover the board?", "¿Recuperar el tablero?"),
    ("{name} had changes that were not saved when OpenRef last stopped.", "{name} tenía cambios sin guardar cuando OpenRef se detuvo la última vez."),
    ("A board", "Un tablero"),
    ("Recover", "Recuperar"),
    ("Discard", "Descartar"),
    ("Untitled", "Sin título"),
    // Notices.
    ("Window {percent} % visible", "Ventana visible al {percent} %"),
    ("The mouse now goes through OpenRef. Ctrl+Alt+T brings it back.", "El ratón atraviesa OpenRef. Ctrl+Alt+T lo devuelve."),
    ("Click-through is not available: {error}", "No se puede dejar pasar el ratón: {error}"),
    ("OpenRef takes the mouse again", "OpenRef vuelve a recibir el ratón"),
    ("There are no images to export", "No hay imágenes que exportar"),
    ("A picture is still being made", "Todavía se está creando una imagen"),
    ("The board is empty", "El tablero está vacío"),
    ("Making a {width} × {height} picture…", "Creando una imagen de {width} × {height}…"),
    ("Select notes to change them", "Selecciona notas para cambiarlas"),
    ("Select one image to crop", "Selecciona una imagen para recortarla"),
    ("No frame: move the window from the bar at the top", "Sin marco: mueve la ventana desde la barra de arriba"),
    (
        "Click anywhere on the screen to copy its colour (drag out of the window)",
        "Haz clic en cualquier punto de la pantalla para copiar su color (arrastra fuera de la ventana)",
    ),
    ("Distributing needs three items or more", "Para distribuir hacen falta tres elementos o más"),
    ("Not on top", "Ya no está siempre encima"),
    ("Could not add {what}", "No se pudo añadir {what}"),
    ("Could not add {count} files", "No se pudieron añadir {count} archivos"),
    ("1 image added", "1 imagen añadida"),
    ("{count} images added", "{count} imágenes añadidas"),
    ("The clipboard is not available", "El portapapeles no está disponible"),
    ("Pasted image", "Imagen pegada"),
    ("Image pasted", "Imagen pegada"),
    ("Could not paste the image: {error}", "No se pudo pegar la imagen: {error}"),
    ("There is nothing to paste", "No hay nada que pegar"),
    ("Select one image or note to copy", "Selecciona una imagen o una nota para copiarla"),
    ("Note copied", "Nota copiada"),
    ("Image copied", "Imagen copiada"),
    ("Could not copy: {error}", "No se pudo copiar: {error}"),
    ("Saved", "Guardado"),
    ("Could not save: {error}", "No se pudo guardar: {error}"),
    ("Could not open the board: {error}", "No se pudo abrir el tablero: {error}"),
    ("1 image exported", "1 imagen exportada"),
    ("{count} images exported", "{count} imágenes exportadas"),
    ("{count} exported; could not export {what}", "{count} exportadas; no se pudo exportar {what}"),
    ("Exported {name}", "Exportado {name}"),
    ("Could not export {name}: {error}", "No se pudo exportar {name}: {error}"),
    ("Board recovered: save it to keep it", "Tablero recuperado: guárdalo para conservarlo"),
    ("Could not recover the board: {error}", "No se pudo recuperar el tablero: {error}"),
    (
        "The language could not be saved for the other Open3D apps: {error}",
        "No se pudo guardar el idioma para las demás aplicaciones de Open3D: {error}",
    ),
    ("{hex} copied", "{hex} copiado"),
    // On the board.
    ("Cannot show this image", "No se puede mostrar esta imagen"),
    ("Loading…", "Cargando…"),
    (
        "Cropping: drag the handles, drag inside to slide the picture. Enter or click outside to finish.",
        "Recortando: arrastra los tiradores o arrastra dentro para desplazar la imagen. Intro o clic fuera para terminar.",
    ),
    ("Paste (Ctrl+V) or drop images here", "Pega (Ctrl+V) o suelta imágenes aquí"),
    ("Right-click for everything else", "Clic derecho para todo lo demás"),
    ("Click-through · Ctrl+Alt+T to get the mouse back", "Ratón atravesando · Ctrl+Alt+T para recuperarlo"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_filled_in_both_languages() {
        assert_eq!(tf(Lang::En, "{count} images added", &[("count", &3)]), "3 images added");
        assert_eq!(tf(Lang::Es, "{count} images added", &[("count", &3)]), "3 imágenes añadidas");
    }

    #[test]
    fn missing_translations_fall_back_to_english() {
        assert_eq!(tr(Lang::Es, "Some text nobody wrote"), "Some text nobody wrote");
    }

    #[test]
    fn spanish_keeps_the_fields_of_each_template() {
        for (en, es) in SPANISH {
            let fields = |s: &str| {
                let mut found: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split_once('}').map(|(f, _)| f.to_string())).collect();
                found.sort();
                found
            };
            assert_eq!(fields(en), fields(es), "{en}");
        }
    }

    #[test]
    fn every_english_text_in_the_code_has_spanish() {
        // The calls that take an English text: tr(lang, "…"), tf(lang, "…", …), t("…"),
        // say(ctx, "…") and say_with(ctx, "…", …).
        let starts = ["tr(lang, \"", "tf(lang, \"", "t(\"", "say(ctx, \"", "say_with(ctx, \""];
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = Vec::new();
        let mut seen = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == "text.rs") {
                continue;
            }
            let code = std::fs::read_to_string(&path).unwrap();
            for start in starts {
                let mut rest = code.as_str();
                while let Some(at) = rest.find(start) {
                    // A call of its own: `t("` but not `expect("` or `id_salt("`.
                    let own = rest[..at].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.'));
                    rest = &rest[at + start.len()..];
                    if !own {
                        continue;
                    }
                    let Some(end) = rest.find('"') else { break };
                    let text = &rest[..end];
                    seen += 1;
                    if !SPANISH.iter().any(|(en, _)| *en == text) {
                        missing.push(format!("{}: {text}", path.file_name().unwrap().to_string_lossy()));
                    }
                }
            }
        }
        assert!(seen > 80, "the scan found only {seen} texts: did the call names change?");
        assert!(missing.is_empty(), "no Spanish for:\n{}", missing.join("\n"));
    }
}
