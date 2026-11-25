//! Form editors for every content kind. The TOML file stays the source of
//! truth; these forms just make it painless to fill in.

use crate::ui::theme::{self, section};
use crate::ui::{id_combo, list_edit};
use dnd_core::model::*;
use dnd_core::Library;
use eframe::egui;
use std::collections::BTreeMap;

pub const SIZES: [&str; 6] = ["Tiny", "Small", "Medium", "Large", "Huge", "Gargantuan"];
pub const ITEM_KINDS: [&str; 8] = ["weapon", "armor", "shield", "gear", "consumable", "wondrous", "tool", "ammunition"];
pub const RARITIES: [&str; 7] = ["", "common", "uncommon", "rare", "very rare", "legendary", "artifact"];
pub const SCHOOLS: [&str; 8] = ["abjuration", "conjuration", "divination", "enchantment", "evocation", "illusion", "necromancy", "transmutation"];
pub const WEAPON_PROPS: [&str; 10] = ["finesse", "light", "heavy", "two_handed", "thrown", "reach", "ammunition", "loading", "versatile", "special"];
pub const COMMON_TARGETS: [&str; 28] = [
    "ability.str", "ability.dex", "ability.con", "ability.int", "ability.wis", "ability.cha",
    "save.str", "save.dex", "save.con", "save.int", "save.wis", "save.cha", "save.all",
    "skill.perception", "skill.stealth", "skill.all", "ac", "ac.base", "hp.max", "hp.per_level",
    "initiative", "speed", "darkvision", "attack.melee", "attack.ranged", "damage.all", "spell.dc", "proficiency.skill.perception",
];

// ------------------------------------------------------------------ small widgets

pub fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized([130.0, 20.0], egui::Label::new(theme::muted(label)).halign(egui::Align::RIGHT));
        add(ui);
    });
}

pub fn text(ui: &mut egui::Ui, label: &str, v: &mut String) {
    row(ui, label, |ui| {
        ui.add(egui::TextEdit::singleline(v).desired_width(f32::INFINITY));
    });
}

pub fn text_w(ui: &mut egui::Ui, label: &str, v: &mut String, width: f32) {
    row(ui, label, |ui| {
        ui.add(egui::TextEdit::singleline(v).desired_width(width));
    });
}

pub fn multiline(ui: &mut egui::Ui, label: &str, v: &mut String, rows: usize) {
    ui.label(theme::muted(label));
    ui.add(egui::TextEdit::multiline(v).desired_rows(rows).desired_width(f32::INFINITY));
}

pub fn num_u32(ui: &mut egui::Ui, label: &str, v: &mut u32, range: std::ops::RangeInclusive<u32>) {
    row(ui, label, |ui| {
        ui.add(egui::DragValue::new(v).range(range));
    });
}

pub fn choice(ui: &mut egui::Ui, label: &str, v: &mut String, options: &[&str]) {
    row(ui, label, |ui| {
        egui::ComboBox::from_id_salt(label).selected_text(if v.is_empty() { "—" } else { v.as_str() }).show_ui(ui, |ui| {
            for o in options {
                ui.selectable_value(v, o.to_string(), if o.is_empty() { "—" } else { *o });
            }
        });
    });
}

/// Chip list with an "add" box. Enter or + adds.
pub fn chips(ui: &mut egui::Ui, label: &str, list: &mut Vec<String>, suggestions: &[&str]) {
    let id = ui.id().with(label).with("chips");
    row(ui, label, |ui| {
        ui.horizontal_wrapped(|ui| {
            let mut remove = None;
            for (i, s) in list.iter().enumerate() {
                if ui.add(egui::Button::new(format!("{s}  ✖")).small().fill(theme::ORANGE_FAINT)).clicked() {
                    remove = Some(i);
                }
            }
            if let Some(i) = remove {
                list.remove(i);
            }
            let mut buf: String = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
            let r = ui.add(egui::TextEdit::singleline(&mut buf).desired_width(110.0).hint_text("add…"));
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.small_button("+").clicked() || enter) && !buf.trim().is_empty() {
                list.push(buf.trim().to_string());
                buf.clear();
                r.request_focus();
            }
            if !suggestions.is_empty() {
                egui::ComboBox::from_id_salt(id.with("sugg")).selected_text("…").width(40.0).show_ui(ui, |ui| {
                    for s in suggestions {
                        if !list.iter().any(|x| x == s) && ui.selectable_label(false, *s).clicked() {
                            list.push(s.to_string());
                        }
                    }
                });
            }
            ui.data_mut(|d| d.insert_temp(id, buf));
        });
    });
}

