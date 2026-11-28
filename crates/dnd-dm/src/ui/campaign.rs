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
                changed |= ui.add(crate::ui::theme::line_edit(&mut w.campaign.name).desired_width(300.0)).changed();
                ui.label("In-game date/time");
                changed |= ui.add(crate::ui::theme::line_edit(&mut w.campaign.game_time).desired_width(220.0)).changed();
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
            ui.add(crate::ui::theme::line_edit(&mut app.campaign.save_as).desired_width(200.0).hint_text("folder name"));
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

        player_rules(app, ui);

        heading(ui, "New campaign");
        ui.horizontal(|ui| {
            ui.add(crate::ui::theme::line_edit(&mut app.campaign.new_name).desired_width(240.0).hint_text("campaign name"));
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
                if ui.button("folder").clicked() {
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
            if ui.button("Clear log").clicked() {
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

/// What players may do in this campaign. Saved with the campaign; set once,
/// not something to reach for in the middle of a session.
fn player_rules(app: &mut DmApp, ui: &mut egui::Ui) {
    heading(ui, "Rules for players");
    let mut w = app.world.write().unwrap();
    let mut changed = false;
    let st = &mut w.campaign.settings;
    let mut rule = |ui: &mut egui::Ui, on: &mut bool, label: &str, what: &str| {
        ui.horizontal(|ui| {
            changed |= ui.checkbox(on, label).changed();
            dnd_mapview::help::text(ui, what);
        });
    };
    rule(ui, &mut st.players_choose_character, "Players may pick their own character", "Off: a player's app waits until you give them a character on the Dashboard, and can only open that one.");
    rule(ui, &mut st.players_move_tokens, "Players move their own token", "On their turn in a fight, freely while exploring. Walls and doors are checked by the DM app.");
    rule(ui, &mut st.players_apply_damage, "Players' attacks change hit points", "On: an attack or spell from a player's app is rolled by the DM app against the target and the damage or healing is applied. Off: it is only rolled and you apply it.");
    rule(ui, &mut st.enforce_range, "Enforce reach, range and movement", "On: an attack or spell must be within its reach or range with no wall in the way (long range and an enemy next to an archer give disadvantage), and a turn's walking is limited by speed. Off: you referee distances yourself.");
    rule(ui, &mut st.show_monster_hp_to_players, "Players see monster HP", "On: players' apps show creatures' hit points. Off: they only see that a creature is hurt or down.");
    rule(ui, &mut st.allow_rule_breaks, "Players may break the rules", "When ticked, every player may pick options their character doesn't qualify for (a feat above their level, a spell they can't cast yet). They still show in red. To allow it for just one character, use the tick on that character's Build tab.");
    ui.horizontal(|ui| {
        let mut lvl = st.player_start_level.clamp(1, 20);
        if ui.add(egui::DragValue::new(&mut lvl).range(1..=20)).changed() {
            st.player_start_level = lvl;
            changed = true;
        }
        ui.label("Level that player-made characters start at");
    });
    if changed {
        w.touch();
    }
}
