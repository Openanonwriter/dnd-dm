//! The character sheet, tab by tab.

use crate::app::{PlayerApp, Screen, Tab};
use crate::client::Cmd;
use crate::theme::{self, section, tile};
use crate::ui::bonus;
use dnd_core::{Character, Sheet};

pub fn show(app: &mut PlayerApp, ctx: &egui::Context) {
    let (inset_top, inset_bottom) = crate::system_insets(ctx);
    // ---- header: who you are, hit points and armor class on every tab
    let down = app.character.as_ref().map(|c| c.hp.current <= 0 && !c.is_dead()).unwrap_or(false);
    // Without the DM's server the sheet is something to read, nothing more:
    // no rolls, no hit point changes, no conditions, no spending of anything.
    let offline = !app.online;
    if offline {
        app.ui_state.hp_open = false;
    } else if down {
        // the death save button lives in the hit point panel
        app.ui_state.hp_open = true;
    }
    egui::TopBottomPanel::top("sheet_top").frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin { left: 12, right: 12, top: 8 + inset_top as i8, bottom: 8 })).show(ctx, |ui| {
        let (name, sub) = match &app.sheet {
            Some(s) => (s.name.clone(), format!("{} · {} · L{}", s.species, s.class_line, s.level)),
            None => (app.config.character_id.clone(), "loading…".into()),
        };
        let vitals = match (&app.sheet, &app.character) {
            (Some(s), Some(c)) => Some((c.hp.current, s.hp_max, c.hp.temp, s.ac)),
            _ => None,
        };
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((hp, max, temp, ac)) = vitals {
                    if vitals_button(ui, hp, max, temp, ac, app.ui_state.hp_open).clicked() && !down && !offline {
                        app.ui_state.hp_open = !app.ui_state.hp_open;
                    }
                }
                let dot = if app.online { ("⏺", theme::GOOD, "connected") } else { ("○", theme::MUTED, "offline (cached)") };
                ui.label(egui::RichText::new(dot.0).color(dot.1)).on_hover_text(dot.2);
                if app.busy {
                    ui.spinner();
                }
                if app.last_edit.is_some() {
                    ui.label(theme::accent("⏺").small());
                }
                // the name takes what is left and is cut short rather than pushing the numbers off
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.add(egui::Label::new(egui::RichText::new(name).strong().size(20.0).color(theme::ORANGE)).truncate());
                    // "Saved to DM", "Connected to ..." show here for a moment, in the line under the name
                    let note = (!app.status_error && !app.status.is_empty() && app.status_at.elapsed().as_secs() < 3).then(|| app.status.clone());
                    match note {
                        Some(n) => {
                            ui.add(egui::Label::new(egui::RichText::new(n).small().color(theme::GOOD)).truncate());
                            ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
                        }
                        None => {
                            ui.add(egui::Label::new(theme::muted(sub).small()).truncate());
                        }
                    }
                });
            });
        });
        // the hit point panel belongs to the tab it was opened on
        if app.ui_state.hp_open && !down && app.ui_state.hp_open_on != Some(app.tab) {
            if app.ui_state.hp_open_on.is_some() {
                app.ui_state.hp_open = false;
            }
            app.ui_state.hp_open_on = if app.ui_state.hp_open { Some(app.tab) } else { None };
        }
        if !app.ui_state.hp_open {
            app.ui_state.hp_open_on = None;
        }
        // whose turn it is: one place, on every tab but the map (which has its own bar)
        let fight = app.encounter.as_ref().filter(|e| e.active).map(|e| (e.round, e.combatants.iter().find(|c| c.is_current).map(|c| c.name.clone()).unwrap_or_default()));
        if offline {
            egui::Frame::new().fill(theme::CARD).stroke(egui::Stroke::new(1.0, theme::BAD)).corner_radius(theme::R_CTRL).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(40.0);
                ui.label(egui::RichText::new("Not connected to the DM").color(theme::BAD));
                ui.label(theme::muted("This is the copy of your sheet kept on this device, to read. Rolls and changes come back when the DM's server does.").small());
            });
        } else if app.my_turn() && app.tab != Tab::Party {
            egui::Frame::new().fill(theme::ORANGE_FAINT).stroke(egui::Stroke::new(1.0, theme::ORANGE)).corner_radius(theme::R_CTRL).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Your turn").strong().size(18.0).color(theme::ORANGE));
                    dnd_mapview::help::term(ui, "turn");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(theme::primary("End turn")).clicked() {
                            app.act(Cmd::Do { action: "end_turn".into(), arg: String::new() });
                        }
                    });
                });
            });
        } else if let Some((round, who)) = fight.filter(|_| !app.my_turn()) {
            // the same height as the "Your turn" banner, so the page does not jump when turns change
            egui::Frame::new().fill(theme::CARD).stroke(egui::Stroke::new(1.0, theme::LINE)).corner_radius(theme::R_CTRL).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(40.0);
                ui.horizontal_centered(|ui| {
                    ui.label(theme::muted(if who.is_empty() { format!("Round {round}") } else { format!("Round {round} · {who}'s turn") }));
                });
            });
        }
        if app.ui_state.hp_open {
            hp_block(app, ui);
        }
    });

    // ---- bottom bar: at most six places, the rest behind "More"
    egui::TopBottomPanel::bottom("tabs").frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin { left: 4, right: 4, top: 4, bottom: 4 + inset_bottom as i8 })).show(ctx, |ui| {
        let caster = app.sheet.as_ref().map(|s| !s.spellcasting.is_empty()).unwrap_or(true);
        let mut nav: Vec<(Tab, &str, &str)> = vec![(Tab::Stats, Tab::Stats.icon(), "Stats"), (Tab::Combat, Tab::Combat.icon(), "Combat")];
        if caster {
            nav.push((Tab::Spells, Tab::Spells.icon(), "Spells"));
        }
        nav.push((Tab::Items, Tab::Items.icon(), "Items"));
        nav.push((Tab::Party, Tab::Party.icon(), "Map"));
        nav.push((Tab::Settings, "☰", "More"));
        let w = ui.available_width() / nav.len() as f32;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (t, icon, label) in nav {
                let more = t == Tab::Settings;
                let sel = if more { app.tab.in_more() } else { app.tab == t };
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 56.0), egui::Sense::click());
                if ui.is_rect_visible(rect) {
                    let col = if sel { theme::ORANGE } else { theme::MUTED };
                    if sel {
                        ui.painter().rect_filled(rect.shrink2(egui::vec2(3.0, 1.0)), theme::R_CTRL, theme::ORANGE_FAINT);
                    }
                    ui.painter().text(rect.center() + egui::vec2(0.0, -9.0), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(21.0), col);
                    ui.painter().text(rect.center() + egui::vec2(0.0, 15.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(13.0), col);
                }
                if resp.clicked() {
                    app.tab = if more { app.ui_state.more_tab.unwrap_or(Tab::Dice) } else { t };
                }
            }
        });
    });

    // scrolling tabs put their scroll bar on the screen edge, so they keep the right margin inside the scroll area
    let scrolls = app.sheet.is_some() && app.character.is_some() && app.tab != Tab::Party;
    let margin = egui::Margin { left: 12, right: if scrolls { 0 } else { 12 }, top: 12, bottom: 12 };
    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(margin)).show(ctx, |ui| {
        if app.sheet.is_none() || app.character.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.spinner();
                ui.label(theme::muted("Fetching your sheet from the DM…"));
                if app.can_choose() && ui.button("Pick a different character").clicked() {
                    app.screen = Screen::Pick;
                }
                if ui.button("Switch game / change name").clicked() {
                    app.screen = Screen::Connect;
                }
            });
            return;
        }
        if app.tab == Tab::Party {
            if offline {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| ui.label(theme::muted("The map comes from the DM's server. It is back when you are connected.")));
                return;
            }
            // the map pans and zooms by itself, so it does not sit in a scroll area
            crate::ui::map::show(app, ui);
            return;
        }
        if app.tab.in_more() {
            app.ui_state.more_tab = Some(app.tab);
            let pages = [Tab::Dice, Tab::Notes, Tab::Settings];
            let at = pages.iter().position(|t| *t == app.tab).unwrap_or(0);
            ui.horizontal(|ui| {
                ui.set_width(ui.available_width() - 12.0);
                if let Some(i) = theme::segmented(ui, &["Dice", "Features", "Settings"], at) {
                    app.tab = pages[i];
                }
            });
            ui.add_space(2.0);
        }
        // DND_PLAYER_SCROLL=<points> starts scrolled down (screenshots)
        let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if let Some(y) = std::env::var("DND_PLAYER_SCROLL").ok().and_then(|v| v.parse::<f32>().ok()) {
            area = area.vertical_scroll_offset(y);
        }
        theme::page_scroll(ui, area, 12, |ui| {
            // offline: everything on the page can be read and scrolled, nothing can be pressed.
            // Settings stay usable (switch game, text size); the Features page handles its own text boxes.
            if offline && !matches!(app.tab, Tab::Settings | Tab::Notes) {
                ui.disable();
            }
            match app.tab {
            Tab::Stats => stats_tab(app, ui),
            Tab::Combat => combat_tab(app, ui),
            Tab::Spells => crate::ui::spells::tab(app, ui),
            Tab::Items => items_tab(app, ui),
            Tab::Notes => notes_tab(app, ui),
            Tab::Party => {}
            Tab::Dice => dice_tab(app, ui),
            Tab::Settings => settings_tab(app, ui),
            }
        });
    });

    // what the dice just said: a card floating above the bottom bar, so nothing
    // on the page moves under the player's finger when it comes and goes
    if app.ui_state.result.as_ref().map(|r| r.3.elapsed().as_secs() >= 8).unwrap_or(false) {
        app.ui_state.result = None;
    }
    // short messages ("Connected to ...", "it is not your turn") use the same card when no roll is showing
    // only refusals float ("it is not your turn"); everything else is in the header
    // (not while offline: the strip under the name already says so, and each failed reconnect would pop up again)
    let message = if !offline && app.ui_state.result.is_none() && app.status_error && !app.status.is_empty() && app.status_at.elapsed().as_secs() < 8 {
        Some((0, Some(false), app.status.clone(), app.status_at))
    } else {
        None
    };
    let is_message = message.is_some();
    if let Some((total, hit, text, _)) = app.ui_state.result.clone().or(message) {
        let hit = if is_message { None } else { hit };
        let edge = if is_message && app.status_error { theme::BAD } else if is_message { theme::GOOD } else { match hit {
            Some(true) => theme::GOOD,
            Some(false) => theme::BAD,
            None => theme::ORANGE,
        } };
        let width = ctx.screen_rect().width() - 24.0;
        let area = egui::Area::new(egui::Id::new(if is_message { "message" } else { "roll_result" })).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -(72.0 + inset_bottom))).show(ctx, |ui| {
            if is_message {
                // a short message: a small pill, just wide enough for its words
                egui::Frame::new().fill(theme::CARD_HI).stroke(egui::Stroke::new(1.0, edge)).corner_radius(theme::R_CTRL).inner_margin(egui::Margin::symmetric(14, 9)).show(ui, |ui| {
                    ui.set_max_width(width - 28.0);
                    ui.add(egui::Label::new(egui::RichText::new(text).size(14.0).color(edge)).wrap());
                });
            } else {
                egui::Frame::new().fill(theme::CARD_HI).stroke(egui::Stroke::new(2.0, edge)).corner_radius(theme::R_CARD).inner_margin(egui::Margin::symmetric(14, 12)).show(ui, |ui| {
                    ui.set_width(width - 28.0);
                    ui.horizontal(|ui| {
                        let big = match hit {
                            Some(true) => "HIT".to_string(),
                            Some(false) => "MISS".to_string(),
                            None => total.to_string(),
                        };
                        if hit.is_some() || total != 0 {
                            ui.label(egui::RichText::new(big).size(28.0).color(edge));
                        }
                        ui.add(egui::Label::new(egui::RichText::new(text).size(14.0)).wrap());
                    });
                });
            }
        });
        if area.response.interact(egui::Sense::click()).clicked() {
            app.ui_state.result = None;
            app.status.clear();
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}

