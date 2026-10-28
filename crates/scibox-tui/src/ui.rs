use crate::{App, KEYS, Kind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, Paragraph};
use scibox_core::calc::{self, CONSTANTS};
use scibox_core::formulas::DOMAINS;
use scibox_core::state::{KEY_COLUMNS, KeyKind, key_kind};
use scibox_core::units::GROUPS;

pub const ACCENT: Color = Color::Cyan;

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
            " Ctrl+P go to · ←→ tab · Tab focus · ↑↓ select · Ctrl+Y or click: copy · Enter or right-click: use · F3 notation"
        }
        Kind::Formulas => {
            draw_formulas(f, app, body);
            " Ctrl+P go to · ←→ tab · Tab focus · ↑↓ select · type a value · Enter: solve for it · Ctrl+Y copy · F3 notation"
        }
        Kind::Graph => {
            crate::graph::draw(f, app, body);
            " Ctrl+P go to · ←→ tab · Tab next field · F2 style · F4 analysis · wheel: zoom · drag: move or turn"
        }
        Kind::Calc => {
            draw_calc(f, app, body);
            " Ctrl+P go to · ←→ tab · type an expression · Enter = · Del clear · Ctrl+Y copy · Esc quit · F3 notation"
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

pub fn block(title: &str, focused: bool) -> Block<'_> {
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
pub fn input_line(text: &str, cursor: bool) -> Line<'_> {
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
    let results: Vec<String> = results
        .unwrap_or_default()
        .iter()
        .map(|r| calc::pretty(r))
        .collect();
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
    // The equation in 2D (fractions, roots, exponents), typeset from its LaTeX form.
    let fm = app.formula();
    let math: Vec<String> = match scibox_core::latex::to_latex(fm.equation) {
        Ok(tex) => term_maths::render(&tex)
            .to_string()
            .lines()
            .map(str::to_string)
            .collect(),
        Err(_) => vec![fm.equation.to_string()],
    };
    let eq_height = (math.len() as u16 + 3).min(right.height / 2).max(3);
    let [eq_area, vars_area] =
        Layout::vertical([Constraint::Length(eq_height), Constraint::Fill(1)]).areas(right);

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

    let eq_block = block(fm.name, false)
        .title(Line::from(format!(" {} ", DOMAINS[app.sel(0)].name)).right_aligned())
        .title_bottom(Line::from(format!(" {} ", fm.equation)).dark_gray());
    let lines: Vec<Line> = math
        .into_iter()
        .map(|l| Line::from(format!(" {l}")).fg(ACCENT).bold())
        .collect();
    f.render_widget(Paragraph::new(lines).block(eq_block), eq_area);

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
        let (radio, value) = if i == app.form.unknown {
            let text = solved
                .as_ref()
                .map(|v| calc::pretty(&app.ctx.fmt(*v)))
                .unwrap_or("?".into());
            (
                Span::from("◉ ").green(),
                Span::from(format!("{text:<16}")).green().bold(),
            )
        } else {
            let mut text = app.form.text[i].clone();
            if app.focus == 2 && app.sel(2) == i {
                text.push('█');
            }
            let ok = calc::eval(&app.form.text[i], &app.ctx).is_ok();
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

/// The width of a calculator key, in cells.
const KEY_W: u16 = 7;

fn draw_calc(f: &mut Frame, app: &mut App, area: Rect) {
    let [input_area, rest] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    // The keypad has a fixed size: 8 keys and 7 gaps in each row, and the borders.
    let cols = KEY_COLUMNS as u16;
    let rows = (KEYS.len() / KEY_COLUMNS) as u16;
    let [history_area, side] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(cols * (KEY_W + 1) + 1),
    ])
    .areas(rest);
    let gap = u16::from(side.height > rows * 2);
    let [keys_area, constants_area] = Layout::vertical([
        Constraint::Length(rows * (1 + gap) - gap + 2),
        Constraint::Fill(1),
    ])
    .areas(side);

    // Expression input
    let mode = if app.ctx.deg { " DEG " } else { " RAD " };
    let input_block = block(" Expression ", true).title(Line::from(mode).right_aligned());
    f.render_widget(
        Paragraph::new(input_line(&app.calc.input, true)).block(input_block),
        input_area,
    );

    // History: show the most recent lines at the bottom.
    let visible = history_area.height.saturating_sub(2) as usize / 2;
    let skip = app.calc.history.len().saturating_sub(visible);
    let lines: Vec<Line> = app.calc.history[skip..]
        .iter()
        .flat_map(|(expr, result)| {
            let result = if result.starts_with("Error") {
                Span::from(result.as_str()).red()
            } else {
                Span::from(calc::pretty(result)).bold()
            };
            [
                Line::from(expr.as_str()).dark_gray(),
                Line::from(vec![Span::from("= "), result]).right_aligned(),
            ]
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).block(block(" History ", false)),
        history_area,
    );

    // Keypad: keys of one line, with a background color.
    let b = block(" Keys ", false);
    let inner = b.inner(keys_area);
    f.render_widget(b, keys_area);
    app.areas.keys.clear();
    for (i, key) in KEYS.iter().enumerate() {
        let (col, row) = ((i % KEY_COLUMNS) as u16, (i / KEY_COLUMNS) as u16);
        let r = Rect::new(
            inner.x + col * (KEY_W + 1),
            inner.y + row * (1 + gap),
            KEY_W,
            1,
        )
        .intersection(inner);
        app.areas.keys.push(r);
        let label = if *key == "DEG" { mode.trim() } else { key };
        let color = match key_kind(key) {
            KeyKind::Digit => Color::White,
            KeyKind::Operator => Color::Yellow,
            KeyKind::Function => ACCENT,
            KeyKind::Constant => Color::Magenta,
            KeyKind::Clear => Color::Red,
            KeyKind::Equals => Color::Green,
        };
        let style = if app.pressed == Some(i) {
            Style::new().fg(Color::Black).bg(ACCENT)
        } else {
            Style::new().fg(color).bg(Color::Indexed(236))
        };
        f.render_widget(
            Line::from(format!("{label:^w$}", w = KEY_W as usize)).style(style.bold()),
            r,
        );
    }

    // The physical constants that the expressions can use.
    let lines: Vec<Line> = CONSTANTS
        .iter()
        .map(|(name, _, text)| {
            Line::from(vec![
                Span::from(format!(" {name:<6}")).magenta(),
                Span::from(*text).dark_gray(),
            ])
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).block(block(" Constants ", false)),
        constants_area,
    );
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
