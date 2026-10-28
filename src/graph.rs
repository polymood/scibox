//! The graph tab: functions, equations, polar and parametric curves, and 3D surfaces.
//!
//! Each item of the input (separated by ";") is one of these kinds:
//!   f(x) or y = f(x)       function
//!   x = 2, x^2 + y^2 = 9   equation in x and y
//!   r = f(t)               polar curve, t from 0 to 2π
//!   (f(t), g(t))           parametric curve, t from 0 to 2π
//!   z = f(x, y)            3D surface

use crate::calc::{self, Ctx, Expr};
use crate::{App, ui};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::widgets::canvas::{Canvas, Context, Line as Segment, Points};
use std::f64::consts::TAU;

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

type Point = (f64, f64);

/// An item of the input, with its expressions compiled once for the whole graph.
enum Item {
    Function(Expr),
    /// The two sides of an equation.
    Equation(Expr, Expr),
    Polar(Expr),
    Parametric(Expr, Expr),
    Surface(Expr),
}

/// The items of the input: the text of each item, and the item or its syntax error.
fn parse(src: &str) -> Vec<(String, Result<Item, String>)> {
    src.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
            (s.to_string(), classify(&compact))
        })
        .collect()
}

fn classify(s: &str) -> Result<Item, String> {
    let c = |e: &str| calc::compile(e).map_err(|err| format!("{e}: {err}"));
    if let Some(rest) = s.strip_prefix("z=") {
        return Ok(Item::Surface(c(rest)?));
    }
    if let Some(rest) = s.strip_prefix("r=") {
        return Ok(Item::Polar(c(rest)?));
    }
    if let Some(rest) = s.strip_prefix("y=").filter(|r| !r.contains(['y', '='])) {
        return Ok(Item::Function(c(rest)?));
    }
    if let Some((a, b)) = s
        .strip_prefix('(')
        .and_then(|r| r.strip_suffix(')'))
        .and_then(split_comma)
    {
        return Ok(Item::Parametric(c(a)?, c(b)?));
    }
    match s.split_once('=') {
        Some((a, b)) => Ok(Item::Equation(c(a)?, c(b)?)),
        None => Ok(Item::Function(c(s)?)),
    }
}

/// Split at the comma that is not inside parentheses.
fn split_comma(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => return Some((&s[..i], &s[i + 1..])),
            _ => {}
        }
    }
    None
}

/// True if the input contains a 3D surface. Then a mouse drag turns the view.
pub fn is_3d(src: &str) -> bool {
    parse(src)
        .iter()
        .any(|(_, i)| matches!(i, Ok(Item::Surface(_))))
}

/// Calculate an expression. Return `None` for an error or a value that is not finite.
fn ev(expr: &Expr, ans: f64, x: f64, y: f64, t: f64) -> Option<f64> {
    let ctx = Ctx {
        ans,
        x: Some(x),
        y: Some(y),
        t: Some(t),
        ..Default::default()
    };
    expr.eval(&ctx).ok().filter(|v| v.is_finite())
}

/// A short number for the axis labels.
fn short(v: f64) -> String {
    let a = v.abs();
    if a != 0.0 && !(1e-3..1e5).contains(&a) {
        format!("{v:.2e}")
    } else {
        let s = format!("{v:.3}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s == "-0" { "0".into() } else { s.into() }
    }
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let [fields, chart_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    let field_areas: [Rect; 5] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
    ])
    .areas(fields);
    let titles = [
        " f(x), equations, r = f(t), (x(t), y(t)), z = f(x, y) ",
        " x min ",
        " x max ",
        " y min ",
        " y max ",
    ];
    app.areas.panes = Default::default();
    app.areas.fields.clear();
    for (i, (r, title)) in field_areas.iter().zip(titles).enumerate() {
        let focused = app.focus == i;
        let line = if i >= 3 && app.graph[i].is_empty() && !focused {
            Line::from("auto").dark_gray()
        } else {
            ui::input_line(&app.graph[i], focused)
        };
        f.render_widget(Paragraph::new(line).block(ui::block(title, focused)), *r);
        app.areas.fields.push(*r);
    }

    let items = parse(&app.graph[0]);
    let style = MARKERS[app.marker];
    // The title shows each item in the color of its curve.
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
    let mut chart_block = ui::block("", false).title(Line::from(title)).title(
        Line::from(format!(" {} (F2) ", style.1))
            .right_aligned()
            .dark_gray(),
    );
    if let Some(e) = items.iter().find_map(|(_, i)| i.as_ref().err()) {
        chart_block = chart_block.title_bottom(Line::from(format!(" {e} ")).red());
    }
    let inner = chart_block.inner(chart_area);
    f.render_widget(chart_block, chart_area);
    app.areas.chart = inner;

    let Some((a, b)) = app.x_range() else {
        let msg = Line::from(" x min must be a number less than x max ").red();
        f.render_widget(msg, inner);
        return;
    };
    let y_fields = app.y_fields();
    if let Some(Ok(Item::Surface(expr))) = items
        .iter()
        .map(|(_, i)| i)
        .find(|i| matches!(i, Ok(Item::Surface(_))))
    {
        let (c, d) = y_fields.unwrap_or((a, b));
        draw_3d(f, app, inner, expr, (a, b), (c, d));
    } else {
        draw_2d(f, app, inner, &items, (a, b), y_fields);
    }
}