fn sheet(app: &PlayerApp) -> Sheet {
    app.sheet.clone().unwrap()
}

fn tiles_row(ui: &mut egui::Ui, items: &[(&str, String, String)]) {
    let n = items.len().max(1) as f32;
    // item spacing between tiles plus 1 px stroke on each side of every tile
    let w = ((ui.available_width() - ui.spacing().item_spacing.x * (n - 1.0)) / n).floor().max(60.0);
    ui.horizontal(|ui| {
        for (l, v, s) in items {
            tile(ui, w, l, v, s);
        }
    });
}

// ------------------------------------------------------------------ Stats

fn stats_tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    let s = sheet(app);
    for w in &s.warnings {
        ui.colored_label(theme::ORANGE, format!("⚠ {w}"));
    }
    // tap a score to roll a check with it
    let ab: Vec<(String, i32, i32)> = s.abilities.iter().map(|(k, v)| (k.to_string(), dnd_core::ability_mod(v), v)).collect();
    for row in ab.chunks(3) {
        let n = row.len() as f32;
        let w = ((ui.available_width() - ui.spacing().item_spacing.x * (n - 1.0)) / n).floor().max(60.0);
        ui.horizontal(|ui| {
            for (k, m, v) in row {
                if tile(ui, w, k, &bonus(*m), &v.to_string()).clicked() {
                    app.roll(&format!("1d20 {}", signed(*m)), &format!("{} check", dnd_core::sheet::ability_name(k)));
                }
            }
        });
    }
    tiles_row(
        ui,
        &[
            ("Armor", s.ac.to_string(), String::new()),
            ("Init", bonus(s.initiative), String::new()),
            ("Speed", format!("{}", s.speed), "ft".into()),
            ("Prof", bonus(s.proficiency_bonus), String::new()),
        ],
    );
    ui.label(theme::muted("Tap a score, a save or a skill to roll it.").small());

    let s = sheet(app);
    section(ui, "Saving throws");
    // one button per save: the number is the roll
    let saves: Vec<_> = s.saves.clone();
    for row in saves.chunks(3) {
        ui.horizontal(|ui| {
            let w = ((ui.available_width() - ui.spacing().item_spacing.x * 2.0) / 3.0).floor();
            for sv in row {
                let mark = if sv.proficient { "⏺ " } else { "" };
                let text = egui::RichText::new(format!("{mark}{}  {}", &sv.name[..3].to_uppercase(), bonus(sv.bonus))).color(if sv.proficient { theme::ORANGE } else { theme::TEXT });
                let mut r = ui.add_sized([w, theme::H_CTRL], egui::Button::new(text));
                if !sv.notes.is_empty() {
                    r = r.on_hover_text(sv.notes.join("; "));
                }
                if r.clicked() {
                    app.roll(&format!("1d20 {}", signed(sv.bonus)), &format!("{} save", sv.name));
                }
            }
        });
    }

    section(ui, "Skills");
    let skills: Vec<_> = s.skills.clone();
    for sk in skills {
        ui.horizontal(|ui| {
            let room = theme::Room::new(ui, ("skill", &sk.key));
            let mut tapped = false;
            ui.horizontal_wrapped(|ui| {
                room.limit(ui);
                ui.label(if sk.expertise { theme::accent("★") } else if sk.proficient { theme::accent("⏺") } else { theme::muted("○") });
                tapped = ui.add(egui::Label::new(format!("{}  {}", sk.name, theme_dim(&sk.ability))).sense(egui::Sense::click())).clicked();
                dnd_mapview::help::term(ui, &sk.key);
                if !sk.notes.is_empty() {
                    ui.label(theme::muted(sk.notes.join("; ")).small());
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(egui::RichText::new(bonus(sk.bonus)).strong()).min_size(egui::vec2(56.0, theme::H_CTRL))).clicked() || tapped {
                    app.roll(&format!("1d20 {}", signed(sk.bonus)), &sk.name);
                }
                room.note(ui);
            });
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::muted(format!("Passive Perception {} · Investigation {} · Insight {}", s.passive_perception, s.passive_investigation, s.passive_insight)).small());
        dnd_mapview::help::term(ui, "passive_perception");
    });

    section(ui, "Proficiencies & languages");
    ui.label(format!("Armor: {}", s.proficiencies.armor.join(", ")));
    ui.label(format!("Weapons: {}", s.proficiencies.weapons.join(", ")));
    ui.label(format!("Tools: {}", s.proficiencies.tools.join(", ")));
    ui.label(format!("Languages: {}", s.proficiencies.languages.join(", ")));
    if s.darkvision > 0 {
        ui.label(format!("Darkvision {} ft", s.darkvision));
    }
}

