//! Spells: the Spells tab of the sheet (slots, prepare, cast, learn) and
//! the spell section of the character builder.

use crate::app::PlayerApp;
use crate::client::Cmd;
use crate::theme::{self, section};
use crate::ui::bonus;
use dnd_core::play;
use dnd_core::{Character, Library, Sheet, Spell};

fn level_name(l: u32) -> String {
    if l == 0 {
        "Cantrips".into()
    } else {
        format!("Level {l}")
    }
}

/// "1 action · 60 feet · V, S · concentration"
fn facts(s: &Spell) -> String {
    let mut v: Vec<String> = Vec::new();
    for p in [&s.casting_time, &s.range, &s.components, &s.duration] {
        if !p.is_empty() {
            v.push(p.clone());
        }
    }
    if s.ritual {
        v.push("ritual".into());
    }
    v.join(" · ")
}

/// What the spell does in numbers, for the row under its name.
fn effect(s: &Spell) -> String {
    let mut v: Vec<String> = Vec::new();
    if s.attack {
        v.push("spell attack".into());
    }
    if !s.save.is_empty() {
        v.push(format!("{} save{}", s.save.to_uppercase(), if s.save_half { " for half" } else { "" }));
    }
    if !s.damage.is_empty() {
        v.push(format!("{} {}", s.damage, s.damage_type));
    }
    if !s.heal.is_empty() {
        v.push(format!("heals {}", s.heal.replace("mod", "your modifier")));
    }
    if s.concentration {
        v.push("concentration".into());
    }
    v.join(" · ")
}

fn details(ui: &mut egui::Ui, s: &Spell) {
    ui.label(theme::muted(format!("{}{}", if s.school.is_empty() { String::new() } else { format!("{} · ", s.school) }, facts(s))).small());
    ui.horizontal_wrapped(|ui| {
        if s.level == 0 {
            dnd_mapview::help::label_term(ui, theme::muted("cantrip").small(), "cantrip");
        }
        if s.concentration {
            dnd_mapview::help::label_term(ui, theme::muted("concentration").small(), "concentration");
        }
        if s.ritual {
            dnd_mapview::help::label_term(ui, theme::muted("ritual").small(), "ritual");
        }
        dnd_mapview::help::label_term(ui, theme::muted("components").small(), "components");
        if !s.higher_levels.is_empty() {
            dnd_mapview::help::label_term(ui, theme::muted("upcasting").small(), "upcast");
        }
    });
    ui.label(&s.description);
    if !s.higher_levels.is_empty() {
        ui.label(theme::muted(format!("Higher levels: {}", s.higher_levels)).small());
    }
}

/// Spell slots of this level still unspent (pact slots count at their level).
fn slots_left(sheet: &Sheet, ch: &Character, level: u32) -> u32 {
    let i = level as usize - 1;
    let mut n = sheet.spell_slots.get(i).copied().unwrap_or(0).saturating_sub(ch.spells.slots_used.get(i).copied().unwrap_or(0));
    if let Some((count, l)) = sheet.pact_slots {
        if l == level {
            n += count.saturating_sub(ch.spells.pact_slots_used);
        }
    }
    n
}

/// Normal / advantage / disadvantage for the next d20.
pub fn mode_row(app: &mut PlayerApp, ui: &mut egui::Ui) {
    use dnd_core::play::RollMode::*;
    ui.horizontal(|ui| {
        ui.label(theme::muted("Next d20"));
        dnd_mapview::help::term(ui, "roll_mode");
        for (m, label) in [(Normal, "normal"), (Advantage, "advantage"), (Disadvantage, "disadvantage")] {
            ui.selectable_value(&mut app.mode, m, label);
        }
    });
}

