pub mod campaign;
pub mod characters;
pub mod content;
pub mod dashboard;
pub mod editors;
pub mod encounter;
pub mod map;
pub mod theme;

use eframe::egui;

/// Combo box over (id, name) pairs. Returns true when the selection changed.
pub fn id_combo(ui: &mut egui::Ui, salt: impl std::hash::Hash, current: &mut String, options: &[(String, String)], none_label: &str) -> bool {
    id_combo_with(ui, salt, current, options, none_label, |_| None)
}

/// Combo box over library entries of one kind: hovering an option shows
/// what it is, and an (i) after the box explains the one picked.
pub fn entry_combo(ui: &mut egui::Ui, salt: impl std::hash::Hash, current: &mut String, options: &[(String, String)], none_label: &str, lib: &dnd_core::Library, kind: dnd_core::Kind) -> bool {
    let changed = id_combo_with(ui, salt, current, options, none_label, |id| lib.get(kind, id).map(|e| dnd_core::describe::short(e.description())).filter(|s| !s.is_empty()));
    if !current.is_empty() {
        dnd_mapview::help::entry(ui, lib, kind, current);
    }
    changed
}

/// Red used for broken rules.
pub const BAD: egui::Color32 = egui::Color32::from_rgb(235, 80, 70);

/// A red outline around something that breaks the rules.
pub fn red_outline(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_stroke(rect.expand(2.0), crate::ui::theme::R_CTRL, egui::Stroke::new(1.6, BAD), egui::StrokeKind::Outside);
}

/// Outline `r` in red with the reason on hover when `why` is set.
pub fn flag(ui: &egui::Ui, r: &egui::Response, why: Option<&str>) {
    if let Some(w) = why {
        red_outline(ui, r.rect);
        r.clone().on_hover_text(egui::RichText::new(w).color(BAD));
    }
}

/// [`entry_combo`] that knows the rules: options the character doesn't
/// qualify for are listed in red with the reason (the DM may still pick
/// them), and a rule-breaking pick gets a red outline.
#[allow(clippy::too_many_arguments)]
pub fn checked_combo(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash,
    current: &mut String,
    options: &[(String, String)],
    none_label: &str,
    lib: &dnd_core::Library,
    kind: dnd_core::Kind,
    bad: impl Fn(&str) -> Option<String>,
) -> bool {
    let (changed, resp) = combo_core(ui, salt, current, options, none_label, &|id| lib.get(kind, id).map(|e| dnd_core::describe::short(e.description())).filter(|s| !s.is_empty()), &bad);
    if !current.is_empty() {
        let why = bad(current);
        flag(ui, &resp, why.as_deref());
        dnd_mapview::help::entry(ui, lib, kind, current);
    }
    changed
}

/// [`id_combo`] with a hover tip for each option.
pub fn id_combo_with(ui: &mut egui::Ui, salt: impl std::hash::Hash, current: &mut String, options: &[(String, String)], none_label: &str, tip: impl Fn(&str) -> Option<String>) -> bool {
    combo_core(ui, salt, current, options, none_label, &tip, &|_| None).0
}

fn combo_core(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash,
    current: &mut String,
    options: &[(String, String)],
    none_label: &str,
    tip: &dyn Fn(&str) -> Option<String>,
    bad: &dyn Fn(&str) -> Option<String>,
) -> (bool, egui::Response) {
    let shown = options.iter().find(|(id, _)| id == current).map(|(_, n)| n.clone()).unwrap_or_else(|| {
        if current.is_empty() {
            none_label.to_string()
        } else {
            format!("{current} (missing)")
        }
    });
    let mut changed = false;
    // egui doesn't wrap a combo box by itself: in a wrapping row, start a
    // new line when it won't fit (narrow windows, Windows display scaling)
    if ui.layout().main_wrap && ui.available_size_before_wrap().x < 200.0 {
        ui.end_row();
    }
    let resp = egui::ComboBox::from_id_salt(salt).selected_text(shown).width(180.0).show_ui(ui, |ui| {
        if ui.selectable_label(current.is_empty(), none_label).clicked() {
            current.clear();
            changed = true;
        }
        for (id, name) in options {
            let why = bad(id);
            let text = match &why {
                Some(_) => egui::RichText::new(format!("{name}  (doesn't qualify)")).color(BAD),
                None => egui::RichText::new(name),
            };
            let mut r = ui.selectable_label(current == id, text);
            let hover = match (tip(id), why) {
                (Some(t), Some(w)) => Some(format!("{w}\n\n{t}")),
                (t, w) => t.or(w),
            };
            if let Some(t) = hover {
                r = r.on_hover_text(t);
            }
            if r.clicked() {
                *current = id.clone();
                changed = true;
            }
        }
    });
    (changed, resp.response)
}

pub fn bonus(v: i32) -> String {
    dnd_core::expr::fmt_bonus(v as i64)
}

pub fn heading(ui: &mut egui::Ui, text: &str) {
    theme::section(ui, text);
}

/// Comma separated text <-> Vec<String> editing helper.
pub fn list_edit(ui: &mut egui::Ui, label: &str, list: &mut Vec<String>) -> bool {
    let mut text = list.join(", ");
    ui.label(label);
    let r = ui.add(crate::ui::theme::line_edit(&mut text).desired_width(f32::INFINITY));
    if r.changed() {
        *list = text.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        true
    } else {
        false
    }
}
