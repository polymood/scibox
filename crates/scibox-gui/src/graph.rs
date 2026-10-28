//! The graph tab of the GUI: draw the result of `scibox_core::plot` with anti-aliased lines.

use crate::App;
use crate::look::{self, ACCENT, boxed};
use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, RichText, Sense, Stroke, TextEdit, Ui, pos2,
};
use scibox_core::calc::{self, Expr};
use scibox_core::plot::{self, Item, SETTINGS, Settings, short};

const CURVES: [Color32; 5] = [
    Color32::from_rgb(0x2e, 0xc4, 0xd6),
    Color32::from_rgb(0xe5, 0xc0, 0x7b),
    Color32::from_rgb(0xc6, 0x78, 0xdd),
    Color32::from_rgb(0x98, 0xc3, 0x79),
    Color32::from_rgb(0xe0, 0x6c, 0x75),
];
/// The colors of a 3D surface, from low to high.
const HEIGHTS: [Color32; 5] = [
    Color32::from_rgb(0x61, 0xaf, 0xef),
    Color32::from_rgb(0x2e, 0xc4, 0xd6),
    Color32::from_rgb(0x98, 0xc3, 0x79),
    Color32::from_rgb(0xe5, 0xc0, 0x7b),
    Color32::from_rgb(0xe0, 0x6c, 0x75),
];
/// The 3D surface modes. F2 selects the next mode.
pub const SURFACE_MODES: [&str; 3] = ["solid", "solid + mesh", "wireframe"];
/// The x and the y range of a graph.
type Ranges = ((f64, f64), (f64, f64));
/// The analysis text: a color and a line.
pub type Report = Vec<(Color32, String)>;

