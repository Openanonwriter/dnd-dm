//! Overview: every campaign, every character, server, content health.

use crate::app::{DmApp, View};
use crate::ui::theme::{self, section};
use dnd_core::save;
use eframe::egui;
use std::path::PathBuf;
use std::time::Instant;

pub struct CharRow {
    pub id: String,
    pub name: String,
    pub player: String,
    pub summary: String,
    pub hp: String,
    pub campaign: String,
    pub campaign_dir: PathBuf,
}

#[derive(Default)]
pub struct DashState {
    pub campaigns: Vec<(String, PathBuf, String, usize)>,
    pub characters: Vec<CharRow>,
    pub scanned_at: Option<Instant>,
    pub filter: String,
}

fn rescan(app: &mut DmApp) {
    let lib = app.world.read().unwrap().library.clone();
    let mut campaigns = Vec::new();
    let mut characters = Vec::new();
    for (name, dir, saved) in save::list_campaigns(&app.paths.saves_dir) {
        let title = save::load_campaign(&dir).map(|c| c.name).unwrap_or_else(|_| name.clone());
        let chars_dir = dir.join("characters");
        let mut n = 0;
        if let Ok(rd) = std::fs::read_dir(&chars_dir) {
            let mut paths: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().map(|x| x == "toml").unwrap_or(false)).collect();
            paths.sort();
            for p in paths {
                if let Ok(ch) = save::load_character(&p) {
                    let s = dnd_core::derive(&ch, &lib);
                    characters.push(CharRow {
                        id: ch.id.clone(),
                        name: ch.name.clone(),
                        player: ch.player.clone(),
                        summary: format!("{} · {} · L{}", s.species, s.class_line, s.level),
                        hp: format!("{}/{}", s.hp_current, s.hp_max),
                        campaign: title.clone(),
                        campaign_dir: dir.clone(),
                    });
                    n += 1;
                }
            }
        }
        campaigns.push((title, dir, saved, n));
    }
    app.dashboard.campaigns = campaigns;
    app.dashboard.characters = characters;
    app.dashboard.scanned_at = Some(Instant::now());
}

fn load_campaign(app: &mut DmApp, dir: &PathBuf, select_char: Option<String>) {
    crate::ui::content::flush(app);
    match save::load_campaign(dir) {
        Ok(c) => {
            let name = c.name.clone();
            {
                let mut w = app.world.write().unwrap();
                w.campaign = c;
                // a session does not outlive the app: the DM starts it again
                w.campaign.settings.game_live = false;
                w.touch();
            }
            let rev = app.world.read().unwrap().revision;
            app.saved_revision = rev; // freshly loaded == saved
            app.seen_revision = rev;
            app.characters.selected = select_char;
            app.encounter.selected = None;
            app.set_status(format!("Loaded '{name}'"), false);
            if app.characters.selected.is_some() {
                app.view = View::Characters;
            }
        }
        Err(e) => app.set_status(format!("Load failed: {e:#}"), true),
    }
}

