//! The character sheet, tab by tab.

use crate::app::{PlayerApp, Screen, Tab};
use crate::client::Cmd;
use crate::theme::{self, section, tile};
use crate::ui::bonus;
use dnd_core::{Character, Sheet};

pub fn show(app: &mut PlayerApp, ctx: &egui::Context) {
    let (inset_top, inset_bottom) = crate::system_insets(ctx);
    // ---- header
    egui::TopBottomPanel::top("sheet_top").frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin { left: 12, right: 12, top: 8 + inset_top as i8, bottom: 8 })).show(ctx, |ui| {
        ui.horizontal(|ui| {
            let (name, sub) = match &app.sheet {
                Some(s) => (s.name.clone(), format!("{} · {} · L{}", s.species, s.class_line, s.level)),
                None => (app.config.character_id.clone(), "loading…".into()),
            };
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(name).strong().size(20.0).color(theme::ORANGE));
                ui.label(theme::muted(sub).small());
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dot = if app.online { ("⏺", theme::GOOD, "connected") } else { ("○", theme::MUTED, "offline (cached)") };
                ui.label(egui::RichText::new(dot.0).color(dot.1)).on_hover_text(dot.2);
                if app.busy {
                    ui.spinner();
                }
                if app.last_edit.is_some() {
                    ui.label(theme::accent("⏺").small());
                }
            });
        });
        if !app.status.is_empty() && app.status_at.elapsed().as_secs() < 12 {
            ui.colored_label(if app.status_error { theme::BAD } else { theme::GOOD }, egui::RichText::new(&app.status).small());
        }
        if app.my_turn() && app.tab != Tab::Combat {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("▶ Your turn").strong().color(theme::ORANGE));
                if ui.small_button("End turn").clicked() {
                    app.act(Cmd::Do { action: "end_turn".into(), arg: String::new() });
                }
            });
        }
    });

    // ---- bottom tab bar
    egui::TopBottomPanel::bottom("tabs").frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin { left: 4, right: 4, top: 6, bottom: 6 + inset_bottom as i8 })).show(ctx, |ui| {
        let w = ui.available_width() / Tab::ALL.len() as f32;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for t in Tab::ALL {
                let sel = app.tab == t;
                let col = if sel { theme::ORANGE } else { theme::MUTED };
                let text = egui::RichText::new(format!("{}\n{}", t.icon(), t.label())).color(col).size(if sel { 13.0 } else { 12.0 });
                let b = egui::Button::new(text).fill(if sel { theme::ORANGE_FAINT } else { egui::Color32::TRANSPARENT }).min_size(egui::vec2(w, 50.0)).frame(sel);
                if ui.add_sized([w, 50.0], b).clicked() {
                    app.tab = t;
                }
            }
        });
    });

    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(12.0)).show(ctx, |ui| {
        if app.sheet.is_none() || app.character.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.spinner();
                ui.label(theme::muted("Fetching your sheet from the DM…"));
                if app.can_choose() && ui.button("Pick a different character").clicked() {
                    app.screen = Screen::Pick;
                }
                if ui.button("Change server / name").clicked() {
                    app.screen = Screen::Connect;
                }
            });
            return;
        }
        if app.tab == Tab::Party {
            // the map pans and zooms by itself, so it does not sit in a scroll area
            crate::ui::map::show(app, ui);
            return;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match app.tab {
            Tab::Stats => stats_tab(app, ui),
            Tab::Combat => combat_tab(app, ui),
            Tab::Spells => crate::ui::spells::tab(app, ui),
            Tab::Items => items_tab(app, ui),
            Tab::Notes => notes_tab(app, ui),
            Tab::Party => {}
            Tab::Dice => dice_tab(app, ui),
            Tab::Settings => settings_tab(app, ui),
        });
    });
}

fn sheet(app: &PlayerApp) -> Sheet {
    app.sheet.clone().unwrap()
}

