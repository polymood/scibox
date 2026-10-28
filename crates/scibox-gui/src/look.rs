//! The look of the TUI in the GUI: one monospace font, square corners, thin borders, no shadows,
//! boxes with the title in the top border, and black text on the accent color for a selection.

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Response, RichText, Sense, Shadow,
    Stroke, TextStyle, Ui, vec2,
};
use scibox_core::state::KeyKind;

pub const ACCENT: Color32 = Color32::from_rgb(0x2e, 0xc4, 0xd6);
pub const GREEN: Color32 = Color32::from_rgb(0x98, 0xc3, 0x79);
pub const RED: Color32 = Color32::from_rgb(0xe0, 0x6c, 0x75);
pub const YELLOW: Color32 = Color32::from_rgb(0xe5, 0xc0, 0x7b);
pub const MAGENTA: Color32 = Color32::from_rgb(0xc6, 0x78, 0xdd);
pub const GRAY: Color32 = Color32::from_rgb(0x5c, 0x63, 0x70);
pub const TEXT: Color32 = Color32::from_rgb(0xd0, 0xd4, 0xda);
pub const BG: Color32 = Color32::from_rgb(0x10, 0x14, 0x18);
pub const KEY_BG: Color32 = Color32::from_rgb(0x2a, 0x30, 0x38);

pub fn style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.global_style_mut(|s| {
        s.text_styles = [
            (TextStyle::Heading, 17.0),
            (TextStyle::Body, 15.0),
            (TextStyle::Button, 15.0),
            (TextStyle::Monospace, 15.0),
            (TextStyle::Small, 12.0),
        ]
        .into_iter()
        .map(|(t, size)| (t, FontId::new(size, FontFamily::Monospace)))
        .collect();
        let v = &mut s.visuals;
        v.panel_fill = BG;
        v.window_fill = BG;
        v.extreme_bg_color = Color32::from_rgb(0x0b, 0x0e, 0x11);
        v.faint_bg_color = KEY_BG;
        v.window_corner_radius = CornerRadius::ZERO;
        v.menu_corner_radius = CornerRadius::ZERO;
        v.window_shadow = Shadow::NONE;
        v.popup_shadow = Shadow::NONE;
        v.window_stroke = Stroke::new(1.0, ACCENT);
        v.selection.bg_fill = ACCENT;
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.override_text_color = Some(TEXT);
        let w = &mut v.widgets;
        for wv in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            wv.corner_radius = CornerRadius::ZERO;
            wv.expansion = 0.0;
        }
        w.noninteractive.bg_stroke = Stroke::new(1.0, GRAY);
        w.inactive.weak_bg_fill = KEY_BG;
        w.inactive.bg_fill = KEY_BG;
        w.inactive.bg_stroke = Stroke::NONE;
        w.hovered.weak_bg_fill = Color32::from_rgb(0x3a, 0x42, 0x4c);
        w.hovered.bg_stroke = Stroke::new(1.0, GRAY);
        w.active.weak_bg_fill = ACCENT;
        w.active.bg_stroke = Stroke::NONE;
        s.spacing.item_spacing = vec2(8.0, 4.0);
        s.spacing.button_padding = vec2(6.0, 2.0);
    });
}

/// The frame of a panel: the background color and a small margin. The boxes inside give the borders.
pub fn panel() -> egui::Frame {
    egui::Frame::new().fill(BG).inner_margin(Margin::same(3))
}

/// A box with a thin border and the title in the top border, like a block of the TUI.
/// If `fill` is true, the box takes all the space that is available.
pub fn boxed<R>(
    ui: &mut Ui,
    title: &str,
    focused: bool,
    fill: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let color = if focused { ACCENT } else { GRAY };
    let frame = egui::Frame::new()
        .stroke(Stroke::new(1.0, color))
        .inner_margin(Margin {
            left: 8,
            right: 8,
            top: 12,
            bottom: 6,
        })
        .outer_margin(Margin {
            left: 1,
            right: 1,
            top: 8,
            bottom: 1,
        });
    let r = frame.show(ui, |ui| {
        if fill {
            ui.set_min_size(ui.available_size());
        }
        add(ui)
    });
    if !title.is_empty() {
        let painter = ui.painter();
        let galley = painter.layout_no_wrap(format!(" {title} "), FontId::monospace(15.0), color);
        // The response includes the outer margin: the border is 10 points below its top.
        let pos = r.response.rect.left_top() + vec2(12.0, 10.0 - galley.size().y / 2.0);
        painter.rect_filled(egui::Rect::from_min_size(pos, galley.size()), 0.0, BG);
        painter.galley(pos, galley, color);
    }
    r.inner
}

/// A list item. A selected item has black text on the accent color, as in the TUI.
pub fn item(ui: &mut Ui, selected: bool, text: impl Into<String>) -> Response {
    let text = RichText::new(text.into());
    let text = if selected {
        text.color(Color32::BLACK).strong()
    } else {
        text
    };
    ui.selectable_label(selected, text)
}

/// The text color of a calculator key.
pub fn key_color(kind: KeyKind) -> Color32 {
    match kind {
        KeyKind::Digit => Color32::WHITE,
        KeyKind::Operator => YELLOW,
        KeyKind::Function => ACCENT,
        KeyKind::Constant => MAGENTA,
        KeyKind::Clear => RED,
        KeyKind::Equals => GREEN,
    }
}

/// A calculator key of a fixed size, with a background color.
pub fn key(ui: &mut Ui, label: &str, color: Color32, size: egui::Vec2) -> Response {
    let (rect, r) = ui.allocate_exact_size(size, Sense::click());
    let bg = if r.is_pointer_button_down_on() {
        ACCENT
    } else if r.hovered() {
        Color32::from_rgb(0x3a, 0x42, 0x4c)
    } else {
        KEY_BG
    };
    let fg = if r.is_pointer_button_down_on() {
        Color32::BLACK
    } else {
        color
    };
    ui.painter().rect_filled(rect, 0.0, bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::monospace(16.0),
        fg,
    );
    r
}

/// Select all the text of a text field: the next key replaces it.
pub fn select_all(ui: &Ui, r: &Response, text: &str) {
    if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), r.id) {
        state
            .cursor
            .set_char_range(Some(egui::text_selection::CCursorRange::select_all_str(
                text,
            )));
        state.store(ui.ctx(), r.id);
    }
}
