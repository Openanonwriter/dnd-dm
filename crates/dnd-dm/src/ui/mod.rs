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
    let shown = options.iter().find(|(id, _)| id == current).map(|(_, n)| n.clone()).unwrap_or_else(|| {
        if current.is_empty() {
            none_label.to_string()
        } else {
            format!("{current} (missing)")
        }
    });
    let mut changed = false;
    egui::ComboBox::from_id_salt(salt).selected_text(shown).width(180.0).show_ui(ui, |ui| {
        if ui.selectable_label(current.is_empty(), none_label).clicked() {
            current.clear();
            changed = true;
        }
        for (id, name) in options {
            if ui.selectable_label(current == id, name).clicked() {
                *current = id.clone();
                changed = true;
            }
        }
    });
    changed
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
    let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY));
    if r.changed() {
        *list = text.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        true
    } else {
        false
    }
}
