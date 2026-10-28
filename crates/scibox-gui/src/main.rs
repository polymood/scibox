//! scibox-gui: the egui version of scibox. All the logic is in `scibox-core`. This crate only shows it.
// On Windows, a release build is a GUI program: Windows does not open a terminal window for it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod graph;
mod look;
mod math;

use eframe::egui::{
    self, Align, Color32, Key, Layout, Modifiers, RichText, ScrollArea, TextEdit, Ui,
};
use look::{ACCENT, GRAY, GREEN, MAGENTA, RED, boxed, item};
use scibox_core::calc::{self, CONSTANTS, Ctx};
use scibox_core::formulas::{DOMAINS, Formula};
use scibox_core::search::{self, Entry, Target};
use scibox_core::state::{self, Calculator, FormulaInputs, KEY_COLUMNS, KEYS, key_help, key_kind};
use scibox_core::units::{Category, GROUPS};

// The tabs after the converter tabs (one tab for each group in `GROUPS`).
const FORMULAS_TAB: usize = GROUPS.len();
const GRAPH_TAB: usize = GROUPS.len() + 1;
const CALC_TAB: usize = GROUPS.len() + 2;

/// The size of a calculator key in points. The keys keep this size in a big window.
const KEY_SIZE: egui::Vec2 = egui::vec2(66.0, 30.0);
const KEY_GAP: f32 = 6.0;

