mod calc;
#[rustfmt::skip]
mod formulas;
mod graph;
mod palette;
mod ui;
mod units;

use calc::Ctx;
use formulas::{DOMAINS, Formula};
use palette::{Entry, Palette, Target};
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::ListState;
use units::{Category, Conv, GROUPS};

/// The calculator keypad, row by row (6 x 6).
pub const KEYS: [&str; 36] = [
    "7", "8", "9", "/", "(", ")", //
    "4", "5", "6", "*", "^", "√", //
    "1", "2", "3", "-", "π", "e", //
    "0", ".", "ans", "+", "C", "⌫", //
    "sin", "cos", "tan", "ln", "log", "=", //
    "asin", "acos", "atan", "exp", "x²", "DEG", //
];

// The tabs after the converter tabs (one tab for each group in `GROUPS`).
pub const FORMULAS_TAB: usize = GROUPS.len();
pub const GRAPH_TAB: usize = GROUPS.len() + 1;
pub const CALC_TAB: usize = GROUPS.len() + 2;
const TABS: usize = GROUPS.len() + 3;

#[derive(PartialEq)]
pub enum Kind {
    Convert,
    Formulas,
    Graph,
    Calc,
}

/// The screen areas of the last frame. The mouse handler uses them to find the clicked item.
#[derive(Default)]
pub struct Areas {
    pub tabs: Vec<Rect>,
    pub search: Rect,
    /// The inner areas of the three list panes.
    pub panes: [Rect; 3],
    /// The graph input fields.
    pub fields: Vec<Rect>,
    /// The plot area of the graph.
    pub chart: Rect,
    pub keys: Vec<Rect>,
    /// The inner area of the palette. The first line is the query, the other lines are the list.
    pub palette: Rect,
}

pub struct App {
    pub tab: usize,
    /// The pane that the keyboard controls (0, 1 or 2).
    pub focus: usize,
    /// Converter: category, "from" unit, result. Formulas: domain, formula, variable.
    pub lists: [ListState; 3],
    pub input: String,
    /// The text of each formula variable.
    pub fin: Vec<String>,
    /// The formula variable that the solver calculates.
    pub unknown: usize,
    /// The formula variables that the user changed, the most recent last.
    edited: Vec<usize>,
    /// Graph: the functions, x min, x max, y min, y max.
    pub graph: [String; 5],
    /// The graph line style: an index in `graph::MARKERS`.
    pub marker: usize,
    /// The 3D view angles (yaw, pitch) in radians.
    pub view: (f64, f64),
    /// The y range of the last 2D graph.
    pub y_shown: (f64, f64),
    pub calc_input: String,
    pub history: Vec<(String, String)>,
    pub ctx: Ctx,
    /// True just after a new selection or "=". Then the next character replaces the text.
    fresh: bool,
    pub pressed: Option<usize>,
    /// The last mouse position during a drag on the graph.
    drag: Option<(u16, u16)>,
    pub entries: Vec<Entry>,
    pub palette: Option<Palette>,
    /// A message for the status line, for example "Copied".
    pub status: Option<String>,
    pub areas: Areas,
    quit: bool,
}

impl App {
    fn new() -> Self {
        let mut app = App {
            tab: 0,
            focus: 0,
            lists: Default::default(),
            input: String::new(),
            fin: Vec::new(),
            unknown: 0,
            edited: Vec::new(),
            graph: [
                "2sin(x); x^2 + y^2 = 9; x = 6".into(),
                "-10".into(),
                "10".into(),
                String::new(),
                String::new(),
            ],
            marker: 0,
            view: (0.6, 0.6),
            y_shown: (-1.0, 1.0),
            calc_input: String::new(),
            history: Vec::new(),
            ctx: Ctx {
                deg: true,
                ..Default::default()
            },
            fresh: false,
            pressed: None,
            drag: None,
            entries: palette::entries(&[
                (FORMULAS_TAB, "Formulas"),
                (GRAPH_TAB, "Graph"),
                (CALC_TAB, "Calc"),
            ]),
            palette: None,
            status: None,
            areas: Areas::default(),
            quit: false,
        };
        app.set_tab(0);
        app
    }

    pub fn kind(&self) -> Kind {
        match self.tab {
            FORMULAS_TAB => Kind::Formulas,
            GRAPH_TAB => Kind::Graph,
            CALC_TAB => Kind::Calc,
            _ => Kind::Convert,
        }
    }

    pub fn sel(&self, pane: usize) -> usize {
        self.lists[pane].selected().unwrap_or(0)
    }