/// Checkbox grid for skill keys.
pub fn skill_picker(ui: &mut egui::Ui, label: &str, list: &mut Vec<String>, lib: &Library) {
    ui.label(theme::muted(label));
    ui.horizontal_wrapped(|ui| {
        for s in &lib.rules.skills {
            let mut on = list.contains(&s.key);
            if ui.checkbox(&mut on, &s.name).changed() {
                if on {
                    list.push(s.key.clone());
                } else {
                    list.retain(|k| k != &s.key);
                }
            }
        }
    });
}

pub fn ability_picker(ui: &mut egui::Ui, label: &str, list: &mut Vec<String>) {
    row(ui, label, |ui| {
        for k in ABILITY_KEYS {
            let mut on = list.iter().any(|x| x == k);
            if ui.checkbox(&mut on, k.to_uppercase()).changed() {
                if on {
                    list.push(k.to_string());
                } else {
                    list.retain(|x| x != k);
                }
            }
        }
    });
}

pub fn abilities(ui: &mut egui::Ui, ab: &mut Abilities) {
    row(ui, "Abilities", |ui| {
        for k in ABILITY_KEYS {
            ui.label(theme::muted(k.to_uppercase()));
            ui.add(egui::DragValue::new(ab.get_mut(k).unwrap()).range(1..=30));
        }
    });
}

/// key -> number map (speeds, saves, skills).
pub fn map_i32(ui: &mut egui::Ui, label: &str, map: &mut BTreeMap<String, i32>, suggestions: &[String]) {
    let id = ui.id().with(label).with("map");
    row(ui, label, |ui| {
        ui.horizontal_wrapped(|ui| {
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                ui.label(&k);
                ui.add(egui::DragValue::new(map.get_mut(&k).unwrap()));
                if ui.small_button("✖").clicked() {
                    map.remove(&k);
                }
            }
            let mut buf: String = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
            let r = ui.add(egui::TextEdit::singleline(&mut buf).desired_width(100.0).hint_text("key…"));
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.small_button("+").clicked() || enter) && !buf.trim().is_empty() {
                map.entry(buf.trim().to_string()).or_insert(0);
                buf.clear();
            }
            if !suggestions.is_empty() {
                egui::ComboBox::from_id_salt(id.with("s")).selected_text("…").width(40.0).show_ui(ui, |ui| {
                    for s in suggestions {
                        if !map.contains_key(s) && ui.selectable_label(false, s).clicked() {
                            map.insert(s.clone(), 0);
                        }
                    }
                });
            }
            ui.data_mut(|d| d.insert_temp(id, buf));
        });
    });
}

pub fn map_u32(ui: &mut egui::Ui, label: &str, map: &mut BTreeMap<String, u32>, suggestions: &[&str]) {
    let mut tmp: BTreeMap<String, i32> = map.iter().map(|(k, v)| (k.clone(), *v as i32)).collect();
    let sugg: Vec<String> = suggestions.iter().map(|s| s.to_string()).collect();
    map_i32(ui, label, &mut tmp, &sugg);
    *map = tmp.into_iter().map(|(k, v)| (k, v.max(0) as u32)).collect();
}

// ------------------------------------------------------------------ modifiers & features

pub fn modifiers(ui: &mut egui::Ui, list: &mut Vec<Modifier>) {
    let mut remove = None;
    if !list.is_empty() {
        egui::Grid::new(ui.id().with("mods")).num_columns(7).spacing([6.0, 3.0]).show(ui, |ui| {
            for h in ["Target", "Op", "Value", "Condition", "Note", "", ""] {
                ui.label(theme::muted(h).small());
            }
            ui.end_row();
            for (i, m) in list.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut m.target).desired_width(140.0));
                    egui::ComboBox::from_id_salt(("tgt", i)).selected_text("…").width(36.0).show_ui(ui, |ui| {
                        for t in COMMON_TARGETS {
                            if ui.selectable_label(false, t).clicked() {
                                m.target = t.to_string();
                            }
                        }
                    });
                });
                egui::ComboBox::from_id_salt(("op", i)).selected_text(format!("{:?}", m.op).to_lowercase()).width(92.0).show_ui(ui, |ui| {
                    for op in [ModOp::Add, ModOp::Set, ModOp::Max, ModOp::Min, ModOp::Mul, ModOp::Grant, ModOp::Expertise, ModOp::Advantage, ModOp::Disadvantage, ModOp::Note] {
                        ui.selectable_value(&mut m.op, op, format!("{op:?}").to_lowercase());
                    }
                });
                ui.add(egui::TextEdit::singleline(&mut m.value).desired_width(90.0).hint_text("2 · level"));
                ui.add(egui::TextEdit::singleline(&mut m.condition).desired_width(110.0).hint_text("armored"));
                ui.add(egui::TextEdit::singleline(&mut m.note).desired_width(120.0));
                ui.label("");
                if ui.small_button("✖").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    }
    if let Some(i) = remove {
        list.remove(i);
    }
    if ui.small_button("➕ modifier").clicked() {
        list.push(Modifier { target: "ac".into(), op: ModOp::Add, value: "1".into(), ..Default::default() });
    }
}

