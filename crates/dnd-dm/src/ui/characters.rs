//! Character builder and live sheet.

use crate::app::DmApp;
use crate::ui::{bonus, heading, id_combo, list_edit};
use dnd_core::model::*;
use dnd_core::{derive, Library, Sheet};
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Build,
    Sheet,
    Inventory,
    Spells,
    Modifiers,
    Notes,
}

#[derive(Default)]
pub struct CharState {
    pub selected: Option<String>,
    pub tab: Tab,
    pub new_name: String,
    pub item_filter: String,
    pub item_pick: String,
    pub item_qty: u32,
    pub custom_item: String,
    pub feat_pick: String,
    pub spell_filter: String,
    pub show_trace: bool,
    pub confirm_delete: bool,
}

pub fn show(app: &mut DmApp, ui: &mut egui::Ui) {
    egui::SidePanel::left("char_list").resizable(true).default_width(220.0).show_inside(ui, |ui| {
        ui.label(egui::RichText::new("Characters").strong());
        let list: Vec<(String, String, String)> = {
            let w = app.world.read().unwrap();
            w.campaign
                .characters
                .iter()
                .map(|c| {
                    let s = derive(c, &w.library);
                    (c.id.clone(), c.name.clone(), format!("{} · L{}", s.class_line, s.level))
                })
                .collect()
        };
        egui::ScrollArea::vertical().max_height(ui.available_height() - 120.0).show(ui, |ui| {
            for (id, name, sub) in &list {
                let sel = app.characters.selected.as_deref() == Some(id.as_str());
                let r = ui.selectable_label(sel, format!("{name}\n{sub}"));
                if r.clicked() {
                    app.characters.selected = Some(id.clone());
                    app.characters.confirm_delete = false;
                }
            }
        });
        ui.separator();
        ui.add(egui::TextEdit::singleline(&mut app.characters.new_name).hint_text("New character name").desired_width(f32::INFINITY));
        if ui.button("➕ Create").clicked() && !app.characters.new_name.trim().is_empty() {
            let name = app.characters.new_name.trim().to_string();
            let mut w = app.world.write().unwrap();
            let mut id = Character::slug(&name);
            let mut n = 2;
            while w.campaign.character(&id).is_some() {
                id = format!("{}_{n}", Character::slug(&name));
                n += 1;
            }
            let mut ch = Character::new(id.clone(), name.clone());
            ch.ability_method = "standard_array".into();
            if let Some(first) = w.library.list(Kind::Class).first() {
                ch.classes.push(ClassLevel { class: first.0.clone(), level: 1, ..Default::default() });
            }
            ch.hp.current = derive(&ch, &w.library).hp_max;
            w.campaign.characters.push(ch);
            w.campaign.log(format!("character '{name}' created"));
            w.touch();
            drop(w);
            app.characters.selected = Some(id);
            app.characters.new_name.clear();
            app.characters.tab = Tab::Build;
        }
    });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        let Some(id) = app.characters.selected.clone() else {
            ui.label("Select or create a character.");
            return;
        };
        ui.horizontal(|ui| {
            for (t, l) in [
                (Tab::Build, "Build"),
                (Tab::Sheet, "Sheet"),
                (Tab::Inventory, "Inventory"),
                (Tab::Spells, "Spells"),
                (Tab::Modifiers, "Modifiers"),
                (Tab::Notes, "Notes & State"),
            ] {
                ui.selectable_value(&mut app.characters.tab, t, l);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.characters.confirm_delete {
                    if ui.button("Confirm delete").clicked() {
                        let mut w = app.world.write().unwrap();
                        w.campaign.characters.retain(|c| c.id != id);
                        w.campaign.log(format!("character '{id}' deleted"));
                        w.touch();
                        drop(w);
                        app.characters.selected = None;
                        app.characters.confirm_delete = false;
                        return;
                    }
                    if ui.button("Cancel").clicked() {
                        app.characters.confirm_delete = false;
                    }
                } else if ui.button("🗑 Delete").clicked() {
                    app.characters.confirm_delete = true;
                }
                if ui.button("🖨 Export / print sheet").clicked() {
                    let html = {
                        let w = app.world.read().unwrap();
                        w.sheet(&id).map(|s| dnd_core::export::character_sheet_html(&s, &w.library))
                    };
                    if let Some(html) = html {
                        let path = app.paths.exports_dir.join(format!("{id}_sheet.html"));
                        match std::fs::write(&path, html) {
                            Ok(()) => {
                                let _ = open::that(&path);
                                app.set_status(format!("Exported {}", path.display()), false);
                            }
                            Err(e) => app.set_status(format!("Export failed: {e}"), true),
                        }
                    }
                }
                if ui.button("Save .toml").clicked() {
                    let res = {
                        let w = app.world.read().unwrap();
                        w.campaign.character(&id).map(|c| {
                            let p = app.paths.exports_dir.join(format!("{id}.toml"));
                            dnd_core::save::save_character(c, &p).map(|_| p)
                        })
                    };
                    match res {
                        Some(Ok(p)) => app.set_status(format!("Wrote {}", p.display()), false),
                        Some(Err(e)) => app.set_status(format!("{e:#}"), true),
                        None => {}
                    }
                }
            });
        });
        ui.separator();
        if app.characters.selected.is_none() {
            return;
        }

        let world = app.world.clone();
        let mut w = world.write().unwrap();
        let w = &mut *w;
        let lib = &w.library;
        let Some(ch) = w.campaign.characters.iter_mut().find(|c| c.id == id) else {
            app.characters.selected = None;
            return;
        };
        let before = ch.clone();
        egui::ScrollArea::vertical().show(ui, |ui| match app.characters.tab {
            Tab::Build => build_tab(ui, ch, lib, &mut app.characters),
            Tab::Sheet => sheet_tab(ui, ch, lib, &mut app.characters),
            Tab::Inventory => inventory_tab(ui, ch, lib, &mut app.characters),
            Tab::Spells => spells_tab(ui, ch, lib, &mut app.characters),
            Tab::Modifiers => modifiers_tab(ui, ch),
            Tab::Notes => notes_tab(ui, ch, lib),
        });
        if *ch != before {
            w.revision += 1;
        }
    });
}