    pub fn category(&self) -> &'static Category {
        &GROUPS[self.tab.min(GROUPS.len() - 1)].categories[self.sel(0)]
    }

    pub fn formula(&self) -> &'static Formula {
        &DOMAINS[self.sel(0)].formulas[self.sel(1)]
    }

    /// The number of panes (or input fields) that can have the focus.
    fn pane_count(&self) -> usize {
        if self.kind() == Kind::Graph { 5 } else { 3 }
    }

    /// The number of rows in a pane.
    fn pane_len(&self, pane: usize) -> usize {
        match (self.kind(), pane) {
            (Kind::Convert, 0) => GROUPS[self.tab].categories.len(),
            (Kind::Convert, _) => self.category().units.len(),
            (Kind::Formulas, 0) => DOMAINS.len(),
            (Kind::Formulas, 1) => DOMAINS[self.sel(0)].formulas.len(),
            (Kind::Formulas, _) => self.formula().vars.len(),
            _ => 0,
        }
    }

    /// The converter value of the input, or an error message. An empty input is 1.
    pub fn value(&self) -> Result<f64, String> {
        if self.input.is_empty() {
            Ok(1.0)
        } else {
            calc::eval(&self.input, &self.ctx)
        }
    }

    /// The converter results: the input value in each unit of the category.
    pub fn results(&self) -> Result<Vec<String>, String> {
        let units = self.category().units;
        let from = &units[self.sel(1)];
        if let Conv::Radix(_) = from.conv {
            let input = if self.input.is_empty() {
                "1"
            } else {
                &self.input
            };
            return units
                .iter()
                .map(|to| units::radix_convert(input, from, to))
                .collect();
        }
        let v = self.value()?;
        Ok(units
            .iter()
            .map(|to| calc::fmt(units::convert(v, from, to)))
            .collect())
    }

    /// Solve the formula for the unknown variable.
    pub fn solve(&self) -> Result<f64, String> {
        let fm = self.formula();
        let mut x = vec![0.0; fm.vars.len()];
        for (i, text) in self.fin.iter().enumerate() {
            if i != self.unknown {
                x[i] = calc::eval(text, &self.ctx)
                    .map_err(|e| format!("{}: {e}", fm.vars[i].symbol))?;
            }
        }
        let guess =
            calc::eval(&self.fin[self.unknown], &self.ctx).unwrap_or(fm.vars[self.unknown].default);
        formulas::solve(fm, &mut x, self.unknown, guess)
    }

    fn set_tab(&mut self, tab: usize) {
        self.tab = tab;
        self.focus = 0;
        self.fresh = false;
        self.select(0, 0);
    }

    /// Select row `i` in a pane, and reset the panes that depend on it.
    fn select(&mut self, pane: usize, i: usize) {
        self.lists[pane].select(Some(i));
        if pane == 0 {
            self.lists[1].select(Some(0));
            self.lists[2].select(Some(0));
        }
        if self.kind() == Kind::Formulas {
            if pane < 2 {
                self.load_formula();
            }
            self.fresh = true;
        }
    }

    /// Put the default values of the formula in the inputs. Solve for the first variable.
    fn load_formula(&mut self) {
        let fm = self.formula();
        let mut x: Vec<f64> = fm.vars.iter().map(|v| v.default).collect();
        x[0] = (fm.f)(&x);
        self.fin = x.iter().map(|&v| calc::fmt(v)).collect();
        self.unknown = 0;
        self.edited.clear();
        self.lists[2].select(Some(1.min(x.len() - 1)));
    }

    fn set_unknown(&mut self, i: usize) {
        if let Ok(v) = self.solve() {
            self.fin[self.unknown] = calc::fmt(v);
        }
        self.unknown = i;
    }

    /// Move the selection of a pane by `d` rows.
    fn move_sel(&mut self, pane: usize, d: isize) {
        let len = self.pane_len(pane);
        let sel = self.sel(pane);
        let new = sel.saturating_add_signed(d).min(len.saturating_sub(1));
        if len > 0 && new != sel {
            self.select(pane, new);
        }
    }

    /// Make converter result `i` the new input value and unit.
    fn use_result(&mut self, i: usize) {
        if let Ok(r) = self.results() {
            self.input = r[i].clone();
            self.select(1, i);
        }
    }

    /// Copy the current result: the selected converter result, the formula result, or the last calculation.
    fn copy_current(&mut self) {
        let text = match self.kind() {
            Kind::Convert => self.results().ok().map(|r| r[self.sel(2)].clone()),
            Kind::Formulas => self.solve().ok().map(calc::fmt),
            Kind::Calc => self.history.last().map(|h| h.1.clone()),
            Kind::Graph => None,
        };
        if let Some(t) = text {
            copy_to_clipboard(&t);
            self.status = Some(format!("Copied: {t}"));
        }
    }

    /// The text that the keyboard edits, or `None` if no text is editable.
    fn input_mut(&mut self) -> Option<&mut String> {
        match self.kind() {
            Kind::Convert => Some(&mut self.input),
            Kind::Formulas => {
                let i = self.sel(2);
                if i == self.unknown {
                    // Solve for the variable that the user did not change for the longest time.
                    let age = |j: &usize| {
                        self.edited
                            .iter()
                            .position(|e| e == j)
                            .map_or(-1, |p| p as isize)
                    };
                    let other = (0..self.fin.len()).filter(|&j| j != i).min_by_key(age)?;
                    self.set_unknown(other);
                    self.status = Some(format!("Solve for {}", self.formula().vars[other].symbol));
                }
                self.edited.retain(|&e| e != i);
                self.edited.push(i);
                if std::mem::take(&mut self.fresh) {
                    self.fin[i].clear();
                }
                Some(&mut self.fin[i])
            }
            Kind::Graph => Some(&mut self.graph[self.focus]),
            Kind::Calc => Some(&mut self.calc_input),
        }
    }

    fn on_key(&mut self, k: KeyEvent) {
        self.status = None;
        if self.palette.is_some() {
            return self.palette_key(k);
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let kind = self.kind();
        match k.code {
            KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if ctrl => self.quit = true,
            KeyCode::Char('p' | 'k') if ctrl => self.palette = Some(Palette::new(&self.entries)),
            KeyCode::Char('y') if ctrl => self.copy_current(),
            KeyCode::Left => self.set_tab((self.tab + TABS - 1) % TABS),
            KeyCode::Right => self.set_tab((self.tab + 1) % TABS),
            KeyCode::F(2) => self.marker = (self.marker + 1) % graph::MARKERS.len(),
            KeyCode::Tab => self.focus = (self.focus + 1) % self.pane_count(),
            KeyCode::BackTab => {
                self.focus = (self.focus + self.pane_count() - 1) % self.pane_count()
            }
            KeyCode::Up if kind == Kind::Graph => self.focus = self.focus.saturating_sub(1),
            KeyCode::Down if kind == Kind::Graph => self.focus = (self.focus + 1).min(4),
            KeyCode::Up => self.move_sel(self.focus, -1),
            KeyCode::Down => self.move_sel(self.focus, 1),
            KeyCode::Enter => match kind {
                Kind::Convert if self.focus == 2 => self.use_result(self.sel(2)),
                Kind::Formulas => self.set_unknown(self.sel(2)),
                Kind::Calc => self.evaluate(),
                _ => {}
            },
            KeyCode::Backspace => self.press("⌫"),
            KeyCode::Delete => self.press("C"),
            KeyCode::Char(c) if !ctrl && kind == Kind::Calc => self.type_text(&c.to_string()),
            KeyCode::Char(c) if !ctrl => {
                if let Some(s) = self.input_mut() {
                    s.push(c);
                }
            }
            _ => {}
        }
    }

    fn palette_key(&mut self, k: KeyEvent) {
        let Some(p) = &mut self.palette else { return };
        match k.code {
            KeyCode::Esc => self.palette = None,
            KeyCode::Enter => {
                let i = p.list.selected().unwrap_or(0);
                self.palette_jump(i);
            }
            KeyCode::Up => p.list.select_previous(),
            KeyCode::Down => p.list.select_next(),
            KeyCode::Backspace => {
                p.query.pop();
                p.update(&self.entries);
            }
            KeyCode::Char(c) => {
                p.query.push(c);
                p.update(&self.entries);
            }
            _ => {}
        }
    }

    /// Go to the palette match `i` and close the palette.
    fn palette_jump(&mut self, i: usize) {
        let Some(p) = self.palette.take() else { return };
        let Some(&e) = p.matches.get(i) else { return };
        match self.entries[e].target {
            Target::Tab(t) => self.set_tab(t),
            Target::Category(t, c, unit) => {
                self.set_tab(t);
                self.select(0, c);
                if let Some(u) = unit {
                    self.select(1, u);
                    self.focus = 1;
                }
            }
            Target::Formula(d, f) => {
                self.set_tab(FORMULAS_TAB);
                self.select(0, d);
                self.select(1, f);
                self.focus = 2;
            }
        }
    }

    /// Do the action of a keypad key. The keyboard uses this function too.
    fn press(&mut self, key: &str) {
        match key {
            "C" => {
                if let Some(s) = self.input_mut() {
                    s.clear();
                }
            }
            "⌫" => {
                if let Some(s) = self.input_mut() {
                    s.pop();
                }
            }
            "=" => self.evaluate(),
            "DEG" => self.ctx.deg = !self.ctx.deg,
            "x²" => self.type_text("^2"),
            "√" => self.type_text("sqrt("),
            "π" => self.type_text("pi"),
            // A function name gets an opening parenthesis.
            f if f.len() > 1 && f != "ans" && f.chars().all(char::is_alphabetic) => {
                self.type_text(&format!("{f}("))
            }
            s => self.type_text(s),
        }
    }

    /// Add text to the calculator input.
    fn type_text(&mut self, s: &str) {
        if self.fresh && !s.starts_with(['+', '-', '*', '/', '^', '%', '!']) {
            self.calc_input.clear();
        }
        self.fresh = false;
        self.calc_input.push_str(s);
    }

    fn evaluate(&mut self) {
        if self.calc_input.is_empty() {
            return;
        }
        match calc::eval(&self.calc_input, &self.ctx) {
            Ok(v) => {
                let result = calc::fmt(v);
                self.history
                    .push((std::mem::take(&mut self.calc_input), result.clone()));
                self.ctx.ans = v;
                self.calc_input = result;
                self.fresh = true;
            }
            Err(e) => self
                .history
                .push((self.calc_input.clone(), format!("Error: {e}"))),
        }
    }

    fn on_mouse(&mut self, m: MouseEvent) {
        let pos = Position::new(m.column, m.row);
        match m.kind {
            MouseEventKind::Down(button) => {
                self.status = None;
                self.click(pos, button);
            }
            MouseEventKind::Drag(MouseButton::Left) => self.drag_to(m.column, m.row),
            MouseEventKind::Up(_) => {
                self.pressed = None;
                self.drag = None;
            }
            MouseEventKind::ScrollUp => self.scroll(pos, -1),
            MouseEventKind::ScrollDown => self.scroll(pos, 1),
            _ => {}
        }
    }

    fn click(&mut self, pos: Position, button: MouseButton) {
        if let Some(p) = &self.palette {
            let area = self.areas.palette;
            let list = Rect {
                y: area.y + 1,
                height: area.height.saturating_sub(1),
                ..area
            };
            if let Some(i) = row(list, p.list.offset(), pos) {
                self.palette_jump(i);
            } else if !area.contains(pos) {
                self.palette = None;
            }
            return;
        }
        if self.areas.search.contains(pos) {
            self.palette = Some(Palette::new(&self.entries));
            return;
        }
        if let Some(i) = self.areas.tabs.iter().position(|r| r.contains(pos)) {
            return self.set_tab(i);
        }
        match self.kind() {
            Kind::Calc => {
                if let Some(i) = self.areas.keys.iter().position(|r| r.contains(pos)) {
                    self.pressed = Some(i);
                    self.press(KEYS[i]);
                }
            }
            Kind::Graph => {
                if let Some(i) = self.areas.fields.iter().position(|r| r.contains(pos)) {
                    self.focus = i;
                } else if self.areas.chart.contains(pos) {
                    self.drag = Some((pos.x, pos.y));
                }
            }
            kind => {
                for pane in 0..3 {
                    let Some(i) = row(self.areas.panes[pane], self.lists[pane].offset(), pos)
                    else {
                        continue;
                    };
                    if i >= self.pane_len(pane) {
                        return;
                    }
                    self.focus = pane;
                    self.select(pane, i);
                    if pane == 2 && kind == Kind::Convert {
                        // Left button: copy the result. Other buttons: use the result as input.
                        if button == MouseButton::Left {
                            self.copy_current();
                        } else {
                            self.use_result(i);
                        }
                    } else if pane == 2 && pos.x <= self.areas.panes[2].x + 2 {
                        // The radio button at the start of a variable row.
                        self.set_unknown(i);
                    } else if pane == 2 && i == self.unknown {
                        self.copy_current();
                    }
                    return;
                }
            }
        }
    }

    fn scroll(&mut self, pos: Position, d: isize) {
        if let Some(p) = &mut self.palette {
            if d < 0 {
                p.list.select_previous()
            } else {
                p.list.select_next()
            }
        } else if self.kind() == Kind::Graph && self.areas.chart.contains(pos) {
            // Zoom around the center. An automatic y range stays automatic.
            let zoom = if d < 0 { 0.8 } else { 1.25 };
            if let Some((a, b)) = self.x_range() {
                self.set_range(1, a, b, zoom, 0.0);
            }
            if let Some((a, b)) = self.y_fields() {
                self.set_range(3, a, b, zoom, 0.0);
            }
        } else if let Some(pane) = self.areas.panes.iter().position(|r| r.contains(pos)) {
            self.move_sel(pane, d);
        }
    }

    /// Move the graph with the mouse, or turn the 3D view.
    fn drag_to(&mut self, col: u16, row: u16) {
        let (Some((c0, r0)), Some((a, b))) = (self.drag, self.x_range()) else {
            return;
        };
        let (dx, dy) = (col as f64 - c0 as f64, row as f64 - r0 as f64);
        self.drag = Some((col, row));
        if graph::is_3d(&self.graph[0]) {
            self.view.0 -= dx * 0.08;
            self.view.1 = (self.view.1 + dy * 0.08).clamp(0.0, std::f64::consts::FRAC_PI_2);
            return;
        }
        let plot = self.areas.chart;
        self.set_range(1, a, b, 1.0, -dx * (b - a) / plot.width.max(1) as f64);
        let (lo, hi) = self.y_fields().unwrap_or(self.y_shown);
        self.set_range(3, lo, hi, 1.0, dy * (hi - lo) / plot.height.max(1) as f64);
    }

    /// Write the range (a, b), zoomed around its center and moved by `shift`, in the fields `i` and `i + 1`.
    fn set_range(&mut self, i: usize, a: f64, b: f64, zoom: f64, shift: f64) {
        let (c, h) = ((a + b) / 2.0 + shift, (b - a) / 2.0 * zoom);
        self.graph[i] = calc::fmt(c - h);
        self.graph[i + 1] = calc::fmt(c + h);
    }

    /// The y range of the fields, or `None` if the y range is automatic.
    pub fn y_fields(&self) -> Option<(f64, f64)> {
        let a = calc::eval(&self.graph[3], &self.ctx).ok()?;
        let b = calc::eval(&self.graph[4], &self.ctx).ok()?;
        (a < b).then_some((a, b))
    }

    pub fn x_range(&self) -> Option<(f64, f64)> {
        let a = calc::eval(&self.graph[1], &self.ctx).ok()?;
        let b = calc::eval(&self.graph[2], &self.ctx).ok()?;
        (a < b).then_some((a, b))
    }
}

