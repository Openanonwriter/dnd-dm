//! Content library: browse everything, edit with forms, autosave to TOML.
//!
//! Base pack files are treated as read-only; editing one writes an override
//! with the same id into the homebrew pack. Homebrew files are edited in
//! place (single-entry files are overwritten, multi-entry files are patched).

use crate::app::DmApp;
use crate::ui::editors;
use crate::ui::theme::{self, section};
use dnd_core::library::entry_to_toml;
use dnd_core::{Entry, Kind};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SAVE_DELAY: Duration = Duration::from_millis(900);

#[derive(Default)]
pub struct ContentState {
    pub kind: Option<Kind>,
    pub filter: String,
    pub selected: Option<String>,
    /// Working copy being edited.
    pub draft: Option<Entry>,
    /// Last version written to disk.
    pub saved: Option<Entry>,
    /// Previous frame, to detect edits.
    pub prev: Option<Entry>,
    pub last_edit: Option<Instant>,
    pub error: String,
    pub info: String,
    pub new_name: String,
    pub new_id: String,
    pub show_toml: bool,
    pub confirm_delete: bool,
}

fn homebrew_dir(app: &DmApp) -> PathBuf {
    app.paths.content_dirs.last().cloned().unwrap_or_else(|| app.paths.root.join("homebrew"))
}

fn in_homebrew(app: &DmApp, p: &Path) -> bool {
    p.starts_with(homebrew_dir(app))
}

/// Where an edit of this entry gets written, and whether it is an override of base content.
fn save_target(app: &DmApp, kind: Kind, id: &str) -> (PathBuf, bool) {
    let w = app.world.read().unwrap();
    if let Some(src) = w.library.source_of(kind, id) {
        if in_homebrew(app, src) {
            return (src.clone(), false);
        }
    }
    (homebrew_dir(app).join(kind.folder()).join(format!("{id}.toml")), true)
}

/// Write `entry` to `path`. Multi-entry files are patched in place.
fn write_entry(path: &Path, entry: &Entry) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if stem == entry.id() || !path.exists() {
        std::fs::write(path, entry_to_toml(entry)?)?;
        return Ok(());
    }
    // multi-entry file: replace the matching element
    let text = std::fs::read_to_string(path)?;
    let mut table: toml::Table = toml::from_str(&text)?;
    let key = if table.contains_key("entries") { "entries".to_string() } else { entry.kind().folder().to_string() };
    let value: toml::Value = toml::from_str(&entry_to_toml(entry)?)?;
    let mut replaced = false;
    if let Some(arr) = table.get_mut(&key).and_then(|v| v.as_array_mut()) {
        for v in arr.iter_mut() {
            if v.get("id").and_then(|i| i.as_str()) == Some(entry.id()) {
                *v = value.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            arr.push(value);
        }
    } else {
        // not a multi-entry file after all; overwrite
        std::fs::write(path, entry_to_toml(entry)?)?;
        return Ok(());
    }
    std::fs::write(path, toml::to_string_pretty(&table)?)?;
    Ok(())
}

fn remove_entry(path: &Path, entry: &Entry) -> anyhow::Result<()> {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if stem == entry.id() {
        std::fs::remove_file(path)?;
        return Ok(());
    }
    let text = std::fs::read_to_string(path)?;
    let mut table: toml::Table = toml::from_str(&text)?;
    let key = if table.contains_key("entries") { "entries".to_string() } else { entry.kind().folder().to_string() };
    if let Some(arr) = table.get_mut(&key).and_then(|v| v.as_array_mut()) {
        arr.retain(|v| v.get("id").and_then(|i| i.as_str()) != Some(entry.id()));
    }
    std::fs::write(path, toml::to_string_pretty(&table)?)?;
    Ok(())
}

/// Save the draft now if it differs from what is on disk.
pub fn flush(app: &mut DmApp) {
    let (Some(draft), Some(saved)) = (app.content.draft.clone(), app.content.saved.clone()) else { return };
    if draft == saved {
        return;
    }
    let (path, override_base) = save_target(app, draft.kind(), draft.id());
    match write_entry(&path, &draft) {
        Ok(()) => {
            app.world.write().unwrap().reload_content();
            app.content.saved = Some(draft);
            app.content.error.clear();
            app.content.info = if override_base {
                format!("Saved as homebrew override: {}", path.display())
            } else {
                format!("Saved: {}", path.display())
            };
        }
        Err(e) => app.content.error = format!("Save failed: {e:#}"),
    }
}

fn select(app: &mut DmApp, kind: Kind, id: &str) {
    flush(app);
    let e = app.world.read().unwrap().library.get(kind, id);
    app.content.selected = Some(id.to_string());
    app.content.draft = e.clone();
    app.content.saved = e.clone();
    app.content.prev = e;
    app.content.last_edit = None;
    app.content.error.clear();
    app.content.info.clear();
    app.content.confirm_delete = false;
}

