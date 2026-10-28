use crate::calc::fmt;
use crate::units::{GROUPS, convert};
use crate::{App, KEYS};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

const ACCENT: Color = Color::Cyan;

pub fn draw(f: &mut Frame, app: &mut App) {
    let [top, body, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(f.area());
    draw_tabs(f, app, top);
    let help_text = if app.is_calc() {
        draw_calc(f, app, body);
        " type an expression · Enter = · Del clear · ←→ tab · Esc quit · click the keys"
    } else {
        draw_converter(f, app, body);
        " ←→ tab · Tab focus · ↑↓ select · Del clear · Esc quit · click a result to use it"
    };
    f.render_widget(Line::from(help_text).dark_gray(), help);
}

/// Draw the tab bar and keep the area of each tab for the mouse.
fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let mut spans = vec![
        Span::from(" unitconv ").bold().black().on_cyan(),
        Span::from(" "),
    ];
    let mut x = area.x + spans.iter().map(|s| s.width() as u16).sum::<u16>();
    app.areas.tabs.clear();
    let names = GROUPS.iter().map(|g| g.name).chain(["Calculator"]);
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
}

fn block(title: &str, focused: bool) -> Block<'_> {
    let color = if focused { ACCENT } else { Color::DarkGray };
    Block::bordered()
        .title(title)
        .border_style(Style::new().fg(color))
}

fn draw_converter(f: &mut Frame, app: &mut App, area: Rect) {
    let [value_area, rest] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    let [cats_area, units_area, results_area] = Layout::horizontal([
        Constraint::Length(18),
        Constraint::Length(26),
        Constraint::Fill(1),
    ])
    .areas(rest);
    app.areas.cats = cats_area;
    app.areas.units = units_area;
    app.areas.results = results_area;

    let units = app.category().units;
    let from = app.from_index();
    let value = app.value();

    // Value input
    let mut value_block = block(" Value ", true).title(
        Line::from(format!(" {} ({}) ", units[from].name, units[from].symbol)).right_aligned(),
    );
    if let Err(e) = &value {
        value_block = value_block.title_bottom(Line::from(format!(" {e} ")).red());
    }
    let typed = if app.input.is_empty() {
        Span::from("1").dark_gray()
    } else {
        app.input.as_str().bold()
    };
    let input = Line::from(vec![typed, Span::from("█").fg(ACCENT)]);
    f.render_widget(Paragraph::new(input).block(value_block), value_area);

    // Category list
    let highlight = Style::new()
        .fg(Color::Black)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD);
    let cats = GROUPS[app.tab]
        .categories
        .iter()
        .map(|c| ListItem::new(c.name));
    let list = List::new(cats)
        .block(block(" Category ", !app.focus_units))
        .highlight_style(highlight);
    f.render_stateful_widget(list, cats_area, &mut app.cats);

    // From unit list
    let items = units.iter().map(|u| ListItem::new(u.name));
    let list = List::new(items)
        .block(block(" From ", app.focus_units))
        .highlight_style(highlight);
    f.render_stateful_widget(list, units_area, &mut app.units);

    // Results: the value in every unit of the category
    let results: Vec<String> = units
        .iter()
        .map(|to| {
            value
                .as_ref()
                .map(|v| fmt(convert(*v, &units[from], to)))
                .unwrap_or_default()
        })
        .collect();
    let width = results.iter().map(|s| s.len()).max().unwrap_or(0);
    let items = units.iter().zip(&results).enumerate().map(|(i, (u, r))| {
        let line = Line::from(vec![
            Span::from(format!(" {r:>width$} ")).bold(),
            Span::from(format!("{:<8}", u.symbol)).fg(ACCENT),
            Span::from(u.name).dark_gray(),
        ]);
        if i == from {
            ListItem::new(line).reversed()
        } else {
            ListItem::new(line)
        }
    });
    let mut state = ListState::default().with_offset(app.res_offset);
    let list = List::new(items).block(block(" Result ", false));
    f.render_stateful_widget(list, results_area, &mut state);
}

fn draw_calc(f: &mut Frame, app: &mut App, area: Rect) {
    let [input_area, rest] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    let [history_area, keys_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Fill(1)]).areas(rest);

    // Expression input
    let mode = if app.ctx.deg { " DEG " } else { " RAD " };
    let input_block = block(" Expression ", true).title(Line::from(mode).right_aligned());
    let input = Line::from(vec![
        Span::from(app.calc_input.as_str()).bold(),
        Span::from("█").fg(ACCENT),
    ]);
    f.render_widget(Paragraph::new(input).block(input_block), input_area);

    // History: show the most recent lines at the bottom
    let visible = history_area.height.saturating_sub(2) as usize / 2;
    let skip = app.history.len().saturating_sub(visible);
    let lines: Vec<Line> = app.history[skip..]
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