pub fn attack(ui: &mut egui::Ui, a: &mut Attack) {
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::muted("to hit"));
        ui.add(egui::DragValue::new(&mut a.to_hit));
        ui.label(theme::muted("reach/range"));
        ui.add(egui::TextEdit::singleline(&mut a.reach).desired_width(150.0).hint_text("reach 5 ft., one target"));
        ui.label(theme::muted("damage"));
        ui.add(egui::TextEdit::singleline(&mut a.damage).desired_width(80.0).hint_text("1d6 + 2"));
        ui.add(egui::TextEdit::singleline(&mut a.damage_type).desired_width(90.0).hint_text("slashing"));
        ui.label(theme::muted("extra"));
        ui.add(egui::TextEdit::singleline(&mut a.extra).desired_width(140.0));
    });
}

/// Editable list of features (traits / class features / creature actions).
pub fn features(ui: &mut egui::Ui, label: &str, list: &mut Vec<Feature>, level_label: Option<&str>, with_attack: bool) {
    section(ui, label);
    let mut remove = None;
    let mut swap: Option<(usize, usize)> = None;
    let n = list.len();
    for (i, f) in list.iter_mut().enumerate() {
        let title = if f.name.is_empty() { format!("{label} #{}", i + 1) } else { f.name.clone() };
        let header = if level_label.is_some() { format!("{title}   (level {})", f.level) } else { title };
        egui::CollapsingHeader::new(egui::RichText::new(header).strong()).id_salt((label, i)).default_open(f.name.is_empty()).show(ui, |ui| {
            theme::card().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(theme::muted("Name"));
                    ui.add(egui::TextEdit::singleline(&mut f.name).desired_width(220.0));
                    if let Some(l) = level_label {
                        ui.label(theme::muted(l));
                        ui.add(egui::DragValue::new(&mut f.level).range(1..=20));
                    }
                    ui.label(theme::muted("Uses"));
                    ui.add(egui::TextEdit::singleline(&mut f.uses).desired_width(90.0).hint_text("formula"));
                    ui.label(theme::muted("Recharge"));
                    egui::ComboBox::from_id_salt(("rech", label, i)).selected_text(if f.recharge.is_empty() { "—" } else { f.recharge.as_str() }).width(110.0).show_ui(ui, |ui| {
                        for r in ["", "short_rest", "long_rest", "dawn", "recharge 5-6", "recharge 6"] {
                            ui.selectable_value(&mut f.recharge, r.to_string(), if r.is_empty() { "—" } else { r });
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖ remove").clicked() {
                            remove = Some(i);
                        }
                        if i + 1 < n && ui.small_button("▼").clicked() {
                            swap = Some((i, i + 1));
                        }
                        if i > 0 && ui.small_button("▲").clicked() {
                            swap = Some((i, i - 1));
                        }
                    });
                });
                ui.add(egui::TextEdit::multiline(&mut f.description).desired_rows(2).desired_width(f32::INFINITY).hint_text("What it does"));
                if with_attack {
                    let mut has = f.attack.is_some();
                    if ui.checkbox(&mut has, "Attack roll").changed() {
                        f.attack = if has { Some(Attack::default()) } else { None };
                    }
                    if let Some(a) = f.attack.as_mut() {
                        attack(ui, a);
                    }
                }
                ui.label(theme::muted("Modifiers").small());
                modifiers(ui, &mut f.modifiers);
            });
        });
    }
    if let Some(i) = remove {
        list.remove(i);
    }
    if let Some((a, b)) = swap {
        list.swap(a, b);
    }
    if ui.add(theme::primary(&format!("➕ Add {}", label.trim_end_matches('s').to_lowercase()))).clicked() {
        list.push(Feature { level: 1, ..Default::default() });
    }
}

