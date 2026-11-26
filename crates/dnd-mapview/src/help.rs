//! The small circled (i) "what's this?" icon both apps put next to game
//! terms for new players. Hovering shows the explanation; a click or tap
//! opens it in a popup that stays until you tap elsewhere (phones can't
//! hover). The icon is drawn from shapes, so it works with any font.

use egui::{vec2, Rect, Sense, Stroke};

/// A circled "i" in the flow of the layout, explaining `text`.
pub fn info(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let size = ui.text_style_height(&egui::TextStyle::Body).clamp(14.0, 18.0);
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    info_in_rect(ui, rect, ("info", rect.min.x as i32, rect.min.y as i32), text)
}

/// The glossary explanation of `key` behind an (i) icon. Draws nothing when
/// the glossary doesn't know the term.
pub fn term(ui: &mut egui::Ui, key: &str) {
    if let Some(t) = dnd_core::glossary::explain(key) {
        info(ui, t);
    }
}

/// A label followed by the (i) for a glossary term.
pub fn label_term(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>, key: &str) -> egui::Response {
    let r = ui.label(text);
    term(ui, key);
    r
}

/// An (i) icon painted inside `rect` (for corners of tiles and cards).
pub fn info_in_rect(ui: &mut egui::Ui, rect: Rect, salt: impl std::hash::Hash, text: &str) -> egui::Response {
    let id = ui.id().with(("help_icon", salt));
    let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(egui::CursorIcon::Help);
    let popup = id.with("popup");
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    let open = ui.memory(|m| m.is_popup_open(popup));
    if ui.is_rect_visible(rect) {
        let col = if resp.hovered() || open { ui.visuals().selection.bg_fill } else { ui.visuals().weak_text_color() };
        let c = rect.center();
        let r = rect.width().min(rect.height()) * 0.45;
        let p = ui.painter();
        p.circle_stroke(c, r, Stroke::new(1.3, col));
        p.circle_filled(c + vec2(0.0, -r * 0.45), r * 0.14, col);
        p.line_segment([c + vec2(0.0, -r * 0.15), c + vec2(0.0, r * 0.55)], Stroke::new((r * 0.24).max(1.2), col));
    }
    let screen = ui.ctx().screen_rect();
    let width = 340.0_f32.min(screen.width() - 24.0);
    let resp = if open {
        resp
    } else {
        // hover shows the start; a click opens the whole thing
        resp.on_hover_ui(|ui| {
            ui.set_max_width(width);
            ui.label(hover_text(text));
        })
    };
    #[allow(deprecated)]
    egui::popup::popup_below_widget(ui, popup, &resp, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
        ui.set_max_width(width);
        egui::ScrollArea::vertical().max_height((screen.height() * 0.6).max(160.0)).show(ui, |ui| {
            ui.label(text);
        });
    });
    resp
}

/// The hover version of a long explanation: the first lines, then a hint.
fn hover_text(text: &str) -> String {
    const MAX: usize = 520;
    if text.chars().count() <= MAX {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(MAX).collect();
    if let Some(i) = cut.rfind('\n') {
        cut.truncate(i);
    }
    format!("{cut}\n… click the (i) for the rest")
}

/// An (i) explaining one content entry (class, spell, item...). Draws
/// nothing if the library doesn't have it.
pub fn entry(ui: &mut egui::Ui, lib: &dnd_core::Library, kind: dnd_core::Kind, id: &str) {
    if let Some(t) = dnd_core::describe::describe(lib, kind, id) {
        if !t.is_empty() {
            info(ui, &t);
        }
    }
}

/// A text (i) for any explanation.
pub fn text(ui: &mut egui::Ui, t: &str) {
    if !t.trim().is_empty() {
        info(ui, t);
    }
}

/// Put the (i) for a glossary term in the top-right corner of `card`.
pub fn term_in_corner(ui: &mut egui::Ui, card: Rect, key: &str) {
    if let Some(t) = dnd_core::glossary::explain(key) {
        let s = 14.0;
        let rect = Rect::from_min_size(card.right_top() + vec2(-s - 4.0, 4.0), vec2(s, s));
        info_in_rect(ui, rect, ("corner", key), t);
    }
}
