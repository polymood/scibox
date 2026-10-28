use crate::calc::{self, Ctx, fmt};
use crate::formulas::DOMAINS;
use crate::units::GROUPS;
use crate::{App, KEYS, Kind};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, Chart, Clear, Dataset, GraphType, List, ListItem, Paragraph};

const ACCENT: Color = Color::Cyan;
const CURVES: [Color; 5] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::Red,
];

pub const LOGO: [&str; 3] = [
    "┌─┐┌─┐┬┌┐ ┌─┐╲ ╱",
    "└─┐│  │├┴┐│ │ ╳ ",
    "└─┘└─┘┴└─┘└─┘╱ ╲",
];

pub fn draw(f: &mut Frame, app: &mut App) {
    let [top, body, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(f.area());
    draw_tabs(f, app, top);
    let help_text = match app.kind() {
        Kind::Convert => {
            draw_converter(f, app, body);
            " Ctrl+P go to · ←→ tab · Tab focus · ↑↓ select · Ctrl+Y or click: copy · Enter or right-click: use"
        }
        Kind::Formulas => {
            draw_formulas(f, app, body);
            " Ctrl+P go to · ←→ tab · Tab focus · ↑↓ select · type a value · Enter: solve for it · Ctrl+Y copy"
        }
        Kind::Graph => {
            draw_graph(f, app, body);
            " Ctrl+P go to · ←→ tab · Tab next field · mouse wheel: zoom · drag: move · Esc quit"
        }
        Kind::Calc => {
            draw_calc(f, app, body);
            " Ctrl+P go to · ←→ tab · type an expression · Enter = · Del clear · Ctrl+Y copy · Esc quit"
        }
    };
    let status = match &app.status {
        Some(s) => Line::from(format!(" {s}")).green().bold(),
        None => Line::from(help_text).dark_gray(),
    };
    f.render_widget(status, help);
    if app.palette.is_some() {
        draw_palette(f, app);
    }
}

/// Draw the tab bar and keep the area of each tab for the mouse.
fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let mut spans = vec![
        Span::from(" scibox ").bold().black().on_cyan(),
        Span::from(" "),
    ];
    let mut x = area.x + spans.iter().map(|s| s.width() as u16).sum::<u16>();
    app.areas.tabs.clear();
    let names = GROUPS
        .iter()
        .map(|g| g.name)
        .chain(["Formulas", "Graph", "Calc"]);
    for (i, name) in names.enumerate() {
        let span = Span::from(format!(" {name} "));
        let w = span.width() as u16;
        app.areas.tabs.push(Rect::new(x, area.y, w, 1));
        x += w + 1;
        spans.push(if i == app.tab {
            span.bold().reversed()
        } else {
            span
        });
        spans.push(Span::from(" "));
    }
    f.render_widget(Line::from(spans), area);
    // The search button at the right end, if there is space.
    let search = Span::from(" ⌕ Ctrl+P ").black().on_dark_gray();
    let w = search.width() as u16;
    app.areas.search = Rect::default();
    if x + w <= area.right() {
        app.areas.search = Rect::new(area.right() - w, area.y, w, 1);
        f.render_widget(search, app.areas.search);
    }
}

fn block(title: &str, focused: bool) -> Block<'_> {
    let color = if focused { ACCENT } else { Color::DarkGray };
    Block::bordered()
        .title(title)
        .border_style(Style::new().fg(color))
}

/// Draw a bordered list in a pane and keep its inner area for the mouse.
fn list_pane(
    f: &mut Frame,
    app: &mut App,
    pane: usize,
    area: Rect,
    b: Block,
    items: Vec<ListItem>,
) {
    let focused = app.focus == pane;
    let highlight = if focused {
        Style::new().fg(Color::Black).bg(ACCENT)
    } else {
        Style::new().fg(ACCENT)
    };
    let b = b.border_style(Style::new().fg(if focused { ACCENT } else { Color::DarkGray }));
    app.areas.panes[pane] = b.inner(area);
    let list = List::new(items)
        .block(b)
        .highlight_style(highlight.add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, area, &mut app.lists[pane]);
}

/// An input text with a cursor.
fn input_line(text: &str, cursor: bool) -> Line<'_> {
    let mut spans = vec![Span::from(text).bold()];
    if cursor {
        spans.push(Span::from("█").fg(ACCENT));
    }
    Line::from(spans)
}