fn hidden_box(ui: &mut egui::Ui, hidden: &mut bool) {
    ui.checkbox(hidden, "Hidden from players").on_hover_text("Player apps will not see this entry at all");
}

fn spellcasting(ui: &mut egui::Ui, sc: &mut Option<Spellcasting>, class_hint: &str) {
    section(ui, "Spellcasting");
    let mut has = sc.is_some();
    if ui.checkbox(&mut has, "Has spellcasting").changed() {
        *sc = if has {
            Some(Spellcasting { ability: "int".into(), progression: "full".into(), prepared: format!("max(1, int_mod + class.{class_hint}.level)"), ..Default::default() })
        } else {
            None
        };
    }
    if let Some(s) = sc.as_mut() {
        row(ui, "Ability", |ui| {
            for k in ["int", "wis", "cha"] {
                ui.selectable_value(&mut s.ability, k.to_string(), k.to_uppercase());
            }
        });
        choice(ui, "Progression", &mut s.progression, &["full", "half", "third", "pact", "none"]);
        text(ui, "Prepared formula", &mut s.prepared);
        row(ui, "Cantrips by level", |ui| {
            let mut t = s.cantrips_known.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
            if ui.add(egui::TextEdit::singleline(&mut t).desired_width(f32::INFINITY).hint_text("3,3,3,4,4,…")).changed() {
                s.cantrips_known = t.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            }
        });
        row(ui, "Spells known by level", |ui| {
            let mut t = s.spells_known.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
            if ui.add(egui::TextEdit::singleline(&mut t).desired_width(f32::INFINITY).hint_text("leave empty for prepared casters")).changed() {
                s.spells_known = t.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            }
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.ritual_casting, "Ritual casting");
        });
        text_w(ui, "Focus", &mut s.focus, 200.0);
        chips(ui, "Extra spell lists", &mut s.spell_lists, &[]);
    }
}

// ------------------------------------------------------------------ per-kind forms

pub fn species(ui: &mut egui::Ui, s: &mut Species) {
    section(ui, "Basics");
    text_w(ui, "Name", &mut s.name, 300.0);
    choice(ui, "Size", &mut s.size, &SIZES);
    num_u32(ui, "Speed (ft)", &mut s.speed, 0..=120);
    map_u32(ui, "Other speeds", &mut s.speeds, &["fly", "swim", "climb", "burrow"]);
    num_u32(ui, "Darkvision (ft)", &mut s.darkvision, 0..=300);
    chips(ui, "Languages", &mut s.languages, &["Common", "Elvish", "Dwarvish", "Halfling", "Draconic", "Giant", "Gnomish", "Goblin", "Orc", "Infernal", "Celestial", "Sylvan", "Undercommon"]);
    chips(ui, "Tags", &mut s.tags, &[]);
    row(ui, "", |ui| hidden_box(ui, &mut s.hidden));
    multiline(ui, "Description", &mut s.description, 3);
    section(ui, "Ability bonuses & other modifiers");
    modifiers(ui, &mut s.modifiers);
    features(ui, "Traits", &mut s.traits, None, false);
}

pub fn class(ui: &mut egui::Ui, c: &mut Class, lib: &Library) {
    section(ui, "Basics");
    text_w(ui, "Name", &mut c.name, 300.0);
    row(ui, "Hit die", |ui| {
        for d in [6u32, 8, 10, 12] {
            ui.selectable_value(&mut c.hit_die, d, format!("d{d}"));
        }
    });
    ability_picker(ui, "Primary abilities", &mut c.primary_abilities);
    ability_picker(ui, "Saving throws", &mut c.saving_throws);
    chips(ui, "Armor", &mut c.armor_proficiencies, &["light", "medium", "heavy", "shields"]);
    chips(ui, "Weapons", &mut c.weapon_proficiencies, &["simple", "martial", "dagger", "quarterstaff", "light_crossbow", "shortsword", "longsword", "rapier", "hand_crossbow"]);
    chips(ui, "Tools", &mut c.tool_proficiencies, &["thieves' tools", "herbalism kit", "musical instrument"]);
    row(ui, "Skill choices", |ui| {
        ui.label(theme::muted("choose"));
        ui.add(egui::DragValue::new(&mut c.skill_choices.count).range(0..=10));
        ui.label(theme::muted("from (none = any)"));
    });
    skill_picker(ui, "", &mut c.skill_choices.from, lib);
    chips(ui, "Starting equipment", &mut c.starting_equipment, &[]);
    num_u32(ui, "Subclass at level", &mut c.subclass_level, 1..=20);
    text_w(ui, "Subclass label", &mut c.subclass_label, 220.0);
    row(ui, "ASI levels", |ui| {
        let mut t = c.asi_levels.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
        if ui.add(egui::TextEdit::singleline(&mut t).desired_width(200.0)).changed() {
            c.asi_levels = t.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        }
    });
    text(ui, "Multiclass prereq.", &mut c.multiclass_prerequisite);
    chips(ui, "Tags", &mut c.tags, &[]);
    row(ui, "", |ui| hidden_box(ui, &mut c.hidden));
    multiline(ui, "Description", &mut c.description, 3);
    let id = c.id.clone();
    spellcasting(ui, &mut c.spellcasting, &id);
    section(ui, "Always-on modifiers");
    modifiers(ui, &mut c.modifiers);
    features(ui, "Features", &mut c.features, Some("Class level"), false);
}

