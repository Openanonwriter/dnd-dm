//! The look of the DM app.

use eframe::egui::{self, Color32, Stroke};

// The same system as the player app, at desk sizes: a near-black page,
// cards one step lighter, controls one step lighter again; one corner radius
// for controls and one for cards; every control the same height.

pub const ORANGE: Color32 = Color32::from_rgb(255, 149, 10);
pub const ORANGE_DIM: Color32 = Color32::from_rgb(176, 92, 0);
pub const ORANGE_FAINT: Color32 = Color32::from_rgb(58, 36, 12);
/// Fill of a selected chip or segment: dark enough for white text (6:1).
pub const SELECT: Color32 = Color32::from_rgb(150, 78, 0);
pub const BG: Color32 = Color32::from_rgb(0, 0, 0);
pub const PANEL: Color32 = Color32::from_rgb(10, 10, 11);
pub const CARD: Color32 = Color32::from_rgb(28, 28, 30);
/// Fill of buttons, chips and other controls.
pub const CARD_HI: Color32 = Color32::from_rgb(44, 44, 46);
pub const FILL_HOVER: Color32 = Color32::from_rgb(58, 58, 60);
pub const FILL_PRESSED: Color32 = Color32::from_rgb(72, 72, 74);
pub const LINE: Color32 = Color32::from_rgb(56, 56, 58);
/// Hairline around controls and text boxes.
pub const EDGE: Color32 = Color32::from_rgb(72, 72, 76);
pub const TEXT: Color32 = Color32::from_rgb(245, 245, 247);
pub const MUTED: Color32 = Color32::from_rgb(152, 152, 159);
pub const GOOD: Color32 = Color32::from_rgb(50, 215, 75);
pub const BAD: Color32 = Color32::from_rgb(255, 69, 58);
pub const PC_BLUE: Color32 = Color32::from_rgb(100, 170, 255);
pub const CONDITION: Color32 = Color32::from_rgb(200, 140, 255);

/// Corner radius of buttons, chips, text boxes.
pub const R_CTRL: f32 = 8.0;
/// Corner radius of cards and tiles.
pub const R_CARD: f32 = 12.0;
/// Height of every button, chip, combo box and text box.
pub const H_CTRL: f32 = 30.0;

pub fn apply(ctx: &egui::Context) {
    dnd_mapview::face::install(ctx);

    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = PANEL;
    v.window_fill = CARD_HI;
    v.extreme_bg_color = CARD;
    v.faint_bg_color = Color32::from_rgb(20, 20, 22);
    v.code_bg_color = CARD;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ORANGE;
    v.warn_fg_color = ORANGE;
    v.error_fg_color = BAD;
    v.selection.bg_fill = SELECT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.window_stroke = Stroke::new(1.0_f32, EDGE);
    v.window_corner_radius = R_CARD.into();
    v.menu_corner_radius = R_CARD.into();
    let hair = Stroke::new(1.0_f32, EDGE);
    let w = &mut v.widgets;
    for state in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        state.corner_radius = R_CTRL.into();
        state.fg_stroke = Stroke::new(1.0_f32, TEXT);
        state.bg_stroke = hair;
        state.expansion = 0.0;
    }
    w.noninteractive.bg_fill = CARD;
    w.noninteractive.weak_bg_fill = CARD;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
    w.inactive.bg_fill = CARD_HI;
    w.inactive.weak_bg_fill = CARD_HI;
    w.hovered.bg_fill = FILL_HOVER;
    w.hovered.weak_bg_fill = FILL_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ORANGE_DIM);
    w.active.bg_fill = FILL_PRESSED;
    w.active.weak_bg_fill = FILL_PRESSED;
    w.open.bg_fill = FILL_HOVER;
    w.open.weak_bg_fill = FILL_HOVER;
    v.slider_trailing_fill = true;
    v.striped = true;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size = egui::vec2(40.0, H_CTRL);
    style.spacing.icon_width = 18.0;
    style.spacing.icon_width_inner = 10.0;
    style.spacing.icon_spacing = 8.0;
    style.spacing.combo_height = 320.0;
    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles.insert(TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));
    ctx.set_style(style);
}