/// The analysis items that the graph can show. F4 shows or hides all of them.
pub const SHOW: [&str; 4] = ["min / max", "roots", "mean", "labels"];
const GRID: Color32 = Color32::from_rgb(0x2a, 0x30, 0x38);
const LABEL: Color32 = Color32::from_rgb(0x8a, 0x93, 0x9e);

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::Panel::top("graph input")
        .show_separator_line(false)
        .frame(look::panel())
        .show(ui, |ui| {
            boxed(
                ui,
                "f(x), equations, r = f(t), (x(t), y(t)), z = f(x, y) · separate the items with ;",
                app.focus == 0,
                false,
                |ui| {
                    let r = ui.add(
                        TextEdit::singleline(&mut app.graph[0])
                            .frame(egui::Frame::new())
                            .desired_width(f32::INFINITY),
                    );
                    keyboard_focus(app, ui, 0, &r);
                },
            );
        });
    egui::Panel::right("graph settings").exact_size(290.0).show_separator_line(false).frame(look::panel()).show(ui, |ui| {
        boxed(ui, "Settings", app.focus > 0, true, |ui| {
            egui::Grid::new("settings").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                for (k, (label, default)) in SETTINGS.iter().enumerate() {
                    let text = &mut app.graph[k + 1];
                    let ok = text.is_empty() || calc::eval(text, &app.ctx).is_ok();
                    let label = RichText::new(*label);
                    ui.label(if app.focus == k + 1 { label.color(ACCENT).strong() } else { label });
                    let hint = if default.is_empty() { "auto" } else { default };
                    let color = if ok { look::TEXT } else { look::RED };
                    let r = ui.add(TextEdit::singleline(text).hint_text(hint).text_color(color).desired_width(110.0));
                    keyboard_focus(app, ui, k + 1, &r);
                    ui.end_row();
                    if k % 2 == 1 && k < 6 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(8.0);
            ui.label("3D surface (F2)");
            ui.horizontal_wrapped(|ui| {
                for (i, mode) in SURFACE_MODES.iter().enumerate() {
                    if look::item(ui, app.surface == i, *mode).clicked() {
                        app.surface = i;
                    }
                }
            });
            ui.add_space(8.0);
            ui.label("Analysis (F4)");
            ui.horizontal_wrapped(|ui| {
                for (i, name) in SHOW.iter().enumerate() {
                    if look::item(ui, app.show[i], *name).clicked() {
                        app.show[i] = !app.show[i];
                    }
                }
            });
            ui.add_space(8.0);
            if ui.button("Export PNG (Ctrl+S)").clicked() {
                app.export = true;
            }
            if ui.button("Reset the view").clicked() {
                for (k, (_, default)) in SETTINGS.iter().enumerate().take(4) {
                    app.graph[k + 1] = default.to_string();
                }
                app.view = (0.6, 0.6);
            }
            ui.add_space(8.0);
            egui::ScrollArea::vertical().max_height(ui.available_height() - 70.0).show(ui, |ui| {
                for (color, text) in &app.report {
                    ui.label(RichText::new(text).color(*color));
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Wheel: zoom at the pointer\nDrag: move (3D: turn)\nEmpty y range: automatic").color(look::GRAY));
        });
    });
    egui::CentralPanel::default()
        .frame(look::panel())
        .show(ui, |ui| {
            let items = plot::parse(&app.graph[0]);
            boxed(ui, "Graph", false, true, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
                let s = match Settings::read(&app.graph[1..], &app.ctx) {
                    Ok(s) => s,
                    Err(e) => {
                        painter.text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            e,
                            FontId::monospace(16.0),
                            look::RED,
                        );
                        return;
                    }
                };
                match plot::surface(&items) {
                    Some(expr) => {
                        let report =
                            draw_3d(&painter, rect, expr, &s, app.view, app.surface, app.show);
                        set_report(app, ui, report);
                        if response.dragged() {
                            let d = response.drag_delta();
                            app.view.0 += d.x as f64 * 0.01;
                            app.view.1 = (app.view.1 + d.y as f64 * 0.01)
                                .clamp(0.0, std::f64::consts::FRAC_PI_2);
                        }
                        let shown = (s.x, s.y.unwrap_or(s.x));
                        zoom_and_pan(app, ui, &response, rect, shown, false);
                    }
                    None => {
                        let hover = response.hover_pos().filter(|_| !response.dragged());
                        let (shown, report) = draw_2d(&painter, rect, &items, &s, app.show, hover);
                        set_report(app, ui, report);
                        zoom_and_pan(app, ui, &response, rect, shown, true);
                    }
                }
                if std::mem::take(&mut app.export) {
                    export_png(ui.ctx(), rect, app.export_done.clone());
                }
                // The legend and the syntax errors, at the top left: a square of the color of each curve.
                let mut y = rect.top() + 8.0;
                for (i, (text, item)) in items.iter().enumerate() {
                    let (color, text) = match item {
                        Ok(Item::Surface(_)) => (Color32::WHITE, text.clone()),
                        Ok(_) => (CURVES[i % CURVES.len()], text.clone()),
                        Err(e) => (look::RED, e.clone()),
                    };
                    let pos = pos2(rect.left() + 12.0, y);
                    painter.rect_filled(
                        Rect::from_min_size(pos + egui::vec2(0.0, 3.0), egui::vec2(10.0, 10.0)),
                        0.0,
                        color,
                    );
                    painter.text(
                        pos + egui::vec2(16.0, 0.0),
                        Align2::LEFT_TOP,
                        text,
                        FontId::monospace(14.0),
                        LABEL,
                    );
                    y += 20.0;
                }
            });
        });
}

/// Keep the analysis text for the sidebar. The sidebar shows it at the next frame.
fn set_report(app: &mut App, ui: &Ui, report: Vec<(Color32, String)>) {
    if report != app.report {
        app.report = report;
        ui.ctx().request_repaint();
    }
}

/// Save the plot area as a PNG file in the home folder. The result message goes to `done`.
fn export_png(
    ctx: &egui::Context,
    rect: Rect,
    done: std::sync::Arc<std::sync::Mutex<Option<String>>>,
) {
    let ppp = ctx.pixels_per_point();
    let repaint = ctx.clone();
    let save = move |image: std::sync::Arc<egui::ColorImage>| {
        let [w, h] = image.size;
        let x0 = ((rect.left() * ppp).round().max(0.0) as usize).min(w);
        let y0 = ((rect.top() * ppp).round().max(0.0) as usize).min(h);
        let x1 = ((rect.right() * ppp).round() as usize).clamp(x0, w);
        let y1 = ((rect.bottom() * ppp).round() as usize).clamp(y0, h);
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .unwrap_or_default();
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let path = std::path::PathBuf::from(home).join(format!("scibox-graph-{secs}.png"));
        let message = match tiny_skia::Pixmap::new((x1 - x0) as u32, (y1 - y0) as u32) {
            Some(mut pixmap) => {
                for (k, px) in pixmap.pixels_mut().iter_mut().enumerate() {
                    let (x, y) = (x0 + k % (x1 - x0), y0 + k / (x1 - x0));
                    let c = image.pixels[y * w + x];
                    *px = tiny_skia::ColorU8::from_rgba(c.r(), c.g(), c.b(), 255).premultiply();
                }
                match pixmap.save_png(&path) {
                    Ok(()) => format!("Saved {}", path.display()),
                    Err(e) => format!("PNG export failed: {e}"),
                }
            }
            None => "PNG export failed: the graph is empty".into(),
        };
        if let Ok(mut d) = done.lock() {
            *d = Some(message);
        }
        repaint.request_repaint();
    };
    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
        egui::ScreenshotCallback::new(save),
    ));
}