fn tiles_row(ui: &mut egui::Ui, items: &[(&str, String, String)]) {
    let n = items.len().max(1) as f32;
    // item spacing between tiles plus 1 px stroke on each side of every tile
    let w = ((ui.available_width() - 8.0 * (n - 1.0)) / n - 3.0).max(60.0);
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
    let ab: Vec<(&str, String, String)> = s.abilities.iter().map(|(k, v)| (k, bonus(dnd_core::ability_mod(v)), v.to_string())).collect();
    let a: Vec<(&str, String, String)> = ab[..3].iter().map(|(k, m, v)| (*k, m.clone(), v.clone())).collect();
    let b: Vec<(&str, String, String)> = ab[3..].iter().map(|(k, m, v)| (*k, m.clone(), v.clone())).collect();
    tiles_row(ui, &a);
    tiles_row(ui, &b);
    tiles_row(
        ui,
        &[
            ("Armor", s.ac.to_string(), String::new()),
            ("Init", bonus(s.initiative), String::new()),
            ("Speed", format!("{}", s.speed), "ft".into()),
            ("Prof", bonus(s.proficiency_bonus), String::new()),
        ],
    );
    hp_block(app, ui);

    let s = sheet(app);
    section(ui, "Saving throws");
    egui::Grid::new("saves").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
        for (i, sv) in s.saves.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(if sv.proficient { theme::accent("⏺") } else { theme::muted("○") });
                ui.label(&sv.name[..3].to_uppercase());
                ui.label(egui::RichText::new(bonus(sv.bonus)).strong());
                if !sv.notes.is_empty() {
                    ui.label(theme::muted("*").small()).on_hover_text(sv.notes.join("; "));
                }
            });
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });
    let save_rolls: Vec<(String, i32)> = s.saves.iter().map(|x| (x.name.clone(), x.bonus)).collect();
    ui.horizontal_wrapped(|ui| {
        for (name, b) in save_rolls {
            if ui.small_button(format!("🎲 {}", &name[..3])).clicked() {
                app.roll(&format!("1d20 {}", signed(b)), &format!("{name} save"));
            }
        }
    });

    section(ui, "Skills");
    let skills: Vec<_> = s.skills.clone();
    for sk in skills {
        ui.horizontal(|ui| {
            ui.label(if sk.expertise { theme::accent("★") } else if sk.proficient { theme::accent("⏺") } else { theme::muted("○") });
            let r = ui.add(egui::Label::new(format!("{}  {}", sk.name, theme_dim(&sk.ability))).sense(egui::Sense::click()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(egui::RichText::new(bonus(sk.bonus)).strong()).clicked() || r.clicked() {
                    app.roll(&format!("1d20 {}", signed(sk.bonus)), &sk.name);
                }
                if !sk.notes.is_empty() {
                    ui.label(theme::muted(sk.notes.join("; ")).small());
                }
            });
        });
    }
    ui.label(theme::muted(format!("Passive Perception {} · Investigation {} · Insight {}", s.passive_perception, s.passive_investigation, s.passive_insight)).small());

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
    theme::card_hi().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(theme::muted("HIT POINTS").small());
                let col = if ch.hp.current <= hp_max / 4 { theme::BAD } else if ch.hp.current <= hp_max / 2 { theme::ORANGE } else { theme::GOOD };
                ui.label(egui::RichText::new(format!("{} / {}", ch.hp.current, hp_max)).color(col).strong().size(30.0));
                if ch.hp.temp > 0 {
                    ui.label(theme::muted(format!("+{} temp", ch.hp.temp)).small());
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
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.label(theme::muted("Death saves"));
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
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut ch.inspiration, "Inspiration");
        });
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
    hp_block(app, ui);
    if app.my_turn() {
        theme::card_hi().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Your turn").strong().size(18.0).color(theme::ORANGE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(theme::primary("End turn")).clicked() {
                        app.act(Cmd::Do { action: "end_turn".into(), arg: String::new() });
                    }
                });
            });
        });
    }
    section(ui, "Attacks");
    crate::ui::spells::mode_row(app, ui);
    crate::ui::spells::target_row(app, ui);
    let target = app.targets.first().cloned();
    if target.is_none() && app.encounter.as_ref().map(|e| !e.combatants.is_empty()).unwrap_or(false) {
        ui.label(theme::muted("Pick a target and the attack is rolled against it. Without one you just roll the dice.").small());
    }
    let attacks = s.attacks.clone();
    for (i, a) in attacks.into_iter().enumerate() {
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 150.0).max(120.0));
                    ui.label(egui::RichText::new(&a.name).strong().size(17.0));
                    ui.label(theme::muted(format!("{} to hit · {} {} · {}{}", bonus(a.bonus), a.damage, a.damage_type, a.range, if a.notes.is_empty() { String::new() } else { format!(" · {}", a.notes.join("; ")) })).small());
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
        ui.horizontal_wrapped(|ui| {
            for c in known {
                let mut on = ch.conditions.iter().any(|x| x == c);
                if ui.toggle_value(&mut on, c).changed() {
                    if on {
                        ch.conditions.push(c.to_string());
                    } else {
                        ch.conditions.retain(|x| x != c);
                    }
                }
            }
            ui.label(theme::muted("Exhaustion"));
            ui.add(egui::DragValue::new(&mut ch.exhaustion).range(0..=6));
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
                if ui.add_enabled(online && used < *n && ch.hp.current < s.hp_max, egui::Button::new(format!("Spend a {d}"))).on_hover_text("During a short rest: roll it, add Constitution, heal that much").clicked() {
                    todo = Some(("hit_die", d.clone()));
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(online, egui::Button::new("☕ Short rest")).clicked() {
                todo = Some(("short_rest", String::new()));
            }
            if ui.add_enabled(online, egui::Button::new("🌙 Long rest")).on_hover_text("Full hit points, all spell slots, half your hit dice").clicked() {
                todo = Some(("long_rest", String::new()));
            }
        });
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
            ("Gold", s.currency.gp.to_string(), format!("{} sp · {} cp", s.currency.sp, s.currency.cp)),
        ],
    );
    let Some(ch) = app.character.as_mut() else { return };
    section(ui, "Inventory");
    let names: std::collections::HashMap<String, (String, String)> = s.inventory.iter().map(|l| (l.item.clone(), (l.name.clone(), l.kind.clone()))).collect();
    let mut remove = None;
    for (i, inv) in ch.inventory.iter_mut().enumerate() {
        let (name, kind) = names.get(&inv.item).cloned().unwrap_or((if inv.name.is_empty() { inv.item.clone() } else { inv.name.clone() }, "custom".into()));
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&name).strong());
                    ui.label(theme::muted(format!("{kind}{}", if inv.notes.is_empty() { String::new() } else { format!(" · {}", inv.notes) })).small());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✖").clicked() {
                        remove = Some(i);
                    }
                    ui.add(egui::DragValue::new(&mut inv.qty).range(0..=999));
                    ui.label(theme::muted("×"));
                    ui.toggle_value(&mut inv.equipped, "equipped");
                    if inv.attuned || kind == "wondrous" {
                        ui.toggle_value(&mut inv.attuned, "attuned");
                    }
                });
            });
        });
    }
    if let Some(i) = remove {
        ch.inventory.remove(i);
    }
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.ui_state.hp_input).hint_text("add a custom item (e.g. “goblin ear”)").desired_width(f32::INFINITY));
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
    let Some(ch) = app.character.as_mut() else { return };
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
        let r = ui.add(egui::TextEdit::singleline(&mut app.dice_formula).desired_width(ui.available_width() - 90.0).font(egui::TextStyle::Heading));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.add(theme::primary("Roll")).clicked() || enter {
            let f = app.dice_formula.clone();
            app.roll(&f, "");
        }
    });
    ui.horizontal_wrapped(|ui| {
        for d in ["d20", "d12", "d10", "d8", "d6", "d4", "d100", "2d20kh1", "2d20kl1", "4d6kh3"] {
            if ui.add(theme::big_button(d)).clicked() {
                app.roll(d, "");
            }
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Advantage").clicked() {
            let f = app.dice_formula.replacen("1d20", "2d20kh1", 1).replacen("d20", "2d20kh1", 1);
            app.roll(&f, "advantage");
        }
        if ui.button("Disadvantage").clicked() {
            let f = app.dice_formula.replacen("1d20", "2d20kl1", 1).replacen("d20", "2d20kl1", 1);
            app.roll(&f, "disadvantage");
        }
        if ui.button("Clear log").clicked() {
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
        if ui.button("Change server / name").clicked() {
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
