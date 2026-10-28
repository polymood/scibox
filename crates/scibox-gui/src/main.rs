//! scibox-gui: the egui version of scibox. All the logic is in `scibox-core`. This crate only shows it.

mod graph;
mod look;

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
    /// Put the keyboard focus on the main input of the tab (after a tab change).
    focus_input: bool,
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
            focus_input: true,
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
            self.focus_input = true;
        }
    }

    fn select_formula(&mut self, dom: usize, formula: usize) {
        self.dom = dom;
        self.formula = formula;
        self.form = FormulaInputs::new(self.fm());
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
        self.focus_input = true;
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
                        if std::mem::take(&mut self.focus_input) {
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
                boxed(ui, "Category", false, true, |ui| {
                    for (i, c) in GROUPS[self.tab].categories.iter().enumerate() {
                        if item(ui, self.cat == i, c.name).clicked() {
                            self.cat = i;
                            self.from = 0;
                        }
                    }
                });
            });
        egui::Panel::left("units")
            .exact_size(260.0)
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "From", true, true, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        for (i, u) in cat.units.iter().enumerate() {
                            if item(ui, self.from == i, u.name).clicked() {
                                self.from = i;
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
                    false,
                    true,
                    |ui| {
                        let results = results.unwrap_or_default();
                        let width = results.iter().map(|r| r.chars().count()).max().unwrap_or(0);
                        ScrollArea::vertical().show(ui, |ui| {
                            for (i, (u, r)) in cat.units.iter().zip(&results).enumerate() {
                                let text = RichText::new(format!(
                                    "{r:>width$} {:<8} {}",
                                    u.symbol, u.name
                                ));
                                let resp = ui.selectable_label(
                                    false,
                                    if i == self.from {
                                        text.color(look::YELLOW)
                                    } else {
                                        text
                                    },
                                );
                                if resp.clicked() {
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
                boxed(ui, "Domain", false, true, |ui| {
                    for (i, d) in DOMAINS.iter().enumerate() {
                        if item(ui, self.dom == i, d.name).clicked() {
                            self.select_formula(i, 0);
                        }
                    }
                });
            });
        egui::Panel::left("formulas")
            .exact_size(330.0)
            .show_separator_line(false)
            .frame(look::panel())
            .show(ui, |ui| {
                boxed(ui, "Formula", false, true, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        for (i, f) in DOMAINS[self.dom].formulas.iter().enumerate() {
                            if item(ui, self.formula == i, f.name).clicked() {
                                self.select_formula(self.dom, i);
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
                    ui.label(RichText::new(fm.equation).size(20.0).color(ACCENT).strong());
                });
                let solved = self.form.solve(fm, &self.ctx);
                if let Ok(v) = &solved {
                    self.form.text[self.form.unknown] = calc::fmt(*v);
                }
                boxed(
                    ui,
                    "Variables · ◉ = solve for this variable",
                    true,
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
                                    calc::fmt(v)
                                );
                                if ui.button(text).clicked() {
                                    self.copy(ui, calc::fmt(v));
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
                        if std::mem::take(&mut self.focus_input) {
                            r.request_focus();
                        }
                        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            self.calc.evaluate(&mut self.ctx);
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
                                    let text =
                                        RichText::new(format!("= {result}")).color(color).strong();
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
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F2)) {
            self.surface = (self.surface + 1) % graph::SURFACE_MODES.len();
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
                            " Ctrl+P go to · click a result to copy it · F2 3D surface mode",
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