fn draw_converter(f: &mut Frame, app: &mut App, area: Rect) {
    let [value_area, rest] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    let [cats_area, units_area, results_area] = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(28),
        Constraint::Fill(1),
    ])
    .areas(rest);

    let units = app.category().units;
    let from = app.sel(1);
    let results = app.results();

    // Value input
    let mut value_block = block(" Value ", true).title(
        Line::from(format!(" {} ({}) ", units[from].name, units[from].symbol)).right_aligned(),
    );
    if let Err(e) = &results {
        value_block = value_block.title_bottom(Line::from(format!(" {e} ")).red());
    }
    let input = if app.input.is_empty() {
        Line::from(vec![
            Span::from("1").dark_gray(),
            Span::from("█").fg(ACCENT),
        ])
    } else {
        input_line(&app.input, true)
    };
    f.render_widget(Paragraph::new(input).block(value_block), value_area);

    let cats = GROUPS[app.tab]
        .categories
        .iter()
        .map(|c| ListItem::new(c.name));
    list_pane(
        f,
        app,
        0,
        cats_area,
        Block::bordered().title(" Category "),
        cats.collect(),
    );
    let items = units.iter().map(|u| ListItem::new(u.name));
    list_pane(
        f,
        app,
        1,
        units_area,
        Block::bordered().title(" From "),
        items.collect(),
    );

    // Results: the value in every unit of the category
    let results = results.unwrap_or_default();
    let width = results.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let items = units.iter().enumerate().map(|(i, u)| {
        let r = results.get(i).map(String::as_str).unwrap_or("");
        let symbol = Span::from(format!("{:<8}", u.symbol));
        ListItem::new(Line::from(vec![
            Span::from(format!(" {r:>width$} ")),
            if i == from { symbol.yellow() } else { symbol },
            Span::from(u.name).dark_gray(),
        ]))
    });
    list_pane(
        f,
        app,
        2,
        results_area,
        Block::bordered().title(" Result "),
        items.collect(),
    );
}

