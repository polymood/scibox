//! The graph math: the TUI and the GUI draw the result of these functions.
//!
//! Each item of the input (separated by ";") is one of these kinds:
//!   f(x) or y = f(x)       function
//!   x = 2, x^2 + y^2 = 9   equation in x and y
//!   r = f(t)               polar curve
//!   (f(t), g(t))           parametric curve
//!   z = f(x, y)            3D surface

use crate::calc::{self, Ctx, Expr};
use std::f64::consts::TAU;

pub type Point = (f64, f64);

/// The settings of the graph: the label and the default text. Setting `k` follows the input of the items.
pub const SETTINGS: [(&str, &str); 8] = [
    ("x min", "-10"),
    ("x max", "10"),
    ("y min", ""),
    ("y max", ""),
    ("t min", "0"),
    ("t max", "2pi"),
    ("3D samples", "20"),
    ("curve samples", "720"),
];

/// An item of the input, with its expressions compiled once for the whole graph.
pub enum Item {
    Function(Expr),
    /// The two sides of an equation.
    Equation(Expr, Expr),
    Polar(Expr),
    Parametric(Expr, Expr),
    Surface(Expr),
}

/// The items of the input: the text of each item, and the item or its syntax error.
pub fn parse(src: &str) -> Vec<(String, Result<Item, String>)> {
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

/// The first 3D surface of the items, if there is one.
pub fn surface(items: &[(String, Result<Item, String>)]) -> Option<&Expr> {
    items.iter().find_map(|(_, i)| match i {
        Ok(Item::Surface(e)) => Some(e),
        _ => None,
    })
}

/// True if the input contains a 3D surface. Then a mouse drag turns the view.
pub fn is_3d(src: &str) -> bool {
    surface(&parse(src)).is_some()
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

/// The value of a function item at `x`, or `None` for another kind of item or a value that is not finite.
pub fn value_at(item: &Item, x: f64, ans: f64) -> Option<f64> {
    match item {
        Item::Function(e) => ev(e, ans, x, 0.0, 0.0),
        _ => None,
    }
}

/// A short number for the axis labels.
pub fn short(v: f64) -> String {
    let a = v.abs();
    if a != 0.0 && !(1e-3..1e5).contains(&a) {
        format!("{v:.2e}")
    } else {
        let s = format!("{v:.3}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s == "-0" { "0".into() } else { s.into() }
    }
}

/// The values of the settings. `None` for an empty y range means "automatic".
pub struct Settings {
    pub x: (f64, f64),
    pub y: Option<(f64, f64)>,
    pub t: (f64, f64),
    /// The grid points on each axis of a 3D surface.
    pub grid: usize,
    /// The points on a polar or a parametric curve.
    pub samples: usize,
    /// The value of `ans` in the expressions.
    pub ans: f64,
}

impl Settings {
    /// Read the settings from their texts (see `SETTINGS`). Return an error if the x range is not correct.
    pub fn read(texts: &[String], ctx: &Ctx) -> Result<Self, String> {
        let num = |k: usize| calc::eval(&texts[k], ctx).ok().filter(|v| v.is_finite());
        let range = |k: usize| Some((num(k)?, num(k + 1)?)).filter(|(a, b)| a < b);
        let x = range(0).ok_or("x min must be a number less than x max")?;
        Ok(Settings {
            x,
            y: range(2),
            t: (num(4).unwrap_or(0.0), num(5).unwrap_or(TAU)),
            grid: (num(6).unwrap_or(20.0).round() as usize).clamp(2, 200),
            samples: (num(7).unwrap_or(720.0).round() as usize).clamp(10, 100_000),
            ans: ctx.ans,
        })
    }
}

/// A 2D graph: the ranges, and the polylines of each item.
pub struct Plot2d {
    pub x: (f64, f64),
    pub y: (f64, f64),
    /// The index of the item and its polylines.
    pub curves: Vec<(usize, Vec<Vec<Point>>)>,
}

/// Calculate a 2D graph.
/// `columns`: the number of x samples for the functions (for example, one for each pixel column).
/// `grid`: the number of cells (columns, rows) for the equations.
/// `aspect`: the height of the plot divided by its width on the screen. It keeps circles round.
pub fn plot_2d(
    items: &[(String, Result<Item, String>)],
    s: &Settings,
    columns: usize,
    grid: (usize, usize),
    aspect: f64,
) -> Plot2d {
    let (a, b) = s.x;
    let n = columns.max(2);
    let xs: Vec<f64> = (0..=n).map(|k| a + (b - a) * k as f64 / n as f64).collect();
    let functions: Vec<Option<Vec<Option<f64>>>> = items
        .iter()
        .map(|(_, item)| match item {
            Ok(Item::Function(e)) => Some(xs.iter().map(|&x| ev(e, s.ans, x, 0.0, 0.0)).collect()),
            _ => None,
        })
        .collect();

    // The y range: the settings, or the 1st to the 99th percentile of the functions (poles do not hide the curves).
    // If there are equations or curves, use the same scale as x (circles are round).
    let only_functions = items.iter().all(|(_, i)| {
        !matches!(
            i,
            Ok(Item::Equation(..) | Item::Polar(_) | Item::Parametric(..))
        )
    });
    let (lo, hi) = s.y.unwrap_or_else(|| {
        let mut ys: Vec<f64> = functions
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .copied()
            .collect();
        if ys.is_empty() || !only_functions {
            let half = (b - a) / 2.0 * aspect;
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

    let ts: Vec<f64> = (0..=s.samples)
        .map(|k| s.t.0 + (s.t.1 - s.t.0) * k as f64 / s.samples as f64)
        .collect();
    let mut curves = Vec::new();
    for (i, (_, item)) in items.iter().enumerate() {
        let Ok(item) = item else { continue };
        let pts: Vec<Option<Point>> = match item {
            Item::Function(_) => {
                let ys = functions[i].as_deref().unwrap_or_default();
                xs.iter().zip(ys).map(|(&x, y)| y.map(|y| (x, y))).collect()
            }
            Item::Polar(e) => ts
                .iter()
                .map(|&t| ev(e, s.ans, 0.0, 0.0, t).map(|r| (r * t.cos(), r * t.sin())))
                .collect(),
            Item::Parametric(ex, ey) => ts
                .iter()
                .map(|&t| Some((ev(ex, s.ans, 0.0, 0.0, t)?, ev(ey, s.ans, 0.0, 0.0, t)?)))
                .collect(),
            Item::Equation(l, r) => {
                let segments = contour(l, r, s.ans, (a, b), (lo, hi), grid);
                curves.push((i, segments.into_iter().map(|[p, q]| vec![p, q]).collect()));
                continue;
            }
            Item::Surface(_) => continue,
        };
        // Break the line at a missing point or at a jump higher than the plot (a pole).
        let mut parts: Vec<Vec<Point>> = vec![Vec::new()];
        for p in pts {
            let last = parts.last_mut().unwrap();
            match (p, last.last()) {
                (Some(p), Some(q)) if (p.1 - q.1).abs() > hi - lo => parts.push(vec![p]),
                (Some(p), _) => last.push(p),
                (None, _) if !last.is_empty() => parts.push(Vec::new()),
                _ => {}
            }
        }
        curves.push((i, parts));
    }
    Plot2d {
        x: (a, b),
        y: (lo, hi),
        curves,
    }
}

/// The analysis of a function on the visible x range.
pub struct Analysis {
    /// The index of the item.
    pub item: usize,
    pub min: Point,
    pub max: Point,
    /// The mean value: the integral divided by the length of the x range.
    pub mean: f64,
    /// The local minimums and maximums (not at the ends of the range), at most 20.
    pub extrema: Vec<Point>,
    /// The points where the function crosses zero, at most 20.
    pub roots: Vec<Point>,
}

/// Analyze the functions of a 2D graph (not the equations and the curves).
pub fn analyze(items: &[(String, Result<Item, String>)], plot: &Plot2d) -> Vec<Analysis> {
    let mut out = Vec::new();
    for (i, parts) in &plot.curves {
        if !matches!(items.get(*i), Some((_, Ok(Item::Function(_))))) {
            continue;
        }
        let pts: Vec<Point> = parts.iter().flatten().copied().collect();
        let (Some(&min), Some(&max)) = (
            pts.iter().min_by(|p, q| p.1.total_cmp(&q.1)),
            pts.iter().max_by(|p, q| p.1.total_cmp(&q.1)),
        ) else {
            continue;
        };
        // The trapezoid rule on each part (a part has no poles and no missing points).
        let (mut area, mut length) = (0.0, 0.0);
        let (mut extrema, mut roots) = (Vec::new(), Vec::new());
        for part in parts {
            for w in part.windows(2) {
                let ((x0, y0), (x1, y1)) = (w[0], w[1]);
                area += (y0 + y1) / 2.0 * (x1 - x0);
                length += x1 - x0;
                if y0 == 0.0 || (y0 < 0.0) != (y1 < 0.0) && y1 != 0.0 {
                    let x = if y0 == 0.0 {
                        x0
                    } else {
                        x0 - y0 * (x1 - x0) / (y1 - y0)
                    };
                    roots.push((x, 0.0));
                }
            }
            for w in part.windows(3) {
                let (a, b, c) = (w[0].1, w[1].1, w[2].1);
                if (b > a && b >= c) || (b < a && b <= c) {
                    extrema.push(w[1]);
                }
            }
        }
        // Too many points (a fast oscillation) make the graph unreadable: show none.
        if extrema.len() > 20 {
            extrema.clear();
        }
        if roots.len() > 20 {
            roots.clear();
        }
        let mean = if length > 0.0 {
            area / length
        } else {
            f64::NAN
        };
        out.push(Analysis {
            item: *i,
            min,
            max,
            mean,
            extrema,
            roots,
        });
    }
    out
}

/// The line segments where `l = r` (marching squares on a grid of `w` x `h` cells).
fn contour(
    l: &Expr,
    r: &Expr,
    ans: f64,
    (a, b): Point,
    (lo, hi): Point,
    (w, h): (usize, usize),
) -> Vec<[Point; 2]> {
    let (w, h) = (w.max(1), h.max(1));
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
            // The corners in order around the cell, and the edges between them.
            let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
            let Some(v) = corners
                .iter()
                .map(|&(ci, cj)| grid[cj][ci])
                .collect::<Option<Vec<f64>>>()
            else {
                continue;
            };
            let mut cross = Vec::new();
            for k in 0..4 {
                let (va, vb) = (v[k], v[(k + 1) % 4]);
                if (va > 0.0) != (vb > 0.0) {
                    let f = va / (va - vb);
                    let (pa, pb) = (corners[k], corners[(k + 1) % 4]);
                    cross.push((
                        x(pa.0) + (x(pb.0) - x(pa.0)) * f,
                        y(pa.1) + (y(pb.1) - y(pa.1)) * f,
                    ));
                }
            }
            for pair in cross.as_chunks::<2>().0 {
                out.push([pair[0], pair[1]]);
            }
        }
    }
    out
}

/// A 3D surface on a grid. The points are in the box: x and y from -1 to 1, z from -0.8 to 0.8.
pub struct Surface {
    pub points: Vec<Vec<Option<[f64; 3]>>>,
    /// The real range of z.
    pub z: (f64, f64),
    /// The lowest and the highest point: the position in the box, and the real (x, y, z).
    pub low: Option<([f64; 3], [f64; 3])>,
    pub high: Option<([f64; 3], [f64; 3])>,
    /// The mean value of z on the grid.
    pub mean: f64,
}

/// The low corner of the box and the ends of the axes x, y and z.
pub const ORIGIN: [f64; 3] = [-1.0, -1.0, -0.8];
pub const AXES: [(&str, [f64; 3]); 3] = [
    ("x", [1.0, -1.0, -0.8]),
    ("y", [-1.0, 1.0, -0.8]),
    ("z", [-1.0, -1.0, 0.8]),
];

pub fn surface_grid(expr: &Expr, s: &Settings) -> Surface {
    let n = s.grid;
    let ((a, b), (c, d)) = (s.x, s.y.unwrap_or(s.x));
    let z: Vec<Vec<Option<f64>>> = (0..=n)
        .map(|i| {
            let x = a + (b - a) * i as f64 / n as f64;
            (0..=n)
                .map(|j| ev(expr, s.ans, x, c + (d - c) * j as f64 / n as f64, 0.0))
                .collect()
        })
        .collect();
    let finite = || z.iter().flatten().flatten().copied();
    let zmin = finite().fold(f64::INFINITY, f64::min);
    let zmax = finite().fold(f64::NEG_INFINITY, f64::max);
    let span = if zmax - zmin > 1e-12 {
        zmax - zmin
    } else {
        1.0
    };
    // The lowest and the highest grid point, in real units and in the box.
    let real = |i: usize, j: usize, v: f64| {
        [
            a + (b - a) * i as f64 / n as f64,
            c + (d - c) * j as f64 / n as f64,
            v,
        ]
    };
    let cells = || {
        z.iter().enumerate().flat_map(|(i, row)| {
            row.iter()
                .enumerate()
                .filter_map(move |(j, v)| v.map(|v| (i, j, v)))
        })
    };
    let pick = |best: Option<(usize, usize, f64)>| {
        best.map(|(i, j, v)| {
            let unit = |k: usize| 2.0 * k as f64 / n as f64 - 1.0;
            (
                [unit(i), unit(j), 1.6 * (v - zmin) / span - 0.8],
                real(i, j, v),
            )
        })
    };
    let low = pick(cells().min_by(|p, q| p.2.total_cmp(&q.2)));
    let high = pick(cells().max_by(|p, q| p.2.total_cmp(&q.2)));
    let count = cells().count().max(1);
    let mean = cells().map(|c| c.2).sum::<f64>() / count as f64;
    let unit = |k: usize| 2.0 * k as f64 / n as f64 - 1.0;
    let points = z
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, v)| v.map(|v| [unit(i), unit(j), 1.6 * (v - zmin) / span - 0.8]))
                .collect()
        })
        .collect();
    Surface {
        points,
        z: (zmin, zmax),
        low,
        high,
        mean,
    }
}