pub fn subclass(ui: &mut egui::Ui, s: &mut Subclass, lib: &Library) {
    section(ui, "Basics");
    text_w(ui, "Name", &mut s.name, 300.0);
    row(ui, "Parent class", |ui| {
        id_combo(ui, "subclass_parent", &mut s.class, &lib.list(Kind::Class), "— class —");
    });
    chips(ui, "Tags", &mut s.tags, &[]);
    row(ui, "", |ui| hidden_box(ui, &mut s.hidden));
    multiline(ui, "Description", &mut s.description, 3);
    let hint = s.class.clone();
    spellcasting(ui, &mut s.spellcasting, &hint);
    section(ui, "Always-on modifiers");
    modifiers(ui, &mut s.modifiers);
    features(ui, "Features", &mut s.features, Some("Class level"), false);
}

pub fn background(ui: &mut egui::Ui, b: &mut Background, lib: &Library) {
    section(ui, "Basics");
    text_w(ui, "Name", &mut b.name, 300.0);
    skill_picker(ui, "Skill proficiencies", &mut b.skill_proficiencies, lib);
    chips(ui, "Tool proficiencies", &mut b.tool_proficiencies, &["thieves' tools", "gaming set", "musical instrument", "vehicles (land)", "disguise kit", "forgery kit"]);
    chips(ui, "Languages", &mut b.languages, &["Common", "Elvish", "Dwarvish"]);
    num_u32(ui, "Language choices", &mut b.language_choices, 0..=5);
    chips(ui, "Starting equipment", &mut b.starting_equipment, &[]);
    chips(ui, "Tags", &mut b.tags, &[]);
    row(ui, "", |ui| hidden_box(ui, &mut b.hidden));
    multiline(ui, "Description", &mut b.description, 3);
    section(ui, "Feature");
    let mut has = b.feature.is_some();
    if ui.checkbox(&mut has, "Has a background feature").changed() {
        b.feature = if has { Some(Feature { level: 1, ..Default::default() }) } else { None };
    }
    if let Some(f) = b.feature.as_mut() {
        text_w(ui, "Feature name", &mut f.name, 300.0);
        multiline(ui, "Feature text", &mut f.description, 3);
    }
    section(ui, "Modifiers");
    modifiers(ui, &mut b.modifiers);
}