fn draw_formulas(f: &mut Frame, app: &mut App, area: Rect) {
    let [doms_area, list_area, right] = Layout::horizontal([
        Constraint::Length(15),
        Constraint::Max(36),
        Constraint::Min(40),
    ])
    .areas(area);
    let [eq_area, vars_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(right);

    let doms = DOMAINS.iter().map(|d| ListItem::new(d.name));
    list_pane(
        f,
        app,
        0,
        doms_area,
        Block::bordered().title(" Domain "),
        doms.collect(),
    );
    let names = DOMAINS[app.sel(0)]
        .formulas
        .iter()
        .map(|fm| ListItem::new(fm.name));
    list_pane(
        f,
        app,
        1,
        list_area,
        Block::bordered().title(" Formula "),
        names.collect(),
    );

    let fm = app.formula();
    let eq_block = block(fm.name, false)
        .title(Line::from(format!(" {} ", DOMAINS[app.sel(0)].name)).right_aligned());
    let eq = Line::from(format!(" {}", fm.equation)).fg(ACCENT).bold();
    f.render_widget(Paragraph::new(eq).block(eq_block), eq_area);

    // Variables: one row each. The radio button marks the unknown.
    let solved = app.solve();
    let sym_w = fm
        .vars
        .iter()
        .map(|v| v.symbol.chars().count())
        .max()
        .unwrap_or(1);
    let unit_w = fm
        .vars
        .iter()
        .map(|v| v.unit.chars().count())
        .max()
        .unwrap_or(1);
    let items = fm.vars.iter().enumerate().map(|(i, var)| {
        let (radio, value) = if i == app.unknown {
            let text = solved.as_ref().map(|v| fmt(*v)).unwrap_or("?".into());
            (
                Span::from("◉ ").green(),
                Span::from(format!("{text:<16}")).green().bold(),
            )
        } else {
            let mut text = app.fin[i].clone();
            if app.focus == 2 && app.sel(2) == i {
                text.push('█');
            }
            let ok = calc::eval(&app.fin[i], &app.ctx).is_ok();
            let span = Span::from(format!("{text:<16}"));
            (Span::from("○ "), if ok { span } else { span.red() })
        };
        ListItem::new(Line::from(vec![
            radio,
            Span::from(format!("{:<sym_w$} ", var.symbol)).bold(),
            value,
            Span::from(format!(" {:<unit_w$}  ", var.unit)).fg(ACCENT),
            Span::from(var.name).dark_gray(),
        ]))
    });
    let mut vars_block = Block::bordered()
        .title(" Variables ")
        .title_bottom(Line::from(" Enter or click ○: solve for this variable ").dark_gray());
    if let Err(e) = &solved {
        vars_block = vars_block.title_bottom(Line::from(format!(" {e} ")).red().right_aligned());
    }
    list_pane(f, app, 2, vars_area, vars_block, items.collect());
}

fn draw_graph(f: &mut Frame, app: &mut App, area: Rect) {
    let [fields, chart_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    let field_areas: [Rect; 3] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(14),
        Constraint::Length(14),
    ])
    .areas(fields);
    let titles = [
        " f(x) =  (separate functions with ;) ",
        " x min ",
        " x max ",
    ];
    for i in 0..3 {
        let focused = app.focus == i;
        let p = Paragraph::new(input_line(&app.graph[i], focused)).block(block(titles[i], focused));
        f.render_widget(p, field_areas[i]);
        app.areas.panes[i] = field_areas[i];
    }
    app.areas.chart = chart_area;

    let mut chart_block = block(" Graph (angles in radians) ", false);
    let Some((a, b)) = app.x_range() else {
        let msg = Line::from(" x min must be a number less than x max ").red();
        f.render_widget(chart_block.title_bottom(msg), chart_area);
        return;
    };

    // Calculate each function at one point for each braille dot column.
    let n = (chart_area.width as usize * 2).max(2);
    let mut ctx = Ctx {
        ans: app.ctx.ans,
        deg: false,
        x: None,
    };
    let mut curves: Vec<(&str, Vec<(f64, f64)>)> = Vec::new();
    for expr in app.graph[0]
        .split(';')
        .map(str::trim)
        .filter(|e| !e.is_empty())
    {
        let mut points = Vec::with_capacity(n + 1);
        for k in 0..=n {
            let x = a + (b - a) * k as f64 / n as f64;
            ctx.x = Some(x);
            match calc::eval(expr, &ctx) {
                Ok(y) if y.is_finite() => points.push((x, y)),
                Ok(_) => {}
                Err(e) => {
                    let msg = Line::from(format!(" {expr}: {e} ")).red();
                    chart_block = chart_block.title_bottom(msg);
                    break;
                }
            }
        }
        curves.push((expr, points));
    }

    // The y range: from the 1st to the 99th percentile, so that poles (tan, 1/x) do not hide the curves.
    let mut ys: Vec<f64> = curves
        .iter()
        .flat_map(|c| c.1.iter().map(|p| p.1))
        .collect();
    ys.sort_by(f64::total_cmp);
    let (mut lo, mut hi) = match ys.len() {
        0 => (-1.0, 1.0),
        len => (ys[len / 100], ys[len - 1 - len / 100]),
    };
    if hi - lo < 1e-12 {
        (lo, hi) = (lo - 1.0, hi + 1.0);
    }
    let pad = (hi - lo) * 0.05;
    (lo, hi) = (lo - pad, hi + pad);

    // The axes y = 0 and x = 0, if they are visible.
    let axes = [[(a, 0.0), (b, 0.0)], [(0.0, lo), (0.0, hi)]];
    let visible = [lo < 0.0 && hi > 0.0, a < 0.0 && b > 0.0];
    let mut datasets: Vec<Dataset> = axes
        .iter()
        .zip(visible)
        .filter(|(_, v)| *v)
        .map(|(line, _)| {
            Dataset::default()
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .dark_gray()
                .data(line)
        })
        .collect();
    for (i, (expr, points)) in curves.iter().enumerate() {
        datasets.push(
            Dataset::default()
                .name(*expr)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .fg(CURVES[i % CURVES.len()])
                .data(points),
        );
    }
    let labels = |a: f64, b: f64| [short(a), short((a + b) / 2.0), short(b)];
    let chart = Chart::new(datasets)
        .block(chart_block)
        .x_axis(
            Axis::default()
                .bounds([a, b])
                .labels(labels(a, b))
                .dark_gray(),
        )
        .y_axis(
            Axis::default()
                .bounds([lo, hi])
                .labels(labels(lo, hi))
                .dark_gray(),
        );
    f.render_widget(chart, chart_area);
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

fn draw_calc(f: &mut Frame, app: &mut App, area: Rect) {
    let [input_area, rest] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    let [history_area, keys_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Fill(1)]).areas(rest);

    // Expression input
    let mode = if app.ctx.deg { " DEG " } else { " RAD " };
    let input_block = block(" Expression ", true).title(Line::from(mode).right_aligned());
    f.render_widget(
        Paragraph::new(input_line(&app.calc_input, true)).block(input_block),
        input_area,
    );

    // History: show the most recent lines at the bottom. Show the logo if the history is empty.
    let visible = history_area.height.saturating_sub(2) as usize / 2;
    let skip = app.history.len().saturating_sub(visible);
    let mut lines: Vec<Line> = app.history[skip..]
        .iter()
        .flat_map(|(expr, result)| {
            let result = if result.starts_with("Error") {
                result.as_str().red()
            } else {
                result.as_str().bold()
            };
            [
                Line::from(expr.as_str()).dark_gray(),
                Line::from(vec![Span::from("= "), result]).right_aligned(),
            ]
        })
        .collect();
    if lines.is_empty() {
        lines = LOGO
            .iter()
            .map(|l| Line::from(*l).fg(ACCENT).centered())
            .collect();
        lines.insert(0, Line::default());
    }
    f.render_widget(
        Paragraph::new(lines).block(block(" History ", false)),
        history_area,
    );

    // Keypad: a 6 x 6 grid of buttons
    let inner = block(" Keys ", false).inner(keys_area);
    f.render_widget(block(" Keys ", false), keys_area);
    app.areas.keys.clear();
    for row in Layout::vertical([Constraint::Fill(1); 6])
        .split(inner)
        .iter()
    {
        for cell in Layout::horizontal([Constraint::Fill(1); 6])
            .split(*row)
            .iter()
        {
            app.areas.keys.push(*cell);
        }
    }
    for (i, (cell, key)) in app.areas.keys.iter().zip(KEYS).enumerate() {
        let label = if key == "DEG" { mode.trim() } else { key };
        let color = match key {
            "=" => Color::Green,
            "C" | "⌫" => Color::Red,
            k if k.len() > 1 || "/*-+^()√π".contains(k) => ACCENT,
            _ => Color::White,
        };
        let mut style = Style::new().fg(color).bold();
        if app.pressed == Some(i) {
            style = style.reversed();
        }
        let key_block = Block::bordered().border_style(Style::new().fg(Color::DarkGray));
        // Center the label vertically inside the button.
        let pad = cell.height.saturating_sub(3) / 2;
        let mut text = vec![Line::default(); pad as usize];
        text.push(Line::from(label));
        let button = Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(style)
            .block(key_block);
        f.render_widget(button, *cell);
    }
}