pub fn show(app: &mut DmApp, ui: &mut egui::Ui) {
    let kind = app.content.kind.unwrap_or(Kind::Species);
    app.content.kind = Some(kind);

    // kind tabs
    ui.horizontal_wrapped(|ui| {
        for k in Kind::ALL {
            let n = app.world.read().unwrap().library.count(k);
            let selected = kind == k;
            let text = egui::RichText::new(format!("{}  {n}", k.label())).color(if selected { egui::Color32::BLACK } else { theme::TEXT });
            if ui.add(egui::Button::new(text).fill(if selected { theme::ORANGE } else { theme::CARD_HI })).clicked() && !selected {
                flush(app);
                app.content.kind = Some(k);
                app.content.selected = None;
                app.content.draft = None;
                app.content.saved = None;
                app.content.prev = None;
            }
        }
    });
    let kind = app.content.kind.unwrap();

    // autosave of the draft
    if let (Some(d), Some(p)) = (&app.content.draft, &app.content.prev) {
        if d != p {
            app.content.last_edit = Some(Instant::now());
            app.content.prev = app.content.draft.clone();
        }
    }
    if let Some(t) = app.content.last_edit {
        if t.elapsed() >= SAVE_DELAY {
            flush(app);
            app.content.last_edit = None;
        }
    }

    egui::SidePanel::left("content_list").resizable(true).default_width(270.0).frame(egui::Frame::new().fill(theme::BG).inner_margin(8.0)).show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(theme::accent("🔍"));
            ui.add(egui::TextEdit::singleline(&mut app.content.filter).desired_width(f32::INFINITY).hint_text("filter by name or id"));
        });
        let (list, hb): (Vec<(String, String)>, Vec<bool>) = {
            let w = app.world.read().unwrap();
            let list = w.library.list(kind);
            let hb = list.iter().map(|(id, _)| w.library.source_of(kind, id).map(|p| in_homebrew(app, p)).unwrap_or(false)).collect();
            (list, hb)
        };
        let filter = app.content.filter.to_lowercase();
        let mut to_select: Option<String> = None;
        egui::ScrollArea::vertical().max_height(ui.available_height() - 150.0).show(ui, |ui| {
            for ((id, name), is_hb) in list.iter().zip(hb) {
                if !(filter.is_empty() || name.to_lowercase().contains(&filter) || id.contains(&filter)) {
                    continue;
                }
                let sel = app.content.selected.as_deref() == Some(id.as_str());
                ui.horizontal(|ui| {
                    if ui.selectable_label(sel, name).on_hover_text(id).clicked() {
                        to_select = Some(id.clone());
                    }
                    if is_hb {
                        ui.label(theme::accent("⏺").small()).on_hover_text("homebrew");
                    }
                });
            }
        });
        if let Some(id) = to_select {
            select(app, kind, &id);
        }
        ui.separator();
        let one = match kind { Kind::Species => "species", Kind::Class => "class", Kind::Subclass => "subclass", Kind::Background => "background", Kind::Item => "item", Kind::Feat => "feat", Kind::Spell => "spell", Kind::Creature => "creature" };
        section(ui, &format!("New {one}"));
        ui.add(egui::TextEdit::singleline(&mut app.content.new_name).hint_text("Name").desired_width(f32::INFINITY));
        ui.add(egui::TextEdit::singleline(&mut app.content.new_id).hint_text("id (auto from name)").desired_width(f32::INFINITY));
        if ui.add(theme::primary("➕ Create")).clicked() && !app.content.new_name.trim().is_empty() {
            let name = app.content.new_name.trim().to_string();
            let id = if app.content.new_id.trim().is_empty() { dnd_core::Character::slug(&name) } else { dnd_core::Character::slug(app.content.new_id.trim()) };
            if app.world.read().unwrap().library.get(kind, &id).is_some() {
                app.content.error = format!("id '{id}' already exists — pick another id");
            } else {
                let entry = editors::blank(kind, &id, &name);
                let path = homebrew_dir(app).join(kind.folder()).join(format!("{id}.toml"));
                match write_entry(&path, &entry) {
                    Ok(()) => {
                        app.world.write().unwrap().reload_content();
                        select(app, kind, &id);
                        app.content.new_name.clear();
                        app.content.new_id.clear();
                        app.content.info = format!("Created {}", path.display());
                    }
                    Err(e) => app.content.error = e.to_string(),
                }
            }
        }
    });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        let Some(id) = app.content.selected.clone() else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Pick an entry on the left, or create a new one.").size(16.0));
                ui.label(theme::muted("Every change is saved to a TOML file automatically. Base content is never modified: your edits become overrides in the homebrew pack."));
            });
            return;
        };
        if app.content.draft.is_none() {
            select(app, kind, &id);
        }
        let (src, is_hb) = {
            let w = app.world.read().unwrap();
            let src = w.library.source_of(kind, &id).cloned();
            let is_hb = src.as_ref().map(|p| in_homebrew(app, p)).unwrap_or(false);
            (src, is_hb)
        };

        ui.horizontal(|ui| {
            let name = app.content.draft.as_ref().map(|e| e.name().to_string()).unwrap_or_default();
            ui.label(egui::RichText::new(name).strong().size(20.0).color(theme::ORANGE));
            ui.label(theme::muted(format!("id: {id}")));
            if is_hb {
                ui.label(theme::accent("homebrew").small());
            } else {
                ui.label(theme::muted("base pack (edits become a homebrew override)").small());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.content.confirm_delete {
                    if ui.add(egui::Button::new("Confirm delete").fill(theme::BAD)).clicked() {
                        if let (Some(p), Some(e)) = (src.clone(), app.content.draft.clone()) {
                            match remove_entry(&p, &e) {
                                Ok(()) => {
                                    app.world.write().unwrap().reload_content();
                                    app.content.selected = None;
                                    app.content.draft = None;
                                    app.content.saved = None;
                                    app.content.prev = None;
                                    app.content.confirm_delete = false;
                                    app.set_status(format!("Deleted {id}"), false);
                                }
                                Err(e) => app.content.error = e.to_string(),
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        app.content.confirm_delete = false;
                    }
                } else if is_hb && ui.button("🗑 Delete").clicked() {
                    app.content.confirm_delete = true;
                }
                if is_hb && src.is_some() && ui.button("Open file").clicked() {
                    let _ = open::that(src.as_ref().unwrap());
                }
                if ui.button("Print / HTML").clicked() {
                    let html = match &app.content.draft {
                        Some(Entry::Creature(c)) => dnd_core::export::creature_html(c),
                        Some(e) => generic_html(e),
                        None => String::new(),
                    };
                    let path = app.paths.exports_dir.join(format!("{}_{}.html", kind.folder(), id));
                    if std::fs::write(&path, html).is_ok() {
                        let _ = open::that(&path);
                    }
                }
                ui.checkbox(&mut app.content.show_toml, "TOML");
                if ui.button("Duplicate").clicked() {
                    if let Some(e) = app.content.draft.clone() {
                        let mut n = 2;
                        let base = e.id().to_string();
                        let mut new_id = format!("{base}_{n}");
                        while app.world.read().unwrap().library.get(kind, &new_id).is_some() {
                            n += 1;
                            new_id = format!("{base}_{n}");
                        }
                        let copy = with_id(e, &new_id);
                        let path = homebrew_dir(app).join(kind.folder()).join(format!("{new_id}.toml"));
                        if let Err(err) = write_entry(&path, &copy) {
                            app.content.error = err.to_string();
                        } else {
                            app.world.write().unwrap().reload_content();
                            select(app, kind, &new_id);
                        }
                    }
                }
            });
        });
        if !app.content.error.is_empty() {
            ui.colored_label(theme::BAD, &app.content.error);
        } else if !app.content.info.is_empty() {
            ui.label(theme::muted(&app.content.info).small());
        } else if app.content.last_edit.is_some() {
            ui.label(theme::accent("⏺ unsaved edits…").small());
        }
        ui.separator();

        let lib_snapshot = app.world.read().unwrap().library.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(draft) = app.content.draft.as_mut() {
                if app.content.show_toml {
                    let text = entry_to_toml(draft).unwrap_or_default();
                    ui.label(theme::muted("Read-only view of what gets written to disk. Edit the file directly with “Open file” if you prefer."));
                    ui.add(egui::TextEdit::multiline(&mut text.as_str()).code_editor().desired_width(f32::INFINITY));
                } else {
                    editors::entry_form(ui, draft, &lib_snapshot);
                }
            }
        });
    });
}