impl Surface {
    /// The cells of the surface for a solid view, from the farthest to the nearest.
    /// Draw them in this order: a near cell hides the far cells behind it (painter's algorithm).
    pub fn quads(&self, view: (f64, f64)) -> Vec<Quad> {
        let p = &self.points;
        // The light comes from the top left, on the side of the viewer.
        let light = [-0.35, 0.55, -0.76];
        let mut out: Vec<Quad> = Vec::new();
        for i in 0..p.len().saturating_sub(1) {
            for j in 0..p[i].len().saturating_sub(1) {
                let (Some(a), Some(b), Some(c), Some(d)) =
                    (p[i][j], p[i + 1][j], p[i + 1][j + 1], p[i][j + 1])
                else {
                    continue;
                };
                let v = [a, b, c, d].map(|q| to_view(q, view));
                // The normal of the cell. Both sides of the surface get light.
                let (e, f) = (sub(v[2], v[0]), sub(v[3], v[1]));
                let n = [
                    e[1] * f[2] - e[2] * f[1],
                    e[2] * f[0] - e[0] * f[2],
                    e[0] * f[1] - e[1] * f[0],
                ];
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
                let dot = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).abs() / len;
                let depth = v.iter().map(|q| q[2]).sum::<f64>() / 4.0;
                let height = ([a, b, c, d].iter().map(|q| q[2]).sum::<f64>() / 4.0 + 0.8) / 1.6;
                let corners = v.map(|q| (q[0], q[1]));
                out.push(Quad {
                    corners,
                    height,
                    light: 0.35 + 0.65 * dot,
                    depth,
                });
            }
        }
        out.sort_by(|x, y| y.depth.total_cmp(&x.depth));
        out
    }

    /// The wireframe: each segment with its mean height from 0 (low) to 1 (high).
    pub fn segments(&self) -> Vec<(f64, [f64; 3], [f64; 3])> {
        let p = &self.points;
        let mut out = Vec::new();
        for i in 0..p.len() {
            for j in 0..p[i].len() {
                for (i2, j2) in [(i + 1, j), (i, j + 1)] {
                    let (Some(a), Some(Some(b))) = (p[i][j], p.get(i2).and_then(|r| r.get(j2)))
                    else {
                        continue;
                    };
                    out.push((((a[2] + b[2]) / 2.0 + 0.8) / 1.6, a, *b));
                }
            }
        }
        out
    }
}