/// Write a label with a dark background, so that it is readable above the curves.
fn label(painter: &Painter, pos: Pos2, align: Align2, text: String, color: Color32) {
    let galley = painter.layout_no_wrap(text, FontId::monospace(12.0), color);
    let rect = align.anchor_size(pos, galley.size()).expand(2.0);
    painter.rect_filled(rect, 0.0, Color32::from_black_alpha(190));
    painter.galley(rect.min + egui::vec2(2.0, 2.0), galley, color);
}

/// A point marker: a filled circle with a dark border.
fn dot(painter: &Painter, pos: Pos2, radius: f32, color: Color32) {
    painter.circle(pos, radius, color, Stroke::new(1.5, Color32::BLACK));
}

/// Give the keyboard to the field `i` if it has the focus of the TUI keys. A click on a field gives it the focus.
fn keyboard_focus(app: &mut App, ui: &Ui, i: usize, r: &egui::Response) {
    if r.gained_focus() && app.focus != i {
        app.focus = i;
    }
    if app.focus == i && app.palette.is_none() {
        if std::mem::take(&mut app.fresh) {
            look::select_all(ui, r, &app.graph[i]);
        }
        r.request_focus();
    }
}

/// Zoom at the pointer with the wheel. Move the graph with a drag (2D only).
/// Write the new ranges in the settings. A zoom or a move makes an automatic y range fixed.
fn zoom_and_pan(
    app: &mut App,
    ui: &Ui,
    response: &egui::Response,
    rect: Rect,
    shown: ((f64, f64), (f64, f64)),
    pan: bool,
) {
    let ((a, b), (lo, hi)) = shown;
    let mut x = (a, b);
    let mut y = (lo, hi);
    let mut changed = false;
    if pan && response.dragged() {
        let d = response.drag_delta();
        let dx = -d.x as f64 * (b - a) / rect.width() as f64;
        let dy = d.y as f64 * (hi - lo) / rect.height() as f64;
        x = (a + dx, b + dx);
        y = (lo + dy, hi + dy);
        changed = true;
    }
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if response.hovered() && scroll != 0.0 {
        let f = (-scroll as f64 * 0.002).exp();
        let p = response.hover_pos().unwrap_or(rect.center());
        let px = a + (b - a) * ((p.x - rect.left()) / rect.width()) as f64;
        let py = hi - (hi - lo) * ((p.y - rect.top()) / rect.height()) as f64;
        x = (px + (x.0 - px) * f, px + (x.1 - px) * f);
        y = (py + (y.0 - py) * f, py + (y.1 - py) * f);
        changed = true;
    }
    if changed {
        for (k, v) in [x.0, x.1, y.0, y.1].into_iter().enumerate() {
            app.graph[k + 1] = calc::fmt(v);
        }
    }
}

