//! Same orange-on-black look as the DM app, tuned for fingers.

use egui::{Color32, Stroke};

// One system for every screen: a near-black page, cards one step lighter,
// controls one step lighter again; one corner radius for controls and one
// for cards; every control at least 44 points tall (a fingertip).

pub const ORANGE: Color32 = Color32::from_rgb(255, 149, 10);
pub const ORANGE_DIM: Color32 = Color32::from_rgb(176, 92, 0);
pub const ORANGE_FAINT: Color32 = Color32::from_rgb(58, 36, 12);
/// Fill of a selected chip or segment: dark enough for white text (6:1).
pub const SELECT: Color32 = Color32::from_rgb(150, 78, 0);
pub const BG: Color32 = Color32::from_rgb(0, 0, 0);
pub const PANEL: Color32 = Color32::from_rgb(0, 0, 0);
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

/// Corner radius of buttons, chips, text boxes.
pub const R_CTRL: f32 = 12.0;
/// Corner radius of cards and tiles.
pub const R_CARD: f32 = 16.0;
/// Height of every button, chip and text box.
pub const H_CTRL: f32 = 44.0;

pub fn apply(ctx: &egui::Context) {
    dnd_mapview::face::install(ctx);

    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = CARD_HI;
    v.extreme_bg_color = CARD;
    v.faint_bg_color = Color32::from_rgb(20, 20, 22);
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
    w.active.bg_fill = FILL_PRESSED;
    w.active.weak_bg_fill = FILL_PRESSED;
    w.open.bg_fill = FILL_HOVER;
    w.open.weak_bg_fill = FILL_HOVER;
    v.slider_trailing_fill = true;
    v.striped = false;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 11.0);
    style.spacing.interact_size = egui::vec2(H_CTRL, H_CTRL);
    // check boxes a finger can hit
    style.spacing.icon_width = 24.0;
    style.spacing.icon_width_inner = 13.0;
    style.spacing.icon_spacing = 10.0;
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles.insert(TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(16.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(16.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace));
    ctx.set_style(style);
}

/// A group heading: small grey capitals with room above, no rule.
fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text.to_uppercase()).color(MUTED).size(12.5).extra_letter_spacing(0.6));
}

pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        heading(ui, text);
        // a (i) with a plain explanation when the heading is a game term
        if let Some(t) = dnd_core::glossary::explain_heading(text) {
            dnd_mapview::help::info(ui, t);
        }
    });
}

/// A section heading with the (i) for a given glossary term.
pub fn section_help(ui: &mut egui::Ui, text: &str, key: &str) {
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        heading(ui, text);
        dnd_mapview::help::term(ui, key);
    });
}

/// A red outline around something the character doesn't qualify for.
pub fn red_outline(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_stroke(rect.expand(2.0), 6.0, Stroke::new(1.6, BAD), egui::StrokeKind::Outside);
}

/// A page that scrolls up and down, with its scroll bar on the edge of the
/// screen. The panel around it must have no right margin: `right_pad` is the
/// gap kept between the content and the screen edge, and the bar is drawn
/// inside that gap instead of taking width of its own.
pub fn page_scroll<R>(ui: &mut egui::Ui, area: egui::ScrollArea, right_pad: i8, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let normal = ui.spacing().scroll;
    ui.spacing_mut().scroll = egui::style::ScrollStyle {
        floating: true,
        floating_width: normal.bar_width,
        floating_allocated_width: 0.0,
        dormant_background_opacity: normal.active_background_opacity,
        dormant_handle_opacity: normal.active_handle_opacity,
        ..normal
    };
    let out = area.show(ui, |ui| {
        ui.spacing_mut().scroll = normal; // scroll areas inside the page keep the usual bars
        egui::Frame::new().inner_margin(egui::Margin { left: 0, right: right_pad, top: 0, bottom: 0 }).show(ui, add).inner
    });
    ui.spacing_mut().scroll = normal;
    out.inner
}

pub fn card() -> egui::Frame {
    egui::Frame::new().fill(CARD).corner_radius(R_CARD).inner_margin(14.0)
}

/// A card that wants attention: the same card with an orange hairline.
pub fn card_hi() -> egui::Frame {
    egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, ORANGE_DIM)).corner_radius(R_CARD).inner_margin(14.0)
}

pub fn muted(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(MUTED)
}

pub fn accent(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ORANGE)
}