/// Turn a point of the box by the view angles (yaw, pitch) and project it to the screen (x right, y up).
pub fn project(p: [f64; 3], view: (f64, f64)) -> Point {
    let [x, y, _] = to_view(p, view);
    (x, y)
}

/// Turn a point of the box by the view angles: x right, y up, and the depth (a high value is far).
fn to_view(p: [f64; 3], (yaw, pitch): (f64, f64)) -> [f64; 3] {
    let (x1, y1) = (
        p[0] * yaw.cos() - p[1] * yaw.sin(),
        p[0] * yaw.sin() + p[1] * yaw.cos(),
    );
    [
        x1,
        y1 * pitch.sin() + p[2] * pitch.cos(),
        y1 * pitch.cos() - p[2] * pitch.sin(),
    ]
}

/// A cell of a solid surface on the screen.
pub struct Quad {
    /// The four corners on the screen, in order around the cell.
    pub corners: [Point; 4],
    /// The mean height, from 0 (low) to 1 (high).
    pub height: f64,
    /// The brightness from the light, from 0.35 to 1.
    pub light: f64,
    /// The distance from the viewer (a high value is far).
    pub depth: f64,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quads_far_to_near() {
        // A flat surface seen from the top (pitch 90°): every cell gets the same light.
        let e = calc::compile("0").unwrap();
        let s = Settings {
            x: (-1.0, 1.0),
            y: None,
            t: (0.0, TAU),
            grid: 4,
            samples: 10,
            ans: 0.0,
        };
        let surface = surface_grid(&e, &s);
        let quads = surface.quads((0.3, std::f64::consts::FRAC_PI_2));
        assert_eq!(quads.len(), 16);
        let l = quads[0].light;
        assert!(quads.iter().all(|q| (q.light - l).abs() < 1e-9));
        // Seen from an angle, the far cells come first.
        let side = surface_grid(&calc::compile("x * y").unwrap(), &s).quads((0.6, 0.5));
        assert!(side.windows(2).all(|w| w[0].depth >= w[1].depth));
        assert!(side[0].depth > side[side.len() - 1].depth);
    }