fn theme_dim(s: &str) -> String {
    s.to_uppercase()
}

fn signed(b: i32) -> String {
    if b >= 0 {
        format!("+ {b}")
    } else {
        format!("- {}", -b)
    }
}

fn hp_block(app: &mut PlayerApp, ui: &mut egui::Ui) {
    let hp_max = app.sheet.as_ref().map(|s| s.hp_max).unwrap_or(1);
    let mut death_save = false;
    let Some(ch) = app.character.as_mut() else { return };
    let dead = ch.is_dead();
    theme::card_hi().show(ui, |ui| {
        ui.set_width(ui.available_width());
        // nothing on this card can bring the dead back
        if dead {
            ui.disable();
        }
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(theme::muted("HIT POINTS").small());
                    dnd_mapview::help::term(ui, "hit_points");
                    if ch.hp.temp == 0 {
                        ui.label(theme::muted("· temp").small());
                        dnd_mapview::help::term(ui, "temp_hp");
                    }
                });
                let col = if ch.hp.current <= hp_max / 4 { theme::BAD } else if ch.hp.current <= hp_max / 2 { theme::ORANGE } else { theme::GOOD };
                ui.label(egui::RichText::new(format!("{} / {}", ch.hp.current, hp_max)).color(col).strong().size(30.0));
                if ch.hp.temp > 0 {
                    ui.horizontal(|ui| {
                        ui.label(theme::muted(format!("+{} temp", ch.hp.temp)).small());
                        dnd_mapview::help::term(ui, "temp_hp");
                    });
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if ui.add(theme::big_button("+5")).clicked() {
                            ch.hp.current = (ch.hp.current + 5).min(hp_max);
                        }
                        if ui.add(theme::big_button("+1")).clicked() {
                            ch.hp.current = (ch.hp.current + 1).min(hp_max);
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.add(theme::big_button("−5")).clicked() {
                            apply_damage(ch, 5);
                        }
                        if ui.add(theme::big_button("−1")).clicked() {
                            apply_damage(ch, 1);
                        }
                    });
                });
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(theme::muted("Amount"));
            ui.add(egui::DragValue::new(&mut app.ui_state.damage_amount).range(0..=999));
            if ui.button("💥 Damage").clicked() {
                apply_damage(ch, app.ui_state.damage_amount);
            }
            if ui.button("💚 Heal").clicked() {
                ch.hp.current = (ch.hp.current + app.ui_state.damage_amount).min(hp_max);
            }
            if ui.button("Temp").clicked() {
                ch.hp.temp = ch.hp.temp.max(app.ui_state.damage_amount);
            }
        });
        let dying = ch.hp.current <= 0 || ch.death_saves.successes > 0 || ch.death_saves.failures > 0;
        if dying {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.label(theme::muted("Death saves"));
            dnd_mapview::help::term(ui, "death_saves");
            ui.end_row();
            ui.label(egui::RichText::new("pass").small().color(theme::GOOD));
            for i in 0..3 {
                let mut on = ch.death_saves.successes > i;
                if ui.checkbox(&mut on, "").changed() {
                    ch.death_saves.successes = if on { i + 1 } else { i };
                }
            }
            ui.label(egui::RichText::new(" fail").small().color(theme::BAD));
            for i in 0..3 {
                let mut on = ch.death_saves.failures > i;
                if ui.checkbox(&mut on, "").changed() {
                    ch.death_saves.failures = if on { i + 1 } else { i };
                }
            }
        });
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut ch.inspiration, "Inspiration");
            dnd_mapview::help::term(ui, "inspiration");
            if ch.hp.current > 0 {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        app.ui_state.hp_open = false;
                    }
                });
            }
        });
        if ch.is_dead() {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Your character has died.").color(theme::BAD).strong());
                dnd_mapview::help::term(ui, "dead");
            });
            ui.label(theme::muted("Only magic such as revivify or raise dead can bring them back. Talk to your DM.").small());
        }
        if ch.hp.current <= 0 && ch.death_saves.failures < 3 {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("You are down.").color(theme::BAD).strong());
                death_save = ui.add(theme::primary("Roll death save")).clicked();
            });
        }
    });
    if death_save {
        app.act(Cmd::Do { action: "death_save".into(), arg: String::new() });
    }
}

