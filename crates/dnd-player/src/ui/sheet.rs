//! The character sheet, tab by tab.

use crate::app::{PlayerApp, Screen, Tab};
use crate::client::Cmd;
use crate::theme::{self, section, tile};
use crate::ui::bonus;
use dnd_core::api::PlayerEncounter;
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
                let dot = if app.online { ("●", theme::GOOD, "connected") } else { ("○", theme::MUTED, "offline (cached)") };
                ui.label(egui::RichText::new(dot.0).color(dot.1)).on_hover_text(dot.2);
                if app.busy {
                    ui.spinner();
                }
                if app.last_edit.is_some() {
                    ui.label(theme::accent("●").small());
                }
            });
        });
        if !app.status.is_empty() && app.status_at.elapsed().as_secs() < 6 {
            ui.colored_label(if app.status_error { theme::BAD } else { theme::GOOD }, egui::RichText::new(&app.status).small());
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
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match app.tab {
            Tab::Stats => stats_tab(app, ui),
            Tab::Combat => combat_tab(app, ui),
            Tab::Spells => spells_tab(app, ui),
            Tab::Items => items_tab(app, ui),
            Tab::Notes => notes_tab(app, ui),
            Tab::Party => party_tab(app, ui),
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
                ui.label(if sv.proficient { theme::accent("●") } else { theme::muted("○") });
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
                app.tab = Tab::Dice;
            }
        }
    });

    section(ui, "Skills");
    let skills: Vec<_> = s.skills.clone();
    for sk in skills {
        ui.horizontal(|ui| {
            ui.label(if sk.expertise { theme::accent("★") } else if sk.proficient { theme::accent("●") } else { theme::muted("○") });
            let r = ui.add(egui::Label::new(format!("{}  {}", sk.name, theme_dim(&sk.ability))).sense(egui::Sense::click()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(egui::RichText::new(bonus(sk.bonus)).strong()).clicked() || r.clicked() {
                    app.roll(&format!("1d20 {}", signed(sk.bonus)), &sk.name);
                    app.tab = Tab::Dice;
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
        ui.horizontal(|ui| {
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
        ui.horizontal(|ui| {
            ui.label(theme::muted("Death saves"));
            ui.label("✓");
            for i in 0..3 {
                let mut on = ch.death_saves.successes > i;
                if ui.checkbox(&mut on, "").changed() {
                    ch.death_saves.successes = if on { i + 1 } else { i };
                }
            }
            ui.label("✗");
            for i in 0..3 {
                let mut on = ch.death_saves.failures > i;
                if ui.checkbox(&mut on, "").changed() {
                    ch.death_saves.failures = if on { i + 1 } else { i };
                }
            }
            ui.checkbox(&mut ch.inspiration, "Inspiration");
        });
    });
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
    section(ui, "Attacks");
    let attacks = s.attacks.clone();
    for a in attacks {
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&a.name).strong().size(17.0));
                    ui.label(theme::muted(format!("{} {} · {}{}", a.damage, a.damage_type, a.range, if a.notes.is_empty() { String::new() } else { format!(" · {}", a.notes.join("; ")) })).small());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(theme::primary(&format!("Hit {}", bonus(a.bonus)))).clicked() {
                        app.roll(&format!("1d20 {}", signed(a.bonus)), &format!("{} attack", a.name));
                        app.tab = Tab::Dice;
                    }
                    if ui.add(theme::big_button("Dmg")).clicked() {
                        app.roll(&a.damage, &format!("{} damage", a.name));
                        app.tab = Tab::Dice;
                    }
                });
            });
        });
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
        section(ui, "Hit dice");
        ui.horizontal_wrapped(|ui| {
            for (d, n) in &s.hit_dice {
                let used = ch.hp.hit_dice_used.entry(d.clone()).or_insert(0);
                ui.label(format!("{d}: {} of {n} left", n.saturating_sub(*used)));
                if ui.add_enabled(*used < *n, egui::Button::new("Spend")).clicked() {
                    *used += 1;
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Short rest").clicked() {
                // nothing automatic besides nothing; players spend dice above
            }
            if ui.button("Long rest").clicked() {
                ch.hp.current = s.hp_max;
                ch.hp.temp = 0;
                ch.spells.slots_used = vec![0; 9];
                ch.spells.pact_slots_used = 0;
                ch.death_saves = Default::default();
                let half: Vec<(String, u32)> = s.hit_dice.iter().map(|(d, n)| (d.clone(), (*n).div_ceil(2))).collect();
                for (d, regain) in half {
                    if let Some(u) = ch.hp.hit_dice_used.get_mut(&d) {
                        *u = u.saturating_sub(regain);
                    }
                }
            }
        });
    }
}