fn draw_palette(f: &mut Frame, app: &mut App) {
    let Some(p) = &mut app.palette else { return };
    let screen = f.area();
    let w = screen.width.saturating_sub(4).min(90);
    let h = screen.height.saturating_sub(2).min(24);
    let area = Rect::new(screen.x + (screen.width - w) / 2, screen.y + 1, w, h);
    f.render_widget(Clear, area);
    let b = block(" Go to ", true)
        .title(Line::from(format!(" {} results ", p.matches.len())).right_aligned())
        .title_bottom(Line::from(" type a name · ↑↓ · Enter · Esc ").dark_gray());
    let inner = b.inner(area);
    f.render_widget(b, area);
    app.areas.palette = inner;

    let query = Line::from(vec![
        Span::from("> ").fg(ACCENT),
        Span::from(p.query.as_str()).bold(),
        Span::from("█").fg(ACCENT),
    ]);
    f.render_widget(query, Rect { height: 1, ..inner });
    let items: Vec<ListItem> = p
        .matches
        .iter()
        .map(|&i| {
            let e = &app.entries[i];
            ListItem::new(Line::from(vec![
                Span::from(e.label.as_str()).bold(),
                Span::from(format!("  {}", e.detail)).dark_gray(),
            ]))
        })
        .collect();
    let list = List::new(items).highlight_style(Style::new().fg(Color::Black).bg(ACCENT));
    let list_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };
    f.render_stateful_widget(list, list_area, &mut p.list);
}