/// The one action a screen is about: filled orange.
pub fn primary(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_owned()).color(Color32::BLACK)).fill(ORANGE).stroke(Stroke::NONE).min_size(egui::vec2(0.0, H_CTRL))
}

/// A square-ish button for one or two characters (+5, d20).
pub fn big_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_owned()).size(17.0)).min_size(egui::vec2(56.0, H_CTRL))
}

/// A choice that is on or off: the same shape as a button, filled when on.
pub fn chip(ui: &mut egui::Ui, on: bool, text: impl Into<egui::WidgetText>) -> egui::Response {
    // in the row's own layout (a nested one would not wrap with its neighbours)
    let before = ui.visuals().override_text_color;
    if on {
        ui.visuals_mut().override_text_color = Some(Color32::WHITE);
    }
    let b = egui::Button::new(text).min_size(egui::vec2(0.0, H_CTRL));
    let r = ui.add(if on { b.fill(SELECT).stroke(Stroke::NONE) } else { b });
    ui.visuals_mut().override_text_color = before;
    r
}

/// Keeps the buttons of a row off its words. A row has words on the left
/// and buttons on the right; the words may only use the width the buttons
/// leave. The buttons' width is measured as they are drawn and used from the
/// next frame on (a guess the first time).
///
/// ```ignore
/// ui.horizontal(|ui| {
///     let room = theme::Room::new(ui, ("spell", &id));
///     ui.vertical(|ui| { room.limit(ui); /* words */ });
///     ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { /* buttons */ room.note(ui); });
/// });
/// ```
#[derive(Clone, Copy)]
pub struct Room {
    id: egui::Id,
    text: f32,
}

impl Room {
    pub fn new(ui: &egui::Ui, key: impl std::hash::Hash) -> Self {
        let id = egui::Id::new(("room", key));
        let buttons: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(150.0);
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

/// A one-line text box of the same height and shape as a button.
pub fn line_edit(text: &mut dyn egui::TextBuffer) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text).min_size(egui::vec2(0.0, H_CTRL)).vertical_align(egui::Align::Center).margin(egui::Margin::symmetric(12, 8))
}

/// A chip bound to a flag.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool, text: impl Into<egui::WidgetText>) -> egui::Response {
    let mut r = chip(ui, *on, text);
    if r.clicked() {
        *on = !*on;
        r.mark_changed();
    }
    r
}

/// One of a few, side by side in one rounded bar that fills the width
/// left in the row. Returns the segment that was tapped.
pub fn segmented(ui: &mut egui::Ui, labels: &[&str], selected: usize) -> Option<usize> {
    let mut picked = None;
    let width = ui.available_width();
    egui::Frame::new().fill(CARD_HI).corner_radius(R_CTRL).inner_margin(3.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.spacing_mut().button_padding.x = 2.0;
            let n = labels.len().max(1) as f32;
            let w = ((width - 6.0 - 2.0 * (n - 1.0)) / n).floor().max(24.0);
            for (i, label) in labels.iter().enumerate() {
                let on = i == selected;
                let text = egui::RichText::new(*label).size(15.0).color(if on { Color32::WHITE } else { TEXT });
                let b = egui::Button::new(text).fill(if on { SELECT } else { Color32::TRANSPARENT }).stroke(Stroke::NONE).corner_radius(R_CTRL - 3.0);
                if ui.add_sized([w, H_CTRL - 6.0], b).clicked() && !on {
                    picked = Some(i);
                }
            }
        });
    });
    picked
}

/// Stat tile. `w` is the tile width. Every tile is the same height, with
/// or without the small line under the number.
pub fn tile(ui: &mut egui::Ui, w: f32, label: &str, value: &str, sub: &str) -> egui::Response {
    let resp = egui::Frame::new()
        .fill(CARD)
        .corner_radius(R_CARD)
        .inner_margin(egui::Margin::symmetric(8, 10))
        .show(ui, |ui| {
            ui.set_width(w - 16.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(label.to_uppercase()).color(MUTED).size(11.0).extra_letter_spacing(0.5));
                ui.label(egui::RichText::new(value).color(ORANGE).size(24.0));
                ui.label(egui::RichText::new(if sub.is_empty() { " " } else { sub }).color(MUTED).size(12.0));
            });
        })
        .response;
    let r = resp.interact(egui::Sense::click());
    dnd_mapview::help::term_in_corner(ui, r.rect, label);
    r
}
