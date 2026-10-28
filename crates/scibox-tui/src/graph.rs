//! The graph tab of the TUI: draw the result of `scibox_core::plot` with pseudo-pixel characters.

use crate::{App, ui};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::widgets::canvas::{Canvas, Line as Segment};
use scibox_core::calc::{self, Expr};
use scibox_core::plot::{self, Item, SETTINGS, Settings, short};

/// The line styles. F2 selects the next style.
pub const MARKERS: [(Marker, &str); 4] = [
    (Marker::Octant, "solid"),
    (Marker::Braille, "dots"),
    (Marker::Quadrant, "blocks"),
    (Marker::HalfBlock, "half blocks"),
];
const CURVES: [Color; 5] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::Red,
];
/// The colors of a 3D surface, from low to high.
const HEIGHTS: [Color; 5] = [
    Color::Blue,
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Red,
];
/// The sidebar row of each setting. The empty rows separate the groups.
const SETTING_ROWS: [u16; 8] = [0, 1, 2, 3, 5, 6, 8, 9];
const STYLE_ROW: u16 = 11;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let [input_area, body] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    let [chart_area, sidebar] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(30)]).areas(body);
    app.areas.panes = Default::default();
    app.areas.fields.clear();

    // The input of the items
    let title = " f(x), equations, r = f(t), (x(t), y(t)), z = f(x, y) ";
    let focused = app.focus == 0;
    let p = Paragraph::new(ui::input_line(&app.graph[0], focused)).block(ui::block(title, focused));
    f.render_widget(p, input_area);
    app.areas.fields.push(input_area);

    // The sidebar: one row for each setting, then the line style.
    let side_block = ui::block(" Settings ", app.focus > 0)
        .title_bottom(Line::from(" Tab: next · Del: default ").dark_gray());
    let inner = side_block.inner(sidebar);
    f.render_widget(side_block, sidebar);
    for (k, ((label, _), row)) in SETTINGS.iter().zip(SETTING_ROWS).enumerate() {
        let i = k + 1;
        let r = Rect {
            y: inner.y + row,
            height: 1,
            ..inner
        };
        let focused = app.focus == i;
        let text = &app.graph[i];
        let value = if text.is_empty() && !focused {
            let auto = if i == 3 || i == 4 { "auto" } else { "default" };
            Span::from(auto).dark_gray()
        } else if text.is_empty() || calc::eval(text, &app.ctx).is_ok() {
            Span::from(text.as_str()).bold()
        } else {
            Span::from(text.as_str()).red()
        };
        let label = Span::from(format!(" {label:<14}"));
        let mut spans = vec![
            if focused {
                label.fg(ui::ACCENT).bold()
            } else {
                label
            },
            value,
        ];
        if focused {
            spans.push(Span::from("█").fg(ui::ACCENT));
        }
        f.render_widget(Line::from(spans), r);
        app.areas.fields.push(r);
    }
    let r = Rect {
        y: inner.y + STYLE_ROW,
        height: 1,
        ..inner
    };
    let style = Line::from(vec![
        Span::from(format!(" {:<14}", "line style")),
        Span::from(format!(" {} ", MARKERS[app.marker].1))
            .black()
            .on_dark_gray(),
        Span::from(" F2").dark_gray(),
    ]);
    f.render_widget(style, r);
    app.areas.style = r;

    // The title shows each item in the color of its curve.
    let items = plot::parse(&app.graph[0]);
    let mut title = vec![Span::from(" ")];
    for (i, (text, item)) in items.iter().enumerate() {
        let color = if matches!(item, Ok(Item::Surface(_))) {
            Color::White
        } else {
            CURVES[i % CURVES.len()]
        };
        title.push(Span::from("■ ").fg(color));
        title.push(Span::from(format!("{text}   ")));
    }
    let mut chart_block = ui::block("", false).title(Line::from(title));
    if let Some(e) = items.iter().find_map(|(_, i)| i.as_ref().err()) {
        chart_block = chart_block.title_bottom(Line::from(format!(" {e} ")).red());
    }
    let inner = chart_block.inner(chart_area);
    f.render_widget(chart_block, chart_area);
    app.areas.chart = inner;

    let s = match Settings::read(&app.graph[1..], &app.ctx) {
        Ok(s) => s,
        Err(e) => return f.render_widget(Line::from(format!(" {e} ")).red(), inner),
    };
    match plot::surface(&items) {
        Some(expr) => draw_3d(f, app, inner, expr, &s),
        None => draw_2d(f, app, inner, &items, &s),
    }
}

