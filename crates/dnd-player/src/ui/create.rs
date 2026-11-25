//! Character builder. Between sessions a player makes a character here and
//! sends it to the DM's server; the draft is kept on the device, so it can
//! be worked on away from the table and uploaded later.

use crate::app::{PlayerApp, Screen};
use crate::client::Cmd;
use crate::theme;
use dnd_core::api::BuilderOptions;
use dnd_core::sheet::ability_name;
use dnd_core::{builder, Advancement, Character, ClassLevel, Library, Sheet, ABILITY_KEYS};

/// Where the draft keeps its starting-equipment choices ("0,1,0").
const GEAR_KEY: &str = "_gear";

/// The starting-equipment option chosen for each line of the draft.
pub fn gear_choices(draft: &Character) -> Vec<usize> {
    draft.custom.get(GEAR_KEY).map(|s| s.split(',').filter_map(|p| p.trim().parse().ok()).collect()).unwrap_or_default()
}

fn set_gear_choices(draft: &mut Character, v: &[usize]) {
    if v.iter().all(|c| *c == 0) {
        draft.custom.remove(GEAR_KEY);
    } else {
        draft.custom.insert(GEAR_KEY.into(), v.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","));
    }
}

pub fn show(app: &mut PlayerApp, ctx: &egui::Context) {
    let (inset_top, inset_bottom) = crate::system_insets(ctx);
    let level = app.start_level();
    egui::TopBottomPanel::top("create_top").frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin { left: 12, right: 12, top: 12 + inset_top as i8, bottom: 12 })).show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("‹").clicked() {
                app.save_draft();
                let has_character = app.seat.as_ref().map(|s| s.character.is_some()).unwrap_or(false);
                app.screen = if has_character && app.character.is_some() {
                    Screen::Sheet
                } else if app.seat.is_some() && app.online {
                    Screen::Pick
                } else {
                    Screen::Connect
                };
            }
            ui.heading(theme::accent("Make your character"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dot = if app.online { ("⏺", theme::GOOD) } else { ("○", theme::MUTED) };
                ui.label(egui::RichText::new(dot.0).color(dot.1));
            });
        });
        ui.label(theme::muted(format!("Level {level} · saved on this device as you go")));
    });
    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin { left: 14, right: 14, top: 10, bottom: 14 + inset_bottom as i8 })).show(ctx, |ui| {
        // borrow the options out of the app while the form edits the draft
        match (app.builder.take(), app.builder_lib.take()) {
            (Some(opts), Some(lib)) => {
                egui::ScrollArea::vertical().show(ui, |ui| form(app, ui, &opts, &lib, level));
                app.builder = Some(opts);
                app.builder_lib = Some(lib);
            }
            (o, l) => {
                app.builder = o;
                app.builder_lib = l;
                ui.add_space(30.0);
                ui.label(theme::muted("Connect to the DM once so this device can download the species, classes and backgrounds of the campaign. After that you can build characters anywhere."));
            }
        }
    });
}

/// A row of tappable choices; returns true when the selection changed.
fn chips(ui: &mut egui::Ui, current: &mut String, options: &[(String, String)]) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(12.0, 8.0);
        for (id, name) in options {
            if ui.selectable_label(current == id, egui::RichText::new(name).size(16.0)).clicked() && current != id {
                *current = id.clone();
                changed = true;
            }
        }
    });
    changed
}

fn text_area(ui: &mut egui::Ui, label: &str, hint: &str, value: &mut String) {
    ui.label(theme::muted(label));
    ui.add(egui::TextEdit::multiline(value).hint_text(hint).desired_rows(3).desired_width(f32::INFINITY));
    ui.add_space(4.0);
}

fn same_numbers(a: &[i32], b: &[i32]) -> bool {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    a.sort();
    b.sort();
    a == b
}

fn deal(draft: &mut Character, pool: &[i32]) {
    for (k, v) in ABILITY_KEYS.iter().zip(pool.iter()) {
        draft.base_abilities.set(k, *v);
    }
}