fn apply_damage(ch: &mut Character, amt: i32) {
    let mut rem = amt.max(0);
    let soak = rem.min(ch.hp.temp);
    ch.hp.temp -= soak;
    rem -= soak;
    ch.hp.current = (ch.hp.current - rem).max(0);
}

// ------------------------------------------------------------------ Combat

fn combat_tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    let s = sheet(app);
    section(ui, "Attacks");
    crate::ui::spells::mode_row(app, ui);
    crate::ui::spells::target_row(app, ui, false);
    let target = app.targets.first().cloned();
    if target.is_none() && app.encounter.as_ref().map(|e| !e.combatants.is_empty()).unwrap_or(false) {
        ui.label(theme::muted("Pick a target and the attack is rolled against it. Without one you just roll the dice.").small());
    }
    let attacks = s.attacks.clone();
    for (i, a) in attacks.into_iter().enumerate() {
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let room = theme::Room::new(ui, ("attack", i));
                ui.vertical(|ui| {
                    room.limit(ui);
                    ui.label(egui::RichText::new(&a.name).strong().size(17.0));
                    ui.label(theme::muted(format!("{} to hit · {} {} · {}{}", bonus(a.bonus), a.damage, a.damage_type, a.range, if a.notes.is_empty() { String::new() } else { format!(" · {}", a.notes.join("; ")) })).small());
                    // weapon properties, each with its (i)
                    // weapon properties in one line, with one (i) that explains them all
                    // (the icon comes first so it can never be left alone on a line)
                    if !a.properties.is_empty() {
                        let names: Vec<String> = a.properties.iter().map(|p| p.replace('_', "-")).collect();
                        let help: Vec<String> = a.properties.iter().filter_map(|p| dnd_core::glossary::explain(p).map(|t| format!("{}: {t}", p.replace('_', "-")))).collect();
                        ui.horizontal(|ui| {
                            dnd_mapview::help::text(ui, &help.join("\n\n"));
                            ui.add(egui::Label::new(theme::muted(names.join(" · ")).small()).wrap());
                        });
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(t) = &target {
                        let dist = app.my_token().and_then(|m| app.map.as_ref().and_then(|v| v.distance_ft(&m, t)));
                        let ok = a.reach == dnd_core::range::Reach::default() || dist.map(|d| a.reach.judge(d).is_some()).unwrap_or(true);
                        if ui.add_enabled(ok, theme::primary("Attack")).on_disabled_hover_text(format!("Out of range: reaches {}", a.reach.describe())).clicked() {
                            let mode = app.mode;
                            app.act(Cmd::Attack { attack: i, target: t.clone(), mode });
                        }
                    } else {
                        if ui.add(theme::primary(&format!("Hit {}", bonus(a.bonus)))).clicked() {
                            app.roll(&format!("1d20 {}", signed(a.bonus)), &format!("{} attack", a.name));
                        }
                        if ui.add(theme::big_button("Dmg")).clicked() {
                            app.roll(&a.damage, &format!("{} damage", a.name));
                        }
                    }
                    room.note(ui);
                });
            });
        });
    }

    // ---- features that run out
    let limited: Vec<(String, u32, String)> = s.features.iter().filter_map(|f| f.uses.filter(|u| *u > 0).map(|u| (f.name.clone(), u as u32, f.recharge.replace('_', " ")))).collect();
    if !limited.is_empty() {
        section(ui, "Limited uses");
        if let Some(ch) = app.character.as_mut() {
            for (name, max, recharge) in &limited {
                ui.horizontal(|ui| {
                    let used = ch.uses.get(name).copied().unwrap_or(0).min(*max);
                    for k in 0..*max {
                        let spent = k < used;
                        if ui.add(egui::Button::new(egui::RichText::new(if spent { "○" } else { "⏺" }).size(22.0).color(if spent { theme::MUTED } else { theme::ORANGE })).frame(false)).clicked() {
                            let now = if spent { k } else { k + 1 };
                            if now == 0 {
                                ch.uses.remove(name);
                            } else {
                                ch.uses.insert(name.clone(), now);
                            }
                        }
                    }
                    ui.label(name);
                    if !recharge.is_empty() {
                        ui.label(theme::muted(format!("back after a {recharge}")).small());
                    }
                });
            }
        }
    }

    section(ui, "Conditions");
    let known = [
        "blinded", "charmed", "deafened", "frightened", "grappled", "incapacitated", "invisible", "paralyzed", "petrified", "poisoned", "prone", "restrained", "stunned",
        "unconscious", "concentrating",
    ];
    if let Some(ch) = app.character.as_mut() {
        // two even columns, each a chip with its (i) beside it
        let gap = ui.spacing().item_spacing.x;
        let col = ((ui.available_width() - gap) / 2.0).floor();
        for pair in known.chunks(2) {
            ui.horizontal(|ui| {
                for c in pair {
                    ui.allocate_ui_with_layout(egui::vec2(col, theme::H_CTRL), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_width(col);
                        let mut on = ch.conditions.iter().any(|x| x == c);
                        let text = egui::RichText::new(*c).color(if on { egui::Color32::WHITE } else { theme::TEXT });
                        let b = egui::Button::new(text);
                        let b = if on { b.fill(theme::SELECT).stroke(egui::Stroke::NONE) } else { b };
                        if ui.add_sized([col - 34.0, theme::H_CTRL], b).clicked() {
                            on = !on;
                            if on {
                                ch.conditions.push(c.to_string());
                            } else {
                                ch.conditions.retain(|x| x != c);
                            }
                        }
                        dnd_mapview::help::term(ui, c);
                    });
                }
            });
        }
        ui.horizontal(|ui| {
            ui.label("Exhaustion");
            dnd_mapview::help::term(ui, "exhaustion");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new("+").min_size(egui::vec2(theme::H_CTRL, theme::H_CTRL))).clicked() {
                    ch.exhaustion = (ch.exhaustion + 1).min(6);
                }
                ui.label(egui::RichText::new(ch.exhaustion.to_string()).strong().size(17.0));
                if ui.add(egui::Button::new("−").min_size(egui::vec2(theme::H_CTRL, theme::H_CTRL))).clicked() {
                    ch.exhaustion = ch.exhaustion.saturating_sub(1);
                }
            });
        });
    }

    section(ui, "Rest");
    let mut todo: Option<(&str, String)> = None;
    let online = app.online;
    if let Some(ch) = app.character.as_ref() {
        ui.horizontal_wrapped(|ui| {
            for (d, n) in &s.hit_dice {
                let used = ch.hp.hit_dice_used.get(d).copied().unwrap_or(0);
                ui.label(format!("{d} hit dice: {} of {n}", n.saturating_sub(used)));
                dnd_mapview::help::term(ui, "hit_dice");
                if ui.add_enabled(online && used < *n && ch.hp.current < s.hp_max, egui::Button::new(format!("Spend a {d}"))).on_hover_text("During a short rest: roll it, add Constitution, heal that much").clicked() {
                    todo = Some(("hit_die", d.clone()));
                }
            }
        });
        // the dead can't rest, and a long rest needs at least 1 hit point
        let dead = ch.is_dead();
        let down = ch.hp.current <= 0;
        ui.horizontal(|ui| {
            dnd_mapview::help::term(ui, "short_rest");
            if ui.add_enabled(online && !dead, egui::Button::new("☕ Short rest")).clicked() {
                todo = Some(("short_rest", String::new()));
            }
            if ui.add_enabled(online && !down, egui::Button::new("🌙 Long rest")).on_hover_text("Full hit points, all spell slots, half your hit dice").clicked() {
                todo = Some(("long_rest", String::new()));
            }
        });
        if dead {
            ui.label(egui::RichText::new("The dead can't rest. Resting won't bring your character back.").color(theme::BAD).small());
        } else if down {
            ui.label(theme::muted("You need at least 1 hit point to benefit from a long rest.").small());
        }
        if !online {
            ui.label(theme::muted("Rests are taken with the DM: connect first.").small());
        }
    }
    if let Some((action, arg)) = todo {
        app.act(Cmd::Do { action: action.into(), arg });
    }
}