/// Pick who an attack or spell is aimed at, from the combatants in view:
/// foes first and nearest first, then friends, then whoever is down.
/// `with_self` adds the character itself (spells can be cast on yourself).
pub fn target_row(app: &mut PlayerApp, ui: &mut egui::Ui, with_self: bool) {
    let Some(enc) = &app.encounter else { return };
    if enc.combatants.is_empty() {
        return;
    }
    let me = app.my_token();
    let down_ids: Vec<String> = app.map.as_ref().map(|m| m.tokens.iter().filter(|t| t.down).map(|t| t.id.clone()).collect()).unwrap_or_default();
    let feet = |id: &String| -> Option<u32> {
        let m = me.as_ref()?;
        if m == id {
            return None;
        }
        app.map.as_ref()?.distance_ft(m, id)
    };
    // (id, name, is a player character, feet away, down)
    let mut list: Vec<(String, String, bool, Option<u32>, bool)> = enc
        .combatants
        .iter()
        .filter(|c| with_self || Some(&c.id) != me.as_ref())
        .map(|c| {
            let down = c.hp.map(|h| h <= 0).unwrap_or(false) || c.conditions.iter().any(|x| x == "dead" || x == "unconscious") || down_ids.contains(&c.id);
            (c.id.clone(), c.name.clone(), c.is_pc, feet(&c.id), down)
        })
        .collect();
    list.sort_by_key(|(id, _, pc, d, down)| (*down, *pc && Some(id) != me.as_ref(), Some(id) == me.as_ref(), d.unwrap_or(u32::MAX)));
    // a target that left or went down is no longer aimed at
    app.targets.retain(|t| list.iter().any(|c| &c.0 == t && !c.4));
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::muted("Target"));
        dnd_mapview::help::term(ui, "target");
        for (id, name, pc, d, down) in &list {
            let on = app.targets.contains(id);
            let label = match (down, d) {
                (true, _) => format!("{name} · down"),
                (false, Some(d)) => format!("{name} · {d} ft"),
                (false, None) if Some(id) == me.as_ref() => format!("{name} (you)"),
                (false, None) => name.clone(),
            };
            let text = egui::RichText::new(label).color(if on { egui::Color32::BLACK } else if *down { theme::MUTED } else if *pc { theme::PC_BLUE } else { theme::TEXT });
            if ui.add(egui::Button::new(text).fill(if on { theme::ORANGE } else { theme::CARD })).clicked() {
                if on {
                    app.targets.retain(|t| t != id);
                } else {
                    app.targets.push(id.clone());
                }
            }
        }
    });
}

// ------------------------------------------------------------------ sheet tab

pub fn tab(app: &mut PlayerApp, ui: &mut egui::Ui) {
    let Some(s) = app.sheet.clone() else { return };
    if s.spellcasting.is_empty() {
        ui.label(theme::muted("No spellcasting on this character."));
        return;
    }
    // the casting numbers in a line: the room goes to the spells
    for sc in &s.spellcasting {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.label(theme::muted(format!("{} ({})", sc.class_name, sc.ability.to_uppercase())).small());
            ui.label(theme::muted("· attack").small());
            ui.label(egui::RichText::new(bonus(sc.attack_bonus)).strong().color(theme::ORANGE));
            dnd_mapview::help::term(ui, "Attack");
            ui.label(theme::muted("· save DC").small());
            ui.label(egui::RichText::new(sc.save_dc.to_string()).strong().color(theme::ORANGE));
            dnd_mapview::help::term(ui, "Save DC");
            if let Some(p) = sc.prepared_max {
                ui.label(theme::muted("· prepare").small());
                ui.label(egui::RichText::new(p.to_string()).strong().color(theme::ORANGE));
                dnd_mapview::help::term(ui, "Prepare");
            }
        });
    }
    slots(app, ui, &s);
    match app.builder_lib.take() {
        Some(lib) => {
            book(app, ui, &s, &lib);
            app.builder_lib = Some(lib);
        }
        None => {
            ui.label(theme::muted("The spell list arrives from the DM the first time this device connects."));
        }
    }
}