// ------------------------------------------------------------------ Spells

fn spells_tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    let s = sheet(app);
    if s.spellcasting.is_empty() {
        ui.label(theme::muted("No spellcasting on this character."));
        return;
    }
    for sc in &s.spellcasting {
        tiles_row(
            ui,
            &[
                (&sc.class_name, sc.ability.to_uppercase(), "ability".into()),
                ("Attack", bonus(sc.attack_bonus), String::new()),
                ("Save DC", sc.save_dc.to_string(), String::new()),
                ("Prepare", sc.prepared_max.map(|p| p.to_string()).unwrap_or("—".into()), String::new()),
            ],
        );
    }
    let Some(ch) = app.character.as_mut() else { return };
    section(ui, "Spell slots (tap to spend / restore)");
    ch.spells.slots_used.resize(9, 0);
    for (i, n) in s.spell_slots.iter().enumerate() {
        if *n == 0 {
            continue;
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("L{}", i + 1)).strong());
            for k in 0..*n {
                let used = k < ch.spells.slots_used[i];
                let txt = if used { "○" } else { "●" };
                if ui.add(egui::Button::new(egui::RichText::new(txt).size(22.0).color(if used { theme::MUTED } else { theme::ORANGE })).frame(false)).clicked() {
                    ch.spells.slots_used[i] = if used { k } else { k + 1 };
                }
            }
        });
    }
    if let Some((n, l)) = s.pact_slots {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("Pact L{l}")).strong());
            for k in 0..n {
                let used = k < ch.spells.pact_slots_used;
                if ui.add(egui::Button::new(egui::RichText::new(if used { "○" } else { "●" }).size(22.0).color(if used { theme::MUTED } else { theme::ORANGE })).frame(false)).clicked() {
                    ch.spells.pact_slots_used = if used { k } else { k + 1 };
                }
            }
        });
    }
    section(ui, "Spells");
    ui.label(theme::muted("Known and prepared spells are managed on the DM's sheet; tap a spell to toggle prepared.").small());
    let mut known = ch.spells.known.clone();
    known.sort();
    for id in known {
        let prepared = ch.spells.prepared.contains(&id);
        let label = id.replace('_', " ");
        ui.horizontal(|ui| {
            let mut p = prepared;
            if ui.checkbox(&mut p, "").changed() {
                if p {
                    ch.spells.prepared.push(id.clone());
                } else {
                    ch.spells.prepared.retain(|x| x != &id);
                }
            }
            ui.label(if prepared { egui::RichText::new(label).strong() } else { theme::muted(label) });
        });
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

// ------------------------------------------------------------------ Party / encounter

fn party_tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    let Some(enc) = app.encounter.clone() else {
        ui.label(theme::muted("No encounter data yet."));
        return;
    };
    section(ui, if enc.active { "Initiative" } else { "Party" });
    if enc.active {
        ui.label(theme::accent(format!("Round {}", enc.round)));
    }
    for c in &enc.combatants {
        let frame = if c.is_current { theme::card_hi() } else { theme::card() };
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{:>2}", c.initiative)).monospace().color(theme::MUTED));
                let name = egui::RichText::new(&c.name).strong().color(if c.is_pc { theme::PC_BLUE } else { theme::TEXT });
                ui.label(name);
                if c.is_current {
                    ui.label(theme::accent("◀ now"));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let (Some(hp), Some(max)) = (c.hp, c.hp_max) {
                        ui.label(theme::muted(format!("{hp}/{max}")));
                    }
                    if !c.conditions.is_empty() {
                        ui.label(theme::muted(c.conditions.join(", ")).small());
                    }
                });
            });
        });
    }
    section(ui, "Map");
    map_view(ui, &enc, app.config.character_id.as_str());
}