/// The list row under `pos`, or `None` if `pos` is not inside `area`.
fn row(area: Rect, offset: usize, pos: Position) -> Option<usize> {
    area.contains(pos)
        .then(|| offset + (pos.y - area.y) as usize)
}

/// Copy text to the system clipboard. Send the OSC 52 sequence (most terminals accept it).
/// Also give the text to the first clipboard program that starts.
fn copy_to_clipboard(text: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut out = std::io::stdout();
    let _ = write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes()));
    let _ = out.flush();
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let x11 = std::env::var_os("DISPLAY").is_some();
    let programs: [(&str, &[&str], bool); 5] = [
        ("clip.exe", &[], true),
        ("pbcopy", &[], true),
        ("wl-copy", &[], wayland),
        ("xclip", &["-selection", "clipboard"], x11),
        ("xsel", &["--clipboard", "--input"], x11),
    ];
    for (program, args, usable) in programs {
        if !usable {
            continue;
        }
        let child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut child) = child {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            // Wait in a thread. Some programs stay open to keep the clipboard.
            std::thread::spawn(move || child.wait());
            return;
        }
    }
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                T[(n >> (18 - 6 * i)) as usize & 63] as char
            } else {
                '='
            });
        }
    }
    out
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let mut app = App::new();
    let result = (|| {
        while !app.quit {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            // Wait for an event that can change the screen. Do not draw again for mouse moves.
            loop {
                match event::read()? {
                    Event::Key(k) if k.kind == KeyEventKind::Press => app.on_key(k),
                    Event::Mouse(m) if m.kind == MouseEventKind::Moved => continue,
                    Event::Mouse(m) => app.on_mouse(m),
                    Event::Resize(..) => {}
                    _ => continue,
                }
                break;
            }
        }
        Ok(())
    })();
    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64() {
        assert_eq!(super::base64(b"Man"), "TWFu");
        assert_eq!(super::base64(b"Ma"), "TWE=");
        assert_eq!(super::base64(b"M"), "TQ==");
    }
}