fn draw_2d(
    f: &mut Frame,
    app: &mut App,
    inner: Rect,
    items: &[(String, Result<Item, String>)],
    s: &Settings,
) {
    let [top, x_labels] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);
    let [y_labels, plot_area] =
        Layout::horizontal([Constraint::Length(9), Constraint::Fill(1)]).areas(top);
    app.areas.chart = plot_area;
    let (cols, rows) = (
        plot_area.width.max(1) as usize,
        plot_area.height.max(1) as usize,
    );
    // A terminal cell is about twice as high as it is wide.
    let aspect = 2.0 * rows as f64 / cols as f64;
    let p = plot::plot_2d(items, s, cols * 2, (cols * 2, rows * 4), aspect);
    app.y_shown = p.y;
    let ((a, b), (lo, hi)) = (p.x, p.y);

    let canvas = Canvas::default()
        .marker(MARKERS[app.marker].0)
        .x_bounds([a, b])
        .y_bounds([lo, hi])
        .paint(|ctx| {
            for (i, parts) in &p.curves {
                let color = CURVES[i % CURVES.len()];
                for part in parts {
                    for w in part.windows(2) {
                        ctx.draw(&Segment::new(w[0].0, w[0].1, w[1].0, w[1].1, color));
                    }
                }
            }
        });
    f.render_widget(canvas, plot_area);
    draw_axes(f, plot_area, (a, b), (lo, hi));

    // Axis labels
    let label = |v: f64| Span::from(short(v)).dark_gray();
    for (row, v) in [(0, hi), (rows / 2, (lo + hi) / 2.0), (rows - 1, lo)] {
        let r = Rect::new(y_labels.x, y_labels.y + row as u16, y_labels.width - 1, 1);
        f.render_widget(Line::from(label(v)).right_aligned(), r);
    }
    let r = Rect {
        x: plot_area.x,
        width: plot_area.width,
        ..x_labels
    };
    let [l, m, rr] = Layout::horizontal([Constraint::Fill(1); 3]).areas(r);
    f.render_widget(Line::from(label(a)), l);
    f.render_widget(Line::from(label((a + b) / 2.0)).centered(), m);
    f.render_widget(Line::from(label(b)).right_aligned(), rr);
}

/// Draw the axes x = 0 and y = 0 as solid white lines in the empty cells of the plot.
fn draw_axes(f: &mut Frame, plot: Rect, (a, b): (f64, f64), (lo, hi): (f64, f64)) {
    let col = (a < 0.0 && b > 0.0)
        .then(|| plot.x + ((-a) / (b - a) * (plot.width - 1) as f64).round() as u16);
    let row = (lo < 0.0 && hi > 0.0)
        .then(|| plot.y + (hi / (hi - lo) * (plot.height - 1) as f64).round() as u16);
    let buf = f.buffer_mut();
    for y in plot.top()..plot.bottom() {
        for x in plot.left()..plot.right() {
            let symbol = match (Some(x) == col, Some(y) == row) {
                (true, true) => "┼",
                (true, false) => "│",
                (false, true) => "─",
                _ => continue,
            };
            let cell = &mut buf[(x, y)];
            if cell.symbol() == " " {
                cell.set_symbol(symbol).set_fg(Color::White);
            }
        }
    }
}

/// Draw z = f(x, y) as a wireframe. The colors show the height.
fn draw_3d(f: &mut Frame, app: &mut App, inner: Rect, expr: &Expr, s: &Settings) {
    let [area, info_row] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);
    let surface = plot::surface_grid(expr, s);
    let view = app.view;
    let segments: Vec<(Color, plot::Point, plot::Point)> = surface
        .segments()
        .into_iter()
        .map(|(h, p, q)| {
            let color = HEIGHTS[((h * HEIGHTS.len() as f64) as usize).min(HEIGHTS.len() - 1)];
            (color, plot::project(p, view), plot::project(q, view))
        })
        .collect();

    // A terminal cell is about twice as high as it is wide. Use almost the same scale on both screen axes,
    // but make the view a little wider to fill a wide plot area.
    let ys = 1.75;
    let xs = (ys * area.width as f64 / (2.0 * area.height.max(1) as f64) / 1.5).max(ys * 1.1);
    let canvas = Canvas::default()
        .marker(MARKERS[app.marker].0)
        .x_bounds([-xs, xs])
        .y_bounds([-ys, ys])
        .paint(|ctx| {
            for (color, p, q) in &segments {
                ctx.draw(&Segment::new(p.0, p.1, q.0, q.1, *color));
            }
            ctx.layer();
            let o = plot::project(plot::ORIGIN, view);
            for (name, end) in plot::AXES {
                let e = plot::project(end, view);
                ctx.draw(&Segment::new(o.0, o.1, e.0, e.1, Color::White));
                let l = plot::project(end.map(|c| c * 1.15), view);
                ctx.print(l.0, l.1, Span::from(name).white().bold());
            }
        });
    f.render_widget(canvas, area);
    let (c, d) = s.y.unwrap_or(s.x);
    let info = format!(
        " x {}..{}   y {}..{}   z {}..{}   drag: turn ",
        short(s.x.0),
        short(s.x.1),
        short(c),
        short(d),
        short(surface.z.0),
        short(surface.z.1)
    );
    f.render_widget(Line::from(info).style(Style::new().dark_gray()), info_row);
}