// ------------------------------------------------------------------ Items

fn items_tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    let s = sheet(app);
    tiles_row(
        ui,
        &[
            ("Weight", format!("{}", s.total_weight), format!("of {} lb", s.carrying_capacity)),
            ("Coins", coins_total(&s.currency), coins_line(&s.currency)),
        ],
    );
    let lib = app.builder_lib.as_ref();
    let Some(ch) = app.character.as_mut() else { return };
    section(ui, "Inventory");
    let names: std::collections::HashMap<String, (String, String)> = s.inventory.iter().map(|l| (l.item.clone(), (l.name.clone(), l.kind.clone()))).collect();
    let mut remove = None;
    for (i, inv) in ch.inventory.iter_mut().enumerate() {
        let (name, kind) = names.get(&inv.item).cloned().unwrap_or((if inv.name.is_empty() { inv.item.clone() } else { inv.name.clone() }, "custom".into()));
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            // the name has the whole line; the buttons sit on a line of their own
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(&name).strong());
                if let (Some(lib), false) = (lib, inv.custom) {
                    dnd_mapview::help::entry(ui, lib, dnd_core::Kind::Item, &inv.item);
                }
            });
            ui.horizontal(|ui| {
                let room = theme::Room::new(ui, ("item", i));
                ui.vertical(|ui| {
                    room.limit(ui);
                    ui.label(theme::muted(format!("{kind}{}", if inv.notes.is_empty() { String::new() } else { format!(" · {}", inv.notes) })).small());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // count with plus and minus; at zero the row offers to remove itself
                    if ui.add(egui::Button::new("+").min_size(egui::vec2(theme::H_CTRL, theme::H_CTRL))).clicked() {
                        inv.qty = (inv.qty + 1).min(999);
                    }
                    ui.label(egui::RichText::new(inv.qty.to_string()).strong().size(17.0));
                    if ui.add(egui::Button::new("−").min_size(egui::vec2(theme::H_CTRL, theme::H_CTRL))).clicked() {
                        inv.qty = inv.qty.saturating_sub(1);
                    }
                    if inv.qty == 0 && ui.add(egui::Button::new(egui::RichText::new("Remove").color(theme::BAD))).clicked() {
                        remove = Some(i);
                    }
                    if inv.qty > 0 {
                        let label = if inv.equipped { "equipped" } else { "equip" };
                        theme::toggle(ui, &mut inv.equipped, label);
                    }
                    if inv.attuned || kind == "wondrous" {
                        theme::toggle(ui, &mut inv.attuned, "attuned");
                        dnd_mapview::help::term(ui, "attuned");
                    }
                    room.note(ui);
                });
            });
        });
    }
    if let Some(i) = remove {
        ch.inventory.remove(i);
    }
    ui.horizontal(|ui| {
        ui.add(crate::theme::line_edit(&mut app.ui_state.hp_input).hint_text("add a custom item (e.g. “goblin ear”)").desired_width(f32::INFINITY));
        if ui.button("Add").clicked() && !app.ui_state.hp_input.trim().is_empty() {
            ch.inventory.push(dnd_core::InventoryItem { item: app.ui_state.hp_input.trim().into(), name: app.ui_state.hp_input.trim().into(), qty: 1, custom: true, ..Default::default() });
            app.ui_state.hp_input.clear();
        }
    });
    section(ui, "Coins");
    ui.horizontal_wrapped(|ui| {
        for (l, v) in [("cp", &mut ch.currency.cp), ("sp", &mut ch.currency.sp), ("ep", &mut ch.currency.ep), ("gp", &mut ch.currency.gp), ("pp", &mut ch.currency.pp)] {
            ui.label(theme::muted(l));
            ui.add(egui::DragValue::new(v).range(0..=1_000_000));
        }
    });
}