pub struct App {
    tab: usize,
    /// Converter: the category, the "from" unit, and the input text.
    cat: usize,
    from: usize,
    input: String,
    /// Formulas: the domain, the formula, and the texts of its variables.
    dom: usize,
    formula: usize,
    form: FormulaInputs,
    /// Graph: the items, then the settings (see `plot::SETTINGS`).
    pub graph: [String; 9],
    /// The 3D view angles (yaw, pitch) in radians.
    pub view: (f64, f64),
    /// The 3D surface mode: an index in `graph::SURFACE_MODES`.
    pub surface: usize,
    calc: Calculator,
    pub ctx: Ctx,
    entries: Vec<Entry>,
    /// The "Go to" window: the query and the selected match.
    palette: Option<(String, usize)>,
    /// A message for the status bar, and the time when it appeared.
    status: Option<(String, f64)>,
    /// The pane (or the graph field) that the keyboard controls, as in the TUI.
    pub focus: usize,
    /// The converter result and the formula variable that the keyboard selects.
    res: usize,
    var: usize,
    /// Select all the text of the active field at the next frame: the next key replaces it.
    pub fresh: bool,
    /// Scroll the lists to the selected items at the next frame.
    scroll: bool,
    /// The typeset equation of the formula.
    equation: math::Cache,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        look::style(&cc.egui_ctx);
        let mut app = App {
            tab: 0,
            cat: 0,
            from: 0,
            input: String::new(),
            dom: 0,
            formula: 0,
            form: FormulaInputs::new(&DOMAINS[0].formulas[0]),
            graph: std::array::from_fn(|i| match i {
                0 => "2sin(x); x^2 + y^2 = 9; x = 6".into(),
                i => scibox_core::plot::SETTINGS[i - 1].1.into(),
            }),
            view: (0.6, 0.6),
            surface: 0,
            calc: Calculator::default(),
            ctx: Ctx {
                deg: true,
                ..Default::default()
            },
            entries: search::entries(&[
                (FORMULAS_TAB, "Formulas"),
                (GRAPH_TAB, "Graph"),
                (CALC_TAB, "Calc"),
            ]),
            palette: None,
            status: None,
            focus: 0,
            res: 0,
            var: 1,
            fresh: false,
            scroll: false,
            equation: math::Cache::default(),
        };
        // Options for the screenshots of the README: go to an entry, set the graph, type in the calculator.
        if let Ok(query) = std::env::var("SCIBOX_GOTO")
            && let Some(&e) = search::find(&app.entries, &query).first()
        {
            app.jump(e);
        }
        if let Ok(mode) = std::env::var("SCIBOX_SURFACE") {
            app.surface = mode.parse().unwrap_or(0) % graph::SURFACE_MODES.len();
        }
        if let Ok(g) = std::env::var("SCIBOX_GRAPH") {
            for (i, part) in g.split('|').enumerate().take(9) {
                app.graph[i] = part.into();
            }
        }
        if let Ok(exprs) = std::env::var("SCIBOX_CALC") {
            for e in exprs.split('|') {
                app.calc.input = e.into();
                app.calc.evaluate(&mut app.ctx);
            }
        }
        app
    }

    fn category(&self) -> &'static Category {
        &GROUPS[self.tab.min(GROUPS.len() - 1)].categories[self.cat]
    }

    fn fm(&self) -> &'static Formula {
        &DOMAINS[self.dom].formulas[self.formula]
    }

    fn set_tab(&mut self, tab: usize) {
        if tab != self.tab {
            self.tab = tab;
            self.cat = 0;
            self.from = 0;
            self.res = 0;
            self.focus = 0;
        }
    }

    fn select_formula(&mut self, dom: usize, formula: usize) {
        self.dom = dom;
        self.formula = formula;
        self.form = FormulaInputs::new(self.fm());
        self.var = 1.min(self.fm().vars.len() - 1);
    }

    /// Go to a search entry.
    fn jump(&mut self, entry: usize) {
        match self.entries[entry].target {
            Target::Tab(t) => self.set_tab(t),
            Target::Category(t, c, unit) => {
                self.set_tab(t);
                self.cat = c;
                self.from = unit.unwrap_or(0);
            }
            Target::Formula(d, f) => {
                self.set_tab(FORMULAS_TAB);
                self.select_formula(d, f);
            }
        }
        if self.tab == FORMULAS_TAB {
            self.focus = 2;
            self.fresh = true;
        }
    }

    /// The keys of the TUI: ←→ tab, Tab focus, ↑↓ select, Enter, Delete, Ctrl+Y, Esc.
    fn keyboard(&mut self, ui: &mut Ui) {
        let none = Modifiers::NONE;
        // Shift+Tab first: a test for Tab without modifiers also accepts Shift+Tab.
        let (back, tab, left, right, up, down, enter, delete, copy, esc) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::SHIFT, Key::Tab),
                i.consume_key(none, Key::Tab),
                i.consume_key(none, Key::ArrowLeft),
                i.consume_key(none, Key::ArrowRight),
                i.consume_key(none, Key::ArrowUp),
                i.consume_key(none, Key::ArrowDown),
                i.consume_key(none, Key::Enter),
                i.consume_key(none, Key::Delete),
                i.consume_key(Modifiers::COMMAND, Key::Y),
                i.consume_key(none, Key::Escape),
            )
        });
        if esc {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let tabs = GROUPS.len() + 3;
        if left {
            self.set_tab((self.tab + tabs - 1) % tabs);
        }
        if right {
            self.set_tab((self.tab + 1) % tabs);
        }
        let panes = match self.tab {
            GRAPH_TAB => 9,
            CALC_TAB => 1,
            _ => 3,
        };
        if tab || back {
            self.focus = (self.focus + if tab { 1 } else { panes - 1 }) % panes;
            self.fresh = true;
        }
        if up || down {
            self.move_sel(if up { -1 } else { 1 });
        }
        if enter {
            match self.tab {
                FORMULAS_TAB => self.form.set_unknown(self.fm(), self.var, &self.ctx),
                CALC_TAB => self.calc.evaluate(&mut self.ctx),
                GRAPH_TAB => {}
                _ if self.focus == 2 => {
                    if let Ok(r) =
                        state::convert_all(self.category(), self.from, &self.input, &self.ctx)
                    {
                        self.input = r[self.res].clone();
                        self.from = self.res;
                    }
                }
                _ => {}
            }
        }
        if delete {
            match self.tab {
                FORMULAS_TAB => {
                    self.form.edit(self.fm(), self.var, &self.ctx);
                    self.form.text[self.var].clear();
                }
                GRAPH_TAB => self.graph[self.focus].clear(),
                CALC_TAB => self.calc.input.clear(),
                _ => self.input.clear(),
            }
        }
        if copy {
            let text = match self.tab {
                FORMULAS_TAB => self.form.solve(self.fm(), &self.ctx).ok().map(calc::fmt),
                CALC_TAB => self.calc.history.last().map(|h| h.1.clone()),
                GRAPH_TAB => None,
                _ => state::convert_all(self.category(), self.from, &self.input, &self.ctx)
                    .ok()
                    .map(|r| r[self.res].clone()),
            };
            if let Some(t) = text {
                self.copy(ui, t);
            }
        }
    }

    /// Move the selection of the focused pane by `d` rows (graph: go to the previous or the next field).
    fn move_sel(&mut self, d: isize) {
        let step = |v: usize, len: usize| v.saturating_add_signed(d).min(len.saturating_sub(1));
        match (self.tab, self.focus) {
            (GRAPH_TAB, f) => {
                self.focus = step(f, 9);
                self.fresh = true;
            }
            (CALC_TAB, _) => {}
            (FORMULAS_TAB, 0) => self.select_formula(step(self.dom, DOMAINS.len()), 0),
            (FORMULAS_TAB, 1) => self.select_formula(
                self.dom,
                step(self.formula, DOMAINS[self.dom].formulas.len()),
            ),
            (FORMULAS_TAB, _) => {
                self.var = step(self.var, self.fm().vars.len());
                self.fresh = true;
            }
            (_, 0) => {
                self.cat = step(self.cat, GROUPS[self.tab].categories.len());
                self.from = 0;
                self.res = 0;
            }
            (_, 1) => self.from = step(self.from, self.category().units.len()),
            _ => self.res = step(self.res, self.category().units.len()),
        }
        self.scroll = true;
    }

    fn copy(&mut self, ui: &Ui, text: String) {
        ui.ctx().copy_text(text.clone());
        self.status = Some((format!("Copied: {text}"), ui.input(|i| i.time)));
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(" scibox ")
                    .strong()
                    .color(Color32::BLACK)
                    .background_color(ACCENT),
            );
            ui.add_space(8.0);
            let names = GROUPS
                .iter()
                .map(|g| g.name)
                .chain(["Formulas", "Graph", "Calc"]);
            for (i, name) in names.enumerate() {
                if item(ui, self.tab == i, format!(" {name} ")).clicked() {
                    self.set_tab(i);
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("⌕ Ctrl+P").clicked() {
                    self.palette = Some((String::new(), 0));
                }
            });
        });
    }

    fn converter(&mut self, ui: &mut Ui) {
        let cat = self.category();
        let results = state::convert_all(cat, self.from, &self.input, &self.ctx);
        let unit = &cat.units[self.from];
        egui::Panel::top("value")
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "Value", true, false, |ui| {
                    ui.horizontal(|ui| {
                        let width = ui.available_width() - 300.0;
                        let r = ui.add(
                            TextEdit::singleline(&mut self.input)
                                .hint_text("1")
                                .frame(egui::Frame::new())
                                .desired_width(width),
                        );
                        if self.palette.is_none() {
                            r.request_focus();
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{} ({})", unit.name, unit.symbol))
                                    .color(ACCENT),
                            );
                        });
                    });
                    if let Err(e) = &results {
                        ui.label(RichText::new(e).color(RED));
                    }
                });
            });
        egui::Panel::left("categories")
            .exact_size(200.0)
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "Category", self.focus == 0, true, |ui| {
                    for (i, c) in GROUPS[self.tab].categories.iter().enumerate() {
                        let r = item(ui, self.cat == i, c.name);
                        if r.clicked() {
                            (self.cat, self.from, self.res, self.focus) = (i, 0, 0, 0);
                        }
                        if self.scroll && self.cat == i {
                            r.scroll_to_me(None);
                        }
                    }
                });
            });
        egui::Panel::left("units")
            .exact_size(260.0)
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "From", self.focus == 1, true, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        for (i, u) in cat.units.iter().enumerate() {
                            let r = item(ui, self.from == i, u.name);
                            if r.clicked() {
                                (self.from, self.focus) = (i, 1);
                            }
                            if self.scroll && self.from == i {
                                r.scroll_to_me(None);
                            }
                        }
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(
                    ui,
                    "Result · click: copy · right-click: use",
                    self.focus == 2,
                    true,
                    |ui| {
                        let results: Vec<String> = results
                            .unwrap_or_default()
                            .iter()
                            .map(|r| calc::pretty(r))
                            .collect();
                        let width = results.iter().map(|r| r.chars().count()).max().unwrap_or(0);
                        ScrollArea::vertical().show(ui, |ui| {
                            for (i, (u, r)) in cat.units.iter().zip(&results).enumerate() {
                                let text = RichText::new(format!(
                                    "{r:>width$} {:<8} {}",
                                    u.symbol, u.name
                                ));
                                // The keyboard cursor (black on the accent color), and the "from" unit (yellow).
                                let selected = self.focus == 2 && i == self.res;
                                let text = if selected {
                                    text.color(Color32::BLACK).strong()
                                } else if i == self.from {
                                    text.color(look::YELLOW)
                                } else {
                                    text
                                };
                                let resp = look::item_rich(ui, selected, text);
                                if self.scroll && selected {
                                    resp.scroll_to_me(None);
                                }
                                if resp.clicked() {
                                    (self.res, self.focus) = (i, 2);
                                    self.copy(ui, r.clone());
                                }
                                if resp.secondary_clicked() {
                                    self.input = r.clone();
                                    self.from = i;
                                }
                            }
                        });
                    },
                );
            });
    }

    fn formulas(&mut self, ui: &mut Ui) {
        egui::Panel::left("domains")
            .exact_size(170.0)
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "Domain", self.focus == 0, true, |ui| {
                    for (i, d) in DOMAINS.iter().enumerate() {
                        if item(ui, self.dom == i, d.name).clicked() {
                            self.select_formula(i, 0);
                            self.focus = 0;
                        }
                    }
                });
            });
        egui::Panel::left("formulas")
            .exact_size(330.0)
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "Formula", self.focus == 1, true, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        for (i, f) in DOMAINS[self.dom].formulas.iter().enumerate() {
                            let r = item(ui, self.formula == i, f.name);
                            if r.clicked() {
                                self.select_formula(self.dom, i);
                                self.focus = 1;
                            }
                            if self.scroll && self.formula == i {
                                r.scroll_to_me(None);
                            }
                        }
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(look::panel())
            .show(ui, |ui| {
                let fm = self.fm();
                boxed(ui, fm.name, false, false, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.add_space(6.0);
                    let typeset = scibox_core::latex::to_latex(fm.equation)
                        .is_ok_and(|tex| self.equation.show(ui, &tex, 26, ACCENT));
                    if !typeset {
                        ui.label(RichText::new(fm.equation).size(20.0).color(ACCENT).strong());
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(fm.equation).color(GRAY));
                });
                let solved = self.form.solve(fm, &self.ctx);
                if let Ok(v) = &solved {
                    self.form.text[self.form.unknown] = self.ctx.fmt(*v);
                }
                boxed(
                    ui,
                    "Variables · ◉ = solve for this variable (Enter)",
                    self.focus == 2,
                    true,
                    |ui| {
                        egui::Grid::new("vars")
                            .num_columns(5)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                for (i, var) in fm.vars.iter().enumerate() {
                                    let unknown = i == self.form.unknown;
                                    let radio = RichText::new(if unknown { "◉" } else { "○" })
                                        .color(if unknown { GREEN } else { GRAY });
                                    if ui
                                        .selectable_label(false, radio)
                                        .on_hover_text("Solve for this variable")
                                        .clicked()
                                    {
                                        self.form.set_unknown(fm, i, &self.ctx);
                                    }
                                    ui.label(RichText::new(var.symbol).strong());
                                    let ok = calc::eval(&self.form.text[i], &self.ctx).is_ok();
                                    let color = if unknown {
                                        GREEN
                                    } else if ok {
                                        look::TEXT
                                    } else {
                                        RED
                                    };
                                    let before = self.form.text[i].clone();
                                    let r = ui.add_sized(
                                        [220.0, 22.0],
                                        TextEdit::singleline(&mut self.form.text[i])
                                            .text_color(color),
                                    );
                                    if r.gained_focus() && i != self.var {
                                        (self.var, self.focus) = (i, 2);
                                    }
                                    if i == self.var && self.palette.is_none() {
                                        if std::mem::take(&mut self.fresh) {
                                            look::select_all(ui, &r, &self.form.text[i]);
                                        }
                                        r.request_focus();
                                    }
                                    if r.changed() {
                                        // An edit of the result row: the solver calculates an other variable.
                                        let typed =
                                            std::mem::replace(&mut self.form.text[i], before);
                                        self.form.edit(fm, i, &self.ctx);
                                        self.form.text[i] = typed;
                                    }
                                    ui.label(RichText::new(var.unit).color(ACCENT));
                                    ui.label(RichText::new(var.name).color(GRAY));
                                    ui.end_row();
                                }
                            });
                        ui.add_space(8.0);
                        match solved {
                            Ok(v) => {
                                let text = format!(
                                    "Copy {} = {}",
                                    fm.vars[self.form.unknown].symbol,
                                    calc::pretty(&self.ctx.fmt(v))
                                );
                                if ui.button(text).clicked() {
                                    self.copy(ui, self.ctx.fmt(v));
                                }
                            }
                            Err(e) => {
                                ui.label(RichText::new(e).color(RED));
                            }
                        }
                    },
                );
            });
    }

    fn calculator(&mut self, ui: &mut Ui) {
        let mode = if self.ctx.deg { "DEG" } else { "RAD" };
        egui::Panel::top("expression")
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "Expression", true, false, |ui| {
                    ui.horizontal(|ui| {
                        let r = ui.add(
                            TextEdit::singleline(&mut self.calc.input)
                                .frame(egui::Frame::new())
                                .font(egui::FontId::monospace(20.0))
                                .desired_width(ui.available_width() - 50.0),
                        );
                        if self.palette.is_none() {
                            r.request_focus();
                        }
                        ui.label(RichText::new(mode).color(ACCENT));
                    });
                });
            });
        // The keypad has a fixed size: it does not grow with the window.
        let cols = KEY_COLUMNS as f32;
        let keys_width = cols * KEY_SIZE.x + (cols - 1.0) * KEY_GAP + 22.0;
        egui::Panel::right("keypad")
            .exact_size(keys_width)
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "Keys", false, false, |ui| {
                    egui::Grid::new("keys")
                        .spacing([KEY_GAP, KEY_GAP])
                        .show(ui, |ui| {
                            for (i, key) in KEYS.iter().enumerate() {
                                let label = if *key == "DEG" { mode } else { key };
                                let mut r =
                                    look::key(ui, label, look::key_color(key_kind(key)), KEY_SIZE);
                                if let Some(help) = key_help(key) {
                                    r = r.on_hover_text(help);
                                }
                                if r.clicked() {
                                    self.calc.press(key, &mut self.ctx);
                                }
                                if i % KEY_COLUMNS == KEY_COLUMNS - 1 {
                                    ui.end_row();
                                }
                            }
                        });
                });
                boxed(ui, "Constants", false, true, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        for (name, value, text) in CONSTANTS {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{name:<6}")).color(MAGENTA));
                                ui.label(RichText::new(text).color(GRAY))
                                    .on_hover_text(calc::fmt(value));
                            });
                        }
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(
                    ui,
                    "History · click a result to copy it",
                    false,
                    true,
                    |ui| {
                        ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                            let mut copy = None;
                            for (expr, result) in &self.calc.history {
                                ui.label(RichText::new(expr).color(GRAY));
                                let color = if result.starts_with("Error") {
                                    RED
                                } else {
                                    Color32::WHITE
                                };
                                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                    let text = RichText::new(format!("= {}", calc::pretty(result)))
                                        .color(color)
                                        .strong();
                                    if ui.selectable_label(false, text).clicked() {
                                        copy = Some(result.clone());
                                    }
                                });
                            }
                            if let Some(t) = copy {
                                self.copy(ui, t);
                            }
                        });
                    },
                );
            });
    }

    fn palette_window(&mut self, ui: &mut Ui) {
        let Some((query, selected)) = &mut self.palette else {
            return;
        };
        let matches = search::find(&self.entries, query);
        let mut jump = None;
        let modal = egui::Modal::new(ui.scope_id().with("goto"))
            .frame(egui::Frame::new().fill(look::BG))
            .show(ui.ctx(), |ui| {
                ui.set_width(720.0);
                boxed(
                    ui,
                    &format!("Go to · {} results", matches.len()),
                    true,
                    false,
                    |ui| {
                        let r = ui.add(
                            TextEdit::singleline(query)
                                .hint_text("> type a name: dbm, ohm law, length...")
                                .frame(egui::Frame::new())
                                .desired_width(f32::INFINITY),
                        );
                        r.request_focus();
                        if r.changed() {
                            *selected = 0;
                        }
                        ui.input_mut(|i| {
                            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                                *selected = (*selected + 1).min(matches.len().saturating_sub(1));
                            }
                            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                                *selected = selected.saturating_sub(1);
                            }
                            if i.consume_key(Modifiers::NONE, Key::Enter) {
                                jump = matches.get(*selected).copied();
                            }
                        });
                        ScrollArea::vertical().max_height(440.0).show(ui, |ui| {
                            for (k, &e) in matches.iter().take(200).enumerate() {
                                let entry = &self.entries[e];
                                let r = item(
                                    ui,
                                    k == *selected,
                                    format!("{}  {}", entry.label, entry.detail),
                                );
                                if k == *selected {
                                    r.scroll_to_me(None);
                                }
                                if r.clicked() {
                                    jump = Some(e);
                                }
                            }
                        });
                    },
                );
            });
        if let Some(e) = jump {
            self.palette = None;
            self.jump(e);
        } else if modal.should_close() {
            self.palette = None;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        if ui.input_mut(|i| {
            i.consume_key(Modifiers::COMMAND, Key::P) || i.consume_key(Modifiers::COMMAND, Key::K)
        }) {
            self.palette = Some((String::new(), 0));
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F3)) {
            self.ctx.notation = self.ctx.notation.next();
            self.status = Some((
                format!("Notation: {}", self.ctx.notation.name()),
                ui.input(|i| i.time),
            ));
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F2)) {
            self.surface = (self.surface + 1) % graph::SURFACE_MODES.len();
        }
        if self.palette.is_none() {
            self.keyboard(ui);
        }
        egui::Panel::top("tabs")
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                ui.add_space(4.0);
                self.top_bar(ui);
            });
        let now = ui.input(|i| i.time);
        egui::Panel::bottom("status")
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| match &self.status {
                Some((text, t)) if now - t < 3.0 => {
                    ui.label(RichText::new(format!(" {text}")).color(GREEN).strong());
                }
                _ => {
                    ui.label(
                        RichText::new(
                            " ←→ tab · Tab focus · ↑↓ select · Enter use/solve/= · Del clear · Ctrl+Y copy · Ctrl+P go to · F2 3D · F3 notation · Esc quit",
                        )
                        .color(GRAY),
                    );
                }
            });
        match self.tab {
            FORMULAS_TAB => self.formulas(ui),
            GRAPH_TAB => graph::show(self, ui),
            CALC_TAB => self.calculator(ui),
            _ => self.converter(ui),
        }
        self.scroll = false;
        self.palette_window(ui);
        screenshot(ui);
    }
}

/// If the variable SCIBOX_SCREENSHOT is set, save the window to that file (PPM format) and quit.
fn screenshot(ui: &Ui) {
    let Ok(path) = std::env::var("SCIBOX_SCREENSHOT") else {
        return;
    };
    let ctx = ui.ctx().clone();
    if ctx.cumulative_frame_nr() == 10 {
        let done = ctx.clone();
        let save = move |image: std::sync::Arc<egui::ColorImage>| {
            let mut out = format!("P6 {} {} 255\n", image.width(), image.height()).into_bytes();
            out.extend(image.pixels.iter().flat_map(|p| [p.r(), p.g(), p.b()]));
            let _ = std::fs::write(&path, out);
            done.send_viewport_cmd(egui::ViewportCommand::Close);
            done.request_repaint();
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
            egui::ScreenshotCallback::new(save),
        ));
    }
    ctx.request_repaint();
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("scibox")
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native("scibox", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
