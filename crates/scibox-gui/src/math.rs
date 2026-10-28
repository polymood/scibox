//! Typeset an equation with LaTeX (latex-rust, STIX Two Math font) and show it as an image.

use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions, Vec2};
use latex_rust::{Color, Dim, MathFont, PngOptions, latex_to_png};
use std::sync::OnceLock;

/// The last typeset equation: the LaTeX source, the color, the scale, and the texture.
#[derive(Default)]
pub struct Cache(Option<(String, Color32, f32, TextureHandle)>);

fn font() -> Option<&'static MathFont> {
    static FONT: OnceLock<Option<MathFont>> = OnceLock::new();
    FONT.get_or_init(|| MathFont::stix_two_math().ok()).as_ref()
}

impl Cache {
    /// Show the LaTeX source `tex` with a font size of `size` points. Return false if the typesetting fails.
    pub fn show(&mut self, ui: &mut egui::Ui, tex: &str, size: i64, color: Color32) -> bool {
        let ppp = ui.ctx().pixels_per_point();
        let fresh = !matches!(&self.0, Some((t, c, p, _)) if t == tex && *c == color && *p == ppp);
        if fresh {
            self.0 =
                render(ui.ctx(), tex, size, color, ppp).map(|t| (tex.to_string(), color, ppp, t));
        }
        match &self.0 {
            Some((_, _, _, texture)) => {
                let s = texture.size_vec2() / ppp;
                ui.image((texture.id(), Vec2::new(s.x, s.y)));
                true
            }
            None => false,
        }
    }
}

fn render(
    ctx: &egui::Context,
    tex: &str,
    size: i64,
    color: Color32,
    ppp: f32,
) -> Option<TextureHandle> {
    let mut options = PngOptions::new();
    options.font_size_pt = Dim::from_i64(size);
    // 72 dots per inch give 1 pixel for each point. Multiply by the pixels per point of the screen.
    options.dpi = Dim::from_i64((72.0 * ppp).round() as i64);
    options.color = Color::rgb(color.r(), color.g(), color.b());
    options.display = true;
    let png = latex_to_png(tex, font()?, &options).ok()?;
    let pixmap = tiny_skia::Pixmap::decode_png(&png).ok()?;
    let image = ColorImage::from_rgba_premultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        pixmap.data(),
    );
    Some(ctx.load_texture("equation", image, TextureOptions::LINEAR))
}