fn with_id(e: Entry, id: &str) -> Entry {
    let id = id.to_string();
    match e {
        Entry::Species(mut x) => {
            x.id = id;
            x.name = format!("{} (copy)", x.name);
            Entry::Species(x)
        }
        Entry::Class(mut x) => {
            x.id = id;
            x.name = format!("{} (copy)", x.name);
            Entry::Class(x)
        }
        Entry::Subclass(mut x) => {
            x.id = id;
            x.name = format!("{} (copy)", x.name);
            Entry::Subclass(x)
        }
        Entry::Background(mut x) => {
            x.id = id;
            x.name = format!("{} (copy)", x.name);
            Entry::Background(x)
        }
        Entry::Item(mut x) => {
            x.id = id;
            x.name = format!("{} (copy)", x.name);
            Entry::Item(x)
        }
        Entry::Feat(mut x) => {
            x.id = id;
            x.name = format!("{} (copy)", x.name);
            Entry::Feat(x)
        }
        Entry::Spell(mut x) => {
            x.id = id;
            x.name = format!("{} (copy)", x.name);
            Entry::Spell(x)
        }
        Entry::Creature(mut x) => {
            x.id = id;
            x.name = format!("{} (copy)", x.name);
            Entry::Creature(x)
        }
    }
}

fn generic_html(e: &Entry) -> String {
    let toml = entry_to_toml(e).unwrap_or_default();
    format!(
        "<!doctype html><html><head><meta charset=utf-8><title>{}</title><style>body{{font-family:Georgia,serif;margin:24px}}pre{{background:#f4f1ea;padding:12px;white-space:pre-wrap}}</style></head><body><h1>{}</h1><p>{}</p><pre>{}</pre></body></html>",
        e.name(),
        e.name(),
        e.description().replace('\n', "<br>"),
        toml.replace('<', "&lt;")
    )
}
