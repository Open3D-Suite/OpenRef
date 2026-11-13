// SPDX-License-Identifier: GPL-3.0-or-later
//! Window extras drawn over the board: the title bar and resize edges of a frameless window,
//! the click-through badge and the colour picker's loupe.

use crate::platform;
use egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Rect, ResizeDirection, Sense, Stroke, StrokeKind, Ui, ViewportCommand};

const EDGE: f32 = 6.0;
const BAR: f32 = 28.0;

/// Resize edges and, while the mouse is near the top, a title bar to move, maximise, minimise and
/// close a window that has no frame.
pub fn frameless_chrome(ui: &mut Ui, title: &str) {
    let rect = ui.max_rect();
    let ctx = ui.ctx().clone();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

    if !maximized {
        let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
        let edges = [
            (Rect::from_min_max(pos2(l + EDGE, t), pos2(r - EDGE, t + EDGE)), ResizeDirection::North, CursorIcon::ResizeVertical),
            (Rect::from_min_max(pos2(l + EDGE, b - EDGE), pos2(r - EDGE, b)), ResizeDirection::South, CursorIcon::ResizeVertical),
            (Rect::from_min_max(pos2(l, t + EDGE), pos2(l + EDGE, b - EDGE)), ResizeDirection::West, CursorIcon::ResizeHorizontal),
            (Rect::from_min_max(pos2(r - EDGE, t + EDGE), pos2(r, b - EDGE)), ResizeDirection::East, CursorIcon::ResizeHorizontal),
            (Rect::from_min_max(pos2(l, t), pos2(l + EDGE, t + EDGE)), ResizeDirection::NorthWest, CursorIcon::ResizeNwSe),
            (Rect::from_min_max(pos2(r - EDGE, b - EDGE), pos2(r, b)), ResizeDirection::SouthEast, CursorIcon::ResizeNwSe),
            (Rect::from_min_max(pos2(r - EDGE, t), pos2(r, t + EDGE)), ResizeDirection::NorthEast, CursorIcon::ResizeNeSw),
            (Rect::from_min_max(pos2(l, b - EDGE), pos2(l + EDGE, b)), ResizeDirection::SouthWest, CursorIcon::ResizeNeSw),
        ];
        for (n, (edge, direction, icon)) in edges.into_iter().enumerate() {
            let response = ui.interact(edge, ui.scope_id().with(("resize-edge", n)), Sense::drag());
            if response.hovered() || response.dragged() {
                ctx.set_cursor_icon(icon);
            }
            if response.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
            }
        }
    }

    let bar = Rect::from_min_max(rect.min + vec2(EDGE, EDGE), pos2(rect.right() - EDGE, rect.top() + EDGE + BAR));
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    let near = pointer.is_some_and(|p| p.y <= bar.bottom() + 8.0 && rect.contains(p));
    if !near {
        return;
    }
    let response = ui.interact(bar, ui.scope_id().with("title-bar"), Sense::click_and_drag());
    if response.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    let painter = ui.painter();
    painter.rect_filled(bar, 6.0, Color32::from_black_alpha(170));
    painter.text(bar.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, title, FontId::proportional(13.0), Color32::from_gray(210));
    // Close, maximise/restore, minimise, from the right. The icons are drawn: the default fonts
    // have no window symbols.
    let commands = [ViewportCommand::Close, ViewportCommand::Maximized(!maximized), ViewportCommand::Minimized(true)];
    for (n, command) in commands.into_iter().enumerate() {
        let button = Rect::from_min_size(pos2(bar.right() - BAR * (n as f32 + 1.0), bar.top()), vec2(BAR, BAR));
        let response = ui.interact(button, ui.scope_id().with(("title-button", n)), Sense::click());
        if response.hovered() {
            let fill = if n == 0 { Color32::from_rgb(196, 43, 28) } else { Color32::from_white_alpha(30) };
            ui.painter().rect_filled(button.shrink(2.0), 4.0, fill);
        }
        let c = button.center();
        let stroke = Stroke::new(1.2, Color32::WHITE);
        let painter = ui.painter();
        match n {
            0 => {
                painter.line_segment([c + vec2(-5.0, -5.0), c + vec2(5.0, 5.0)], stroke);
                painter.line_segment([c + vec2(-5.0, 5.0), c + vec2(5.0, -5.0)], stroke);
            }
            1 if maximized => {
                painter.rect_stroke(Rect::from_center_size(c + vec2(-1.5, 1.5), vec2(8.0, 8.0)), 0.0, stroke, StrokeKind::Middle);
                painter.line_segment([c + vec2(-2.5, -4.5), c + vec2(5.5, -4.5)], stroke);
                painter.line_segment([c + vec2(5.5, -4.5), c + vec2(5.5, 3.5)], stroke);
            }
            1 => {
                painter.rect_stroke(Rect::from_center_size(c, vec2(10.0, 10.0)), 0.0, stroke, StrokeKind::Middle);
            }
            _ => {
                painter.line_segment([c + vec2(-5.0, 0.0), c + vec2(5.0, 0.0)], stroke);
            }
        }
        if response.clicked() {
            ctx.send_viewport_cmd(command);
        }
    }
}