/// Round numbers for the grid lines: about `count` values from `lo` to `hi`.
fn ticks(lo: f64, hi: f64, count: f64) -> Vec<f64> {
    let raw = (hi - lo) / count;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|m| m * mag)
        .find(|&s| s >= raw)
        .unwrap_or(10.0 * mag);
    let mut v = (lo / step).ceil() * step;
    let mut out = Vec::new();
    while v <= hi && out.len() < 100 {
        out.push(if v.abs() < step * 1e-9 { 0.0 } else { v });
        v += step;
    }
    out
}

/// Draw a 2D graph and return the ranges that it shows.
/// `show`: the analysis items (see `SHOW`). `hover`: the mouse pointer, for the value readout.
/// Return the ranges that the graph shows, and the analysis text.
fn draw_2d(
    painter: &Painter,
    rect: Rect,
    items: &[(String, Result<Item, String>)],
    s: &Settings,
    show: [bool; 4],
    hover: Option<Pos2>,
) -> (Ranges, Report) {
    let aspect = (rect.height() / rect.width()) as f64;
    let w = rect.width() as usize;
    let grid = (
        (rect.width() / 3.0) as usize,
        (rect.height() / 3.0) as usize,
    );
    let p = plot::plot_2d(items, s, w, grid, aspect);
    let ((a, b), (lo, hi)) = (p.x, p.y);
    let to_screen = |x: f64, y: f64| {
        pos2(
            rect.left() + ((x - a) / (b - a)) as f32 * rect.width(),
            rect.top() + ((hi - y) / (hi - lo)) as f32 * rect.height(),
        )
    };

    // The grid, the labels and the axes
    let font = FontId::monospace(12.0);
    for x in ticks(a, b, rect.width() as f64 / 110.0) {
        let sx = to_screen(x, 0.0).x;
        painter.line_segment(
            [pos2(sx, rect.top()), pos2(sx, rect.bottom())],
            Stroke::new(1.0, GRID),
        );
        painter.text(
            pos2(sx + 3.0, rect.bottom() - 3.0),
            Align2::LEFT_BOTTOM,
            short(x),
            font.clone(),
            LABEL,
        );
    }
    for y in ticks(lo, hi, rect.height() as f64 / 70.0) {
        let sy = to_screen(0.0, y).y;
        painter.line_segment(
            [pos2(rect.left(), sy), pos2(rect.right(), sy)],
            Stroke::new(1.0, GRID),
        );
        painter.text(
            pos2(rect.right() - 4.0, sy - 2.0),
            Align2::RIGHT_BOTTOM,
            short(y),
            font.clone(),
            LABEL,
        );
    }
    let axis = Stroke::new(1.5, Color32::WHITE);
    if a < 0.0 && b > 0.0 {
        let sx = to_screen(0.0, 0.0).x;
        painter.line_segment([pos2(sx, rect.top()), pos2(sx, rect.bottom())], axis);
    }
    if lo < 0.0 && hi > 0.0 {
        let sy = to_screen(0.0, 0.0).y;
        painter.line_segment([pos2(rect.left(), sy), pos2(rect.right(), sy)], axis);
    }

    // The curves
    for (i, parts) in &p.curves {
        let stroke = Stroke::new(2.0, CURVES[i % CURVES.len()]);
        for part in parts.iter().filter(|part| part.len() > 1) {
            let points: Vec<Pos2> = part.iter().map(|&(x, y)| to_screen(x, y)).collect();
            if points.len() == 2 {
                painter.line_segment([points[0], points[1]], stroke);
            } else {
                painter.line(points, stroke);
            }
        }
    }

    // The analysis of each function: markers, labels, and a text for the sidebar.
    let mut report = Vec::new();
    let xy = |p: plot::Point| format!("({}, {})", short(p.0), short(p.1));
    for an in plot::analyze(items, &p) {
        let color = CURVES[an.item % CURVES.len()];
        report.push((color, format!("■ {}", items[an.item].0)));
        report.push((
            LABEL,
            format!("  max {} at x = {}", short(an.max.1), short(an.max.0)),
        ));
        report.push((
            LABEL,
            format!("  min {} at x = {}", short(an.min.1), short(an.min.0)),
        ));
        report.push((LABEL, format!("  mean {}", short(an.mean))));
        if !an.roots.is_empty() {
            let roots: Vec<String> = an.roots.iter().take(6).map(|r| short(r.0)).collect();
            report.push((LABEL, format!("  roots {}", roots.join(", "))));
        }
        if show[2] && an.mean.is_finite() {
            let y = to_screen(0.0, an.mean).y;
            painter.extend(egui::Shape::dashed_line(
                &[pos2(rect.left(), y), pos2(rect.right(), y)],
                Stroke::new(1.0, color),
                6.0,
                4.0,
            ));
            if show[3] {
                label(
                    painter,
                    pos2(rect.right() - 60.0, y - 3.0),
                    Align2::RIGHT_BOTTOM,
                    format!("mean {}", short(an.mean)),
                    color,
                );
            }
        }
        if show[1] {
            for &r in &an.roots {
                painter.circle_stroke(to_screen(r.0, 0.0), 4.0, Stroke::new(1.5, color));
                if show[3] {
                    label(
                        painter,
                        to_screen(r.0, 0.0) + egui::vec2(4.0, 6.0),
                        Align2::LEFT_TOP,
                        short(r.0),
                        color,
                    );
                }
            }
        }
        if show[0] {
            for &e in &an.extrema {
                dot(painter, to_screen(e.0, e.1), 3.5, color);
                if show[3] && e != an.max && e != an.min {
                    label(
                        painter,
                        to_screen(e.0, e.1) + egui::vec2(0.0, -8.0),
                        Align2::CENTER_BOTTOM,
                        xy(e),
                        color,
                    );
                }
            }
            for (name, e) in [("max", an.max), ("min", an.min)] {
                let pos = to_screen(e.0, e.1);
                dot(painter, pos, 5.0, color);
                if show[3] {
                    let (align, dy) = if name == "max" {
                        (Align2::CENTER_BOTTOM, -9.0)
                    } else {
                        (Align2::CENTER_TOP, 9.0)
                    };
                    label(
                        painter,
                        pos + egui::vec2(0.0, dy),
                        align,
                        format!("{name} {}", xy(e)),
                        color,
                    );
                }
            }
        }
    }

    // The value readout at the mouse pointer: a vertical line, and the value of each function.
    if let Some(h) = hover.filter(|h| rect.contains(*h)) {
        let x = a + (b - a) * ((h.x - rect.left()) / rect.width()) as f64;
        painter.line_segment(
            [pos2(h.x, rect.top()), pos2(h.x, rect.bottom())],
            Stroke::new(1.0, LABEL),
        );
        let mut lines = vec![(LABEL, format!("x = {}", short(x)))];
        for (i, (_, item)) in items.iter().enumerate() {
            if let Some(y) = item
                .as_ref()
                .ok()
                .and_then(|it| plot::value_at(it, x, s.ans))
            {
                let color = CURVES[i % CURVES.len()];
                dot(painter, to_screen(x, y), 4.0, color);
                lines.push((color, format!("y = {}", short(y))));
            }
        }
        for (k, (color, text)) in lines.into_iter().enumerate() {
            label(
                painter,
                pos2(h.x + 10.0, rect.top() + 10.0 + 18.0 * k as f32),
                Align2::LEFT_TOP,
                text,
                color,
            );
        }
    }
    ((p.x, p.y), report)
}