fn build_tab(ui: &mut egui::Ui, ch: &mut Character, lib: &Library, st: &mut CharState) {
    heading(ui, "Identity");
    egui::Grid::new("ident").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut ch.name);
        ui.label("Player");
        ui.text_edit_singleline(&mut ch.player);
        ui.end_row();
        ui.label("Alignment");
        ui.text_edit_singleline(&mut ch.alignment);
        ui.label("XP");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut ch.xp).speed(10));
            let lvl = lib.rules.level_for_xp(ch.xp);
            ui.label(egui::RichText::new(format!("(XP level {lvl})")).weak());
        });
        ui.end_row();
        ui.label("Species");
        id_combo(ui, "species", &mut ch.species, &lib.list(Kind::Species), "— none —");
        ui.label("Background");
        id_combo(ui, "background", &mut ch.background, &lib.list(Kind::Background), "— none —");
        ui.end_row();
    });

    heading(ui, "Classes");
    let classes = lib.list(Kind::Class);
    let mut remove: Option<usize> = None;
    for (i, cl) in ch.classes.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            id_combo(ui, ("class", i), &mut cl.class, &classes, "— class —");
            let subs: Vec<(String, String)> = lib.subclasses_of(&cl.class).into_iter().map(|s| (s.id.clone(), s.name.clone())).collect();
            let mut sub = cl.subclass.clone().unwrap_or_default();
            let label = lib.classes.get(&cl.class).map(|c| if c.subclass_label.is_empty() { "— subclass —".to_string() } else { format!("— {} —", c.subclass_label) }).unwrap_or("— subclass —".into());
            if id_combo(ui, ("subclass", i), &mut sub, &subs, &label) {
                cl.subclass = if sub.is_empty() { None } else { Some(sub) };
            }
            ui.label("Level");
            ui.add(egui::DragValue::new(&mut cl.level).range(1..=20));
            if let Some(c) = lib.classes.get(&cl.class) {
                ui.label(egui::RichText::new(format!("d{}", c.hit_die)).weak());
                if lib.rules.hp_mode == "roll" || !cl.hp_rolls.is_empty() {
                    ui.label(egui::RichText::new(format!("rolls {:?}", cl.hp_rolls)).weak().small());
                }
            }
            if ui.small_button("✖").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        ch.classes.remove(i);
    }
    if ui.button("➕ Add class (multiclass)").clicked() {
        ch.classes.push(ClassLevel { class: classes.first().map(|c| c.0.clone()).unwrap_or_default(), level: 1, ..Default::default() });
    }

    heading(ui, "Ability scores (base, before species/feat bonuses)");
    ui.horizontal(|ui| {
        ui.label("Method");
        egui::ComboBox::from_id_salt("abmethod").selected_text(if ch.ability_method.is_empty() { "manual" } else { &ch.ability_method }).show_ui(ui, |ui| {
            for m in ["point_buy", "standard_array", "rolled", "manual"] {
                ui.selectable_value(&mut ch.ability_method, m.to_string(), m);
            }
        });
        if ui.button("Apply standard array").clicked() {
            let a = &lib.rules.standard_array;
            let get = |i: usize| a.get(i).copied().unwrap_or(10);
            ch.base_abilities = Abilities { str: get(0), dex: get(1), con: get(2), int: get(3), wis: get(4), cha: get(5) };
        }
        if ui.button("Reset to 10s").clicked() {
            ch.base_abilities = Abilities::default();
        }
        if ch.ability_method == "point_buy" {
            let scores: Vec<i32> = ABILITY_KEYS.iter().map(|k| ch.base_abilities.get(k).unwrap()).collect();
            match lib.rules.point_buy_cost(&scores) {
                Some(cost) => {
                    let c = if cost > lib.rules.point_buy.budget { egui::Color32::from_rgb(220, 80, 80) } else { egui::Color32::from_rgb(120, 170, 120) };
                    ui.colored_label(c, format!("Point buy: {cost} / {}", lib.rules.point_buy.budget));
                }
                None => {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), format!("scores must be {}–{} for point buy", lib.rules.point_buy.min, lib.rules.point_buy.max));
                }
            }
        }
    });
    let sheet = derive(ch, lib);
    ui.horizontal(|ui| {
        for k in ABILITY_KEYS {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(k.to_uppercase()).strong());
                let v = ch.base_abilities.get_mut(k).unwrap();
                ui.add(egui::DragValue::new(v).range(1..=30));
                let fin = sheet.abilities.get(k).unwrap();
                ui.label(egui::RichText::new(format!("→ {} ({})", fin, bonus(dnd_core::ability_mod(fin)))).weak());
            });
            ui.add_space(6.0);
        }
    });

    // features that offer a pick: Fighting Style, Eldritch Invocations, Draconic Ancestry…
    let open: Vec<_> = sheet.features.iter().filter(|f| !f.options.is_empty()).collect();
    if !open.is_empty() {
        heading(ui, "Choices (fighting style, invocations, ancestry…)");
        for f in open {
            let opts: Vec<(String, String)> = dnd_core::builder::feature_options(lib, &f.options).iter().map(|x| (x.id.clone(), x.name.clone())).collect();
            let mut slots: Vec<String> = ch.choices.get(&f.name).cloned().unwrap_or_default();
            let n = (f.picks as usize).max(slots.len());
            slots.resize(n, String::new());
            let mut changed = false;
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(format!("{} ({}, pick {})", f.name, f.source, f.picks)).strong()).on_hover_text(&f.description);
                for (i, slot) in slots.iter_mut().enumerate() {
                    changed |= id_combo(ui, ("choice", &f.name, i), slot, &opts, "— choose —");
                }
            });
            if changed {
                let mut picked: Vec<String> = Vec::new();
                for s in slots.into_iter().filter(|s| !s.is_empty()) {
                    if !picked.contains(&s) {
                        picked.push(s);
                    }
                }
                if picked.is_empty() {
                    ch.choices.remove(&f.name);
                } else {
                    ch.choices.insert(f.name.clone(), picked);
                }
            }
        }
    }

    heading(ui, "Advancements (ASI / feats at class levels)");
    // class options (tag `option`) are picked above, not as general feats
    let feats: Vec<(String, String)> = lib.list(Kind::Feat).into_iter().filter(|(id, _)| lib.feats.get(id).map(|f| !dnd_core::builder::is_option_feat(f)).unwrap_or(true)).collect();
    let mut remove: Option<usize> = None;
    for (i, adv) in ch.advancements.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label("Level");
            ui.add(egui::DragValue::new(&mut adv.level).range(1..=20));
            for k in ABILITY_KEYS {
                let mut v = adv.ability_increases.get(k).copied().unwrap_or(0);
                ui.label(k.to_uppercase());
                if ui.add(egui::DragValue::new(&mut v).range(0..=2)).changed() {
                    if v == 0 {
                        adv.ability_increases.remove(k);
                    } else {
                        adv.ability_increases.insert(k.to_string(), v);
                    }
                }
            }
            ui.label("or feat");
            let mut f = adv.feat.clone().unwrap_or_default();
            if id_combo(ui, ("advfeat", i), &mut f, &feats, "— none —") {
                adv.feat = if f.is_empty() { None } else { Some(f) };
            }
            if ui.small_button("✖").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        ch.advancements.remove(i);
    }
    ui.horizontal(|ui| {
        if ui.button("➕ Add advancement").clicked() {
            let next = ch.advancements.iter().map(|a| a.level).max().map(|l| l + 4).unwrap_or(4);
            ch.advancements.push(Advancement { level: next, ..Default::default() });
        }
        let asi: Vec<String> = ch
            .classes
            .iter()
            .filter_map(|c| lib.classes.get(&c.class).map(|k| (k, c.level)))
            .map(|(k, lvl)| format!("{}: {}", k.name, k.asi_levels.iter().filter(|l| **l <= lvl).map(|l| l.to_string()).collect::<Vec<_>>().join(", ")))
            .collect();
        ui.label(egui::RichText::new(format!("ASI levels reached — {}", asi.join(" · "))).weak());
    });

    heading(ui, "Bonus feats (not from an ASI)");
    ui.horizontal_wrapped(|ui| {
        let mut remove: Option<usize> = None;
        for (i, f) in ch.feats.iter().enumerate() {
            let name = lib.feats.get(f).map(|x| x.name.clone()).unwrap_or_else(|| f.clone());
            if ui.button(format!("{name} ✖")).clicked() {
                remove = Some(i);
            }
        }
        if let Some(i) = remove {
            ch.feats.remove(i);
        }
        id_combo(ui, "featpick", &mut st.feat_pick, &feats, "— pick feat —");
        if ui.button("Add").clicked() && !st.feat_pick.is_empty() {
            ch.feats.push(st.feat_pick.clone());
            st.feat_pick.clear();
        }
    });

    heading(ui, "Skill proficiencies");
    let hints: Vec<String> = ch
        .classes
        .iter()
        .filter_map(|c| lib.classes.get(&c.class))
        .map(|k| format!("{}: choose {} from {}", k.name, k.skill_choices.count, if k.skill_choices.from.is_empty() { "any".to_string() } else { k.skill_choices.from.join(", ") }))
        .collect();
    for h in hints {
        ui.label(egui::RichText::new(h).weak());
    }
    if let Some(bg) = lib.backgrounds.get(&ch.background) {
        if !bg.skill_proficiencies.is_empty() {
            ui.label(egui::RichText::new(format!("{} background grants: {}", bg.name, bg.skill_proficiencies.join(", "))).weak());
        }
    }
    ui.columns(3, |cols| {
        for (i, s) in lib.rules.skills.iter().enumerate() {
            let ui = &mut cols[i % 3];
            let mut on = ch.skill_proficiencies.contains(&s.key);
            let granted = sheet.skills.iter().find(|k| k.key == s.key).map(|k| k.proficient && !on).unwrap_or(false);
            let label = format!("{} ({}){}", s.name, s.ability.to_uppercase(), if granted { " ✓granted" } else { "" });
            if ui.checkbox(&mut on, label).changed() {
                if on {
                    ch.skill_proficiencies.push(s.key.clone());
                } else {
                    ch.skill_proficiencies.retain(|k| k != &s.key);
                }
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Expertise:");
        for s in sheet.skills.iter().filter(|s| s.proficient) {
            let mut on = ch.expertise.contains(&s.key);
            if ui.checkbox(&mut on, &s.name).changed() {
                if on {
                    ch.expertise.push(s.key.clone());
                } else {
                    ch.expertise.retain(|k| k != &s.key);
                }
            }
        }
    });

    heading(ui, "Other proficiencies");
    list_edit(ui, "Tools", &mut ch.tool_proficiencies);
    list_edit(ui, "Languages", &mut ch.languages);
}

fn stat_box(ui: &mut egui::Ui, label: &str, value: &str, sub: &str) {
    egui::Frame::group(ui.style()).inner_margin(6.0).show(ui, |ui| {
        ui.set_min_width(90.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(label).weak().small());
            ui.label(egui::RichText::new(value).strong().size(20.0));
            if !sub.is_empty() {
                ui.label(egui::RichText::new(sub).weak().small());
            }
        });
    });
}

