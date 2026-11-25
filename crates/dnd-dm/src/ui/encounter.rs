//! Initiative tracker, HP, positions, distance / radius / area tools.

use crate::app::DmApp;
use crate::ui::{bonus, heading, id_combo};
use dnd_core::grid::{self, Pos};
use dnd_core::model::*;
use dnd_core::World;
use eframe::egui;

#[derive(Default)]
pub struct EncounterState {
    pub selected: Option<String>,
    pub pc_pick: String,
    pub creature_filter: String,
    pub creature_pick: String,
    pub creature_count: u32,
    pub custom_name: String,
    pub custom_hp: i32,
    pub custom_ac: i32,
    pub amount: i32,
    pub dist_a: String,
    pub dist_b: String,
    pub radius_ft: u32,
    pub area_shape: usize,
    pub area_size: u32,
    pub area_dir: usize,
    pub area_origin: String,
    pub show_stat_block: bool,
    /// Who the selected combatant attacks, how, and what came of it.
    pub target: String,
    pub mode: usize,
    pub last_result: String,
}

pub const MODES: [(&str, dnd_core::play::RollMode); 3] =
    [("normal", dnd_core::play::RollMode::Normal), ("advantage", dnd_core::play::RollMode::Advantage), ("disadvantage", dnd_core::play::RollMode::Disadvantage)];

pub const SHAPES: [&str; 4] = ["sphere", "cube", "cone", "line"];
pub const DIRS: [(&str, (f64, f64)); 8] = [
    ("→ E", (1.0, 0.0)),
    ("↘ SE", (1.0, 1.0)),
    ("↓ S", (0.0, 1.0)),
    ("↙ SW", (-1.0, 1.0)),
    ("← W", (-1.0, 0.0)),
    ("↖ NW", (-1.0, -1.0)),
    ("↑ N", (0.0, -1.0)),
    ("↗ NE", (1.0, -1.0)),
];

/// Cells highlighted by the current area tool (used by the map view too).
pub fn area_cells(st: &EncounterState, w: &World) -> Vec<Pos> {
    let origin = w.campaign.encounter.find(&st.area_origin).and_then(|c| c.pos);
    let Some(origin) = origin else { return Vec::new() };
    let fps = w.library.rules.feet_per_square;
    let rule = w.campaign.settings.distance_rule;
    let dir = DIRS[st.area_dir.min(DIRS.len() - 1)].1;
    match SHAPES[st.area_shape.min(3)] {
        "sphere" => grid::cells_in_sphere(origin, st.area_size, fps, rule),
        "cube" => grid::cells_in_cube(origin, st.area_size, fps),
        "cone" => grid::cells_in_cone(origin, dir, st.area_size, fps),
        _ => grid::cells_in_line(origin, dir, st.area_size, 5, fps),
    }
}

