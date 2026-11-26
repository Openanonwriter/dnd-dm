//! Orange-on-black look.

use eframe::egui::{self, Color32, Stroke};

pub const ORANGE: Color32 = Color32::from_rgb(255, 138, 0);
pub const ORANGE_DIM: Color32 = Color32::from_rgb(176, 92, 0);
pub const ORANGE_FAINT: Color32 = Color32::from_rgb(64, 40, 16);
pub const BG: Color32 = Color32::from_rgb(12, 12, 12);
pub const PANEL: Color32 = Color32::from_rgb(20, 20, 20);
pub const CARD: Color32 = Color32::from_rgb(30, 30, 30);
pub const CARD_HI: Color32 = Color32::from_rgb(40, 40, 40);
pub const LINE: Color32 = Color32::from_rgb(58, 58, 58);
pub const TEXT: Color32 = Color32::from_rgb(232, 226, 214);
pub const MUTED: Color32 = Color32::from_rgb(150, 144, 134);
pub const GOOD: Color32 = Color32::from_rgb(120, 190, 90);
pub const BAD: Color32 = Color32::from_rgb(230, 80, 70);
pub const PC_BLUE: Color32 = Color32::from_rgb(90, 150, 240);
pub const CONDITION: Color32 = Color32::from_rgb(200, 140, 255);

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = PANEL;
    v.window_fill = CARD;
    v.extreme_bg_color = BG;
    v.faint_bg_color = Color32::from_rgb(26, 26, 26);
    v.code_bg_color = BG;
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
    v.widgets.open.bg_stroke = Stroke::new(1.0, ORANGE_DIM);
    v.slider_trailing_fill = true;
    v.striped = true;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    style.spacing.interact_size.y = 24.0;
    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles.insert(TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(11.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));
    ctx.set_style(style);
}

/// Orange section title with a rule under it.
pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new(text.to_uppercase()).color(ORANGE).strong().size(12.5));
    let r = ui.available_rect_before_wrap();
    let y = r.top();
    ui.painter().line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], Stroke::new(1.0, ORANGE_DIM));
    ui.add_space(4.0);
}

/// A card frame.
pub fn card() -> egui::Frame {
    egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0, LINE)).corner_radius(6.0).inner_margin(10.0)
}

pub fn card_hi() -> egui::Frame {
    egui::Frame::new().fill(CARD_HI).stroke(Stroke::new(1.0, ORANGE_DIM)).corner_radius(6.0).inner_margin(10.0)
}

/// Big number tile.
pub fn stat(ui: &mut egui::Ui, label: &str, value: &str, sub: &str) {
    let label = egui::RichText::new(label.to_uppercase()).color(MUTED).size(10.5);
    let value = egui::RichText::new(value).color(ORANGE).strong().size(22.0);
    let sub = (!sub.is_empty()).then(|| egui::RichText::new(sub).color(MUTED).size(11.0));
    // A centred layout takes all the width it is offered, so the tile is
    // measured first and given exactly that much room; otherwise one tile
    // fills the row and pushes the rest of the page past the edge of the
    // window. Asking for the room up front also lets a row of tiles wrap.
    let measure = |t: &egui::RichText| egui::WidgetText::from(t.clone()).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Body).size();
    let sizes: Vec<egui::Vec2> = [Some(&label), Some(&value), sub.as_ref()].into_iter().flatten().map(measure).collect();
    let width = sizes.iter().map(|s| s.x).fold(110.0, f32::max).ceil();
    let height = sizes.iter().map(|s| s.y).sum::<f32>() + ui.spacing().item_spacing.y * (sizes.len() as f32 - 1.0);
    let margin = 2.0 * 10.0 + 2.0; // card padding and border, both sides
    ui.allocate_ui(egui::vec2(width + margin, height.ceil() + margin), |ui| {
        card().show(ui, |ui| {
            ui.set_width(width);
            ui.vertical_centered(|ui| {
                ui.label(label);
                ui.label(value);
                if let Some(sub) = sub {
                    ui.label(sub);
                }
            });
        });
    });
}

pub fn muted(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(MUTED)
}

pub fn accent(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ORANGE)
}

/// Primary (orange) button.
pub fn primary(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_owned()).color(Color32::BLACK).strong()).fill(ORANGE)
}