fn sheet_tab(ui: &mut egui::Ui, ch: &mut Character, lib: &Library, st: &mut CharState) {
    let s: Sheet = derive(ch, lib);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&s.name).strong().size(22.0));
        ui.label(egui::RichText::new(format!("{} · {} · {} · level {}", s.class_line, s.species, s.background, s.level)).weak());
        ui.checkbox(&mut st.show_trace, "show modifier trace");
    });
    for wmsg in &s.warnings {
        ui.colored_label(egui::Color32::from_rgb(220, 150, 60), format!("⚠ {wmsg}"));
    }
    ui.horizontal_wrapped(|ui| {
        for (k, v) in s.abilities.iter() {
            stat_box(ui, &k.to_uppercase(), &bonus(dnd_core::ability_mod(v)), &v.to_string());
        }
    });
    ui.horizontal_wrapped(|ui| {
        stat_box(ui, "Armor Class", &s.ac.to_string(), &s.ac_breakdown);
        stat_box(ui, "Initiative", &bonus(s.initiative), "");
        stat_box(ui, "Speed", &format!("{} ft", s.speed), &s.speeds.iter().filter(|(k, _)| *k != "walk").map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(" "));
        stat_box(ui, "Prof. bonus", &bonus(s.proficiency_bonus), "");
        stat_box(ui, "Passive Perc.", &s.passive_perception.to_string(), if s.darkvision > 0 { "darkvision" } else { "" });
        stat_box(ui, "Hit dice", &s.hit_dice.iter().map(|(d, n)| format!("{n}{d}")).collect::<Vec<_>>().join(" "), "");
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Hit points").strong());
        ui.add(egui::DragValue::new(&mut ch.hp.current).range(-999..=s.hp_max));
        ui.label(format!("/ {}", s.hp_max));
        ui.label("temp");
        ui.add(egui::DragValue::new(&mut ch.hp.temp).range(0..=999));
        if ui.small_button("full heal").clicked() {
            ch.hp.current = s.hp_max;
            ch.hp.temp = 0;
        }
        let mut ov = ch.hp.max_override.is_some();
        if ui.checkbox(&mut ov, "override max").changed() {
            ch.hp.max_override = if ov { Some(s.hp_max) } else { None };
        }
        if let Some(m) = ch.hp.max_override.as_mut() {
            ui.add(egui::DragValue::new(m).range(1..=999));
        }
        ui.label("Death saves ✓");
        ui.add(egui::DragValue::new(&mut ch.death_saves.successes).range(0..=3));
        ui.label("✗");
        ui.add(egui::DragValue::new(&mut ch.death_saves.failures).range(0..=3));
        ui.checkbox(&mut ch.inspiration, "Inspiration");
    });

    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        heading(ui, "Saving throws");
        for sv in &s.saves {
            ui.horizontal(|ui| {
                ui.label(if sv.proficient { "⏺" } else { "○" });
                ui.label(format!("{:<13}", sv.name));
                ui.label(egui::RichText::new(bonus(sv.bonus)).strong());
                if !sv.notes.is_empty() {
                    ui.label(egui::RichText::new(sv.notes.join("; ")).weak().small());
                }
            });
        }
        heading(ui, "Attacks");
        egui::Grid::new("atk").striped(true).show(ui, |ui| {
            ui.label(egui::RichText::new("Name").weak());
            ui.label(egui::RichText::new("Hit").weak());
            ui.label(egui::RichText::new("Damage").weak());
            ui.label(egui::RichText::new("Range").weak());
            ui.end_row();
            for a in &s.attacks {
                ui.label(&a.name).on_hover_text(a.properties.join(", "));
                ui.label(egui::RichText::new(bonus(a.bonus)).strong());
                ui.label(format!("{} {}", a.damage, a.damage_type));
                ui.vertical(|ui| {
                    ui.label(&a.range);
                    if !a.notes.is_empty() {
                        ui.label(egui::RichText::new(a.notes.join("; ")).weak().small());
                    }
                });
                ui.end_row();
            }
        });
        if !s.spellcasting.is_empty() {
            heading(ui, "Spellcasting");
            for sc in &s.spellcasting {
                ui.label(format!(
                    "{}: {} · attack {} · DC {}{}{}",
                    sc.class_name,
                    sc.ability.to_uppercase(),
                    bonus(sc.attack_bonus),
                    sc.save_dc,
                    sc.prepared_max.map(|p| format!(" · prepare {p}")).unwrap_or_default(),
                    if sc.cantrips_known > 0 { format!(" · {} cantrips", sc.cantrips_known) } else { String::new() }
                ));
            }
            let slots: Vec<String> = s
                .spell_slots
                .iter()
                .enumerate()
                .filter(|(_, n)| **n > 0)
                .map(|(i, n)| format!("L{} {}/{}", i + 1, n.saturating_sub(s.spell_slots_used.get(i).copied().unwrap_or(0)), n))
                .collect();
            if !slots.is_empty() {
                ui.label(format!("Slots: {}", slots.join("  ")));
            }
            if let Some((n, l)) = s.pact_slots {
                ui.label(format!("Pact magic: {n} slots of level {l}"));
            }
        }
        heading(ui, "Proficiencies");
        ui.label(format!("Armor: {}", s.proficiencies.armor.join(", ")));
        ui.label(format!("Weapons: {}", s.proficiencies.weapons.join(", ")));
        ui.label(format!("Tools: {}", s.proficiencies.tools.join(", ")));
        ui.label(format!("Languages: {}", s.proficiencies.languages.join(", ")));
        if !s.other.is_empty() {
            heading(ui, "Other values");
            for (k, v) in &s.other {
                ui.label(format!("{k} = {v}"));
            }
        }

        let ui = &mut cols[1];
        heading(ui, "Skills");
        for sk in &s.skills {
            ui.horizontal(|ui| {
                ui.label(if sk.expertise { "★" } else if sk.proficient { "⏺" } else { "○" });
                ui.label(format!("{} ({})", sk.name, sk.ability.to_uppercase()));
                ui.label(egui::RichText::new(bonus(sk.bonus)).strong());
                if !sk.notes.is_empty() {
                    ui.label(egui::RichText::new(sk.notes.join("; ")).weak().small());
                }
            });
        }
        ui.label(egui::RichText::new(format!("Passive: Perception {} · Investigation {} · Insight {}", s.passive_perception, s.passive_investigation, s.passive_insight)).weak());
    });

    heading(ui, "Features & traits");
    for f in &s.features {
        ui.collapsing(format!("{} — {}{}", f.name, f.source, f.uses.map(|u| format!(" ({u}× per {})", if f.recharge.is_empty() { "rest" } else { &f.recharge })).unwrap_or_default()), |ui| {
            ui.label(&f.description);
        });
    }

    if st.show_trace {
        heading(ui, "Modifier trace (why is this number what it is?)");
        egui::Grid::new("trace").striped(true).show(ui, |ui| {
            for t in &s.trace {
                ui.label(if t.applied { "✓" } else { "–" });
                ui.label(&t.target);
                ui.label(&t.op);
                ui.label(format!("{}{}", t.value, t.resolved.map(|v| format!(" = {v}")).unwrap_or_default()));
                ui.label(egui::RichText::new(&t.source).weak());
                ui.label(egui::RichText::new(&t.reason).weak().small());
                ui.end_row();
            }
        });
    }
}