pub fn show(app: &mut DmApp, ui: &mut egui::Ui) {
    let world = app.world.clone();
    let mut w = world.write().unwrap();
    let w = &mut *w;
    let st = &mut app.encounter;
    if st.creature_count == 0 {
        st.creature_count = 1;
    }
    if st.radius_ft == 0 {
        st.radius_ft = 20;
    }
    if st.area_size == 0 {
        st.area_size = 20;
    }
    let before = w.campaign.encounter.clone();

    // ---------------------------------------------------------- add panel
    egui::SidePanel::right("enc_tools").resizable(true).default_width(330.0).show_inside(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            heading(ui, "Add combatants");
            let pcs: Vec<(String, String)> = w.campaign.characters.iter().map(|c| (c.id.clone(), c.name.clone())).collect();
            ui.horizontal(|ui| {
                id_combo(ui, "pcpick", &mut st.pc_pick, &pcs, "— player character —");
                if ui.button("Add").clicked() && !st.pc_pick.is_empty() {
                    if let Some(c) = w.combatant_from_character(&st.pc_pick) {
                        w.campaign.encounter.combatants.push(c);
                    }
                }
                if ui.button("All PCs").clicked() {
                    let ids: Vec<String> = pcs.iter().map(|p| p.0.clone()).collect();
                    for id in ids {
                        let already = w.campaign.encounter.combatants.iter().any(|c| matches!(&c.kind, CombatantKind::Pc { id: x } if x == &id));
                        if !already {
                            if let Some(c) = w.combatant_from_character(&id) {
                                w.campaign.encounter.combatants.push(c);
                            }
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut st.creature_filter).hint_text("filter").desired_width(90.0));
                let f = st.creature_filter.to_lowercase();
                let list: Vec<(String, String)> = w.library.list(Kind::Creature).into_iter().filter(|(id, n)| f.is_empty() || n.to_lowercase().contains(&f) || id.contains(&f)).collect();
                id_combo(ui, "crpick", &mut st.creature_pick, &list, "— creature —");
                ui.add(egui::DragValue::new(&mut st.creature_count).range(1..=30));
                if ui.button("Add").clicked() && !st.creature_pick.is_empty() {
                    for _ in 0..st.creature_count {
                        if let Some(c) = w.combatant_from_creature(&st.creature_pick, None) {
                            let name = if st.creature_count > 1 { format!("{} {}", c.name, c.id.rsplit('_').next().filter(|s| s.parse::<u32>().is_ok()).unwrap_or("1")) } else { c.name.clone() };
                            let mut c = c;
                            c.name = name;
                            w.campaign.encounter.combatants.push(c);
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut st.custom_name).hint_text("custom name").desired_width(110.0));
                ui.label("HP");
                ui.add(egui::DragValue::new(&mut st.custom_hp).range(0..=999));
                ui.label("AC");
                ui.add(egui::DragValue::new(&mut st.custom_ac).range(0..=40));
                if ui.button("Add").clicked() && !st.custom_name.trim().is_empty() {
                    let id = w.campaign.encounter.unique_id(&st.custom_name);
                    w.campaign.encounter.combatants.push(Combatant { id, name: st.custom_name.trim().into(), hp: st.custom_hp, hp_max: st.custom_hp, ac: st.custom_ac, ..Default::default() });
                    st.custom_name.clear();
                }
            });

            heading(ui, "Selected combatant");
            let sel_id = st.selected.clone();
            match sel_id.as_ref().and_then(|id| w.campaign.encounter.find(id).cloned()) {
                None => {
                    ui.label(egui::RichText::new("Click a row in the tracker.").weak());
                }
                Some(c) => {
                    ui.label(egui::RichText::new(&c.name).strong().size(16.0));
                    ui.horizontal(|ui| {
                        ui.label("Amount");
                        ui.add(egui::DragValue::new(&mut st.amount).range(0..=999));
                        if ui.button("💥 Damage").clicked() {
                            let amt = st.amount;
                            if let Some(cm) = w.campaign.encounter.find_mut(&c.id) {
                                let lost = cm.damage(amt);
                                let hp = cm.hp;
                                w.campaign.log(format!("{} takes {amt} damage ({lost} to HP), now {hp}", c.name));
                            }
                        }
                        if ui.button("💚 Heal").clicked() {
                            let amt = st.amount;
                            if let Some(cm) = w.campaign.encounter.find_mut(&c.id) {
                                cm.heal(amt);
                                let hp = cm.hp;
                                w.campaign.log(format!("{} heals {amt}, now {hp}", c.name));
                            }
                        }
                        if ui.button("Temp").clicked() {
                            let amt = st.amount;
                            if let Some(cm) = w.campaign.encounter.find_mut(&c.id) {
                                cm.temp_hp = cm.temp_hp.max(amt);
                            }
                        }
                    });
                    if let Some(cm) = w.campaign.encounter.find_mut(&c.id) {
                        ui.horizontal(|ui| {
                            ui.label("Concentrating on");
                            ui.add(egui::TextEdit::singleline(&mut cm.concentrating_on).desired_width(140.0));
                        });
                        ui.label("Conditions");
                        ui.horizontal_wrapped(|ui| {
                            for cond in &w.library.rules.conditions {
                                let mut on = cm.conditions.contains(cond);
                                if ui.checkbox(&mut on, cond).changed() {
                                    if on {
                                        cm.conditions.push(cond.clone());
                                    } else {
                                        cm.conditions.retain(|x| x != cond);
                                    }
                                }
                            }
                        });
                        ui.label("Notes");
                        ui.add(egui::TextEdit::multiline(&mut cm.notes).desired_rows(2).desired_width(f32::INFINITY));
                    }
                    // ---- attack someone: rolled against their armor class, damage applied
                    let targets: Vec<(String, String)> = w.campaign.encounter.combatants.iter().filter(|x| x.id != c.id).map(|x| (x.id.clone(), x.name.clone())).collect();
                    let attacks: Vec<(usize, String)> = match &c.kind {
                        CombatantKind::Creature { template } => w
                            .library
                            .creatures
                            .get(template)
                            .map(|cr| cr.actions.iter().enumerate().filter_map(|(i, a)| a.attack.as_ref().map(|k| (i, format!("{} {} · {}", a.name, bonus(k.to_hit), k.damage)))).collect())
                            .unwrap_or_default(),
                        CombatantKind::Pc { id } => w.sheet(id).map(|s| s.attacks.iter().enumerate().map(|(i, a)| (i, format!("{} {} · {}", a.name, bonus(a.bonus), a.damage))).collect()).unwrap_or_default(),
                        CombatantKind::Custom => Vec::new(),
                    };
                    if !attacks.is_empty() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Attack").strong());
                        ui.horizontal(|ui| {
                            id_combo(ui, "atk_target", &mut st.target, &targets, "— target —");
                            egui::ComboBox::from_id_salt("atk_mode").width(90.0).selected_text(MODES[st.mode.min(2)].0).show_ui(ui, |ui| {
                                for (i, m) in MODES.iter().enumerate() {
                                    ui.selectable_value(&mut st.mode, i, m.0);
                                }
                            });
                        });
                        ui.horizontal_wrapped(|ui| {
                            for (i, label) in &attacks {
                                if ui.add_enabled(!st.target.is_empty(), egui::Button::new(format!("⚔ {label}"))).on_disabled_hover_text("Pick a target first").clicked() {
                                    let mode = MODES[st.mode.min(2)].1;
                                    let r = match &c.kind {
                                        CombatantKind::Pc { id } => w.pc_attack(id, *i, &st.target, mode),
                                        _ => w.creature_attack(&c.id, *i, &st.target, mode),
                                    };
                                    st.last_result = match r {
                                        Ok(r) => r.text,
                                        Err(e) => e,
                                    };
                                }
                            }
                        });
                        if !st.last_result.is_empty() {
                            ui.label(egui::RichText::new(&st.last_result).color(crate::ui::theme::ORANGE));
                        }
                    }
                    match &c.kind {
                        CombatantKind::Creature { template } => {
                            ui.checkbox(&mut st.show_stat_block, "Show stat block");
                            if st.show_stat_block {
                                if let Some(cr) = w.library.creatures.get(template) {
                                    stat_block(ui, cr);
                                }
                            }
                        }
                        CombatantKind::Pc { id } => {
                            if let Some(s) = w.sheet(id) {
                                ui.label(egui::RichText::new(format!("AC {} · HP {}/{} · init {} · PP {} · speed {}", s.ac, s.hp_current, s.hp_max, bonus(s.initiative), s.passive_perception, s.speed)).weak());
                                ui.label(egui::RichText::new("HP changes here sync back to the character sheet.").weak().small());
                            }
                        }
                        CombatantKind::Custom => {}
                    }
                }
            }

            heading(ui, "Distance");
            let names: Vec<(String, String)> = w.campaign.encounter.combatants.iter().map(|c| (c.id.clone(), c.name.clone())).collect();
            ui.horizontal(|ui| {
                id_combo(ui, "dista", &mut st.dist_a, &names, "— A —");
                id_combo(ui, "distb", &mut st.dist_b, &names, "— B —");
            });
            if let (Some(a), Some(b)) = (w.campaign.encounter.find(&st.dist_a), w.campaign.encounter.find(&st.dist_b)) {
                match grid::combatant_distance_ft(a, b, &w.library.rules, w.campaign.settings.distance_rule) {
                    Some(d) => {
                        ui.label(egui::RichText::new(format!("{d} ft apart ({:?} rule)", w.campaign.settings.distance_rule)).strong());
                    }
                    None => {
                        ui.label(egui::RichText::new("Set positions on both (x,y) or on the Map tab.").weak());
                    }
                }
            }

            heading(ui, "Within radius");
            ui.horizontal(|ui| {
                ui.label("Center");
                id_combo(ui, "radc", &mut st.area_origin, &names, "— combatant —");
                ui.label("ft");
                ui.add(egui::DragValue::new(&mut st.radius_ft).range(5..=1000).speed(5));
            });
            if let Some(center) = w.campaign.encounter.find(&st.area_origin).and_then(|c| c.pos) {
                let hits = grid::combatants_within(center, st.radius_ft, &w.campaign.encounter.combatants, &w.library.rules, w.campaign.settings.distance_rule);
                for (c, d) in hits.iter().filter(|(c, _)| c.id != st.area_origin) {
                    ui.label(format!("• {} — {d} ft", c.name));
                }
                if hits.len() <= 1 {
                    ui.label(egui::RichText::new("nobody else in range").weak());
                }
            }

            heading(ui, "Area of effect");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("shape").selected_text(SHAPES[st.area_shape.min(3)]).show_ui(ui, |ui| {
                    for (i, s) in SHAPES.iter().enumerate() {
                        ui.selectable_value(&mut st.area_shape, i, *s);
                    }
                });
                ui.add(egui::DragValue::new(&mut st.area_size).range(5..=500).speed(5).suffix(" ft"));
                if SHAPES[st.area_shape.min(3)] == "cone" || SHAPES[st.area_shape.min(3)] == "line" {
                    egui::ComboBox::from_id_salt("dir").selected_text(DIRS[st.area_dir.min(7)].0).show_ui(ui, |ui| {
                        for (i, d) in DIRS.iter().enumerate() {
                            ui.selectable_value(&mut st.area_dir, i, d.0);
                        }
                    });
                }
            });
            ui.label(egui::RichText::new("Origin = the 'Center' combatant above. Highlighted on the Map tab.").weak().small());
            let cells = area_cells(st, w);
            if !cells.is_empty() {
                let hits = grid::combatants_in_cells(&cells, &w.campaign.encounter.combatants, &w.library.rules);
                ui.label(format!("{} squares; affected:", cells.len()));
                for c in hits.iter().filter(|c| c.id != st.area_origin) {
                    ui.label(format!("• {}", c.name));
                }
            }
        });
    });

    // ---------------------------------------------------------- what the table sees
    egui::TopBottomPanel::bottom("table_feed").resizable(true).default_height(120.0).show_inside(ui, |ui| {
        ui.label(egui::RichText::new("At the table (players see this)").weak().small());
        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
            let t = &w.campaign.table;
            for e in &t[t.len().saturating_sub(40)..] {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(&e.time).weak().small());
                    if !e.who.is_empty() {
                        ui.label(egui::RichText::new(&e.who).strong().color(crate::ui::theme::ORANGE));
                    }
                    ui.label(&e.text);
                });
            }
        });
    });

    // ---------------------------------------------------------- tracker
    egui::CentralPanel::default().show_inside(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Encounter");
            ui.add(egui::TextEdit::singleline(&mut w.campaign.encounter.name).desired_width(160.0).hint_text("name"));
            if ui.button("🎲 Roll initiative (all)").clicked() {
                w.roll_initiative(false);
            }
            if ui.button("Roll missing").clicked() {
                w.roll_initiative(true);
            }
            if ui.button("Sort").clicked() {
                w.campaign.encounter.sort_by_initiative();
            }
            ui.separator();
            if !w.campaign.encounter.active {
                if ui.button("▶ Start").clicked() {
                    let name = w.campaign.encounter.name.clone();
                    w.campaign.log(format!("encounter '{name}' started"));
                    w.start_encounter();
                }
            } else {
                if ui.button("◀ Prev").clicked() {
                    w.campaign.encounter.prev_turn();
                }
                if ui.button("Next ▶").clicked() {
                    w.next_turn();
                }
                ui.label(egui::RichText::new(format!("Round {} · {}", w.campaign.encounter.round, w.campaign.encounter.current().map(|c| c.name.clone()).unwrap_or_default())).strong());
                if ui.button("⏹ End").clicked() {
                    w.campaign.encounter.end();
                    w.campaign.log("encounter ended");
                }
            }
            ui.separator();
            if ui.button("Clear all").clicked() {
                w.campaign.encounter = Encounter::default();
                st.selected = None;
            }
            ui.label("Distance rule");
            egui::ComboBox::from_id_salt("drule").selected_text(format!("{:?}", w.campaign.settings.distance_rule)).show_ui(ui, |ui| {
                for r in [DistanceRule::Chebyshev, DistanceRule::Alternating, DistanceRule::Euclidean] {
                    ui.selectable_value(&mut w.campaign.settings.distance_rule, r, format!("{r:?}"));
                }
            });
        });
        ui.separator();
        let turn = w.campaign.encounter.turn;
        let active = w.campaign.encounter.active;
        let mut remove: Option<String> = None;
        let mut select: Option<String> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("tracker").striped(true).num_columns(11).spacing([8.0, 4.0]).show(ui, |ui| {
                for h in ["", "Init", "Name", "HP", "Max", "Temp", "AC", "X", "Y", "Conditions", ""] {
                    ui.label(egui::RichText::new(h).weak());
                }
                ui.end_row();
                for (i, c) in w.campaign.encounter.combatants.iter_mut().enumerate() {
                    let is_turn = active && i == turn;
                    let is_sel = st.selected.as_deref() == Some(c.id.as_str());
                    let marker = if is_turn { "▶" } else if is_sel { "⏺" } else { "" };
                    if ui.selectable_label(is_sel, marker).clicked() {
                        select = Some(c.id.clone());
                    }
                    ui.add(egui::DragValue::new(&mut c.initiative));
                    let mut name = egui::RichText::new(&c.name);
                    if c.is_down() {
                        name = name.strikethrough();
                    }
                    if !c.visible {
                        name = name.weak();
                    }
                    if matches!(c.kind, CombatantKind::Pc { .. }) {
                        name = name.color(crate::ui::theme::PC_BLUE);
                    }
                    if ui.selectable_label(false, name).clicked() {
                        select = Some(c.id.clone());
                    }
                    ui.add(egui::DragValue::new(&mut c.hp));
                    ui.add(egui::DragValue::new(&mut c.hp_max));
                    ui.add(egui::DragValue::new(&mut c.temp_hp));
                    ui.add(egui::DragValue::new(&mut c.ac));
                    let mut p = c.pos.unwrap_or(Pos::new(-1, -1));
                    let mut changed = false;
                    changed |= ui.add(egui::DragValue::new(&mut p.x).range(-1..=200)).changed();
                    changed |= ui.add(egui::DragValue::new(&mut p.y).range(-1..=200)).changed();
                    if changed {
                        c.pos = if p.x < 0 || p.y < 0 { None } else { Some(p) };
                    }
                    ui.label(egui::RichText::new(c.conditions.join(", ")).small());
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut c.visible, "").on_hover_text("visible to players");
                        if ui.small_button("✖").clicked() {
                            remove = Some(c.id.clone());
                        }
                    });
                    ui.end_row();
                }
            });
        });
        if let Some(id) = select {
            st.selected = Some(id.clone());
            st.area_origin = id;
        }
        if let Some(id) = remove {
            w.campaign.encounter.remove(&id);
            if st.selected.as_deref() == Some(id.as_str()) {
                st.selected = None;
            }
        }
    });

    // sync PC hp back to character sheets
    if w.campaign.encounter != before {
        let updates: Vec<(String, i32, i32, Vec<String>)> = w
            .campaign
            .encounter
            .combatants
            .iter()
            .filter_map(|c| match &c.kind {
                CombatantKind::Pc { id } => Some((id.clone(), c.hp, c.temp_hp, c.conditions.clone())),
                _ => None,
            })
            .collect();
        for (id, hp, temp, conditions) in updates {
            if let Some(ch) = w.campaign.character_mut(&id) {
                ch.hp.current = hp;
                ch.hp.temp = temp;
                ch.conditions = conditions;
            }
        }
        w.touch();
    }
}