/// Every phone/PC that joined by name, with a drop-down to hand it a character.
fn players_section(app: &mut DmApp, ui: &mut egui::Ui) {
    section(ui, "Players at the table");
    struct Row {
        id: String,
        name: String,
        character: Option<String>,
        online: bool,
        last_seen: String,
        rolled: Vec<i32>,
        made: bool,
    }
    let (rows, chars, can_choose, live, start_level) = {
        let w = app.world.read().unwrap();
        let mut rows: Vec<Row> = w
            .campaign
            .devices
            .iter()
            .map(|d| Row {
                id: d.id.clone(),
                name: d.name.clone(),
                character: w.campaign.device_character(&d.id).map(|c| c.id.clone()),
                online: w.device_online(&d.id),
                last_seen: d.last_seen.clone(),
                rolled: d.rolled.clone(),
                made: !d.made.is_empty() && d.made == d.character,
            })
            .collect();
        // online first, waiting-for-a-character first within that
        rows.sort_by_key(|r| (!r.online, r.character.is_some(), r.name.to_lowercase()));
        let chars: Vec<(String, String)> = w.campaign.characters.iter().map(|c| (c.id.clone(), c.name.clone())).collect();
        (rows, chars, w.campaign.settings.players_choose_character, w.campaign.settings.game_live, w.campaign.settings.player_start_level.clamp(1, 20))
    };

    // between sessions players build their own characters; during one the DM hands them out
    let frame = if live { theme::card_hi() } else { theme::card() };
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            if live {
                ui.label(egui::RichText::new("⏺ Game in progress").color(theme::GOOD).strong().size(15.0));
            } else {
                ui.label(egui::RichText::new("○ Between sessions").strong().size(15.0));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !live {
                    let mut lvl = start_level;
                    if ui.add(egui::DragValue::new(&mut lvl).range(1..=20)).on_hover_text("Level that player-made characters start at").changed() {
                        let mut w = app.world.write().unwrap();
                        w.campaign.settings.player_start_level = lvl;
                        w.touch();
                    }
                    ui.label(theme::muted("start level"));
                }
                let label = if live { "⏹ End game" } else { "▶ Start game" };
                if ui.add(theme::primary(label)).clicked() {
                    let mut w = app.world.write().unwrap();
                    w.campaign.settings.game_live = !live;
                    w.campaign.log(if live { "game ended: players may make characters again" } else { "game started: the DM assigns characters" });
                    w.touch();
                }
            });
        });
        // under the buttons, so it wraps in a narrow column instead of running beneath them
        ui.label(theme::muted(if live { "Players can't make new characters now. Give anyone without one a character below." } else { "Players can build a character in their app and upload it; it becomes theirs automatically." }).small());
    });

    ui.horizontal_wrapped(|ui| {
        let mut choose = can_choose;
        if ui
            .checkbox(&mut choose, "Players may pick their own character")
            .on_hover_text("Off: a player's app waits until you assign a character here, and can only open that one.")
            .changed()
        {
            let mut w = app.world.write().unwrap();
            w.campaign.settings.players_choose_character = choose;
            w.touch();
        }
        let (mut walk, mut hurt) = {
            let w = app.world.read().unwrap();
            (w.campaign.settings.players_move_tokens, w.campaign.settings.players_apply_damage)
        };
        if ui.checkbox(&mut walk, "Players move their own token").on_hover_text("On their turn in a fight, freely while exploring. Walls and doors are checked here.").changed() {
            let mut w = app.world.write().unwrap();
            w.campaign.settings.players_move_tokens = walk;
            w.touch();
        }
        if ui
            .checkbox(&mut hurt, "Players' attacks change hit points")
            .on_hover_text("On: an attack or spell from a player's app is rolled here against the target and the damage or healing is applied. Off: it is only rolled and you apply it.")
            .changed()
        {
            let mut w = app.world.write().unwrap();
            w.campaign.settings.players_apply_damage = hurt;
            w.touch();
        }
        let mut ranges = app.world.read().unwrap().campaign.settings.enforce_range;
        if ui
            .checkbox(&mut ranges, "Enforce reach, range and movement")
            .on_hover_text("On: an attack or spell must be within its reach or range with no wall in the way (long range and an enemy next to an archer give disadvantage), and a turn's walking is limited by speed. Off: you referee it.")
            .changed()
        {
            let mut w = app.world.write().unwrap();
            w.campaign.settings.enforce_range = ranges;
            w.touch();
        }
        for (label, long) in [("🌙 Long rest (party)", true), ("☕ Short rest (party)", false)] {
            if ui.button(label).on_hover_text("Every character: hit points, spell slots and features come back as the rules say").clicked() {
                let mut w = app.world.write().unwrap();
                let ids: Vec<String> = w.campaign.characters.iter().map(|c| c.id.clone()).collect();
                // the dead (and, for a long rest, anyone at 0 HP) don't rest
                let skipped: Vec<String> = ids.iter().filter_map(|id| w.rest(id, long).err()).collect();
                drop(w);
                if !skipped.is_empty() {
                    app.set_status(skipped.join(" "), true);
                }
            }
        }
        if ui.button("✨ Auto-assign by name").on_hover_text("Give each unassigned device the character whose Player (or character) name matches the device's name").clicked() {
            let n = {
                let mut w = app.world.write().unwrap();
                let n = w.campaign.auto_assign_devices();
                if n > 0 {
                    w.touch();
                }
                n
            };
            app.set_status(if n == 0 { "No matching names found".to_string() } else { format!("Assigned {n} device(s)") }, n == 0);
        }
    });

    if rows.is_empty() {
        let hint = if app.server.is_some() {
            "Nobody has joined yet. Players open dnd-player, type their name and connect; they show up here."
        } else {
            "Start the server, then players open dnd-player, type their name and connect; they show up here."
        };
        ui.label(theme::muted(hint));
        return;
    }

    let mut assign: Option<(String, Option<String>)> = None;
    let mut forget: Option<String> = None;
    let mut reroll: Option<String> = None;
    for r in &rows {
        let waiting = r.online && r.character.is_none();
        let frame = if waiting { theme::card_hi() } else { theme::card() };
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                let (dot, col, tip) = if r.online { ("⏺", theme::GOOD, "connected".to_string()) } else { ("○", theme::MUTED, format!("offline · last seen {}", r.last_seen)) };
                ui.label(egui::RichText::new(dot).color(col)).on_hover_text(tip);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&r.name).strong().size(15.0));
                    let short = &r.id[..r.id.len().min(4)];
                    let state = if waiting && live { "waiting for a character" } else if waiting { "no character yet (can make one)" } else if r.online { "online" } else { "offline" };
                    let made = if r.made { " · made by the player" } else { "" };
                    ui.label(theme::muted(format!("{state}{made} · device {short}")).small());
                    if !r.rolled.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(theme::muted(format!("rolled {}", r.rolled.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))).small());
                            if ui.small_button("↻").on_hover_text("Throw these rolls away so the player can roll again").clicked() {
                                reroll = Some(r.id.clone());
                            }
                        });
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✖").on_hover_text("Forget this device (it can join again)").clicked() {
                        forget = Some(r.id.clone());
                    }
                    let current = r.character.as_ref().and_then(|cid| chars.iter().find(|(id, _)| id == cid)).map(|(_, n)| n.clone()).unwrap_or_else(|| "— no character —".to_string());
                    egui::ComboBox::from_id_salt(("seat", &r.id)).selected_text(current).width(180.0).show_ui(ui, |ui| {
                        if ui.selectable_label(r.character.is_none(), "— no character —").clicked() && r.character.is_some() {
                            assign = Some((r.id.clone(), None));
                        }
                        for (cid, cname) in &chars {
                            let holder = rows.iter().find(|o| o.id != r.id && o.character.as_deref() == Some(cid.as_str()));
                            let label = match holder {
                                Some(h) => format!("{cname}  (now: {})", h.name),
                                None => cname.clone(),
                            };
                            if ui.selectable_label(r.character.as_deref() == Some(cid.as_str()), label).clicked() && r.character.as_deref() != Some(cid.as_str()) {
                                assign = Some((r.id.clone(), Some(cid.clone())));
                            }
                        }
                    });
                });
            });
        });
    }
    if chars.is_empty() {
        ui.label(theme::muted("Create characters on the Characters tab, then assign them here.").small());
    }

    if let Some((dev, ch)) = assign {
        let mut w = app.world.write().unwrap();
        // one device per character: giving it to someone new takes it from the old holder
        if let Some(cid) = &ch {
            let others: Vec<String> = w.campaign.devices.iter().filter(|d| d.id != dev && d.character == *cid).map(|d| d.id.clone()).collect();
            for o in others {
                w.campaign.assign_device(&o, None);
            }
        }
        w.campaign.assign_device(&dev, ch.as_deref());
        w.touch();
    }
    if let Some(dev) = reroll {
        let mut w = app.world.write().unwrap();
        if let Some(d) = w.campaign.devices.iter_mut().find(|d| d.id == dev) {
            d.rolled.clear();
        }
        w.touch();
    }
    if let Some(dev) = forget {
        let mut w = app.world.write().unwrap();
        w.campaign.forget_device(&dev);
        w.presence.remove(&dev);
        w.touch();
    }
}