fn inventory_tab(ui: &mut egui::Ui, ch: &mut Character, lib: &Library, st: &mut CharState) {
    let s = derive(ch, lib);
    heading(ui, "Add item");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut st.item_filter).hint_text("filter").desired_width(120.0));
        let f = st.item_filter.to_lowercase();
        let items: Vec<(String, String)> = lib.list(Kind::Item).into_iter().filter(|(id, n)| f.is_empty() || n.to_lowercase().contains(&f) || id.contains(&f)).collect();
        id_combo(ui, "itempick", &mut st.item_pick, &items, "— item —");
        if st.item_qty == 0 {
            st.item_qty = 1;
        }
        ui.add(egui::DragValue::new(&mut st.item_qty).range(1..=999));
        if ui.button("Add").clicked() && !st.item_pick.is_empty() {
            if let Some(existing) = ch.inventory.iter_mut().find(|i| i.item == st.item_pick && !i.custom) {
                existing.qty += st.item_qty;
            } else {
                ch.inventory.push(InventoryItem { item: st.item_pick.clone(), qty: st.item_qty, ..Default::default() });
            }
        }
        ui.separator();
        ui.add(egui::TextEdit::singleline(&mut st.custom_item).hint_text("custom item name").desired_width(160.0));
        if ui.button("Add custom").clicked() && !st.custom_item.trim().is_empty() {
            ch.inventory.push(InventoryItem { item: st.custom_item.trim().to_string(), name: st.custom_item.trim().to_string(), qty: 1, custom: true, ..Default::default() });
            st.custom_item.clear();
        }
    });
    heading(ui, "Inventory");
    ui.label(egui::RichText::new(format!("Weight {} / {} lb", s.total_weight, s.carrying_capacity)).weak());
    let mut remove: Option<usize> = None;
    ui.label(egui::RichText::new("“Hidden” = fog of war for loot: the player app does not see the item at all (its bonuses still apply). DM notes are never sent to players.").weak().small());
    egui::Grid::new("inv").striped(true).num_columns(9).spacing([10.0, 4.0]).show(ui, |ui| {
        ui.label(egui::RichText::new("Item").weak());
        ui.label(egui::RichText::new("Qty").weak());
        ui.label(egui::RichText::new("Equipped").weak());
        ui.label(egui::RichText::new("Attuned").weak());
        ui.label(egui::RichText::new("Hidden").weak());
        ui.label(egui::RichText::new("Wt").weak());
        ui.label(egui::RichText::new("Notes").weak());
        ui.label(egui::RichText::new("DM notes").weak());
        ui.label("");
        ui.end_row();
        for (i, inv) in ch.inventory.iter_mut().enumerate() {
            let item = if inv.custom { None } else { lib.items.get(&inv.item) };
            let name = item.map(|x| x.name.clone()).unwrap_or_else(|| if inv.name.is_empty() { inv.item.clone() } else { inv.name.clone() });
            ui.label(&name).on_hover_text(item.map(|x| x.description.clone()).unwrap_or_default());
            ui.add(egui::DragValue::new(&mut inv.qty).range(0..=999));
            let equippable = item.map(|x| x.weapon.is_some() || x.armor.is_some() || !x.modifiers.is_empty()).unwrap_or(true);
            ui.add_enabled(equippable, egui::Checkbox::new(&mut inv.equipped, ""));
            let attunable = item.map(|x| x.requires_attunement).unwrap_or(false);
            ui.add_enabled(attunable, egui::Checkbox::new(&mut inv.attuned, ""));
            ui.checkbox(&mut inv.hidden, "").on_hover_text("Hidden from the player app");
            ui.label(format!("{}", item.map(|x| x.weight).unwrap_or(0.0) * inv.qty as f64));
            ui.add(egui::TextEdit::singleline(&mut inv.notes).desired_width(160.0));
            ui.add(egui::TextEdit::singleline(&mut inv.dm_notes).desired_width(160.0).hint_text("secret"));
            if ui.small_button("✖").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        ch.inventory.remove(i);
    }
    heading(ui, "Coins");
    ui.horizontal(|ui| {
        for (l, v) in [("cp", &mut ch.currency.cp), ("sp", &mut ch.currency.sp), ("ep", &mut ch.currency.ep), ("gp", &mut ch.currency.gp), ("pp", &mut ch.currency.pp)] {
            ui.label(l);
            ui.add(egui::DragValue::new(v).range(0..=1_000_000));
        }
    });
}

fn spells_tab(ui: &mut egui::Ui, ch: &mut Character, lib: &Library, st: &mut CharState) {
    let s = derive(ch, lib);
    if s.spellcasting.is_empty() {
        ui.label("No spellcasting class. (Classes or subclasses get a [spellcasting] table in their TOML.)");
    }
    heading(ui, "Spell slots used");
    ui.horizontal(|ui| {
        ch.spells.slots_used.resize(9, 0);
        for (i, n) in s.spell_slots.iter().enumerate() {
            if *n == 0 {
                continue;
            }
            ui.label(format!("L{}", i + 1));
            ui.add(egui::DragValue::new(&mut ch.spells.slots_used[i]).range(0..=*n));
            ui.label(egui::RichText::new(format!("/{n}")).weak());
        }
        if let Some((n, l)) = s.pact_slots {
            ui.label(format!("Pact (L{l})"));
            ui.add(egui::DragValue::new(&mut ch.spells.pact_slots_used).range(0..=n));
            ui.label(egui::RichText::new(format!("/{n}")).weak());
        }
        if ui.button("Long rest (reset)").clicked() {
            ch.spells.slots_used = vec![0; 9];
            ch.spells.pact_slots_used = 0;
        }
    });
    heading(ui, "Spell list");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut st.spell_filter).hint_text("filter").desired_width(160.0));
        ui.label(egui::RichText::new(format!("known {} · prepared {}{}", ch.spells.known.len(), ch.spells.prepared.len(), s.spellcasting.iter().filter_map(|c| c.prepared_max).map(|m| format!(" (max {m})")).collect::<String>())).weak());
    });
    let f = st.spell_filter.to_lowercase();
    let mut class_ids: Vec<String> = s.spellcasting.iter().map(|c| c.class.clone()).collect();
    for cl in &ch.classes {
        if let Some(c) = lib.classes.get(&cl.class) {
            if let Some(sc) = &c.spellcasting {
                class_ids.extend(sc.spell_lists.iter().cloned());
            }
        }
    }
    let mut spells: Vec<&Spell> = lib
        .spells
        .values()
        .filter(|sp| class_ids.is_empty() || sp.classes.iter().any(|c| class_ids.contains(c)) || ch.spells.known.contains(&sp.id))
        .filter(|sp| f.is_empty() || sp.name.to_lowercase().contains(&f))
        .collect();
    spells.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.name.cmp(&b.name)));
    egui::Grid::new("spells").striped(true).num_columns(6).spacing([10.0, 3.0]).show(ui, |ui| {
        ui.label(egui::RichText::new("Known").weak());
        ui.label(egui::RichText::new("Prep").weak());
        ui.label(egui::RichText::new("Lvl").weak());
        ui.label(egui::RichText::new("Spell").weak());
        ui.label(egui::RichText::new("Time / Range").weak());
        ui.label(egui::RichText::new("Effect").weak());
        ui.end_row();
        for sp in spells {
            let mut known = ch.spells.known.contains(&sp.id);
            if ui.checkbox(&mut known, "").changed() {
                if known {
                    ch.spells.known.push(sp.id.clone());
                } else {
                    ch.spells.known.retain(|k| k != &sp.id);
                    ch.spells.prepared.retain(|k| k != &sp.id);
                }
            }
            let mut prep = ch.spells.prepared.contains(&sp.id);
            if ui.add_enabled(known, egui::Checkbox::new(&mut prep, "")).changed() {
                if prep {
                    ch.spells.prepared.push(sp.id.clone());
                } else {
                    ch.spells.prepared.retain(|k| k != &sp.id);
                }
            }
            ui.label(if sp.level == 0 { "C".to_string() } else { sp.level.to_string() });
            ui.label(&sp.name).on_hover_text(&sp.description);
            ui.label(format!("{} / {}", sp.casting_time, sp.range));
            let mut eff = String::new();
            if !sp.damage.is_empty() {
                eff.push_str(&format!("{} {}", sp.damage, sp.damage_type));
            }
            if !sp.save.is_empty() {
                eff.push_str(&format!(" {} save", sp.save.to_uppercase()));
            }
            if let Some(a) = &sp.area {
                eff.push_str(&format!(" · {} ft {}", a.size, a.shape));
            }
            if sp.concentration {
                eff.push_str(" · conc.");
            }
            ui.label(eff.trim());
            ui.end_row();
        }
    });
}