    #[test]
    fn analysis_of_sin() {
        use std::f64::consts::PI;
        let items = parse("sin(x)");
        let s = Settings {
            x: (0.0, TAU),
            y: None,
            t: (0.0, TAU),
            grid: 20,
            samples: 720,
            ans: 0.0,
        };
        let plot = plot_2d(&items, &s, 2000, (10, 10), 0.5);
        let a = &analyze(&items, &plot)[0];
        assert!((a.max.0 - PI / 2.0).abs() < 0.01 && (a.max.1 - 1.0).abs() < 1e-4);
        assert!((a.min.0 - 3.0 * PI / 2.0).abs() < 0.01);
        assert!(a.mean.abs() < 1e-6);
        assert_eq!(a.extrema.len(), 2);
        assert!(a.roots.iter().any(|r| (r.0 - PI).abs() < 1e-3));
        let surface = surface_grid(
            &calc::compile("x * y").unwrap(),
            &Settings {
                x: (-1.0, 1.0),
                ..s
            },
        );
        assert_eq!(surface.high.unwrap().1[2], 1.0);
        assert!(surface.mean.abs() < 1e-12);
    }

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
        // x = 2.1 on a 40 x 20 cell grid from -4 to 4: one segment in each row, at x = 2.1.
        let (l, r) = (calc::compile("x").unwrap(), calc::compile("2.1").unwrap());
        let segs = contour(&l, &r, 0.0, (-4.0, 4.0), (-4.0, 4.0), (40, 20));
        assert_eq!(segs.len(), 20);
        assert!(segs.iter().flatten().all(|p| (p.0 - 2.1).abs() < 1e-9));
    }

    #[test]
    fn circle_is_round() {
        let items = parse("x^2 + y^2 = 4");
        let s = Settings {
            x: (-5.0, 5.0),
            y: None,
            t: (0.0, TAU),
            grid: 20,
            samples: 720,
            ans: 0.0,
        };
        let plot = plot_2d(&items, &s, 100, (100, 50), 0.5);
        assert_eq!(plot.y, (-2.5, 2.5));
        for p in plot.curves[0].1.iter().flatten() {
            assert!((p.0.hypot(p.1) - 2.0).abs() < 0.05, "{p:?}");
        }
    }
}
