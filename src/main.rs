mod calc;
mod ui;
mod units;

use calc::Ctx;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::ListState;
use units::{Category, GROUPS};

/// The calculator keypad, row by row (6 x 6).
pub const KEYS: [&str; 36] = [
    "7", "8", "9", "/", "(", ")", //
    "4", "5", "6", "*", "^", "√", //
    "1", "2", "3", "-", "π", "e", //
    "0", ".", "ans", "+", "C", "⌫", //
    "sin", "cos", "tan", "ln", "log", "=", //
    "asin", "acos", "atan", "exp", "x²", "DEG", //
];

/// The screen areas of the last frame. The mouse handler uses them to find the clicked item.
#[derive(Default)]
pub struct Areas {
    pub tabs: Vec<Rect>,
    pub cats: Rect,
    pub units: Rect,
    pub results: Rect,
    pub keys: Vec<Rect>,
}

pub struct App {
    /// An index in `GROUPS`. The value `GROUPS.len()` is the calculator tab.
    pub tab: usize,
    /// The list that the arrow keys move: the unit list (true) or the category list (false).
    pub focus_units: bool,
    pub cats: ListState,
    pub units: ListState,
    pub res_offset: usize,
    pub input: String,
    pub calc_input: String,
    pub history: Vec<(String, String)>,
    pub ctx: Ctx,
    /// True just after "=". Then a new number replaces the result, and an operator continues it.
    fresh: bool,
    pub pressed: Option<usize>,
    pub areas: Areas,
    quit: bool,
}

const TABS: usize = GROUPS.len() + 1;

impl App {
    fn new() -> Self {
        App {
            tab: 0,
            focus_units: false,
            cats: ListState::default().with_selected(Some(0)),
            units: ListState::default().with_selected(Some(0)),
            res_offset: 0,
            input: String::new(),
            calc_input: String::new(),
            history: Vec::new(),
            ctx: Ctx {
                ans: 0.0,
                deg: true,
            },
            fresh: false,
            pressed: None,
            areas: Areas::default(),
            quit: false,
        }
    }

    pub fn is_calc(&self) -> bool {
        self.tab == GROUPS.len()
    }

    pub fn category(&self) -> &'static Category {
        &GROUPS[self.tab].categories[self.cats.selected().unwrap_or(0)]
    }

    pub fn from_index(&self) -> usize {
        self.units.selected().unwrap_or(0)
    }

    /// The value in the converter input, or an error message. An empty input is 1.
    pub fn value(&self) -> Result<f64, String> {
        if self.input.is_empty() {
            Ok(1.0)
        } else {
            calc::eval(&self.input, &self.ctx)
        }
    }

    fn input_mut(&mut self) -> &mut String {
        if self.is_calc() {
            &mut self.calc_input
        } else {
            &mut self.input
        }
    }

    fn set_tab(&mut self, tab: usize) {
        self.tab = tab;
        self.select_cat(0);
    }

    fn select_cat(&mut self, i: usize) {
        self.cats.select(Some(i));
        self.units.select(Some(0));
        self.res_offset = 0;
    }

    /// Move the selection of the category list or the unit list by `d` rows.
    fn move_sel(&mut self, units: bool, d: isize) {
        if self.is_calc() {
            return;
        }
        let (sel, len) = if units {
            (self.from_index(), self.category().units.len())
        } else {
            (
                self.cats.selected().unwrap_or(0),
                GROUPS[self.tab].categories.len(),
            )
        };
        let new = sel.saturating_add_signed(d).min(len - 1);
        if units {
            self.units.select(Some(new));
        } else if new != sel {
            self.select_cat(new);
        }
    }

    fn on_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if ctrl => self.quit = true,
            KeyCode::Left => self.set_tab((self.tab + TABS - 1) % TABS),
            KeyCode::Right => self.set_tab((self.tab + 1) % TABS),
            KeyCode::Tab | KeyCode::BackTab => self.focus_units = !self.focus_units,
            KeyCode::Up => self.move_sel(self.focus_units, -1),
            KeyCode::Down => self.move_sel(self.focus_units, 1),
            KeyCode::Enter => self.press("="),
            KeyCode::Backspace => self.press("⌫"),
            KeyCode::Delete => self.press("C"),
            KeyCode::Char(c) if !ctrl && self.is_calc() => self.type_text(&c.to_string()),
            KeyCode::Char(c) if !ctrl => self.input.push(c),
            _ => {}
        }
    }

    /// Do the action of a keypad key. The keyboard uses this function too.
    fn press(&mut self, key: &str) {
        match key {
            "C" => self.input_mut().clear(),
            "⌫" => {
                self.input_mut().pop();
            }
            "=" if self.is_calc() => self.evaluate(),
            "=" => {}
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
            MouseEventKind::Down(MouseButton::Left) => self.click(pos),
            MouseEventKind::Up(_) => self.pressed = None,
            MouseEventKind::ScrollUp => self.scroll(pos, -1),
            MouseEventKind::ScrollDown => self.scroll(pos, 1),
            _ => {}
        }
    }

    fn click(&mut self, pos: Position) {
        if let Some(i) = self.areas.tabs.iter().position(|r| r.contains(pos)) {
            return self.set_tab(i);
        }
        if self.is_calc() {
            if let Some(i) = self.areas.keys.iter().position(|r| r.contains(pos)) {
                self.pressed = Some(i);
                self.press(KEYS[i]);
            }
            return;
        }
        let units = self.category().units;
        if let Some(i) = row(self.areas.cats, self.cats.offset(), pos) {
            if i < GROUPS[self.tab].categories.len() {
                self.focus_units = false;
                self.select_cat(i);
            }
        } else if let Some(i) = row(self.areas.units, self.units.offset(), pos) {
            if i < units.len() {
                self.focus_units = true;
                self.units.select(Some(i));
            }
        } else if let Some(i) = row(self.areas.results, self.res_offset, pos) {
            // Make the clicked result the new input value and unit.
            if let (Some(to), Ok(v)) = (units.get(i), self.value()) {
                self.input = calc::fmt(units::convert(v, &units[self.from_index()], to));
                self.units.select(Some(i));
            }
        }
    }

    fn scroll(&mut self, pos: Position, d: isize) {
        if self.areas.cats.contains(pos) {
            self.move_sel(false, d);
        } else if self.areas.units.contains(pos) {
            self.move_sel(true, d);
        } else if self.areas.results.contains(pos) {
            let visible = self.areas.results.height.saturating_sub(2) as usize;
            let max = self.category().units.len().saturating_sub(visible);
            self.res_offset = self.res_offset.saturating_add_signed(d).min(max);
        }
    }
}

/// The list row under `pos`, or `None` if `pos` is not inside the bordered list `area`.
fn row(area: Rect, offset: usize, pos: Position) -> Option<usize> {
    let inside = area.contains(pos) && pos.y > area.y && pos.y + 1 < area.bottom();
    inside.then(|| offset + (pos.y - area.y - 1) as usize)
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let mut app = App::new();
    let result = (|| {
        while !app.quit {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => app.on_key(k),
                Event::Mouse(m) => app.on_mouse(m),
                _ => {}
            }
        }
        Ok(())
    })();
    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    result
}