fn modifiers_tab(ui: &mut egui::Ui, ch: &mut Character) {
    heading(ui, "Custom modifiers (DM / house rules / blessings)");
    ui.label(egui::RichText::new("Targets: ability.str, save.wis, save.all, skill.stealth, skill.all, ac, ac.base, hp.max, hp.per_level, initiative, speed, speed.fly, darkvision, attack.melee, attack.ranged, attack.all, damage.melee, damage.ranged, damage.all, spell.attack, spell.dc, passive.perception, proficiency_bonus, proficiency.skill.x / save.x / armor.x / weapon.x / tool.x / language.x, carrying_capacity — or any name of your own.").weak().small());
    ui.label(egui::RichText::new("Values are formulas: 2, -1, level, floor(level/2)+1, str_mod, pb, class.fighter.level, armored, 1d4. Conditions: level >= 5, armored, !shield, dex > str.").weak().small());
    let mut remove: Option<usize> = None;
    egui::Grid::new("mods").striped(true).num_columns(7).spacing([8.0, 4.0]).show(ui, |ui| {
        for h in ["Target", "Op", "Value", "Condition", "Source", "Note", ""] {
            ui.label(egui::RichText::new(h).weak());
        }
        ui.end_row();
        for (i, m) in ch.modifiers.iter_mut().enumerate() {
            ui.add(egui::TextEdit::singleline(&mut m.target).desired_width(140.0));
            egui::ComboBox::from_id_salt(("op", i)).selected_text(format!("{:?}", m.op).to_lowercase()).width(90.0).show_ui(ui, |ui| {
                for op in [ModOp::Add, ModOp::Set, ModOp::Max, ModOp::Min, ModOp::Mul, ModOp::Grant, ModOp::Expertise, ModOp::Advantage, ModOp::Disadvantage, ModOp::Note] {
                    ui.selectable_value(&mut m.op, op, format!("{op:?}").to_lowercase());
                }
            });
            ui.add(egui::TextEdit::singleline(&mut m.value).desired_width(110.0));
            ui.add(egui::TextEdit::singleline(&mut m.condition).desired_width(120.0));
            ui.add(egui::TextEdit::singleline(&mut m.source).desired_width(120.0));
            ui.add(egui::TextEdit::singleline(&mut m.note).desired_width(140.0));
            if ui.small_button("✖").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        ch.modifiers.remove(i);
    }
    if ui.button("➕ Add modifier").clicked() {
        ch.modifiers.push(Modifier { target: "ac".into(), op: ModOp::Add, value: "1".into(), source: "DM".into(), ..Default::default() });
    }
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Custom fields (anything else to track)").strong());
    let mut remove_key: Option<String> = None;
    let keys: Vec<String> = ch.custom.keys().cloned().collect();
    for k in keys {
        ui.horizontal(|ui| {
            ui.label(&k);
            if let Some(v) = ch.custom.get_mut(&k) {
                ui.text_edit_singleline(v);
            }
            if ui.small_button("✖").clicked() {
                remove_key = Some(k.clone());
            }
        });
    }
    if let Some(k) = remove_key {
        ch.custom.remove(&k);
    }
    if ui.button("➕ Add field").clicked() {
        let n = ch.custom.len() + 1;
        ch.custom.insert(format!("field_{n}"), String::new());
    }
}

fn notes_tab(ui: &mut egui::Ui, ch: &mut Character, lib: &Library) {
    heading(ui, "Conditions");
    ui.horizontal_wrapped(|ui| {
        for c in &lib.rules.conditions {
            let mut on = ch.conditions.contains(c);
            if ui.checkbox(&mut on, c).changed() {
                if on {
                    ch.conditions.push(c.clone());
                } else {
                    ch.conditions.retain(|x| x != c);
                }
            }
        }
        ui.label("Exhaustion");
        ui.add(egui::DragValue::new(&mut ch.exhaustion).range(0..=6));
    });
    heading(ui, "Hit dice spent");
    ui.horizontal(|ui| {
        let s = derive(ch, lib);
        for (d, n) in &s.hit_dice {
            let used = ch.hp.hit_dice_used.entry(d.clone()).or_insert(0);
            ui.label(d);
            ui.add(egui::DragValue::new(used).range(0..=*n));
            ui.label(egui::RichText::new(format!("/{n}")).weak());
        }
        if ui.button("Long rest").clicked() {
            ch.hp.hit_dice_used.clear();
            ch.spells.slots_used = vec![0; 9];
            ch.spells.pact_slots_used = 0;
            ch.hp.current = s.hp_max;
            ch.hp.temp = 0;
            ch.death_saves = DeathSaves::default();
        }
    });
    heading(ui, "Roleplay");
    for (l, v) in [
        ("Appearance", &mut ch.appearance),
        ("Personality", &mut ch.personality),
        ("Ideals", &mut ch.ideals),
        ("Bonds", &mut ch.bonds),
        ("Flaws", &mut ch.flaws),
    ] {
        ui.label(l);
        ui.add(egui::TextEdit::multiline(v).desired_rows(2).desired_width(f32::INFINITY));
    }
    ui.label("Backstory");
    ui.add(egui::TextEdit::multiline(&mut ch.backstory).desired_rows(6).desired_width(f32::INFINITY));
    ui.label("Notes (player can see)");
    ui.add(egui::TextEdit::multiline(&mut ch.notes).desired_rows(6).desired_width(f32::INFINITY));
    ui.label(egui::RichText::new("DM notes (never sent to the player app)").color(crate::ui::theme::ORANGE));
    ui.add(egui::TextEdit::multiline(&mut ch.dm_notes).desired_rows(4).desired_width(f32::INFINITY));
}