pub fn item(ui: &mut egui::Ui, it: &mut Item, lib: &Library) {
    section(ui, "Basics");
    text_w(ui, "Name", &mut it.name, 300.0);
    choice(ui, "Kind", &mut it.kind, &ITEM_KINDS);
    text_w(ui, "Cost", &mut it.cost, 120.0);
    row(ui, "Weight (lb)", |ui| {
        ui.add(egui::DragValue::new(&mut it.weight).speed(0.5).range(0.0..=5000.0));
    });
    choice(ui, "Rarity", &mut it.rarity, &RARITIES);
    row(ui, "", |ui| {
        ui.checkbox(&mut it.requires_attunement, "Requires attunement");
        hidden_box(ui, &mut it.hidden);
    });
    chips(ui, "Tags", &mut it.tags, &[]);
    multiline(ui, "Description", &mut it.description, 3);

    section(ui, "Weapon stats");
    let mut has = it.weapon.is_some();
    if ui.checkbox(&mut has, "Is a weapon").changed() {
        it.weapon = if has { Some(Weapon { damage: "1d6".into(), damage_type: "slashing".into(), category: "simple".into(), kind: "melee".into(), ..Default::default() }) } else { None };
        if has && it.kind == "gear" {
            it.kind = "weapon".into();
        }
    }
    if let Some(w) = it.weapon.as_mut() {
        row(ui, "Damage", |ui| {
            ui.add(egui::TextEdit::singleline(&mut w.damage).desired_width(70.0));
            egui::ComboBox::from_id_salt("dmgtype").selected_text(if w.damage_type.is_empty() { "type" } else { w.damage_type.as_str() }).width(120.0).show_ui(ui, |ui| {
                for t in &lib.rules.damage_types {
                    ui.selectable_value(&mut w.damage_type, t.clone(), t);
                }
            });
            ui.label(theme::muted("versatile"));
            ui.add(egui::TextEdit::singleline(&mut w.versatile).desired_width(60.0).hint_text("1d10"));
            ui.label(theme::muted("magic bonus"));
            ui.add(egui::DragValue::new(&mut w.bonus).range(-3..=5));
        });
        row(ui, "Category", |ui| {
            for c in ["simple", "martial"] {
                ui.selectable_value(&mut w.category, c.to_string(), c);
            }
            ui.separator();
            for k in ["melee", "ranged"] {
                ui.selectable_value(&mut w.kind, k.to_string(), k);
            }
            ui.label(theme::muted("range"));
            ui.add(egui::TextEdit::singleline(&mut w.range).desired_width(80.0).hint_text("80/320"));
        });
        chips(ui, "Properties", &mut w.properties, &WEAPON_PROPS);
    }

    section(ui, "Armor stats");
    let mut has_a = it.armor.is_some();
    if ui.checkbox(&mut has_a, "Is armor or a shield").changed() {
        it.armor = if has_a { Some(Armor { base_ac: 11, category: "light".into(), ..Default::default() }) } else { None };
        if has_a && it.kind == "gear" {
            it.kind = "armor".into();
        }
    }
    if let Some(a) = it.armor.as_mut() {
        row(ui, "Base AC", |ui| {
            ui.add(egui::DragValue::new(&mut a.base_ac).range(0..=30));
            ui.label(theme::muted("magic bonus"));
            ui.add(egui::DragValue::new(&mut a.bonus).range(-3..=5));
        });
        row(ui, "Category", |ui| {
            for c in ["light", "medium", "heavy", "shield"] {
                if ui.selectable_value(&mut a.category, c.to_string(), c).clicked() {
                    a.dex_cap = match c {
                        "light" => None,
                        "medium" => Some(2),
                        _ => Some(0),
                    };
                    if c == "shield" {
                        it.kind = "shield".into();
                    }
                }
            }
        });
        row(ui, "Dex bonus cap", |ui| {
            let mut capped = a.dex_cap.is_some();
            if ui.checkbox(&mut capped, "capped").changed() {
                a.dex_cap = if capped { Some(2) } else { None };
            }
            if let Some(c) = a.dex_cap.as_mut() {
                ui.add(egui::DragValue::new(c).range(0..=10));
            }
        });
        row(ui, "Str requirement", |ui| {
            ui.add(egui::DragValue::new(&mut a.strength_requirement).range(0..=30));
            ui.checkbox(&mut a.stealth_disadvantage, "Stealth disadvantage");
        });
    }

    section(ui, "Modifiers while equipped");
    modifiers(ui, &mut it.modifiers);
}

pub fn feat(ui: &mut egui::Ui, f: &mut Feat) {
    section(ui, "Basics");
    text_w(ui, "Name", &mut f.name, 300.0);
    text(ui, "Prerequisite", &mut f.prerequisite);
    chips(ui, "Tags", &mut f.tags, &[]);
    row(ui, "", |ui| hidden_box(ui, &mut f.hidden));
    multiline(ui, "Description", &mut f.description, 4);
    section(ui, "Modifiers");
    modifiers(ui, &mut f.modifiers);
}

