//! Same orange-on-black look as the DM app, tuned for fingers.

use egui::{Color32, Stroke};

pub const ORANGE: Color32 = Color32::from_rgb(255, 138, 0);
pub const ORANGE_DIM: Color32 = Color32::from_rgb(176, 92, 0);
pub const ORANGE_FAINT: Color32 = Color32::from_rgb(64, 40, 16);
pub const BG: Color32 = Color32::from_rgb(12, 12, 12);
pub const PANEL: Color32 = Color32::from_rgb(20, 20, 20);
pub const CARD: Color32 = Color32::from_rgb(30, 30, 30);
pub const CARD_HI: Color32 = Color32::from_rgb(42, 42, 42);
pub const LINE: Color32 = Color32::from_rgb(58, 58, 58);
pub const TEXT: Color32 = Color32::from_rgb(232, 226, 214);
pub const MUTED: Color32 = Color32::from_rgb(150, 144, 134);
pub const GOOD: Color32 = Color32::from_rgb(120, 190, 90);
pub const BAD: Color32 = Color32::from_rgb(230, 80, 70);
pub const PC_BLUE: Color32 = Color32::from_rgb(90, 150, 240);

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = CARD;
    v.extreme_bg_color = BG;
    v.faint_bg_color = Color32::from_rgb(26, 26, 26);
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ORANGE;
    v.warn_fg_color = ORANGE;
    v.error_fg_color = BAD;
    v.selection.bg_fill = ORANGE_DIM;
    v.selection.stroke = Stroke::new(1.0, ORANGE);
    v.window_stroke = Stroke::new(1.0, ORANGE_DIM);
    v.widgets.noninteractive.bg_fill = CARD;
    v.widgets.noninteractive.weak_bg_fill = CARD;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.bg_fill = CARD_HI;
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.hovered.bg_fill = ORANGE_FAINT;
    v.widgets.hovered.weak_bg_fill = ORANGE_FAINT;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ORANGE);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    v.widgets.active.bg_fill = ORANGE_DIM;
    v.widgets.active.weak_bg_fill = ORANGE_DIM;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ORANGE);
    v.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
    v.widgets.open.bg_fill = ORANGE_FAINT;
    v.widgets.open.weak_bg_fill = ORANGE_FAINT;
    v.slider_trailing_fill = true;
    v.striped = true;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.interact_size = egui::vec2(44.0, 36.0);
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles.insert(TextStyle::Heading, FontId::new(24.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(16.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(16.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace));
    ctx.set_style(style);
}

pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text.to_uppercase()).color(ORANGE).strong().size(13.0));
        // a (i) with a plain explanation when the heading is a game term
        if let Some(t) = dnd_core::glossary::explain_heading(text) {
            dnd_mapview::help::info(ui, t);
        }
    });
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment([egui::pos2(r.left(), r.top()), egui::pos2(r.right(), r.top())], Stroke::new(1.0, ORANGE_DIM));
    ui.add_space(4.0);
}

/// A section heading with the (i) for a given glossary term.
pub fn section_help(ui: &mut egui::Ui, text: &str, key: &str) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text.to_uppercase()).color(ORANGE).strong().size(13.0));
        dnd_mapview::help::term(ui, key);
    });
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment([egui::pos2(r.left(), r.top()), egui::pos2(r.right(), r.top())], Stroke::new(1.0, ORANGE_DIM));
    ui.add_space(4.0);
}

/// A red outline around something the character doesn't qualify for.
pub fn red_outline(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_stroke(rect.expand(2.0), 6.0, Stroke::new(1.6, BAD), egui::StrokeKind::Outside);
}

pub fn card() -> egui::Frame {
    egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0, LINE)).corner_radius(8.0).inner_margin(10.0)
}

pub fn card_hi() -> egui::Frame {
    egui::Frame::new().fill(CARD_HI).stroke(Stroke::new(1.0, ORANGE_DIM)).corner_radius(8.0).inner_margin(10.0)
}

pub fn muted(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(MUTED)
}

pub fn accent(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ORANGE)
}

pub fn primary(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_owned()).color(Color32::BLACK).strong()).fill(ORANGE).min_size(egui::vec2(0.0, 40.0))
}

pub fn big_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_owned()).size(18.0)).min_size(egui::vec2(52.0, 44.0))
}

/// Stat tile. `w` is the tile width.
pub fn tile(ui: &mut egui::Ui, w: f32, label: &str, value: &str, sub: &str) -> egui::Response {
    let frame = card();
    let resp = frame
        .show(ui, |ui| {
            ui.set_width(w - 20.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(label.to_uppercase()).color(MUTED).size(10.5));
                ui.label(egui::RichText::new(value).color(ORANGE).strong().size(24.0));
                if !sub.is_empty() {
                    ui.label(egui::RichText::new(sub).color(MUTED).size(11.5));
                }
            });
        })
        .response;
    let r = resp.interact(egui::Sense::click());
    dnd_mapview::help::term_in_corner(ui, r.rect, label);
    r
}