fn slots(app: &mut PlayerApp, ui: &mut egui::Ui, s: &Sheet) {
    let Some(ch) = app.character.as_mut() else { return };
    if s.spell_slots.iter().all(|n| *n == 0) && s.pact_slots.is_none() {
        return;
    }
    section(ui, "Spell slots");
    ch.spells.slots_used.resize(9, 0);
    for (i, n) in s.spell_slots.iter().enumerate() {
        if *n == 0 {
            continue;
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("L{}", i + 1)).strong());
            for k in 0..*n {
                let used = k < ch.spells.slots_used[i];
                let txt = if used { "○" } else { "⏺" };
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
                if ui.add(egui::Button::new(egui::RichText::new(if used { "○" } else { "⏺" }).size(22.0).color(if used { theme::MUTED } else { theme::ORANGE })).frame(false)).clicked() {
                    ch.spells.pact_slots_used = if used { k } else { k + 1 };
                }
            }
        });
    }
    ui.label(theme::muted("Casting spends a slot by itself. Tap a slot to fix it by hand.").small());
}

fn book(app: &mut PlayerApp, ui: &mut egui::Ui, s: &Sheet, lib: &Library) {
    let Some(ch) = app.character.clone() else { return };
    let live = app.game_live();
    mode_row(app, ui);
    target_row(app, ui, true);

    let prepares = s.spellcasting.iter().any(|b| b.prepared_max.is_some());
    let prepared_max: i32 = s.spellcasting.iter().filter_map(|b| b.prepared_max).sum();
    let prepared_now = ch.spells.prepared.iter().filter(|p| lib.spells.get(*p).map(|x| x.level > 0).unwrap_or(true)).count() as i32;
    let mut available: Vec<&Spell> = play::spells_available(lib, &ch, s).iter().filter_map(|id| lib.spells.get(id)).collect();
    available.sort_by(|a, b| a.level.cmp(&b.level).then(a.name.cmp(&b.name)));
    if available.is_empty() {
        ui.add_space(6.0);
        ui.label(theme::muted("No spells yet. Choose some below."));
    }
    let mut cast: Option<(String, u32)> = None;
    let mut toggle: Option<String> = None;
    let mut forget: Option<String> = None;
    let mut last_level = u32::MAX;
    for sp in &available {
        if sp.level != last_level {
            last_level = sp.level;
            let title = if sp.level == 0 {
                "Cantrips".to_string()
            } else {
                format!("Level {} · {} slot(s) left", sp.level, slots_left(s, &ch, sp.level))
            };
            section(ui, &title);
            if sp.level == 1 && prepares {
                let col = if prepared_now > prepared_max { theme::BAD } else { theme::MUTED };
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(format!("Prepared {prepared_now} of {prepared_max}. Tick the spells you have ready today.")).small().color(col));
                    dnd_mapview::help::term(ui, "prepared");
                });
            }
        }
        let is_prepared = ch.spells.prepared.contains(&sp.id);
        // granted by a feature (domain, oath, circle, racial magic): always ready
        let granted = s.spells_granted.contains(&sp.id);
        let ready = sp.level == 0 || !prepares || is_prepared || granted;
        let open = app.ui_state.spell_open == sp.id;
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if granted && sp.level > 0 {
                    ui.label(theme::accent("always").small());
                    dnd_mapview::help::term(ui, "always_prepared");
                } else if sp.level > 0 && prepares {
                    let mut p = is_prepared;
                    if ui.add_enabled(is_prepared || prepared_now < prepared_max, egui::Checkbox::new(&mut p, "")).changed() {
                        toggle = Some(sp.id.clone());
                    }
                }
                ui.vertical(|ui| {
                    let name = egui::RichText::new(&sp.name).size(16.0);
                    let r = ui.add(egui::Label::new(if ready { name.strong() } else { name.color(theme::MUTED) }).sense(egui::Sense::click()));
                    let e = effect(sp);
                    let r2 = ui.add(egui::Label::new(theme::muted(if e.is_empty() { facts(sp) } else { e }).small()).sense(egui::Sense::click()));
                    if r.clicked() || r2.clicked() {
                        app.ui_state.spell_open = if open { String::new() } else { sp.id.clone() };
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if sp.level == 0 {
                        if ui.add(theme::primary("Cast")).clicked() {
                            cast = Some((sp.id.clone(), 0));
                        }
                    } else if ready || sp.ritual {
                        let lowest = (sp.level..=9).find(|l| slots_left(s, &ch, *l) > 0);
                        let higher: Vec<u32> = (sp.level..=9).filter(|l| Some(*l) != lowest && slots_left(s, &ch, *l) > 0).collect();
                        if !higher.is_empty() || sp.ritual {
                            ui.menu_button("⬆", |ui| {
                                for l in &higher {
                                    if ui.button(format!("Level {l} slot ({} left)", slots_left(s, &ch, *l))).clicked() {
                                        cast = Some((sp.id.clone(), *l));
                                        ui.close_menu();
                                    }
                                }
                                if sp.ritual && ui.button("As a ritual (no slot, 10 minutes)").clicked() {
                                    cast = Some((sp.id.clone(), 0));
                                    ui.close_menu();
                                }
                            });
                        }
                        if ready {
                            let label = match lowest {
                                Some(l) if l > sp.level => format!("Cast L{l}"),
                                _ => "Cast".to_string(),
                            };
                            if ui.add_enabled(lowest.is_some(), theme::primary(&label)).on_disabled_hover_text("No spell slot left").clicked() {
                                cast = lowest.map(|l| (sp.id.clone(), l));
                            }
                        }
                    }
                });
            });
            if open {
                ui.separator();
                details(ui, sp);
                let chosen = ch.spells.known.contains(&sp.id);
                if chosen && !live && ui.small_button("Forget this spell").on_hover_text("Between games you may swap the spells you chose").clicked() {
                    forget = Some(sp.id.clone());
                }
            }
        });
    }
    // spells the sheet lists that this app has no text for (DM-only content)
    for id in ch.spells.known.iter().filter(|id| !lib.spells.contains_key(*id)) {
        ui.label(theme::muted(format!("{} (ask the DM what it does)", id.replace('_', " "))));
    }

    if let Some(id) = toggle {
        if let Some(c) = app.character.as_mut() {
            if c.spells.prepared.contains(&id) {
                c.spells.prepared.retain(|p| p != &id);
            } else {
                c.spells.prepared.push(id);
            }
        }
    }
    if let Some((spell, slot)) = cast {
        let (targets, mode) = (app.targets.clone(), app.mode);
        app.act(Cmd::Cast { spell, slot, targets, mode });
    }
    if let Some(id) = forget {
        let known: Vec<String> = ch.spells.known.iter().filter(|k| *k != &id).cloned().collect();
        app.act(Cmd::ChooseSpells(known));
    }

    // ---- beyond the rules, when the DM allows it
    let free = app.seat.as_ref().map(|x| x.allow_rule_breaks).unwrap_or(false) || ch.allow_rule_breaks;
    if free {
        let (_, _, normal) = play::spell_picks(lib, &ch, s);
        let mut extra: Vec<&Spell> = lib
            .spells
            .values()
            .filter(|sp| !sp.hidden && !ch.spells.known.contains(&sp.id) && !normal.contains(&sp.id) && !s.spells_granted.contains(&sp.id))
            .filter(|sp| s.spellcasting.iter().any(|b| play::on_list(lib, b, sp) && (sp.level == 0 || b.spell_limit.is_some())))
            .collect();
        extra.sort_by(|a, b| a.level.cmp(&b.level).then(a.name.cmp(&b.name)));
        if !extra.is_empty() {
            let mut learn: Option<String> = None;
            egui::CollapsingHeader::new(egui::RichText::new(format!("More spells your DM allows ({})", extra.len())).color(theme::BAD)).id_salt("dm_allowed_spells").show(ui, |ui| {
                ui.label(theme::muted("Your character doesn't qualify for these yet (too high a level, or no picks left), but your DM lets you take them.").small());
                for sp in extra {
                    ui.horizontal_wrapped(|ui| {
                        let r = ui.add(theme::big_button("Learn"));
                        theme::red_outline(ui, r.rect);
                        if r.clicked() {
                            learn = Some(sp.id.clone());
                        }
                        ui.label(egui::RichText::new(format!("{} · {}", sp.name, level_name(sp.level))).size(15.0));
                    });
                }
            });
            if let Some(id) = learn {
                let mut known = ch.spells.known.clone();
                known.push(id);
                app.act(Cmd::ChooseSpells(known));
            }
        }
    }

    // ---- learn
    let (cantrips_left, spells_left, options) = play::spell_picks(lib, &ch, s);
    if cantrips_left > 0 || spells_left > 0 {
        let mut what: Vec<String> = Vec::new();
        if cantrips_left > 0 {
            what.push(format!("{cantrips_left} cantrip(s)"));
        }
        if spells_left > 0 {
            what.push(format!("{spells_left} spell(s)"));
        }
        section(ui, &format!("Learn {}", what.join(" and ")));
        let mut list: Vec<&Spell> = options.iter().filter_map(|id| lib.spells.get(id)).collect();
        list.sort_by(|a, b| a.level.cmp(&b.level).then(a.name.cmp(&b.name)));
        let mut learn: Option<String> = None;
        for sp in list {
            let open = app.ui_state.spell_open == sp.id;
            theme::card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let r = ui.add(egui::Label::new(egui::RichText::new(&sp.name).size(16.0)).sense(egui::Sense::click()));
                        let e = effect(sp);
                        let r2 = ui.add(egui::Label::new(theme::muted(format!("{}{}", level_name(sp.level), if e.is_empty() { String::new() } else { format!(" · {e}") })).small()).sense(egui::Sense::click()));
                        if r.clicked() || r2.clicked() {
                            app.ui_state.spell_open = if open { String::new() } else { sp.id.clone() };
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(theme::big_button("Learn")).clicked() {
                            learn = Some(sp.id.clone());
                        }
                    });
                });
                if open {
                    ui.separator();
                    details(ui, sp);
                }
            });
        }
        if let Some(id) = learn {
            let mut known = ch.spells.known.clone();
            known.push(id);
            app.act(Cmd::ChooseSpells(known));
        }
    }
}