pub fn show(app: &mut DmApp, ui: &mut egui::Ui) {
    let stale = app.dashboard.scanned_at.map(|t| t.elapsed().as_secs() > 15).unwrap_or(true);
    if stale || app.saved_revision != app.seen_revision && app.dashboard.scanned_at.map(|t| t.elapsed().as_secs() > 3).unwrap_or(true) {
        rescan(app);
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        let (name, pcs, enc_active, enc_round, enc_n, rev, log_len, join, port) = {
            let w = app.world.read().unwrap();
            (
                w.campaign.name.clone(),
                w.campaign.characters.len(),
                w.campaign.encounter.active,
                w.campaign.encounter.round,
                w.campaign.encounter.combatants.len(),
                w.revision,
                w.campaign.log.len(),
                w.campaign.settings.join_code.clone(),
                w.campaign.settings.port,
            )
        };
        ui.horizontal(|ui| {
            ui.heading(theme::accent("Dashboard"));
            ui.label(theme::muted(format!("· {} · revision {rev}", app.paths.root.display())))
                .on_hover_text(format!("config: {}", app.paths.config_path.display()));
        });
        ui.horizontal_wrapped(|ui| {
            theme::stat(ui, "Current campaign", &name, &format!("{pcs} characters"));
            theme::stat(ui, "Encounter", if enc_active { &"running" } else { &"idle" }, &if enc_active { format!("round {enc_round}, {enc_n} combatants") } else { format!("{enc_n} combatants staged") });
            match &app.server {
                Some(h) => {
                    let ip = app.ips.first().map(|s| s.split(' ').next().unwrap_or(s).to_string()).unwrap_or_else(|| "127.0.0.1".into());
                    theme::stat(ui, "Server", "ON", &format!("http://{ip}:{}", h.addr.port()));
                }
                None => theme::stat(ui, "Server", "OFF", &format!("port {port}")),
            }
            {
                let w = app.world.read().unwrap();
                let online = w.campaign.devices.iter().filter(|d| w.device_online(&d.id)).count();
                let waiting = w.campaign.devices.iter().filter(|d| w.device_online(&d.id) && w.campaign.device_character(&d.id).is_none()).count();
                let sub = if waiting > 0 { format!("{waiting} waiting for a character") } else { format!("{} devices known", w.campaign.devices.len()) };
                theme::stat(ui, "Players", &format!("{online} online"), &sub);
            }
            theme::stat(ui, "Campaigns on disk", &app.dashboard.campaigns.len().to_string(), &format!("{} characters total", app.dashboard.characters.len()));
            theme::stat(ui, "Log entries", &log_len.to_string(), if join.is_empty() { "no join code" } else { "join code set" });
        });

        // two columns side by side, or one under the other in a narrow window
        let n_cols = if ui.available_width() < 820.0 { 1 } else { 2 };
        ui.columns(n_cols, |cols| {
            // ---------------- campaigns
            let ui = &mut cols[0];
            section(ui, "Campaigns");
            if ui.button("⟳ Rescan saves").clicked() {
                rescan(app);
            }
            let rows = app.dashboard.campaigns.clone();
            let current = app.world.read().unwrap().campaign.name.clone();
            let mut to_load: Option<PathBuf> = None;
            for (title, dir, saved, n) in &rows {
                let is_current = *title == current;
                let frame = if is_current { theme::card_hi() } else { theme::card() };
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(title).strong().size(15.0));
                            ui.label(theme::muted(format!("{n} characters · saved {}", saved.split('.').next().unwrap_or(saved).replace('T', " "))).small());
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_current {
                                ui.label(theme::accent("current"));
                            } else if ui.add(theme::primary("Load")).clicked() {
                                to_load = Some(dir.clone());
                            }
                            if ui.small_button("folder").clicked() {
                                let _ = open::that(dir);
                            }
                        });
                    });
                });
            }
            if let Some(dir) = to_load {
                load_campaign(app, &dir, None);
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut app.campaign.new_name).hint_text("new campaign name").desired_width(200.0));
                if ui.button("➕ New campaign").clicked() && !app.campaign.new_name.trim().is_empty() {
                    crate::ui::content::flush(app);
                    let nm = app.campaign.new_name.trim().to_string();
                    {
                        let mut w = app.world.write().unwrap();
                        let port = w.campaign.settings.port;
                        w.campaign = dnd_core::Campaign::new(nm.clone());
                        w.campaign.settings.port = port;
                        w.touch();
                    }
                    app.characters.selected = None;
                    app.campaign.new_name.clear();
                    app.set_status(format!("New campaign '{nm}' (autosaves to saves/)"), false);
                }
                if ui.button("Go to Campaign tab").clicked() {
                    app.view = View::Campaign;
                }
            });

            // ---------------- server
            section(ui, "Players connect to");
            ui.horizontal_wrapped(|ui| {
                if app.server.is_some() {
                    if ui.button("⏹ Stop server").clicked() {
                        app.stop_server();
                    }
                } else if ui.add(theme::primary("▶ Start server")).clicked() {
                    app.start_server();
                }
                ui.label(theme::muted("Join code"));
                let mut code = join.clone();
                if ui.add(egui::TextEdit::singleline(&mut code).desired_width(90.0).hint_text("optional")).on_hover_text("Players need this to save changes").changed() {
                    let mut w = app.world.write().unwrap();
                    w.campaign.settings.join_code = code;
                    w.touch();
                }
                ui.label(theme::muted("DM code"));
                let mut dm = app.world.read().unwrap().campaign.settings.dm_code.clone();
                if ui.add(egui::TextEdit::singleline(&mut dm).desired_width(90.0).hint_text("off")).on_hover_text("Required for remote DM tools (/api/dm/*). Empty = disabled").changed() {
                    let mut w = app.world.write().unwrap();
                    w.campaign.settings.dm_code = dm;
                    w.touch();
                }
                let mut show_hp = app.world.read().unwrap().campaign.settings.show_monster_hp_to_players;
                if ui.checkbox(&mut show_hp, "Players see monster HP").changed() {
                    let mut w = app.world.write().unwrap();
                    w.campaign.settings.show_monster_hp_to_players = show_hp;
                    w.touch();
                }
                let mut free = app.world.read().unwrap().campaign.settings.allow_rule_breaks;
                if ui.checkbox(&mut free, "Players may break the rules").changed() {
                    let mut w = app.world.write().unwrap();
                    w.campaign.settings.allow_rule_breaks = free;
                    w.touch();
                }
                dnd_mapview::help::text(ui, "When ticked, every player may pick options their character doesn't qualify for (a feat above their level, a spell they can't cast yet). They still show in red. To allow it for just one character, use the checkbox on that character's Build tab.");
            });
            if app.server.is_some() {
                for ip in &app.ips {
                    let addr = ip.split(' ').next().unwrap_or(ip);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(format!("http://{addr}:{port}")).monospace().color(theme::ORANGE));
                        ui.label(theme::muted(ip.split(' ').skip(1).collect::<Vec<_>>().join(" ")).small());
                        if ui.small_button("copy").clicked() {
                            ui.ctx().copy_text(format!("http://{addr}:{port}"));
                        }
                    });
                }
                if !app.server_error.is_empty() {
                    ui.colored_label(theme::BAD, &app.server_error);
                }
            }

            // ---------------- content health
            section(ui, "Content packs");
            let (packs, warnings) = {
                let w = app.world.read().unwrap();
                (w.library.packs.clone(), w.library.warnings.clone())
            };
            for p in &packs {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(&p.name).strong());
                    ui.label(theme::muted(p.path.display().to_string()).small());
                    if ui.small_button("folder").clicked() {
                        let _ = open::that(&p.path);
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.button("⟳ Reload content").clicked() {
                    app.world.write().unwrap().reload_content();
                    app.set_status("Content reloaded", false);
                }
                if ui.button("Open Content tab").clicked() {
                    app.view = View::Content;
                }
            });
            if !warnings.is_empty() {
                ui.collapsing(egui::RichText::new(format!("⚠ {} content warning(s)", warnings.len())).color(theme::ORANGE), |ui| {
                    for wmsg in &warnings {
                        ui.colored_label(theme::ORANGE, wmsg);
                    }
                });
            }

            // ---------------- characters (all campaigns)
            let ui = &mut cols[n_cols - 1];
            players_section(app, ui);
            section(ui, "All characters (every campaign)");
            ui.horizontal_wrapped(|ui| {
                ui.label(theme::accent("🔍"));
                ui.add(egui::TextEdit::singleline(&mut app.dashboard.filter).desired_width(200.0).hint_text("name, player, class, campaign"));
                if ui.button("Open Characters tab").clicked() {
                    app.view = View::Characters;
                }
            });
            let f = app.dashboard.filter.to_lowercase();
            let rows: Vec<(usize, String, String, String, String, String, PathBuf, String)> = app
                .dashboard
                .characters
                .iter()
                .enumerate()
                .filter(|(_, c)| f.is_empty() || c.name.to_lowercase().contains(&f) || c.player.to_lowercase().contains(&f) || c.summary.to_lowercase().contains(&f) || c.campaign.to_lowercase().contains(&f))
                .map(|(i, c)| (i, c.name.clone(), c.player.clone(), c.summary.clone(), c.hp.clone(), c.campaign.clone(), c.campaign_dir.clone(), c.id.clone()))
                .collect();
            if rows.is_empty() {
                ui.label(theme::muted("No saved characters yet. Create one on the Characters tab; it is saved automatically."));
            }
            let mut open_char: Option<(PathBuf, String)> = None;
            for (_, name, player, summary, hp, campaign, dir, id) in rows {
                let is_current = campaign == current;
                theme::card().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&name).strong().size(15.0));
                                if !player.is_empty() {
                                    ui.label(theme::muted(format!("({player})")));
                                }
                            });
                            ui.label(theme::muted(&summary).small());
                            ui.label(theme::muted(format!("HP {hp} · {campaign}")).small());
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(theme::primary("Open")).clicked() {
                                open_char = Some((dir.clone(), id.clone()));
                            }
                            if !is_current && ui.small_button("Import here").clicked() {
                                let p = dir.join("characters").join(format!("{id}.toml"));
                                if let Ok(mut ch) = save::load_character(&p) {
                                    let mut w = app.world.write().unwrap();
                                    let mut new_id = ch.id.clone();
                                    let mut n = 2;
                                    while w.campaign.character(&new_id).is_some() {
                                        new_id = format!("{}_{n}", ch.id);
                                        n += 1;
                                    }
                                    ch.id = new_id;
                                    w.campaign.log(format!("imported '{}' from '{campaign}'", ch.name));
                                    w.campaign.characters.push(ch);
                                    w.touch();
                                }
                            }
                        });
                    });
                });
            }
            if let Some((dir, id)) = open_char {
                if app.world.read().unwrap().campaign.name == app.dashboard.characters.iter().find(|c| c.id == id && c.campaign_dir == dir).map(|c| c.campaign.clone()).unwrap_or_default() {
                    app.characters.selected = Some(id);
                    app.view = View::Characters;
                } else {
                    load_campaign(app, &dir, Some(id));
                }
            }

            section(ui, "Recent log");
            let log: Vec<String> = {
                let w = app.world.read().unwrap();
                w.campaign.log.iter().rev().take(12).map(|l| format!("{}  {}", l.time, l.text)).collect()
            };
            for l in log {
                ui.label(egui::RichText::new(l).monospace().small());
            }
        });
    });
}