pub fn stat_block(ui: &mut egui::Ui, c: &Creature) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new(&c.name).strong());
        ui.label(egui::RichText::new(format!("{} {}, {}", c.size, c.kind, c.alignment)).italics().small());
        ui.label(format!("AC {} {} · HP {} ({}) · Speed {}", c.ac, c.ac_note, c.hp, c.hp_formula, c.speed.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", ")));
        let ab: Vec<String> = c.abilities.iter().map(|(k, v)| format!("{} {}({})", k.to_uppercase(), v, bonus(dnd_core::ability_mod(v)))).collect();
        ui.label(egui::RichText::new(ab.join(" ")).monospace().small());
        if !c.senses.is_empty() {
            ui.label(format!("Senses {}", c.senses.join(", ")));
        }
        ui.label(format!("CR {} ({} XP) · PB {}", c.cr, c.xp, bonus(c.proficiency_bonus())));
        for (title, list) in [("Traits", &c.traits), ("Actions", &c.actions), ("Bonus actions", &c.bonus_actions), ("Reactions", &c.reactions), ("Legendary", &c.legendary_actions)] {
            if list.is_empty() {
                continue;
            }
            ui.label(egui::RichText::new(title).strong().small());
            for f in list {
                let atk = f.attack.as_ref().map(|a| format!(" {} to hit, {}, {} {}", bonus(a.to_hit), a.reach, a.damage, a.damage_type)).unwrap_or_default();
                ui.label(egui::RichText::new(format!("{}.{atk} {}", f.name, f.description)).small());
            }
        }
    });
}