// ------------------------------------------------------------------ builder

/// The spell part of the character builder. Keeps `draft.spells` legal for
/// the class it has now and returns what is still open to choose.
pub fn builder_section(ui: &mut egui::Ui, lib: &Library, draft: &mut Character, sheet: &Sheet, open: &mut String, free: bool) -> Vec<String> {
    if sheet.spellcasting.is_empty() {
        draft.spells.known.clear();
        draft.spells.prepared.clear();
        return Vec::new();
    }
    let allowed = |s: &Spell| sheet.spellcasting.iter().any(|b| play::on_list(lib, b, s) && (s.level == 0 || (b.spell_limit.is_some() && s.level <= b.max_spell_level)));
    // on the class list but too high a level: locked unless the DM allows rule breaks
    let learnable = |s: &Spell| sheet.spellcasting.iter().any(|b| play::on_list(lib, b, s) && (s.level == 0 || b.spell_limit.is_some()));
    draft.spells.known.retain(|id| lib.spells.get(id).map(|s| !s.hidden && (allowed(s) || (free && learnable(s)))).unwrap_or(false));
    let cantrip_max: usize = sheet.spellcasting.iter().map(|b| b.cantrips_known as usize).sum();
    let spell_max: usize = sheet.spellcasting.iter().filter_map(|b| b.spell_limit).sum::<u32>() as usize;
    let prepared_max: usize = sheet.spellcasting.iter().filter_map(|b| b.prepared_max).sum::<i32>().max(0) as usize;
    let book = sheet.spellcasting.iter().any(|b| b.spell_limit.is_some());

    let mut all: Vec<&Spell> = lib.spells.values().filter(|s| !s.hidden).collect();
    all.sort_by(|a, b| a.level.cmp(&b.level).then(a.name.cmp(&b.name)));
    let is_cantrip = |id: &String| lib.spells.get(id).map(|s| s.level == 0).unwrap_or(false);

    theme::section(ui, "Spells");
    let group = |ui: &mut egui::Ui, title: &str, hint: &str, list: Vec<&Spell>, chosen: &mut Vec<String>, count: usize, max: usize, open: &mut String| {
        ui.label(egui::RichText::new(format!("{title}  {count} of {max}")).strong().color(if count == max { theme::GOOD } else { theme::ORANGE }));
        if !hint.is_empty() {
            ui.label(theme::muted(hint).small());
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(10.0, 7.0);
            for s in &list {
                let on = chosen.contains(&s.id);
                let label = if s.level > 0 { format!("{} · L{}", s.name, s.level) } else { s.name.clone() };
                let text = egui::RichText::new(label).size(15.0).color(if on { egui::Color32::BLACK } else { theme::TEXT });
                if ui.add(egui::Button::new(text).fill(if on { theme::ORANGE } else { theme::CARD })).clicked() {
                    if on {
                        chosen.retain(|x| x != &s.id);
                    } else if count < max {
                        chosen.push(s.id.clone());
                    }
                    *open = s.id.clone();
                }
            }
        });
        if let Some(s) = list.iter().find(|s| &s.id == open) {
            theme::card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(&s.name).strong());
                let e = effect(s);
                if !e.is_empty() {
                    ui.label(theme::accent(e).small());
                }
                details(ui, s);
            });
        }
        ui.add_space(6.0);
    };

    // cantrips
    let cantrips: Vec<&Spell> = all.iter().copied().filter(|s| s.level == 0 && allowed(s)).collect();
    let n = draft.spells.known.iter().filter(|id| is_cantrip(id)).count();
    group(ui, "Cantrips", "Small spells you can cast as often as you like.", cantrips, &mut draft.spells.known, n, cantrip_max, open);
    // spellbook / spells known
    if book {
        let spells: Vec<&Spell> = all.iter().copied().filter(|s| s.level > 0 && allowed(s)).collect();
        let n = draft.spells.known.iter().filter(|id| !is_cantrip(id)).count();
        group(ui, if prepared_max > 0 { "Spellbook" } else { "Spells known" }, "The spells your character has learned.", spells, &mut draft.spells.known, n, spell_max, open);
    }
    // higher-level spells: shown so players know what's coming, picked only with the DM's say-so
    let locked: Vec<&Spell> = all.iter().copied().filter(|s| s.level > 0 && learnable(s) && !allowed(s)).collect();
    if !locked.is_empty() {
        egui::CollapsingHeader::new(egui::RichText::new(format!("Higher-level spells ({})", locked.len())).color(theme::BAD)).id_salt("locked_spells").show(ui, |ui| {
            ui.label(theme::muted(if free { "Your character can't cast these yet, but your DM allows picking them." } else { "Your character can't learn these yet; they unlock as you level up. Ask your DM if you want one now." }).small());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().button_padding = egui::vec2(10.0, 7.0);
                for s in &locked {
                    let on = draft.spells.known.contains(&s.id);
                    let r = ui.add_enabled(free || on, egui::Button::new(egui::RichText::new(format!("{} · L{}", s.name, s.level)).size(15.0)).selected(on));
                    theme::red_outline(ui, r.rect);
                    if r.clicked() {
                        if on {
                            draft.spells.known.retain(|x| x != &s.id);
                        } else {
                            draft.spells.known.push(s.id.clone());
                        }
                        *open = s.id.clone();
                    }
                }
            });
        });
    }
    // prepared
    let available = play::spells_available(lib, draft, sheet);
    draft.spells.prepared.retain(|p| available.contains(p) && !is_cantrip(p));
    if prepared_max > 0 {
        let spells: Vec<&Spell> = all.iter().copied().filter(|s| s.level > 0 && available.contains(&s.id)).collect();
        let n = draft.spells.prepared.len();
        let hint = if book { "Which spells from your book are ready today. You can change this after every long rest." } else { "Your class can prepare any spell on its list. Pick today's; you can change them after every long rest." };
        group(ui, "Prepared", hint, spells, &mut draft.spells.prepared, n, prepared_max, open);
    }

    let mut open_picks = Vec::new();
    let c = draft.spells.known.iter().filter(|id| is_cantrip(id)).count();
    if c < cantrip_max {
        open_picks.push(format!("{} more cantrip(s)", cantrip_max - c));
    }
    let k = draft.spells.known.len() - c;
    if book && k < spell_max {
        open_picks.push(format!("{} more spell(s)", spell_max - k));
    }
    if prepared_max > 0 && draft.spells.prepared.len() < prepared_max.min(available.iter().filter(|a| !is_cantrip(a)).count()) {
        open_picks.push("more prepared spells".to_string());
    }
    open_picks
}