fn form(app: &mut PlayerApp, ui: &mut egui::Ui, opts: &BuilderOptions, lib: &Library, level: u32) {
    if !app.status.is_empty() && app.status_at.elapsed().as_secs() < 12 {
        ui.colored_label(if app.status_error { theme::BAD } else { theme::GOOD }, &app.status);
    }
    if let Some(c) = app.draft.classes.first_mut() {
        c.level = level;
    }
    let sheet: Sheet = dnd_core::derive(&app.draft, lib);
    let rolled: Vec<i32> = app.seat.as_ref().map(|s| s.rolled.clone()).unwrap_or_default();
    let replacing = app.seat.as_ref().filter(|s| s.made_character).map(|s| s.character_name.clone());

    // ------------------------------------------------------------ name
    theme::section(ui, "Name");
    ui.add(egui::TextEdit::singleline(&mut app.draft.name).hint_text("Your character's name").char_limit(60).desired_width(f32::INFINITY).font(egui::TextStyle::Heading));

    // ------------------------------------------------------------ species
    theme::section(ui, "Species");
    let list: Vec<(String, String)> = opts.species.iter().map(|s| (s.id.clone(), s.name.clone())).collect();
    chips(ui, &mut app.draft.species, &list);
    if let Some(s) = opts.species.iter().find(|s| s.id == app.draft.species) {
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut facts = vec![format!("speed {} ft.", s.speed)];
            if !s.size.is_empty() {
                facts.push(s.size.clone());
            }
            if s.darkvision > 0 {
                facts.push(format!("darkvision {} ft.", s.darkvision));
            }
            ui.label(theme::accent(facts.join(" · ")).small());
            if !s.description.is_empty() {
                ui.label(&s.description);
            }
            for t in &s.traits {
                ui.label(theme::muted(format!("• {}", t.name)).small());
            }
        });
    }

    // ------------------------------------------------------------ class
    theme::section(ui, "Class");
    let list: Vec<(String, String)> = opts.classes.iter().map(|c| (c.id.clone(), c.name.clone())).collect();
    let mut class_id = app.draft.classes.first().map(|c| c.class.clone()).unwrap_or_default();
    if chips(ui, &mut class_id, &list) {
        app.draft.classes = vec![ClassLevel { class: class_id.clone(), level, ..Default::default() }];
        app.draft.advancements.clear();
        set_gear_choices(&mut app.draft, &[]);
    }
    let class = opts.classes.iter().find(|c| c.id == class_id);
    if let Some(c) = class {
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut facts = vec![format!("hit die d{}", c.hit_die)];
            if !c.primary_abilities.is_empty() {
                facts.push(format!("best with {}", c.primary_abilities.iter().map(|a| ability_name(a)).collect::<Vec<_>>().join(" / ")));
            }
            if !c.saving_throws.is_empty() {
                facts.push(format!("saves {}", c.saving_throws.iter().map(|a| a.to_uppercase()).collect::<Vec<_>>().join(", ")));
            }
            ui.label(theme::accent(facts.join(" · ")).small());
            if !c.description.is_empty() {
                ui.label(&c.description);
            }
            for f in c.features.iter().filter(|f| f.level <= level) {
                ui.label(theme::muted(format!("• {}", f.name)).small());
            }
        });
    }

    // ------------------------------------------------------------ subclass
    let subs = class.map(|c| builder::subclass_options(c, level, opts.subclasses.iter())).unwrap_or_default();
    let mut needs_subclass = false;
    if let Some(c) = class {
        let mut sub = app.draft.classes.first().and_then(|cl| cl.subclass.clone()).unwrap_or_default();
        if !subs.iter().any(|s| s.id == sub) {
            sub.clear();
        }
        if !subs.is_empty() {
            theme::section(ui, if c.subclass_label.is_empty() { "Subclass" } else { &c.subclass_label });
            let list: Vec<(String, String)> = subs.iter().map(|s| (s.id.clone(), s.name.clone())).collect();
            chips(ui, &mut sub, &list);
            if let Some(s) = subs.iter().find(|s| s.id == sub) {
                theme::card().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if !s.description.is_empty() {
                        ui.label(&s.description);
                    }
                    for f in s.features.iter().filter(|f| f.level <= level) {
                        ui.label(theme::muted(format!("• {}", f.name)).small());
                    }
                });
            }
            needs_subclass = sub.is_empty();
        }
        if let Some(cl) = app.draft.classes.first_mut() {
            cl.subclass = if sub.is_empty() { None } else { Some(sub) };
        }
    }

    // ------------------------------------------------------------ background
    theme::section(ui, "Background");
    let mut list: Vec<(String, String)> = vec![(String::new(), "None".to_string())];
    list.extend(opts.backgrounds.iter().map(|b| (b.id.clone(), b.name.clone())));
    if !app.draft.background.is_empty() && !opts.backgrounds.iter().any(|b| b.id == app.draft.background) {
        app.draft.background.clear();
    }
    if chips(ui, &mut app.draft.background, &list) {
        set_gear_choices(&mut app.draft, &[]);
    }
    let background = opts.backgrounds.iter().find(|b| b.id == app.draft.background);
    if let Some(b) = background {
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !b.skill_proficiencies.is_empty() {
                let names: Vec<String> = b.skill_proficiencies.iter().map(|k| opts.rules.skill(k).map(|s| s.name.clone()).unwrap_or_else(|| k.clone())).collect();
                ui.label(theme::accent(format!("skills: {}", names.join(", "))).small());
            }
            if !b.description.is_empty() {
                ui.label(&b.description);
            }
            if let Some(f) = &b.feature {
                ui.label(theme::muted(format!("• {}", f.name)).small());
            }
        });
    }

    // ------------------------------------------------------------ abilities
    theme::section(ui, "Ability scores");
    let methods = [("standard_array".to_string(), "Standard array".to_string()), ("point_buy".to_string(), "Point buy".to_string()), ("rolled".to_string(), "Rolled".to_string())];
    let mut method = app.draft.ability_method.clone();
    if chips(ui, &mut method, &methods) {
        match method.as_str() {
            "standard_array" => deal(&mut app.draft, &opts.rules.standard_array),
            "point_buy" => {
                for k in ABILITY_KEYS {
                    app.draft.base_abilities.set(k, opts.rules.point_buy.min);
                }
            }
            _ => {}
        }
        app.draft.ability_method = method.clone();
    }
    // the numbers to hand out, when the method is a fixed set
    let mut pool: Option<Vec<i32>> = None;
    match method.as_str() {
        "standard_array" => {
            ui.label(theme::muted("Give each number to one ability. Picking a number swaps it with wherever it was.").small());
            pool = Some(opts.rules.standard_array.clone());
        }
        "point_buy" => {
            let scores: Vec<i32> = app.draft.base_abilities.iter().map(|(_, v)| v).collect();
            let budget = opts.rules.point_buy.budget;
            match opts.rules.point_buy_cost(&scores) {
                Some(cost) => {
                    ui.colored_label(if cost > budget { theme::BAD } else { theme::GOOD }, format!("{cost} of {budget} points spent"));
                }
                None => {
                    ui.colored_label(theme::BAD, "scores are off the point-buy table");
                }
            }
        }
        _ => {
            if rolled.len() == 6 {
                ui.label(theme::muted(format!("The DM's server rolled {} for you. Give each number to one ability.", rolled.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))).small());
                let scores: Vec<i32> = app.draft.base_abilities.iter().map(|(_, v)| v).collect();
                if !same_numbers(&scores, &rolled) {
                    deal(&mut app.draft, &rolled);
                }
                pool = Some(rolled.clone());
            } else {
                ui.label(theme::muted("The DM's server rolls 4d6 and drops the lowest, six times, so everyone at the table can trust the numbers. You get one set; the DM can let you roll again.").small());
                if ui.add_enabled(app.online && !app.game_live(), theme::primary("🎲  Roll my scores")).clicked() {
                    let _ = app.client.tx.send(Cmd::RollAbilities);
                }
                if !app.online {
                    ui.label(theme::muted("Connect to the DM to roll.").small());
                }
            }
        }
    }
    let show_rows = method != "rolled" || pool.is_some();
    for k in ABILITY_KEYS.iter().filter(|_| show_rows) {
        let cur = app.draft.base_abilities.get(k).unwrap_or(10);
        theme::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(ability_name(k)).strong().size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let total = sheet.abilities.get(k).unwrap_or(cur);
                    let modifier = sheet.ability_mods.get(*k).copied().unwrap_or(0);
                    ui.label(theme::accent(format!("{total} ({})", super::bonus(modifier))).strong().size(16.0)).on_hover_text("with species bonuses and improvements");
                    ui.label(theme::muted("→"));
                    if let Some(pool) = &pool {
                        egui::ComboBox::from_id_salt(("abil", k)).selected_text(cur.to_string()).width(64.0).show_ui(ui, |ui| {
                            for (i, v) in pool.iter().enumerate() {
                                if ui.selectable_label(*v == cur, v.to_string()).clicked() && *v != cur {
                                    if let Some(other) = ABILITY_KEYS.iter().find(|o| *o != k && app.draft.base_abilities.get(o) == Some(*v)) {
                                        app.draft.base_abilities.set(other, cur);
                                    }
                                    app.draft.base_abilities.set(k, *v);
                                }
                                let _ = i;
                            }
                        });
                    } else {
                        let (lo, hi) = (opts.rules.point_buy.min, opts.rules.point_buy.max);
                        if ui.add_enabled(cur < hi, theme::big_button("+")).clicked() {
                            app.draft.base_abilities.set(k, cur + 1);
                        }
                        ui.label(egui::RichText::new(cur.to_string()).strong().size(18.0));
                        if ui.add_enabled(cur > lo, theme::big_button("−")).clicked() {
                            app.draft.base_abilities.set(k, cur - 1);
                        }
                    }
                });
            });
        });
    }

    // ------------------------------------------------------------ improvements
    let levels = class.map(|c| builder::improvement_levels(c, level)).unwrap_or_default();
    app.draft.advancements.retain(|a| levels.contains(&a.level));
    let mut improvements_open = 0usize;
    if !levels.is_empty() {
        theme::section(ui, "Improvements");
        ui.label(theme::muted("At each of these levels raise one ability by 2, two abilities by 1, or take a feat.").small());
        for l in &levels {
            let mut adv = app.draft.advancements.iter().find(|a| a.level == *l).cloned().unwrap_or(Advancement { level: *l, ..Default::default() });
            let before = adv.clone();
            theme::card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let spent: i32 = adv.ability_increases.values().sum();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Level {l}")).strong().size(16.0));
                    let mut feat_mode = adv.feat.is_some();
                    if ui.selectable_label(!feat_mode, "Abilities").clicked() {
                        feat_mode = false;
                        adv.feat = None;
                    }
                    if !opts.feats.is_empty() && ui.selectable_label(feat_mode, "Feat").clicked() && adv.feat.is_none() {
                        adv.feat = Some(opts.feats[0].id.clone());
                        adv.ability_increases.clear();
                    }
                });
                if adv.feat.is_some() {
                    let mut id = adv.feat.clone().unwrap_or_default();
                    let list: Vec<(String, String)> = opts.feats.iter().map(|f| (f.id.clone(), f.name.clone())).collect();
                    chips(ui, &mut id, &list);
                    if let Some(f) = opts.feats.iter().find(|f| f.id == id) {
                        if !f.prerequisite.is_empty() {
                            ui.label(theme::accent(format!("needs: {}", f.prerequisite)).small());
                        }
                        ui.label(theme::muted(&f.description).small());
                    }
                    adv.feat = Some(id);
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for k in ABILITY_KEYS {
                            let n = adv.ability_increases.get(k).copied().unwrap_or(0);
                            let text = if n > 0 { format!("{} +{n}", k.to_uppercase()) } else { k.to_uppercase() };
                            if ui.add_enabled(spent < 2 || n > 0, egui::Button::new(egui::RichText::new(text).size(15.0)).selected(n > 0).min_size(egui::vec2(52.0, 36.0))).clicked() {
                                // tap to add a point; tap again when the two points are spent to take it back
                                if spent < 2 {
                                    adv.ability_increases.insert(k.to_string(), n + 1);
                                } else {
                                    adv.ability_increases.remove(k);
                                }
                            }
                        }
                    });
                    ui.label(theme::muted(format!("{} of 2 points used", spent.min(2))).small());
                }
            });
            let done = adv.feat.is_some() || adv.ability_increases.values().sum::<i32>() == 2;
            if !done {
                improvements_open += 1;
            }
            if adv != before || !app.draft.advancements.iter().any(|a| a.level == *l) {
                app.draft.advancements.retain(|a| a.level != *l);
                if adv.feat.is_some() || !adv.ability_increases.is_empty() {
                    app.draft.advancements.push(adv);
                }
            }
        }
    }

    // ------------------------------------------------------------ skills
    let mut skills_short = 0usize;
    if let Some(c) = class {
        let max = c.skill_choices.count as usize;
        let options = builder::skill_options(c, &opts.rules);
        app.draft.skill_proficiencies.retain(|s| options.contains(s));
        let from_bg: Vec<String> = background.map(|b| b.skill_proficiencies.clone()).unwrap_or_default();
        // a skill the background already gives is not worth a class pick
        app.draft.skill_proficiencies.retain(|s| !from_bg.contains(s));
        app.draft.skill_proficiencies.truncate(max);
        let picked = app.draft.skill_proficiencies.len();
        skills_short = max.saturating_sub(picked);
        if max > 0 {
            theme::section(ui, "Skills");
            ui.label(theme::muted(format!("Choose {max} · {picked} chosen")));
            for key in &options {
                let name = opts.rules.skill(key).map(|s| s.name.clone()).unwrap_or_else(|| key.clone());
                if from_bg.contains(key) {
                    let mut yes = true;
                    ui.add_enabled(false, egui::Checkbox::new(&mut yes, format!("{name}  (from your background)")));
                    continue;
                }
                let mut on = app.draft.skill_proficiencies.contains(key);
                if ui.add_enabled(on || picked < max, egui::Checkbox::new(&mut on, egui::RichText::new(name).size(16.0))).changed() {
                    if on {
                        app.draft.skill_proficiencies.push(key.clone());
                    } else {
                        app.draft.skill_proficiencies.retain(|s| s != key);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ gear
    if let Some(c) = class {
        let lines = builder::gear_lines(c, background);
        if !lines.is_empty() {
            theme::section(ui, "Starting gear");
            ui.label(theme::muted("Where there is a choice, pick one. Everything lands in your Items tab.").small());
            let mut choices = gear_choices(&app.draft);
            choices.resize(lines.len(), 0);
            let before = choices.clone();
            for (i, line) in lines.iter().enumerate() {
                let options = builder::gear_options(line, &opts.items);
                if choices[i] >= options.len() {
                    choices[i] = 0;
                }
                let describe = |o: &builder::GearOption| -> String {
                    o.items
                        .iter()
                        .map(|g| {
                            let name = g.item.as_ref().and_then(|id| opts.items.get(id)).cloned().unwrap_or_else(|| g.text.clone());
                            if g.qty > 1 || g.coins.is_some() {
                                format!("{} {name}", g.qty)
                            } else {
                                name
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" + ")
                };
                if options.len() <= 1 {
                    if let Some(o) = options.first() {
                        ui.label(format!("• {}", describe(o)));
                    }
                } else {
                    theme::card().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for (n, o) in options.iter().enumerate() {
                            ui.radio_value(&mut choices[i], n, egui::RichText::new(describe(o)).size(15.0));
                        }
                    });
                }
            }
            if choices != before || gear_choices(&app.draft).len() != lines.len() {
                set_gear_choices(&mut app.draft, &choices);
            }
        }
    }

    // ------------------------------------------------------------ spells
    let spells_open = crate::ui::spells::builder_section(ui, lib, &mut app.draft, &sheet, &mut app.ui_state.spell_open);

    // ------------------------------------------------------------ story
    theme::section(ui, "About (optional)");
    ui.label(theme::muted("Alignment"));
    ui.add(egui::TextEdit::singleline(&mut app.draft.alignment).hint_text("e.g. chaotic good").char_limit(40).desired_width(f32::INFINITY));
    ui.add_space(4.0);
    text_area(ui, "Appearance", "What do people see?", &mut app.draft.appearance);
    text_area(ui, "Personality", "How do you act?", &mut app.draft.personality);
    text_area(ui, "Backstory", "Where do you come from?", &mut app.draft.backstory);

    // ------------------------------------------------------------ preview
    theme::section(ui, &format!("At level {level}"));
    ui.horizontal_wrapped(|ui| {
        let w = ((ui.available_width() - 16.0) / 3.0).clamp(84.0, 150.0);
        theme::tile(ui, w, "Hit points", &sheet.hp_max.to_string(), "");
        theme::tile(ui, w, "Armor class", &sheet.ac.to_string(), "before armor");
        theme::tile(ui, w, "Speed", &sheet.speed.to_string(), "ft.");
    });

    // ------------------------------------------------------------ send
    theme::section(ui, "Send to the DM");
    let mut problems: Vec<String> = Vec::new();
    if app.draft.name.trim().is_empty() {
        problems.push("Give your character a name".into());
    }
    if !opts.species.iter().any(|s| s.id == app.draft.species) {
        problems.push("Pick a species".into());
    }
    if class.is_none() {
        problems.push("Pick a class".into());
    }
    if needs_subclass {
        problems.push(format!("Pick a {}", class.map(|c| c.subclass_label.to_lowercase()).filter(|l| !l.is_empty()).unwrap_or_else(|| "subclass".into())));
    }
    if let Err(e) = builder::check_abilities(&app.draft.base_abilities, &app.draft.ability_method, &opts.rules, &rolled) {
        problems.push(format!("Ability scores: {e}"));
    }
    for a in &app.draft.advancements {
        if a.feat.is_none() && a.ability_increases.values().sum::<i32>() != 2 {
            problems.push(format!("Level {} improvement: use both points or take a feat", a.level));
        }
    }
    for p in &problems {
        ui.colored_label(theme::BAD, format!("• {p}"));
    }
    if problems.is_empty() {
        if skills_short > 0 {
            ui.label(theme::muted(format!("You can still choose {skills_short} more skill(s).")));
        }
        if !spells_open.is_empty() {
            ui.label(theme::muted(format!("Spells: you can still choose {}.", spells_open.join(", "))));
        }
        if improvements_open > 0 {
            ui.label(theme::muted(format!("{improvements_open} improvement(s) not chosen yet; the DM can add them later.")));
        }
    }
    let given_by_dm = app.seat.as_ref().map(|s| s.character.is_some() && !s.made_character).unwrap_or(false);
    if !app.online {
        ui.label(theme::muted("Not connected. The draft is saved on this device; connect to the DM before the game starts to send it."));
    } else if app.game_live() {
        ui.label(theme::muted("The game has started, so the DM hands out characters now. Your draft is kept for next time."));
    } else if given_by_dm {
        ui.label(theme::muted("The DM gave you the character you have. Ask the DM if you would rather play this one."));
    } else if let Some(old) = &replacing {
        ui.label(theme::muted(format!("This replaces {old}, the character you made before: it starts fresh with its starting gear and full health.")));
    }
    ui.add_space(4.0);
    let mut start_over = false;
    ui.horizontal(|ui| {
        let label = if replacing.is_some() { "⬆  Replace my character" } else { "⬆  Send to the DM" };
        if ui.add_enabled(problems.is_empty() && app.can_create() && !app.busy, theme::primary(label)).clicked() {
            app.upload_draft();
        }
        if app.busy {
            ui.spinner();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            start_over = ui.button("Start over").clicked();
        });
    });
    if start_over {
        app.discard_draft();
        app.draft.ability_method = "standard_array".into();
        deal(&mut app.draft, &opts.rules.standard_array);
    }
    ui.add_space(20.0);
}