fn draw_2d(
    f: &mut Frame,
    app: &mut App,
    inner: Rect,
    items: &[(String, Result<Item, String>)],
    (a, b): (f64, f64),
    y_fields: Option<(f64, f64)>,
) {
    let [top, x_labels] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);
    let [y_labels, plot] =
        Layout::horizontal([Constraint::Length(9), Constraint::Fill(1)]).areas(top);
    app.areas.chart = plot;
    let (cols, rows) = (plot.width.max(1) as usize, plot.height.max(1) as usize);
    let ans = app.ctx.ans;

    // Functions first: their values give the automatic y range.
    let n = cols * 2;
    let xs: Vec<f64> = (0..=n).map(|k| a + (b - a) * k as f64 / n as f64).collect();
    let functions: Vec<Option<Vec<Option<f64>>>> = items
        .iter()
        .map(|(_, item)| match item {
            Ok(Item::Function(e)) => Some(xs.iter().map(|&x| ev(e, ans, x, 0.0, 0.0)).collect()),
            _ => None,
        })
        .collect();

    // The y range: the fields, or the 1st to the 99th percentile of the functions (poles do not hide the curves).
    // If there are equations or curves, use the same scale as x (circles are round).
    // A terminal cell is about twice as high as it is wide.
    let only_functions = items.iter().all(|(_, i)| {
        !matches!(
            i,
            Ok(Item::Equation(..) | Item::Polar(_) | Item::Parametric(..))
        )
    });
    let (lo, hi) = y_fields.unwrap_or_else(|| {
        let mut ys: Vec<f64> = functions
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .copied()
            .collect();
        if ys.is_empty() || !only_functions {
            let half = (b - a) / 2.0 * 2.0 * rows as f64 / cols as f64;
            return (-half, half);
        }
        ys.sort_by(f64::total_cmp);
        let (mut lo, mut hi) = (ys[ys.len() / 100], ys[ys.len() - 1 - ys.len() / 100]);
        if hi - lo < 1e-12 {
            (lo, hi) = (lo - 1.0, hi + 1.0);
        }
        let pad = (hi - lo) * 0.05;
        (lo - pad, hi + pad)
    });
    app.y_shown = (lo, hi);

    // Make the polylines (functions, polar and parametric curves) and the points (equations).
    let mut lines: Vec<(Color, Vec<Vec<Point>>)> = Vec::new();
    let mut points: Vec<(Color, Vec<(f64, f64)>)> = Vec::new();
    let ts: Vec<f64> = (0..=720).map(|k| TAU * k as f64 / 720.0).collect();
    for (i, (_, item)) in items.iter().enumerate() {
        let Ok(item) = item else { continue };
        let color = CURVES[i % CURVES.len()];
        let pts: Vec<Option<(f64, f64)>> = match item {
            Item::Function(_) => {
                let ys = functions[i].as_ref().map_or(&[][..], |v| v.as_slice());
                xs.iter().zip(ys).map(|(&x, y)| y.map(|y| (x, y))).collect()
            }
            Item::Polar(e) => ts
                .iter()
                .map(|&t| ev(e, ans, 0.0, 0.0, t).map(|r| (r * t.cos(), r * t.sin())))
                .collect(),
            Item::Parametric(ex, ey) => ts
                .iter()
                .map(|&t| Some((ev(ex, ans, 0.0, 0.0, t)?, ev(ey, ans, 0.0, 0.0, t)?)))
                .collect(),
            Item::Equation(l, r) => {
                points.push((
                    color,
                    equation_points(l, r, ans, (a, b), (lo, hi), cols * 2, rows * 4),
                ));
                continue;
            }
            Item::Surface(_) => continue,
        };
        // Break the line at a missing point or at a jump higher than the plot (a pole).
        let mut parts: Vec<Vec<(f64, f64)>> = vec![Vec::new()];
        for p in pts {
            let last = parts.last_mut().unwrap();
            match (p, last.last()) {
                (Some(p), Some(q)) if (p.1 - q.1).abs() > hi - lo => parts.push(vec![p]),
                (Some(p), _) => last.push(p),
                (None, _) if !last.is_empty() => parts.push(Vec::new()),
                _ => {}
            }
        }
        lines.push((color, parts));
    }

    let canvas = Canvas::default()
        .marker(MARKERS[app.marker].0)
        .x_bounds([a, b])
        .y_bounds([lo, hi])
        .paint(|ctx| {
            for (color, parts) in &lines {
                for part in parts {
                    for w in part.windows(2) {
                        ctx.draw(&Segment::new(w[0].0, w[0].1, w[1].0, w[1].1, *color));
                    }
                }
            }
            for (color, coords) in &points {
                ctx.draw(&Points {
                    coords,
                    color: *color,
                });
            }
        });
    f.render_widget(canvas, plot);
    draw_axes(f, plot, (a, b), (lo, hi));

    // Axis labels
    let label = |v: f64| Span::from(short(v)).dark_gray();
    for (row, v) in [(0, hi), (rows / 2, (lo + hi) / 2.0), (rows - 1, lo)] {
        let r = Rect::new(y_labels.x, y_labels.y + row as u16, y_labels.width - 1, 1);
        f.render_widget(Line::from(label(v)).right_aligned(), r);
    }
    let r = Rect {
        x: plot.x,
        width: plot.width,
        ..x_labels
    };
    let [l, m, rr] = Layout::horizontal([Constraint::Fill(1); 3]).areas(r);
    f.render_widget(Line::from(label(a)), l);
    f.render_widget(Line::from(label((a + b) / 2.0)).centered(), m);
    f.render_widget(Line::from(label(b)).right_aligned(), rr);
}