pub fn spell(ui: &mut egui::Ui, s: &mut Spell, lib: &Library) {
    section(ui, "Basics");
    text_w(ui, "Name", &mut s.name, 300.0);
    row(ui, "Level", |ui| {
        ui.add(egui::DragValue::new(&mut s.level).range(0..=9));
        ui.label(theme::muted(if s.level == 0 { "(cantrip)" } else { "" }));
    });
    choice(ui, "School", &mut s.school, &SCHOOLS);
    text_w(ui, "Casting time", &mut s.casting_time, 260.0);
    text_w(ui, "Range", &mut s.range, 260.0);
    text_w(ui, "Components", &mut s.components, 260.0);
    text_w(ui, "Duration", &mut s.duration, 260.0);
    row(ui, "", |ui| {
        ui.checkbox(&mut s.concentration, "Concentration");
        ui.checkbox(&mut s.ritual, "Ritual");
        ui.checkbox(&mut s.attack, "Spell attack roll");
        hidden_box(ui, &mut s.hidden);
    });
    let classes: Vec<&str> = lib.classes.keys().map(|k| k.as_str()).collect();
    chips(ui, "Classes", &mut s.classes, &classes);
    section(ui, "Effect");
    row(ui, "Damage / healing", |ui| {
        ui.add(egui::TextEdit::singleline(&mut s.damage).desired_width(120.0).hint_text("8d6"));
        egui::ComboBox::from_id_salt("sp_dmgtype").selected_text(if s.damage_type.is_empty() { "type" } else { s.damage_type.as_str() }).width(120.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut s.damage_type, String::new(), "—");
            ui.selectable_value(&mut s.damage_type, "healing".to_string(), "healing");
            for t in &lib.rules.damage_types {
                ui.selectable_value(&mut s.damage_type, t.clone(), t);
            }
        });
    });
    row(ui, "Saving throw", |ui| {
        ui.selectable_value(&mut s.save, String::new(), "none");
        for k in ABILITY_KEYS {
            ui.selectable_value(&mut s.save, k.to_string(), k.to_uppercase());
        }
    });
    row(ui, "Area", |ui| {
        let mut has = s.area.is_some();
        if ui.checkbox(&mut has, "has an area").changed() {
            s.area = if has { Some(Area { shape: "sphere".into(), size: 20, width: None }) } else { None };
        }
        if let Some(a) = s.area.as_mut() {
            egui::ComboBox::from_id_salt("shape").selected_text(a.shape.clone()).width(90.0).show_ui(ui, |ui| {
                for sh in ["sphere", "cube", "cone", "line", "cylinder"] {
                    ui.selectable_value(&mut a.shape, sh.to_string(), sh);
                }
            });
            ui.add(egui::DragValue::new(&mut a.size).range(5..=1000).speed(5).suffix(" ft"));
            if a.shape == "line" {
                let mut w = a.width.unwrap_or(5);
                ui.label(theme::muted("width"));
                if ui.add(egui::DragValue::new(&mut w).range(5..=100).speed(5).suffix(" ft")).changed() {
                    a.width = Some(w);
                }
            }
        }
    });
    chips(ui, "Tags", &mut s.tags, &[]);
    multiline(ui, "Description", &mut s.description, 5);
    multiline(ui, "At higher levels", &mut s.higher_levels, 2);
}