/// A group heading: small grey capitals with room above, no rule.
pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text.to_uppercase()).color(MUTED).size(11.5).extra_letter_spacing(0.6));
        // a (i) with a plain explanation when the heading is a game term
        if let Some(t) = dnd_core::glossary::explain_heading(text) {
            dnd_mapview::help::info(ui, t);
        }
    });
}

/// A card frame.
pub fn card() -> egui::Frame {
    egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, CARD)).corner_radius(R_CARD).inner_margin(10.0)
}

/// A card that wants attention: the same card with an orange hairline.
pub fn card_hi() -> egui::Frame {
    egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, ORANGE_DIM)).corner_radius(R_CARD).inner_margin(10.0)
}

/// A choice that is on or off: the same shape as a button, filled when on.
pub fn chip(ui: &mut egui::Ui, on: bool, text: impl Into<egui::WidgetText>) -> egui::Response {
    // in the row's own layout (a nested one would not wrap with its neighbours)
    let before = ui.visuals().override_text_color;
    if on {
        ui.visuals_mut().override_text_color = Some(Color32::WHITE);
    }
    let b = egui::Button::new(text);
    let r = ui.add(if on { b.fill(SELECT).stroke(Stroke::NONE) } else { b });
    ui.visuals_mut().override_text_color = before;
    r
}

/// One of several choices as chips: sets `current` to `value` when tapped.
pub fn choice<V: PartialEq>(ui: &mut egui::Ui, current: &mut V, value: V, text: impl Into<egui::WidgetText>) -> egui::Response {
    let mut r = chip(ui, *current == value, text);
    if r.clicked() && *current != value {
        *current = value;
        r.mark_changed();
    }
    r
}

/// A one-line text box of the same height and shape as a button.
pub fn line_edit(text: &mut dyn egui::TextBuffer) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text).min_size(egui::vec2(0.0, H_CTRL)).vertical_align(egui::Align::Center).margin(egui::Margin::symmetric(8, 4))
}

/// Keeps the buttons of a row off its words: the words may only use the
/// width the buttons leave (measured as they are drawn, used from the next
/// frame on). See the player app's `theme::Room`.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct Room {
    id: egui::Id,
    text: f32,
}

#[allow(dead_code)]
impl Room {
    pub fn new(ui: &egui::Ui, key: impl std::hash::Hash) -> Self {
        let id = egui::Id::new(("room", key));
        let buttons: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(120.0);
        let text = (ui.available_width() - buttons - ui.spacing().item_spacing.x).max(60.0);
        Self { id, text }
    }

    /// Call first in the words' column: lines wrap inside the room.
    pub fn limit(&self, ui: &mut egui::Ui) {
        ui.set_max_width(self.text);
    }

    /// Call last in the buttons' layout: remembers how wide they are.
    pub fn note(&self, ui: &egui::Ui) {
        let w = ui.min_rect().width();
        let before: Option<f32> = ui.ctx().data(|d| d.get_temp(self.id));
        if before.map(|b| (b - w).abs() > 0.5).unwrap_or(true) {
            ui.ctx().data_mut(|d| d.insert_temp(self.id, w));
            ui.ctx().request_repaint();
        }
    }
}

/// Big number tile.
pub fn stat(ui: &mut egui::Ui, label: &str, value: &str, sub: &str) {
    let label = egui::RichText::new(label.to_uppercase()).color(MUTED).size(10.5);
    let value = egui::RichText::new(value).color(ORANGE).size(22.0);
    // every tile has the small line (blank when there is nothing to say), so a row of tiles is one height
    let sub = Some(egui::RichText::new(if sub.is_empty() { " " } else { sub }).color(MUTED).size(11.0));
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
    egui::Button::new(egui::RichText::new(text.to_owned()).color(Color32::BLACK)).fill(ORANGE).stroke(Stroke::NONE)
}
