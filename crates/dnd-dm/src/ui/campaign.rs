use crate::app::DmApp;
use crate::ui::heading;
use dnd_core::model::Campaign;
use dnd_core::save;
use eframe::egui;

#[derive(Default)]
pub struct CampaignState {
    pub save_as: String,
    pub new_name: String,
    pub confirm_new: bool,
}

/// Save to `saves/<slug of campaign name>/`. Used by autosave.
pub fn save_quiet(app: &mut DmApp) -> anyhow::Result<std::path::PathBuf> {
    let name = {
        let w = app.world.read().unwrap();
        dnd_core::Character::slug(&w.campaign.name)
    };
    if name.is_empty() {
        anyhow::bail!("campaign has no name");
    }
    if app.no_save {
        anyhow::bail!("this run doesn't save (picture-taking mode)");
    }
    let dir = app.campaign_dir.clone().unwrap_or_else(|| app.paths.saves_dir.join(&name));
    let mut w = app.world.write().unwrap();
    save::save_campaign(&mut w.campaign, &dir)?;
    Ok(dir)
}

pub fn save_current(app: &mut DmApp) {
    match save_quiet(app) {
        Ok(dir) => {
            let rev = app.world.read().unwrap().revision;
            app.saved_revision = rev;
            app.set_status(format!("Saved to {}", dir.display()), false);
        }
        Err(e) => app.set_status(format!("Save failed: {e:#}"), true),
    }
}

pub fn show(app: &mut DmApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        heading(ui, "Current campaign");
        {
            let mut w = app.world.write().unwrap();
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("Name");
                changed |= ui.add(egui::TextEdit::singleline(&mut w.campaign.name).desired_width(300.0)).changed();
                ui.label("In-game date/time");
                changed |= ui.add(egui::TextEdit::singleline(&mut w.campaign.game_time).desired_width(220.0)).changed();
            });
            ui.label("Description");
            changed |= ui.add(egui::TextEdit::multiline(&mut w.campaign.description).desired_rows(2).desired_width(f32::INFINITY)).changed();
            ui.label("DM notes");
            changed |= ui.add(egui::TextEdit::multiline(&mut w.campaign.notes).desired_rows(6).desired_width(f32::INFINITY)).changed();
            ui.label(egui::RichText::new(format!("Created {} · last saved {}", w.campaign.created, w.campaign.saved)).weak().small());
            if changed {
                w.touch();
            }
        }
        ui.horizontal(|ui| {
            if ui.button("💾 Save").clicked() {
                save_current(app);
            }
            ui.label("Save as:");
            ui.add(egui::TextEdit::singleline(&mut app.campaign.save_as).desired_width(200.0).hint_text("folder name"));
            if ui.button("Save copy").clicked() && !app.campaign.save_as.trim().is_empty() {
                let dir = app.paths.saves_dir.join(dnd_core::Character::slug(&app.campaign.save_as));
                let res = {
                    let mut w = app.world.write().unwrap();
                    save::save_campaign(&mut w.campaign, &dir)
                };
                match res {
                    Ok(()) => app.set_status(format!("Saved copy to {}", dir.display()), false),
                    Err(e) => app.set_status(format!("Save failed: {e:#}"), true),
                }
            }
            if ui.button("Open saves folder").clicked() {
                let _ = open::that(&app.paths.saves_dir);
            }
        });

        heading(ui, "New campaign");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut app.campaign.new_name).desired_width(240.0).hint_text("campaign name"));
            if !app.campaign.confirm_new {
                if ui.button("Start new campaign…").clicked() && !app.campaign.new_name.trim().is_empty() {
                    app.campaign.confirm_new = true;
                }
            } else {
                ui.colored_label(egui::Color32::from_rgb(220, 150, 60), "Unsaved changes to the current campaign will be lost.");
                if ui.button("Confirm").clicked() {
                    let name = app.campaign.new_name.trim().to_string();
                    let port = app.world.read().unwrap().campaign.settings.port;
                    {
                        let mut w = app.world.write().unwrap();
                        w.campaign = Campaign::new(name.clone());
                        w.campaign.settings.port = port;
                        w.touch();
                    }
                    app.characters.selected = None;
                    app.campaign.confirm_new = false;
                    app.campaign.new_name.clear();
                    app.set_status(format!("New campaign '{name}'"), false);
                }
                if ui.button("Cancel").clicked() {
                    app.campaign.confirm_new = false;
                }
            }
        });

        heading(ui, "Saved campaigns");
        let saves = save::list_campaigns(&app.paths.saves_dir);
        if saves.is_empty() {
            ui.label(egui::RichText::new("No saves yet.").weak());
        }
        egui::Grid::new("saves").striped(true).num_columns(4).show(ui, |ui| {
            for (name, path, saved) in saves {
                ui.label(egui::RichText::new(&name).strong());
                ui.label(egui::RichText::new(saved.split('T').next().unwrap_or("").to_string()).weak());
                if ui.button("Load").clicked() {
                    match save::load_campaign(&path) {
                        Ok(c) => {
                            let mut w = app.world.write().unwrap();
                            w.campaign = c;
                            w.touch();
                            drop(w);
                            app.characters.selected = None;
                            app.set_status(format!("Loaded '{name}'"), false);
                        }
                        Err(e) => app.set_status(format!("Load failed: {e:#}"), true),
                    }
                }
                if ui.small_button("folder").clicked() {
                    let _ = open::that(&path);
                }
                ui.end_row();
            }
        });

        heading(ui, "Log");
        let log: Vec<String> = {
            let w = app.world.read().unwrap();
            w.campaign.log.iter().rev().take(200).map(|l| format!("{}  {}", l.time, l.text)).collect()
        };
        ui.horizontal(|ui| {
            if ui.small_button("Clear log").clicked() {
                let mut w = app.world.write().unwrap();
                w.campaign.log.clear();
                w.touch();
            }
        });
        for l in log {
            ui.label(egui::RichText::new(l).monospace().small());
        }
    });
}