// ------------------------------------------------------------------ Notes & features

fn notes_tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    let s = sheet(app);
    section(ui, "Features & traits");
    for f in &s.features {
        let title = format!("{}  {}", f.name, theme_sub(&f.source, f.uses, &f.recharge));
        egui::CollapsingHeader::new(egui::RichText::new(title)).id_salt(("feat", &f.name, &f.source)).show(ui, |ui| {
            ui.label(&f.description);
        });
    }
    let online = app.online;
    let Some(ch) = app.character.as_mut() else { return };
    if !online {
        // features above can still be opened and read; the text below can be read, not changed
        ui.disable();
    }
    section(ui, "Notes");
    ui.add(egui::TextEdit::multiline(&mut ch.notes).desired_rows(5).desired_width(f32::INFINITY).hint_text("Session notes, loot to split, NPC names…"));
    section(ui, "Roleplay");
    for (l, v) in [("Appearance", &mut ch.appearance), ("Personality", &mut ch.personality), ("Ideals", &mut ch.ideals), ("Bonds", &mut ch.bonds), ("Flaws", &mut ch.flaws)] {
        ui.label(theme::muted(l));
        ui.add(egui::TextEdit::multiline(v).desired_rows(2).desired_width(f32::INFINITY));
    }
    ui.label(theme::muted("Backstory"));
    ui.add(egui::TextEdit::multiline(&mut ch.backstory).desired_rows(5).desired_width(f32::INFINITY));
}