/// The points where `l = r` changes from true to false: one test for each pixel of the plot.
fn equation_points(
    l: &Expr,
    r: &Expr,
    ans: f64,
    (a, b): (f64, f64),
    (lo, hi): (f64, f64),
    w: usize,
    h: usize,
) -> Vec<(f64, f64)> {
    let x = |i: usize| a + (b - a) * i as f64 / w as f64;
    let y = |j: usize| hi - (hi - lo) * j as f64 / h as f64;
    let grid: Vec<Vec<Option<f64>>> = (0..=h)
        .map(|j| {
            (0..=w)
                .map(|i| Some(ev(l, ans, x(i), y(j), 0.0)? - ev(r, ans, x(i), y(j), 0.0)?))
                .collect()
        })
        .collect();
    let mut out = Vec::new();
    for j in 0..h {
        for i in 0..w {
            let corners = [
                grid[j][i],
                grid[j][i + 1],
                grid[j + 1][i],
                grid[j + 1][i + 1],
            ];
            let Some(c) = corners.iter().copied().collect::<Option<Vec<f64>>>() else {
                continue;
            };
            if c.iter().any(|&v| v > 0.0) && c.iter().any(|&v| v <= 0.0) {
                out.push(((x(i) + x(i + 1)) / 2.0, (y(j) + y(j + 1)) / 2.0));
            }
        }
    }
    out
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
fn draw_3d(
    f: &mut Frame,
    app: &mut App,
    inner: Rect,
    expr: &Expr,
    (a, b): (f64, f64),
    (c, d): (f64, f64),
) {
    let [area, info_row] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);
    let n = 20;
    let ans = app.ctx.ans;
    let z: Vec<Vec<Option<f64>>> = (0..=n)
        .map(|i| {
            let x = a + (b - a) * i as f64 / n as f64;
            (0..=n)
                .map(|j| ev(expr, ans, x, c + (d - c) * j as f64 / n as f64, 0.0))
                .collect()
        })
        .collect();
    let finite: Vec<f64> = z.iter().flatten().flatten().copied().collect();
    let zmin = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let zmax = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let zspan = if zmax - zmin > 1e-12 {
        zmax - zmin
    } else {
        1.0
    };

    // Rotate a point of the box (x and y from -1 to 1, z from -0.8 to 0.8) and project it to the screen.
    let (yaw, pitch) = app.view;
    let project = |x: f64, y: f64, z: f64| {
        let (x1, y1) = (x * yaw.cos() - y * yaw.sin(), x * yaw.sin() + y * yaw.cos());
        (x1, y1 * pitch.sin() + z * pitch.cos())
    };
    let unit = |i: usize, j: usize| -> Option<(f64, f64, f64)> {
        let h = (z[i][j]? - zmin) / zspan;
        Some((
            2.0 * i as f64 / n as f64 - 1.0,
            2.0 * j as f64 / n as f64 - 1.0,
            1.6 * h - 0.8,
        ))
    };
    let mut segments: Vec<(Color, Point, Point)> = Vec::new();
    for i in 0..=n {
        for j in 0..=n {
            for (i2, j2) in [(i + 1, j), (i, j + 1)] {
                if i2 > n || j2 > n {
                    continue;
                }
                let (Some(p), Some(q)) = (unit(i, j), unit(i2, j2)) else {
                    continue;
                };
                let h = ((p.2 + q.2) / 2.0 + 0.8) / 1.6;
                let color = HEIGHTS[((h * HEIGHTS.len() as f64) as usize).min(HEIGHTS.len() - 1)];
                segments.push((color, project(p.0, p.1, p.2), project(q.0, q.1, q.2)));
            }
        }
    }
    // The axes start at the low corner of the box.
    let o = (-1.0, -1.0, -0.8);
    let axes = [
        ("x", (1.0, -1.0, -0.8)),
        ("y", (-1.0, 1.0, -0.8)),
        ("z", (-1.0, -1.0, 0.8)),
    ];

    // A terminal cell is about twice as high as it is wide. Use almost the same scale on both screen axes,
    // but make the view a little wider to fill a wide plot area.
    let ys = 1.75;
    let xs = (ys * area.width as f64 / (2.0 * area.height.max(1) as f64) / 1.5).max(ys * 1.1);
    let canvas = Canvas::default()
        .marker(MARKERS[app.marker].0)
        .x_bounds([-xs, xs])
        .y_bounds([-ys, ys])
        .paint(|ctx: &mut Context| {
            for (color, p, q) in &segments {
                ctx.draw(&Segment::new(p.0, p.1, q.0, q.1, *color));
            }
            ctx.layer();
            let po = project(o.0, o.1, o.2);
            for (name, end) in axes {
                let pe = project(end.0, end.1, end.2);
                ctx.draw(&Segment::new(po.0, po.1, pe.0, pe.1, Color::White));
                let pl = project(end.0 * 1.15, end.1 * 1.15, end.2 * 1.15);
                ctx.print(pl.0, pl.1, Span::from(name).white().bold());
            }
        });
    f.render_widget(canvas, area);
    let info = format!(
        " x {}..{}   y {}..{}   z {}..{}   drag: turn ",
        short(a),
        short(b),
        short(c),
        short(d),
        short(zmin),
        short(zmax)
    );
    f.render_widget(Line::from(info).style(Style::new().dark_gray()), info_row);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        let items = parse(
            "sin(x); y = x^2; sin(; x = 2; x^2 + y^2 = 9; r = 1 + cos(t); (cos(t), sin(2t)); z = x*y",
        );
        let kinds: Vec<&str> = items
            .iter()
            .map(|(_, i)| match i {
                Ok(Item::Function(_)) => "f",
                Ok(Item::Equation(..)) => "eq",
                Ok(Item::Polar(_)) => "polar",
                Ok(Item::Parametric(..)) => "param",
                Ok(Item::Surface(_)) => "3d",
                Err(_) => "error",
            })
            .collect();
        assert_eq!(
            kinds,
            ["f", "f", "error", "eq", "eq", "polar", "param", "3d"]
        );
    }

    #[test]
    fn vertical_line() {
        // x = 2 on a 40 x 20 pixel grid from -4 to 4: one column of points at x = 2.
        let (l, r) = (calc::compile("x").unwrap(), calc::compile("2").unwrap());
        let pts = equation_points(&l, &r, 0.0, (-4.0, 4.0), (-4.0, 4.0), 40, 20);
        assert_eq!(pts.len(), 20);
        assert!(pts.iter().all(|p| (p.0 - 2.0).abs() < 0.2));
    }
}