fn map_view(ui: &mut egui::Ui, enc: &PlayerEncounter, me: &str) {
    let m = &enc.map;
    let avail = ui.available_width().max(100.0);
    let cell = (avail / m.width.max(1) as f32).clamp(6.0, 40.0);
    let size = egui::vec2(m.width as f32 * cell + 1.0, m.height as f32 * cell + 1.0);
    egui::ScrollArea::horizontal().show(ui, |ui| {
        let (resp, p) = ui.allocate_painter(size, egui::Sense::hover());
        let o = resp.rect.min;
        p.rect_filled(resp.rect, 0.0, egui::Color32::from_rgb(40, 42, 46));
        for (k, v) in &m.marks {
            if let Some((x, y)) = k.split_once(',') {
                if let (Ok(x), Ok(y)) = (x.parse::<i32>(), y.parse::<i32>()) {
                    let r = egui::Rect::from_min_size(egui::pos2(o.x + x as f32 * cell, o.y + y as f32 * cell), egui::vec2(cell, cell));
                    let col = match v.as_str() {
                        "wall" => egui::Color32::from_rgb(90, 90, 100),
                        "water" => egui::Color32::from_rgb(40, 80, 140),
                        _ => egui::Color32::from_rgb(80, 65, 45),
                    };
                    p.rect_filled(r, 0.0, col);
                }
            }
        }
        let line = egui::Stroke::new(1.0, egui::Color32::from_gray(70));
        for x in 0..=m.width {
            let sx = o.x + x as f32 * cell;
            p.line_segment([egui::pos2(sx, o.y), egui::pos2(sx, o.y + m.height as f32 * cell)], line);
        }
        for y in 0..=m.height {
            let sy = o.y + y as f32 * cell;
            p.line_segment([egui::pos2(o.x, sy), egui::pos2(o.x + m.width as f32 * cell, sy)], line);
        }
        for c in &enc.combatants {
            let Some(pos) = c.pos else { continue };
            let side = match c.size.to_lowercase().as_str() {
                "large" => 2.0,
                "huge" => 3.0,
                "gargantuan" => 4.0,
                _ => 1.0,
            };
            let r = egui::Rect::from_min_size(egui::pos2(o.x + pos.x as f32 * cell, o.y + pos.y as f32 * cell), egui::vec2(cell * side, cell * side));
            let col = if c.is_pc { theme::PC_BLUE } else { egui::Color32::from_rgb(200, 70, 70) };
            p.circle_filled(r.center(), r.width() * 0.45, col);
            if c.id == me || c.name.to_lowercase() == me.replace('_', " ") {
                p.circle_stroke(r.center(), r.width() * 0.5, egui::Stroke::new(2.0, theme::ORANGE));
            }
            if c.is_current {
                p.circle_stroke(r.center(), r.width() * 0.55, egui::Stroke::new(2.0, egui::Color32::from_rgb(240, 200, 60)));
            }
            let initials: String = c.name.split_whitespace().filter_map(|s| s.chars().next()).take(2).collect();
            p.text(r.center(), egui::Align2::CENTER_CENTER, initials, egui::FontId::proportional(cell * 0.5), egui::Color32::WHITE);
        }
    });
    ui.label(theme::muted("The DM moves tokens; this view follows live.").small());
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
    section(ui, "Results");
    for (i, l) in app.dice_log.iter().enumerate() {
        let big = i == 0;
        ui.label(egui::RichText::new(l).monospace().size(if big { 20.0 } else { 14.0 }).color(if big { theme::ORANGE } else { theme::TEXT }));
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