fn theme_sub(source: &str, uses: Option<i64>, recharge: &str) -> String {
    let mut s = format!("({source}");
    if let Some(u) = uses {
        s.push_str(&format!(", {u}× / {}", if recharge.is_empty() { "rest" } else { recharge }));
    }
    s.push(')');
    s
}

// ------------------------------------------------------------------ Dice

fn dice_tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    section(ui, "Roll");
    ui.horizontal(|ui| {
        let r = ui.add(crate::theme::line_edit(&mut app.dice_formula).desired_width(ui.available_width() - 150.0).font(egui::TextStyle::Heading));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.add(theme::primary("Roll")).clicked() || enter {
            let f = app.dice_formula.clone();
            app.roll(&f, "");
        }
        dnd_mapview::help::term(ui, "dice_notation");
    });
    // an even grid: every die the same size
    let gap = ui.spacing().item_spacing.x;
    let full = ui.available_width();
    for (row, n) in [(&["d20", "d12", "d10", "d8", "d6", "d4"][..], 6.0_f32), (&["d100", "2d20kh1", "2d20kl1", "4d6kh3"][..], 4.0)] {
        ui.horizontal(|ui| {
            let w = (full - gap * (n - 1.0)) / n;
            for d in row {
                if ui.add_sized([w, theme::H_CTRL], egui::Button::new(*d)).clicked() {
                    app.roll(d, "");
                }
            }
        });
    }
    ui.horizontal(|ui| {
        let w = (full - gap * 2.0) / 3.0;
        if ui.add_sized([w, theme::H_CTRL], egui::Button::new("Advantage")).clicked() {
            let f = app.dice_formula.replacen("1d20", "2d20kh1", 1).replacen("d20", "2d20kh1", 1);
            app.roll(&f, "advantage");
        }
        if ui.add_sized([w, theme::H_CTRL], egui::Button::new("Disadvantage")).clicked() {
            let f = app.dice_formula.replacen("1d20", "2d20kl1", 1).replacen("d20", "2d20kl1", 1);
            app.roll(&f, "disadvantage");
        }
        if ui.add_sized([w, theme::H_CTRL], egui::Button::new("Clear log")).clicked() {
            app.dice_log.clear();
        }
    });
    crate::ui::spells::mode_row(app, ui);
    if let Some(l) = app.dice_log.first() {
        section(ui, "Your last roll");
        ui.label(egui::RichText::new(l).size(19.0).color(theme::ORANGE));
    }
    if !app.table.is_empty() {
        section(ui, "At the table");
        let me = app.character.as_ref().map(|c| c.name.clone()).unwrap_or_default();
        for e in app.table.iter().rev().take(40) {
            ui.horizontal_wrapped(|ui| {
                ui.label(theme::muted(&e.time).small());
                if !e.who.is_empty() {
                    ui.label(egui::RichText::new(&e.who).strong().color(if e.who == me { theme::ORANGE } else { theme::PC_BLUE }));
                }
                ui.label(if e.kind == "turn" { theme::muted(&e.text) } else { egui::RichText::new(&e.text) });
            });
        }
    } else if app.dice_log.len() > 1 {
        section(ui, "Earlier");
        for l in app.dice_log.iter().skip(1) {
            ui.label(egui::RichText::new(l).monospace().size(14.0));
        }
    }
}

// ------------------------------------------------------------------ Settings

fn settings_tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    section(ui, "Connection");
    if let Some(i) = &app.info {
        ui.label(format!("Campaign: {}", i.campaign));
    }
    ui.label(theme::muted(&app.config.server_url));
    let can_choose = app.can_choose();
    ui.label(theme::muted(format!(
        "Signed in as {} · {}",
        app.config.player_name,
        if can_choose { "you picked this character" } else { "character assigned by the DM" }
    )));
    ui.horizontal(|ui| {
        if ui.button("Switch game / change name").clicked() {
            app.screen = Screen::Connect;
        }
        if can_choose && ui.button("Switch character").clicked() {
            let _ = app.client.tx.send(Cmd::LoadList);
            app.screen = Screen::Pick;
        }
        if ui.button("⟳ Refresh").clicked() {
            let _ = app.client.tx.send(Cmd::Refresh);
        }
    });
    if app.can_rebuild() {
        ui.add_space(4.0);
        if ui.button("✏  Rebuild this character").clicked() {
            app.rebuild_character();
        }
        ui.label(theme::muted("You made this character, so until the game starts you can change its build. Rebuilding starts it fresh: starting gear again, full health.").small());
    }
    section(ui, "Display");
    if ui.checkbox(&mut app.config.show_help, "Show the (i) explanations of game terms").changed() {
        dnd_mapview::help::set_enabled(app.config.show_help);
        app.store.save_config(&app.config);
    }
    ui.horizontal(|ui| {
        ui.label("Text size");
        let mut z = app.config.zoom;
        if ui.add(egui::Slider::new(&mut z, 0.7..=1.8).show_value(false)).changed() {
            app.config.zoom = z;
            ui.ctx().set_zoom_factor(z);
            app.store.save_config(&app.config);
        }
    });
    section(ui, "Sheet");
    if ui.button("Export printable sheet (HTML)").clicked() {
        if let Some(s) = &app.sheet {
            let lib = dnd_core::Library::new();
            let html = dnd_core::export::character_sheet_html(s, &lib);
            let path = app.store.export_path(&s.character_id);
            match std::fs::write(&path, html) {
                Ok(()) => {
                    #[cfg(not(target_os = "android"))]
                    let _ = open::that(&path);
                    app.set_status(format!("Saved {}", path.display()), false);
                }
                Err(e) => app.set_status(e.to_string(), true),
            }
        }
    }
    ui.label(theme::muted(format!("Data folder: {}", app.store.dir.display())).small());
    ui.label(theme::muted(format!("dnd-player v{}", env!("CARGO_PKG_VERSION"))).small());
}