/// A reminder in a corner while the window lets the mouse through.
pub fn click_through_badge(ui: &Ui, lang: crate::text::Lang) {
    let rect = ui.max_rect();
    let text = crate::text::tr(lang, "Click-through · Ctrl+Alt+T to get the mouse back");
    let galley = ui.painter().layout_no_wrap(text.into(), FontId::proportional(12.0), Color32::WHITE);
    let at = rect.right_top() + vec2(-galley.size().x - 16.0, 10.0);
    let back = Rect::from_min_size(at, galley.size()).expand2(vec2(8.0, 4.0));
    ui.painter().rect_filled(back, 6.0, Color32::from_rgba_unmultiplied(0, 90, 160, 200));
    ui.painter().galley(at, galley, Color32::WHITE);
}

/// What the colour picker found this frame.
pub struct Picked {
    pub rgb: [u8; 3],
}

/// Reads the screen under the mouse and shows the colour next to it. Works outside the window
/// while a mouse button is held (the window keeps the mouse then).
pub fn color_picker(ui: &Ui) -> Option<Picked> {
    let ctx = ui.ctx();
    ctx.set_cursor_icon(CursorIcon::Crosshair);
    ctx.request_repaint();
    let (pointer, inner, ppp) = ctx.input(|i| (i.pointer.latest_pos(), i.viewport().inner_rect, i.pixels_per_point()));
    let pointer = pointer?;
    let origin = inner.map(|r| r.min.to_vec2()).unwrap_or_default();
    let screen = (pointer + origin) * ppp;
    let rgb = platform::screen_pixel(screen.x.round() as i32, screen.y.round() as i32)?;
    let text = platform::describe(rgb);

    let rect = ui.max_rect();
    let size = vec2(190.0, 74.0);
    let mut at = pointer + vec2(18.0, 18.0);
    at.x = at.x.min(rect.right() - size.x - 6.0).max(rect.left() + 6.0);
    at.y = at.y.min(rect.bottom() - size.y - 6.0).max(rect.top() + 6.0);
    let panel = Rect::from_min_size(at, size);
    let painter = ui.painter();
    painter.rect(panel, 6.0, Color32::from_black_alpha(220), Stroke::new(1.0, Color32::from_gray(90)), StrokeKind::Inside);
    let swatch = Rect::from_min_size(panel.min + vec2(8.0, 8.0), vec2(58.0, 58.0));
    painter.rect(swatch, 4.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]), Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
    let lines = [text.hex.as_str(), &format!("RGB {}", text.rgb), &format!("HSV {}", text.hsv)];
    for (n, line) in lines.iter().enumerate() {
        let font = FontId::proportional(if n == 0 { 15.0 } else { 12.0 });
        painter.text(pos2(swatch.right() + 10.0, panel.top() + 10.0 + n as f32 * 20.0), Align2::LEFT_TOP, *line, font, Color32::WHITE);
    }
    Some(Picked { rgb })
}