pub fn creature(ui: &mut egui::Ui, c: &mut Creature, lib: &Library) {
    section(ui, "Basics");
    text_w(ui, "Name", &mut c.name, 300.0);
    row(ui, "Size / type", |ui| {
        egui::ComboBox::from_id_salt("csize").selected_text(c.size.clone()).width(110.0).show_ui(ui, |ui| {
            for s in SIZES {
                ui.selectable_value(&mut c.size, s.to_string(), s);
            }
        });
        ui.add(egui::TextEdit::singleline(&mut c.kind).desired_width(180.0).hint_text("humanoid (goblinoid)"));
        ui.label(theme::muted("alignment"));
        ui.add(egui::TextEdit::singleline(&mut c.alignment).desired_width(140.0));
    });
    row(ui, "Armor class", |ui| {
        ui.add(egui::DragValue::new(&mut c.ac).range(0..=40));
        ui.add(egui::TextEdit::singleline(&mut c.ac_note).desired_width(200.0).hint_text("natural armor"));
    });
    row(ui, "Hit points", |ui| {
        ui.add(egui::DragValue::new(&mut c.hp).range(0..=2000));
        ui.add(egui::TextEdit::singleline(&mut c.hp_formula).desired_width(120.0).hint_text("2d8 + 2"));
        if ui.small_button("average from formula").clicked() && !c.hp_formula.is_empty() {
            if let Ok(r) = dnd_core::expr::eval(&c.hp_formula, &dnd_core::expr::Env::new(), dnd_core::expr::Mode::Average, None) {
                c.hp = r.value.floor() as i32;
            }
        }
    });
    row(ui, "Challenge", |ui| {
        ui.add(egui::TextEdit::singleline(&mut c.cr).desired_width(50.0).hint_text("1/4"));
        ui.label(theme::muted("XP"));
        ui.add(egui::DragValue::new(&mut c.xp).range(0..=200000).speed(10));
        ui.label(theme::muted(format!("PB {}", crate::ui::bonus(c.proficiency_bonus()))));
    });
    map_u32(ui, "Speed", &mut c.speed, &["walk", "fly", "swim", "climb", "burrow"]);
    abilities(ui, &mut c.abilities);
    let keys: Vec<String> = ABILITY_KEYS.iter().map(|k| k.to_string()).collect();
    map_i32(ui, "Save bonuses", &mut c.saves, &keys);
    let skills: Vec<String> = lib.rules.skills.iter().map(|s| s.key.clone()).collect();
    map_i32(ui, "Skill bonuses", &mut c.skills, &skills);
    let dt: Vec<&str> = lib.rules.damage_types.iter().map(|s| s.as_str()).collect();
    chips(ui, "Vulnerabilities", &mut c.damage_vulnerabilities, &dt);
    chips(ui, "Resistances", &mut c.damage_resistances, &dt);
    chips(ui, "Immunities", &mut c.damage_immunities, &dt);
    let conds: Vec<&str> = lib.rules.conditions.iter().map(|s| s.as_str()).collect();
    chips(ui, "Condition immunities", &mut c.condition_immunities, &conds);
    chips(ui, "Senses", &mut c.senses, &["darkvision 60 ft.", "darkvision 120 ft.", "blindsight 10 ft.", "tremorsense 60 ft.", "truesight 60 ft."]);
    chips(ui, "Languages", &mut c.languages, &["Common", "Goblin", "Orc", "Giant", "Draconic", "Abyssal", "Infernal", "Undercommon"]);
    let spells: Vec<&str> = lib.spells.keys().map(|k| k.as_str()).collect();
    chips(ui, "Spells", &mut c.spells, &spells);
    chips(ui, "Tags", &mut c.tags, &[]);
    row(ui, "", |ui| hidden_box(ui, &mut c.hidden));
    multiline(ui, "Description / lore", &mut c.description, 3);
    features(ui, "Traits", &mut c.traits, None, false);
    features(ui, "Actions", &mut c.actions, None, true);
    features(ui, "Bonus actions", &mut c.bonus_actions, None, true);
    features(ui, "Reactions", &mut c.reactions, None, true);
    features(ui, "Legendary actions", &mut c.legendary_actions, None, true);
}

/// Dispatch to the right form. Returns nothing; the caller diffs the entry.
pub fn entry_form(ui: &mut egui::Ui, e: &mut Entry, lib: &Library) {
    match e {
        Entry::Species(x) => species(ui, x),
        Entry::Class(x) => class(ui, x, lib),
        Entry::Subclass(x) => subclass(ui, x, lib),
        Entry::Background(x) => background(ui, x, lib),
        Entry::Item(x) => item(ui, x, lib),
        Entry::Feat(x) => feat(ui, x),
        Entry::Spell(x) => spell(ui, x, lib),
        Entry::Creature(x) => creature(ui, x, lib),
    }
}

/// A blank entry of the given kind.
pub fn blank(kind: Kind, id: &str, name: &str) -> Entry {
    let id = id.to_string();
    let name = name.to_string();
    match kind {
        Kind::Species => Entry::Species(Species { id, name, size: "Medium".into(), speed: 30, languages: vec!["Common".into()], ..Default::default() }),
        Kind::Class => Entry::Class(Class { id, name, hit_die: 8, subclass_level: 3, asi_levels: vec![4, 8, 12, 16, 19], ..Default::default() }),
        Kind::Subclass => Entry::Subclass(Subclass { id, name, ..Default::default() }),
        Kind::Background => Entry::Background(Background { id, name, ..Default::default() }),
        Kind::Item => Entry::Item(Item { id, name, kind: "gear".into(), ..Default::default() }),
        Kind::Feat => Entry::Feat(Feat { id, name, ..Default::default() }),
        Kind::Spell => Entry::Spell(Spell { id, name, level: 1, casting_time: "1 action".into(), range: "60 feet".into(), components: "V, S".into(), duration: "Instantaneous".into(), ..Default::default() }),
        Kind::Creature => Entry::Creature(Creature { id, name, size: "Medium".into(), ac: 10, hp: 10, cr: "1/4".into(), speed: [("walk".to_string(), 30u32)].into_iter().collect(), ..Default::default() }),
    }
}

/// Unused-import guard for `list_edit` kept for other modules.
#[allow(dead_code)]
fn _keep(ui: &mut egui::Ui, v: &mut Vec<String>) {
    list_edit(ui, "", v);
}