/// Hit points and armor class in the header: visible on every tab, and the
/// way into the hit point panel.
fn vitals_button(ui: &mut egui::Ui, hp: i32, max: i32, temp: i32, ac: i32, open: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(168.0, 44.0), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let p = ui.painter();
    let col = if hp <= max / 4 { theme::BAD } else if hp <= max / 2 { theme::ORANGE } else { theme::GOOD };
    let split = rect.right() - 54.0;
    let hp_rect = egui::Rect::from_min_max(rect.min, egui::pos2(split - 4.0, rect.bottom()));
    let ac_rect = egui::Rect::from_min_max(egui::pos2(split, rect.top()), rect.max);
    p.rect(hp_rect, theme::R_CTRL, theme::CARD_HI, egui::Stroke::new(1.0, if open || resp.hovered() { theme::ORANGE } else { theme::EDGE }), egui::StrokeKind::Inside);
    p.text(hp_rect.left_center() + egui::vec2(9.0, -3.0), egui::Align2::LEFT_CENTER, "♥", egui::FontId::proportional(15.0), col);
    let label = if temp > 0 { format!("{hp}/{max} +{temp}") } else { format!("{hp} / {max}") };
    p.text(hp_rect.left_center() + egui::vec2(29.0, -3.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(17.0), col);
    // how full, as a bar along the bottom
    let bar = egui::Rect::from_min_max(egui::pos2(hp_rect.left() + 8.0, hp_rect.bottom() - 8.0), egui::pos2(hp_rect.right() - 8.0, hp_rect.bottom() - 4.0));
    p.rect_filled(bar, 2.0, theme::LINE);
    let frac = if max > 0 { (hp as f32 / max as f32).clamp(0.0, 1.0) } else { 0.0 };
    p.rect_filled(egui::Rect::from_min_max(bar.min, egui::pos2(bar.left() + bar.width() * frac, bar.bottom())), 2.0, col);
    p.rect(ac_rect, theme::R_CTRL, theme::CARD, egui::Stroke::new(1.0, theme::LINE), egui::StrokeKind::Inside);
    p.text(ac_rect.center() + egui::vec2(0.0, -11.0), egui::Align2::CENTER_CENTER, "AC", egui::FontId::proportional(10.5), theme::MUTED);
    p.text(ac_rect.center() + egui::vec2(0.0, 6.0), egui::Align2::CENTER_CENTER, ac.to_string(), egui::FontId::proportional(18.0), theme::TEXT);
    resp
}

/// What all the coins are worth, in gold pieces (1 pp = 10 gp, 1 ep = 1/2 gp,
/// 1 sp = 1/10 gp, 1 cp = 1/100 gp).
fn coins_total(c: &dnd_core::Currency) -> String {
    let cp = c.pp as i64 * 1000 + c.gp as i64 * 100 + c.ep as i64 * 50 + c.sp as i64 * 10 + c.cp as i64;
    let text = format!("{}.{:02}", cp / 100, cp % 100);
    format!("{} gp", text.trim_end_matches('0').trim_end_matches('.'))
}

/// The coins held, largest first, leaving out the kinds there are none of.
fn coins_line(c: &dnd_core::Currency) -> String {
    let parts: Vec<String> = [(c.pp, "pp"), (c.gp, "gp"), (c.ep, "ep"), (c.sp, "sp"), (c.cp, "cp")].iter().filter(|(n, _)| *n > 0).map(|(n, k)| format!("{n} {k}")).collect();
    if parts.is_empty() {
        "no coins".into()
    } else {
        parts.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coins_add_up_in_gold() {
        let c = dnd_core::Currency { cp: 7, sp: 4, ep: 3, gp: 12, pp: 2 };
        // 20 + 12 + 1.5 + 0.4 + 0.07
        assert_eq!(coins_total(&c), "33.97 gp");
        assert_eq!(coins_line(&c), "2 pp · 12 gp · 3 ep · 4 sp · 7 cp");
        assert_eq!(coins_total(&dnd_core::Currency { gp: 5, sp: 5, ..Default::default() }), "5.5 gp");
        assert_eq!(coins_total(&dnd_core::Currency { gp: 10, ..Default::default() }), "10 gp");
        assert_eq!(coins_total(&dnd_core::Currency::default()), "0 gp");
        assert_eq!(coins_line(&dnd_core::Currency::default()), "no coins");
    }
}