/// Draw z = f(x, y) as a wireframe. The colors show the height.
/// `mode` is an index in `SURFACE_MODES`.
fn draw_3d(
    painter: &Painter,
    rect: Rect,
    expr: &Expr,
    s: &Settings,
    view: (f64, f64),
    mode: usize,
    show: [bool; 4],
) -> Vec<(Color32, String)> {
    let surface = plot::surface_grid(expr, s);
    let scale = (rect.width().min(rect.height()) / 3.4) as f64;
    let c = rect.center();
    let to_screen = |p: [f64; 3]| {
        let (x, y) = plot::project(p, view);
        pos2(c.x + (x * scale) as f32, c.y - (y * scale) as f32)
    };
    let o = to_screen(plot::ORIGIN);
    for (name, end) in plot::AXES {
        painter.line_segment([o, to_screen(end)], Stroke::new(1.5, Color32::WHITE));
        let l = to_screen(end.map(|v| v * 1.12));
        painter.text(
            l,
            Align2::CENTER_CENTER,
            name,
            FontId::monospace(18.0),
            Color32::WHITE,
        );
    }
    if mode == 2 {
        for (h, p, q) in surface.segments() {
            painter.line_segment(
                [to_screen(p), to_screen(q)],
                Stroke::new(1.2, height_color(h)),
            );
        }
    } else {
        // The cells come from the farthest to the nearest: a near cell covers the far cells behind it.
        let to_pos = |(x, y): plot::Point| pos2(c.x + (x * scale) as f32, c.y - (y * scale) as f32);
        for q in surface.quads(view) {
            let color = shade(height_color(q.height), q.light);
            let corners = q.corners.map(to_pos);
            let mut mesh = egui::Mesh::default();
            for p in corners {
                mesh.colored_vertex(p, color);
            }
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(egui::Shape::mesh(mesh));
            // Separate segments, not a closed line: the joins of a closed line make spikes at sharp corners.
            if mode == 1 {
                let edge = Stroke::new(1.0, shade(color, 0.55));
                for k in 0..4 {
                    painter.line_segment([corners[k], corners[(k + 1) % 4]], edge);
                }
            }
        }
    }
    // The highest and the lowest point, and the analysis text.
    let mut report = vec![(Color32::WHITE, "■ surface".to_string())];
    for (name, point) in [("max", surface.high), ("min", surface.low)] {
        let Some((pos, real)) = point else { continue };
        let text = format!(
            "{name} z = {} at ({}, {})",
            short(real[2]),
            short(real[0]),
            short(real[1])
        );
        if show[0] {
            let p = to_screen(pos);
            dot(painter, p, 5.0, Color32::WHITE);
            if show[3] {
                label(
                    painter,
                    p + egui::vec2(0.0, -9.0),
                    Align2::CENTER_BOTTOM,
                    text.clone(),
                    Color32::WHITE,
                );
            }
        }
        report.push((LABEL, format!("  {text}")));
    }
    report.push((LABEL, format!("  mean z = {}", short(surface.mean))));
    let (y0, y1) = s.y.unwrap_or(s.x);
    let info = format!(
        "x {}..{}   y {}..{}   z {}..{}",
        short(s.x.0),
        short(s.x.1),
        short(y0),
        short(y1),
        short(surface.z.0),
        short(surface.z.1)
    );
    painter.text(
        rect.left_bottom() + egui::vec2(12.0, -10.0),
        Align2::LEFT_BOTTOM,
        info,
        FontId::monospace(13.0),
        LABEL,
    );
    painter.text(
        rect.right_bottom() + egui::vec2(-12.0, -10.0),
        Align2::RIGHT_BOTTOM,
        "drag: turn",
        FontId::monospace(13.0),
        ACCENT,
    );
    report
}

/// Multiply the brightness of a color by `light`.
fn shade(c: Color32, light: f64) -> Color32 {
    let f = |v: u8| (v as f64 * light).round().min(255.0) as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

/// A color from blue (h = 0) to red (h = 1).
fn height_color(h: f64) -> Color32 {
    let x = h.clamp(0.0, 1.0) * (HEIGHTS.len() - 1) as f64;
    let (i, f) = (x.floor() as usize, (x - x.floor()) as f32);
    let (a, b) = (HEIGHTS[i], HEIGHTS[(i + 1).min(HEIGHTS.len() - 1)]);
    let mix = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * f) as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn ticks_are_round() {
        assert_eq!(
            super::ticks(-10.0, 10.0, 4.0),
            [-10.0, -5.0, 0.0, 5.0, 10.0]
        );
    }
}
