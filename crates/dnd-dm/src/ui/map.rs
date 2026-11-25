//! Maps: the shelf of maps for the campaign, the editor (terrain, walls,
//! doors, lights, props, notes, fog) and the table view with tokens. The
//! party is on one map ("in play"); any other can be opened for preparing
//! without moving the party.

use crate::app::DmApp;
use crate::ui::encounter::area_cells;
use crate::ui::theme;
use dnd_core::grid::{self, Pos};
use dnd_core::model::*;
use dnd_core::view::{MapView, Viewer};
use dnd_core::World;
use dnd_mapview::{Anim, Camera, DrawOpts};
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    Paint,
    Room,
    Wall,
    Door,
    Light,
    Prop,
    Note,
    Fog,
    Erase,
    Start,
}

impl Tool {
    const ALL: [(Tool, &'static str, &'static str); 11] = [
        (Tool::Select, "✋ Move", "Drag tokens. Click a door to open or close it; right-click to lock it. Right-click a token, prop or note to hide it from players."),
        (Tool::Paint, "🖌 Terrain", "Paint squares with the chosen terrain. Solid rock is a wall: it blocks sight and movement by itself."),
        (Tool::Room, "⬛ Room", "Drag a rectangle to fill it with the chosen terrain. Dig rooms and corridors out of rock."),
        (Tool::Wall, "╱ Wall", "Drag from one point to another to draw a thin wall (snaps to half squares)."),
        (Tool::Door, "🚪 Door", "Click the edge between two squares to put a door, secret door or window there. Click again to remove it."),
        (Tool::Light, "🔥 Light", "Click to place a torch or lamp. With Move, click a light to switch it on or off."),
        (Tool::Prop, "🪑 Prop", "Click to place furniture and scenery."),
        (Tool::Note, "📌 Note", "Click a square to pin a note on it."),
        (Tool::Fog, "☁ Fog", "Drag to reveal squares to the players (or hide them again)."),
        (Tool::Erase, "✖ Erase", "Click a wall, door, light, prop or note to remove it."),
        (Tool::Start, "⚑ Start", "Click where the party arrives when the game moves to this map."),
    ];
}

#[derive(Default)]
pub struct MapViewState {
    pub cam: Camera,
    pub anim: Anim,
    pub tool: Tool,
    pub tile: char,
    pub brush: i32,
    pub door_kind: usize,
    pub prop: String,
    pub prop_rot: u8,
    pub prop_hidden: bool,
    pub light_bright: u32,
    pub note: String,
    pub note_public: bool,
    pub fog_hide: bool,
    pub no_grid: bool,
    pub player_view: bool,
    /// Shelf map being prepared; `None` = the map in play.
    pub editing: Option<String>,
    pub undo: Vec<MapState>,
    pub redo: Vec<MapState>,
    /// Token in hand: id, where it is held (top-left, squares), where it came from.
    pub held: Option<(String, (f32, f32), Pos)>,
    pub drag_from: Option<(f32, f32)>,
    pub cache: Option<(u64, bool, Option<String>, MapView)>,
    pub new_name: String,
    pub new_w: u32,
    pub new_h: u32,
    pub confirm_delete: bool,
    pub creature_pick: String,
    pub hover: Option<(i32, i32)>,
}

const DOOR_KINDS: [(WallKind, &str); 3] = [(WallKind::Door, "Door"), (WallKind::SecretDoor, "Secret door"), (WallKind::Window, "Window")];

/// Bring a shelf map in as if it were in play, so every tool works on it.
/// Call again with the same index to put things back.
fn swap_shelf(w: &mut World, index: usize) {
    let c = &mut w.campaign;
    if c.map.id == c.maps[index].id {
        return;
    }
    std::mem::swap(&mut c.map, &mut c.maps[index]);
    std::mem::swap(&mut c.encounter.combatants, &mut c.map.parked);
}

fn swap_back(w: &mut World, index: usize) {
    let c = &mut w.campaign;
    std::mem::swap(&mut c.encounter.combatants, &mut c.map.parked);
    std::mem::swap(&mut c.map, &mut c.maps[index]);
}

fn remember(st: &mut MapViewState, map: &MapState) {
    st.undo.push(map.clone());
    if st.undo.len() > 40 {
        st.undo.remove(0);
    }
    st.redo.clear();
}

pub fn show(app: &mut DmApp, ui: &mut egui::Ui) {
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    let world = app.world.clone();
    let mut guard = world.write().unwrap();
    let w = &mut *guard;
    {
        let st = &mut app.map;
        if st.tile == '\0' {
            st.tile = tile::FLOOR;
            st.brush = 1;
            st.prop = "chest".into();
            st.light_bright = 20;
            st.new_w = 40;
            st.new_h = 28;
        }
        if w.campaign.map.id.is_empty() {
            w.campaign.map.id = w.campaign.unique_map_id(&w.campaign.map.name.clone());
        }
        if st.editing.as_ref().map(|id| !w.campaign.maps.iter().any(|m| &m.id == id)).unwrap_or(false) {
            st.editing = None;
        }
    }
    let mut status: Option<(String, bool)> = None;

    shelf_panel(app, w, ui, &mut status);

    let before_rev = w.revision;
    let shelf_index = app.map.editing.as_ref().and_then(|id| w.campaign.maps.iter().position(|m| &m.id == id));
    if let Some(i) = shelf_index {
        swap_shelf(w, i);
    }
    let before = (w.campaign.map.clone(), w.campaign.encounter.combatants.clone());
    token_panel(app, w, ui, shelf_index.is_some());
    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL)).show_inside(ui, |ui| {
        toolbar(app, w, ui);
        canvas(app, w, ui, &mut status);
    });
    if (&w.campaign.map, &w.campaign.encounter.combatants) != (&before.0, &before.1) {
        w.update_explored();
        w.touch();
    }
    if let Some(i) = shelf_index {
        swap_back(w, i);
    }
    if w.revision != before_rev {
        app.map.cache = None;
    }
    drop(guard);
    if let Some((text, error)) = status {
        app.set_status(text, error);
    }
}

/// Left: every map of the campaign, grouped by chapter.
fn shelf_panel(app: &mut DmApp, w: &mut World, ui: &mut egui::Ui, status: &mut Option<(String, bool)>) {
    let st = &mut app.map;
    egui::SidePanel::left("map_shelf").resizable(true).default_width(210.0).show_inside(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            theme::section(ui, "Maps");
            let mut rows: Vec<(String, String, String, bool)> = vec![(w.campaign.map.chapter.clone(), w.campaign.map.id.clone(), w.campaign.map.name.clone(), true)];
            rows.extend(w.campaign.maps.iter().map(|m| (m.chapter.clone(), m.id.clone(), m.name.clone(), false)));
            rows.sort_by(|a, b| (a.0.to_lowercase(), a.2.to_lowercase()).cmp(&(b.0.to_lowercase(), b.2.to_lowercase())));
            let mut chapter: Option<String> = None;
            let mut go: Option<String> = None;
            for (ch, id, name, live) in &rows {
                if chapter.as_ref() != Some(ch) {
                    chapter = Some(ch.clone());
                    ui.add_space(4.0);
                    ui.label(theme::muted(if ch.is_empty() { "No chapter" } else { ch.as_str() }).small());
                }
                let open = if *live { st.editing.is_none() } else { st.editing.as_deref() == Some(id.as_str()) };
                ui.horizontal(|ui| {
                    let label = if *live { format!("▶ {name}") } else { name.clone() };
                    let text = egui::RichText::new(label).color(if open { egui::Color32::BLACK } else if *live { theme::ORANGE } else { theme::TEXT });
                    let b = egui::Button::new(text).fill(if open { theme::ORANGE } else { egui::Color32::TRANSPARENT }).min_size(egui::vec2(ui.available_width() - 34.0, 24.0));
                    if ui.add(b).on_hover_text(if *live { "The party is here. Players see this map." } else { "Open to prepare it. The party stays where it is." }).clicked() {
                        st.editing = if *live { None } else { Some(id.clone()) };
                        st.undo.clear();
                        st.redo.clear();
                        st.cache = None;
                        st.confirm_delete = false;
                    }
                    if !*live && ui.small_button("▶").on_hover_text("Move the party here. Creatures on the current map stay there; the ones prepared here appear.").clicked() {
                        go = Some(id.clone());
                    }
                });
            }
            if let Some(id) = go {
                let rules = w.library.rules.clone();
                if w.campaign.switch_map(&id, |s| rules.size_squares(s)) {
                    w.update_explored();
                    w.touch();
                    st.editing = None;
                    st.undo.clear();
                    st.cache = None;
                    *status = Some((format!("The party is now in '{}'", w.campaign.map.name), false));
                }
            }

            theme::section(ui, "New map");
            ui.add(egui::TextEdit::singleline(&mut st.new_name).hint_text("name").desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut st.new_w).range(12..=120).prefix("W "));
                ui.add(egui::DragValue::new(&mut st.new_h).range(10..=120).prefix("H "));
            });
            let mut made: Option<MapState> = None;
            let name = |st: &MapViewState, fallback: &str| if st.new_name.trim().is_empty() { fallback.to_string() } else { st.new_name.trim().to_string() };
            ui.horizontal_wrapped(|ui| {
                if ui.button("Dungeon").on_hover_text("Rooms, corridors and doors, in the dark").clicked() {
                    made = Some(dnd_core::mapgen::dungeon(&name(st, "Dungeon"), st.new_w, st.new_h, &mut w.rng));
                }
                if ui.button("Cave").clicked() {
                    made = Some(dnd_core::mapgen::cave(&name(st, "Cave"), st.new_w, st.new_h, &mut w.rng));
                }
                if ui.button("Outdoors").clicked() {
                    made = Some(dnd_core::mapgen::wilderness(&name(st, "Wilderness"), st.new_w, st.new_h, &mut w.rng));
                }
                if ui.button("Blank").on_hover_text("An empty stone floor").clicked() {
                    made = Some(MapState::new(name(st, "New map"), st.new_w, st.new_h));
                }
                if ui.button("Solid rock").on_hover_text("All rock: dig rooms out with the Room tool").clicked() {
                    let mut m = MapState::new(name(st, "New map"), st.new_w, st.new_h);
                    m.fill(tile::ROCK);
                    made = Some(m);
                }
                if ui.button("Copy this one").clicked() {
                    let src = st.editing.as_ref().and_then(|id| w.campaign.maps.iter().find(|m| &m.id == id)).unwrap_or(&w.campaign.map);
                    let mut m = src.clone();
                    m.name = name(st, &format!("{} (copy)", src.name));
                    m.parked.clear();
                    made = Some(m);
                }
            });
            if let Some(mut m) = made {
                m.chapter = st.editing.as_ref().and_then(|id| w.campaign.maps.iter().find(|m| &m.id == id)).unwrap_or(&w.campaign.map).chapter.clone();
                let id = w.campaign.add_map(m);
                w.touch();
                st.editing = Some(id);
                st.undo.clear();
                st.cache = None;
                st.new_name.clear();
                *status = Some(("New map added. The party has not moved; press ▶ next to it when they go there.".into(), false));
            }

            if let Some(id) = st.editing.clone() {
                theme::section(ui, "This map");
                if !st.confirm_delete {
                    if ui.button("🗑 Delete map…").clicked() {
                        st.confirm_delete = true;
                    }
                } else {
                    ui.colored_label(theme::BAD, "Delete this map and everything on it?");
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            w.campaign.maps.retain(|m| m.id != id);
                            w.touch();
                            st.editing = None;
                            st.confirm_delete = false;
                            st.cache = None;
                        }
                        if ui.button("Keep").clicked() {
                            st.confirm_delete = false;
                        }
                    });
                }
            }
            ui.add_space(6.0);
            ui.label(theme::muted("Each map is a text file in the campaign's maps folder; they save with the campaign.").small());
        });
    });
}

/// Right: who is on this map, and adding creatures to it.
fn token_panel(app: &mut DmApp, w: &mut World, ui: &mut egui::Ui, preparing: bool) {
    let st = &mut app.map;
    let selected = &mut app.encounter.selected;
    egui::SidePanel::right("map_tokens").resizable(true).default_width(220.0).show_inside(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            theme::section(ui, if preparing { "Waiting on this map" } else { "On the table" });
            if preparing {
                ui.label(theme::muted("Creatures you put here wait until the party arrives.").small());
            }
            let mut remove: Option<String> = None;
            for c in &w.campaign.encounter.combatants {
                let sel = selected.as_deref() == Some(c.id.as_str());
                ui.horizontal(|ui| {
                    let col = match c.kind {
                        CombatantKind::Pc { .. } => theme::PC_BLUE,
                        CombatantKind::Creature { .. } => theme::BAD,
                        CombatantKind::Custom => theme::MUTED,
                    };
                    ui.colored_label(col, "●");
                    let label = if c.pos.is_some() { c.name.clone() } else { format!("{} (not placed)", c.name) };
                    if ui.selectable_label(sel, label).on_hover_text("Select, then click a square to put it there").clicked() {
                        *selected = if sel { None } else { Some(c.id.clone()) };
                        st.tool = Tool::Select;
                    }
                    if !c.visible {
                        ui.label(theme::muted("hidden").small());
                    }
                });
            }
            if w.campaign.encounter.combatants.is_empty() {
                ui.label(theme::muted("Nobody yet. Add player characters on the Encounter tab; add creatures below."));
            }
            ui.add_space(4.0);
            let creatures: Vec<(String, String)> = w.library.creatures.iter().map(|(id, c)| (id.clone(), c.name.clone())).collect();
            ui.horizontal(|ui| {
                crate::ui::id_combo(ui, "map_creature", &mut st.creature_pick, &creatures, "— creature —");
            });
            if ui.button("➕ Add creature").clicked() && !st.creature_pick.is_empty() {
                if let Some(c) = w.combatant_from_creature(&st.creature_pick, None) {
                    *selected = Some(c.id.clone());
                    w.campaign.encounter.combatants.push(c);
                    st.tool = Tool::Select;
                }
            }
            if !preparing && ui.button("Place the party at ⚑").on_hover_text("Stand every player character that is not on the map at the start point").clicked() {
                let rules = w.library.rules.clone();
                w.campaign.place_party(|s| rules.size_squares(s));
            }

            if let Some(id) = selected.clone() {
                if let Some(c) = w.campaign.encounter.find_mut(&id) {
                    theme::section(ui, &c.name.clone());
                    ui.checkbox(&mut c.visible, "Players can see it");
                    ui.horizontal(|ui| {
                        ui.label("Light");
                        ui.add(egui::DragValue::new(&mut c.light).range(0..=120).suffix(" ft"));
                        if ui.small_button("torch").clicked() {
                            c.light = 20;
                        }
                        if ui.small_button("none").clicked() {
                            c.light = 0;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Darkvision");
                        ui.add(egui::DragValue::new(&mut c.darkvision).range(0..=300).suffix(" ft"));
                    });
                    ui.horizontal(|ui| {
                        if c.pos.is_some() && ui.button("Take off the map").clicked() {
                            c.pos = None;
                        }
                        if !matches!(c.kind, CombatantKind::Pc { .. }) && ui.button("Remove").clicked() {
                            remove = Some(id.clone());
                        }
                    });
                }
            }
            if let Some(id) = remove {
                w.campaign.encounter.remove(&id);
                *selected = None;
            }
        });
    });
}

fn toolbar(app: &mut DmApp, w: &mut World, ui: &mut egui::Ui) {
    let st = &mut app.map;
    let map = &mut w.campaign.map;
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(&mut map.name).desired_width(150.0).hint_text("map name"));
        ui.add(egui::TextEdit::singleline(&mut map.chapter).desired_width(110.0).hint_text("chapter / place")).on_hover_text("Which part of the campaign this map belongs to; maps are grouped by it");
        let (mut mw, mut mh) = (map.width, map.height);
        ui.add(egui::DragValue::new(&mut mw).range(4..=200).prefix("W "));
        ui.add(egui::DragValue::new(&mut mh).range(4..=200).prefix("H "));
        if (mw, mh) != (map.width, map.height) {
            map.resize(mw, mh);
        }
        ui.add(egui::DragValue::new(&mut map.feet_per_square).range(1..=100).suffix(" ft/sq"));
        ui.separator();
        ui.label("Fog");
        egui::ComboBox::from_id_salt("fogmode")
            .selected_text(match map.fog {
                FogMode::Off => "Off",
                FogMode::Manual => "Painted by me",
                FogMode::Vision => "What characters see",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut map.fog, FogMode::Off, "Off").on_hover_text("Players see the whole map");
                ui.selectable_value(&mut map.fog, FogMode::Manual, "Painted by me").on_hover_text("Players see only what you reveal with the Fog tool");
                ui.selectable_value(&mut map.fog, FogMode::Vision, "What characters see").on_hover_text("Walls, doors and darkness decide; each player sees through their own character");
            });
        if map.fog != FogMode::Off {
            if ui.small_button("Reveal all").clicked() {
                remember(st, map);
                map.reveal_all();
            }
            if ui.small_button("Forget all").on_hover_text("Players forget everything they have seen here").clicked() {
                remember(st, map);
                map.reset_fog();
            }
        }
        ui.label("Darkness");
        ui.add(egui::Slider::new(&mut map.darkness, 0.0..=1.0).show_value(false)).on_hover_text("Left: daylight. Middle: dim. Right: pitch black, only lights and darkvision help.");
        ui.separator();
        ui.toggle_value(&mut st.player_view, "👁 Players' view").on_hover_text("Show exactly what the players can see right now");
        let mut grid = !st.no_grid;
        ui.toggle_value(&mut grid, "# Grid");
        st.no_grid = !grid;
        if ui.button("Fit").clicked() {
            st.cam = Camera::default();
        }
    });
    ui.horizontal_wrapped(|ui| {
        for (t, label, tip) in Tool::ALL {
            let sel = st.tool == t;
            let text = egui::RichText::new(label).color(if sel { egui::Color32::BLACK } else { theme::TEXT });
            if ui.add(egui::Button::new(text).fill(if sel { theme::ORANGE } else { theme::CARD })).on_hover_text(tip).clicked() {
                st.tool = t;
            }
        }
        ui.separator();
        if ui.add_enabled(!st.undo.is_empty(), egui::Button::new("↶ Undo")).on_hover_text("Ctrl+Z").clicked() {
            undo(st, map);
        }
        if ui.add_enabled(!st.redo.is_empty(), egui::Button::new("↷ Redo")).clicked() {
            if let Some(m) = st.redo.pop() {
                st.undo.push(std::mem::replace(map, m));
            }
        }
    });
    // options for the tool in hand
    ui.horizontal_wrapped(|ui| match st.tool {
        Tool::Paint | Tool::Room => {
            for (c, name) in tile::ALL {
                let sel = st.tile == c;
                let text = egui::RichText::new(name).color(if sel { egui::Color32::BLACK } else { theme::TEXT }).small();
                if ui.add(egui::Button::new(text).fill(if sel { theme::ORANGE } else { theme::CARD })).clicked() {
                    st.tile = c;
                }
            }
            if st.tool == Tool::Paint {
                ui.add(egui::Slider::new(&mut st.brush, 1..=6).text("brush"));
            }
        }
        Tool::Door => {
            for (i, (_, name)) in DOOR_KINDS.iter().enumerate() {
                ui.selectable_value(&mut st.door_kind, i, *name);
            }
        }
        Tool::Light => {
            ui.label("Bright light");
            ui.add(egui::DragValue::new(&mut st.light_bright).range(5..=120).suffix(" ft"));
            ui.label(theme::muted("(dim light reaches twice as far)"));
        }
        Tool::Prop => {
            egui::ComboBox::from_id_salt("propkind").selected_text(st.prop.clone()).show_ui(ui, |ui| {
                for k in PROP_KINDS {
                    ui.selectable_value(&mut st.prop, k.to_string(), k);
                }
            });
            if ui.button("⟳ Turn").clicked() {
                st.prop_rot = (st.prop_rot + 1) % 4;
            }
            ui.checkbox(&mut st.prop_hidden, "Hidden from players").on_hover_text("A trap or hidden loot: only you see it until you right-click it");
        }
        Tool::Note => {
            ui.add(egui::TextEdit::singleline(&mut st.note).desired_width(260.0).hint_text("what to remember here"));
            ui.checkbox(&mut st.note_public, "Players can read it");
        }
        Tool::Fog => {
            ui.selectable_value(&mut st.fog_hide, false, "Reveal");
            ui.selectable_value(&mut st.fog_hide, true, "Hide");
            ui.add(egui::Slider::new(&mut st.brush, 1..=6).text("brush"));
            if map.fog == FogMode::Off {
                ui.colored_label(theme::ORANGE, "Fog is off: choose a fog mode above first.");
            }
        }
        _ => {
            let tip = Tool::ALL.iter().find(|(t, _, _)| *t == st.tool).map(|t| t.2).unwrap_or("");
            ui.label(theme::muted(tip).small());
        }
    });
}

fn undo(st: &mut MapViewState, map: &mut MapState) {
    if let Some(m) = st.undo.pop() {
        st.redo.push(std::mem::replace(map, m));
    }
}

fn brush_cells(cx: i32, cy: i32, size: i32) -> Vec<(i32, i32)> {
    let lo = -(size - 1) / 2;
    let hi = size / 2;
    (lo..=hi).flat_map(|dy| (lo..=hi).map(move |dx| (cx + dx, cy + dy))).collect()
}

fn canvas(app: &mut DmApp, w: &mut World, ui: &mut egui::Ui, status: &mut Option<(String, bool)>) {
    let highlight_area: Vec<(i32, i32)> = if app.map.editing.is_none() { area_cells(&app.encounter, w).into_iter().map(|p| (p.x, p.y)).collect() } else { Vec::new() };
    let st = &mut app.map;
    let selected = &mut app.encounter.selected;
    let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
    let rect = resp.rect;
    let time = ui.input(|i| i.time);

    // the picture, rebuilt only when something changed
    let key = w.revision;
    let fresh = st.cache.as_ref().map(|(k, pv, ed, _)| *k == key && *pv == st.player_view && ed == &st.editing).unwrap_or(false);
    if !fresh {
        let view = w.map_view(if st.player_view { &Viewer::Party } else { &Viewer::Dm });
        st.cache = Some((key, st.player_view, st.editing.clone(), view));
    }
    let view = st.cache.as_ref().map(|c| c.3.clone()).unwrap_or_default();
    st.cam.fit_once(rect, view.width, view.height, dnd_mapview::key_of(&view.id) ^ ((view.width as u64) << 32 | view.height as u64));
    st.cam.zoom_input(ui, &resp);
    if resp.dragged_by(egui::PointerButton::Secondary) || resp.dragged_by(egui::PointerButton::Middle) {
        st.cam.pan(resp.drag_delta());
    }
    let cam = st.cam;
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    let at = pointer.map(|p| cam.to_map(rect, p));
    let cell_at = at.map(|(x, y)| (x.floor() as i32, y.floor() as i32));
    st.hover = cell_at.filter(|_| resp.hovered());
    let rules = w.library.rules.clone();
    let token_at = |w: &World, c: (i32, i32)| {
        w.campaign.encounter.combatants.iter().rev().find(|t| t.pos.map(|p| grid::footprint(p, rules.size_squares(&t.size)).contains(&Pos::new(c.0, c.1))).unwrap_or(false)).map(|t| t.id.clone())
    };
    let primary_down = resp.drag_started_by(egui::PointerButton::Primary);
    let primary_drag = resp.dragged_by(egui::PointerButton::Primary);
    let primary_up = resp.drag_stopped_by(egui::PointerButton::Primary);
    let mut preview: Vec<(i32, i32)> = Vec::new();
    let mut preview_line: Option<[(f32, f32); 2]> = None;

    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
        undo(st, &mut w.campaign.map);
    }

    if let (Some((fx, fy)), Some((cx, cy))) = (at, cell_at) {
        match st.tool {
            Tool::Select => {
                if primary_down {
                    if let Some(id) = token_at(w, (cx, cy)) {
                        let from = w.campaign.encounter.find(&id).and_then(|c| c.pos).unwrap_or_default();
                        *selected = Some(id.clone());
                        st.held = Some((id, (from.x as f32, from.y as f32), from));
                    }
                }
                if primary_drag {
                    if let Some((id, held, _)) = &mut st.held {
                        let side = w.campaign.encounter.find(id).map(|c| rules.size_squares(&c.size)).unwrap_or(1) as f32;
                        *held = (fx - side / 2.0, fy - side / 2.0);
                    }
                }
                if primary_up {
                    if let Some((id, held, _)) = st.held.take() {
                        let to = Pos::new(held.0.round() as i32, held.1.round() as i32);
                        if let Err(e) = w.move_token(&id, to, false, 0) {
                            *status = Some((e, true));
                        }
                    }
                }
                if resp.clicked() {
                    if let Some(i) = w.campaign.map.door_near(fx, fy, 0.3) {
                        let _ = w.toggle_door(i, true);
                    } else if let Some(i) = w.campaign.map.lights.iter().position(|l| (l.x - fx).abs() < 0.4 && (l.y - fy).abs() < 0.4) {
                        w.campaign.map.lights[i].on = !w.campaign.map.lights[i].on;
                    } else if let Some(id) = token_at(w, (cx, cy)) {
                        *selected = Some(id);
                    } else if let Some(id) = selected.clone() {
                        // put the selected token here
                        if w.campaign.map.in_bounds(cx, cy) {
                            let _ = w.move_token(&id, Pos::new(cx, cy), false, 0);
                        }
                    }
                }
                if resp.secondary_clicked() {
                    if let Some(i) = w.campaign.map.door_near(fx, fy, 0.3) {
                        let d = &mut w.campaign.map.walls[i];
                        d.locked = !d.locked;
                    } else if let Some(id) = token_at(w, (cx, cy)) {
                        if let Some(c) = w.campaign.encounter.find_mut(&id) {
                            c.visible = !c.visible;
                        }
                    } else if let Some(p) = w.campaign.map.props.iter_mut().find(|p| (p.x - fx).abs() < 0.5 && (p.y - fy).abs() < 0.5) {
                        p.hidden = !p.hidden;
                    } else if let Some(m) = w.campaign.map.markers.iter_mut().find(|m| m.x == cx && m.y == cy) {
                        m.hidden = !m.hidden;
                    }
                }
            }
            Tool::Paint | Tool::Fog => {
                let cells = brush_cells(cx, cy, st.brush.max(1));
                if resp.hovered() {
                    preview = cells.clone();
                }
                if primary_down || resp.clicked() {
                    remember(st, &w.campaign.map);
                }
                if primary_down || primary_drag || resp.clicked() {
                    for (x, y) in cells {
                        if st.tool == Tool::Paint {
                            if w.campaign.map.in_bounds(x, y) && w.campaign.map.tile(x, y) != st.tile {
                                w.campaign.map.set_tile(x, y, st.tile);
                            }
                        } else if w.campaign.map.is_explored(x, y) == st.fog_hide {
                            w.campaign.map.set_explored(x, y, !st.fog_hide);
                        }
                    }
                }
            }
            Tool::Room => {
                if primary_down {
                    st.drag_from = Some((fx, fy));
                }
                if let Some((sx, sy)) = st.drag_from {
                    let (ax, ay) = (sx.floor() as i32, sy.floor() as i32);
                    for y in ay.min(cy)..=ay.max(cy) {
                        for x in ax.min(cx)..=ax.max(cx) {
                            preview.push((x, y));
                        }
                    }
                    if primary_up {
                        remember(st, &w.campaign.map);
                        for (x, y) in preview.drain(..) {
                            w.campaign.map.set_tile(x, y, st.tile);
                        }
                        st.drag_from = None;
                    }
                }
            }
            Tool::Wall => {
                let snap = |v: f32| (v * 2.0).round() / 2.0;
                if primary_down {
                    st.drag_from = Some((snap(fx), snap(fy)));
                }
                if let Some(from) = st.drag_from {
                    let to = (snap(fx), snap(fy));
                    preview_line = Some([from, to]);
                    if primary_up {
                        if from != to {
                            remember(st, &w.campaign.map);
                            w.campaign.map.walls.push(Wall { x1: from.0, y1: from.1, x2: to.0, y2: to.1, ..Default::default() });
                        }
                        st.drag_from = None;
                    }
                }
            }
            Tool::Door => {
                // the nearest edge between two squares
                let (rx, ry) = (fx.round(), fy.round());
                let seg = if (fx - rx).abs() < (fy - ry).abs() { [(rx, fy.floor()), (rx, fy.floor() + 1.0)] } else { [(fx.floor(), ry), (fx.floor() + 1.0, ry)] };
                preview_line = Some(seg);
                if resp.clicked() {
                    remember(st, &w.campaign.map);
                    let same = |d: &Wall| ((d.x1, d.y1), (d.x2, d.y2)) == (seg[0], seg[1]);
                    if w.campaign.map.walls.iter().any(same) {
                        w.campaign.map.walls.retain(|d| !same(d));
                    } else {
                        w.campaign.map.walls.push(Wall { x1: seg[0].0, y1: seg[0].1, x2: seg[1].0, y2: seg[1].1, kind: DOOR_KINDS[st.door_kind.min(2)].0, ..Default::default() });
                    }
                }
            }
            Tool::Light | Tool::Prop | Tool::Note | Tool::Start => {
                preview.push((cx, cy));
                if resp.clicked() && w.campaign.map.in_bounds(cx, cy) {
                    remember(st, &w.campaign.map);
                    let (mx, my) = (cx as f32 + 0.5, cy as f32 + 0.5);
                    let map = &mut w.campaign.map;
                    match st.tool {
                        Tool::Light => map.lights.push(Light { x: mx, y: my, bright: st.light_bright, dim: st.light_bright * 2, ..Default::default() }),
                        Tool::Prop => map.props.push(Prop { kind: st.prop.clone(), x: mx, y: my, rot: st.prop_rot, hidden: st.prop_hidden, note: String::new() }),
                        Tool::Note => {
                            map.markers.retain(|m| !(m.x == cx && m.y == cy));
                            if !st.note.trim().is_empty() {
                                map.markers.push(Marker { x: cx, y: cy, text: st.note.trim().to_string(), hidden: !st.note_public });
                            }
                        }
                        _ => map.start = Some(Pos::new(cx, cy)),
                    }
                }
            }
            Tool::Erase => {
                preview.push((cx, cy));
                if resp.clicked() {
                    remember(st, &w.campaign.map);
                    let map = &mut w.campaign.map;
                    let near = |x: f32, y: f32| (x - fx).abs() < 0.5 && (y - fy).abs() < 0.5;
                    if let Some(i) = map.props.iter().rposition(|p| near(p.x, p.y)) {
                        map.props.remove(i);
                    } else if let Some(i) = map.lights.iter().position(|l| near(l.x, l.y)) {
                        map.lights.remove(i);
                    } else if let Some(i) = map.markers.iter().position(|m| m.x == cx && m.y == cy) {
                        map.markers.remove(i);
                    } else if let Some(i) = map.wall_near(fx, fy, 0.3) {
                        map.walls.remove(i);
                    } else {
                        st.undo.pop();
                    }
                }
            }
        }
    }
    if !primary_drag && !primary_up && st.tool != Tool::Room && st.tool != Tool::Wall {
        st.drag_from = None;
    }

    // ---- draw
    let held = st.held.as_ref().map(|(id, at, _)| (id.as_str(), *at));
    let highlight: Vec<(i32, i32)> = if preview.is_empty() { highlight_area } else { preview };
    let opts = DrawOpts {
        grid: !st.no_grid,
        selected: selected.as_deref(),
        held,
        highlight: &highlight,
        highlight_color: None,
        solid_fog: st.player_view,
    };
    dnd_mapview::draw(&painter, rect, &cam, &view, &mut st.anim, time, &opts);
    if let Some([a, b]) = preview_line {
        painter.line_segment([cam.to_screen(rect, a.0, a.1), cam.to_screen(rect, b.0, b.1)], egui::Stroke::new(3.0, theme::ORANGE));
    }
    // how far the token in hand has come
    if let (Some((_, at, from)), Some(p)) = (&st.held, pointer) {
        let to = Pos::new(at.0.round() as i32, at.1.round() as i32);
        let ft = grid::distance_ft(*from, to, w.campaign.settings.distance_rule, w.campaign.map.feet_per_square);
        let text = format!("{ft} ft");
        painter.text(p + egui::vec2(15.0, -17.0), egui::Align2::LEFT_BOTTOM, &text, egui::FontId::proportional(16.0), egui::Color32::BLACK);
        painter.text(p + egui::vec2(14.0, -18.0), egui::Align2::LEFT_BOTTOM, &text, egui::FontId::proportional(16.0), egui::Color32::WHITE);
    }
    // corner read-out
    if let Some((x, y)) = st.hover {
        let name = tile::ALL.iter().find(|(c, _)| *c == w.campaign.map.tile(x, y)).map(|t| t.1).unwrap_or("");
        let note = w.campaign.map.markers.iter().find(|m| m.x == x && m.y == y).map(|m| format!("  📌 {}", m.text)).unwrap_or_default();
        let text = format!("({x}, {y})  {name}{note}");
        let pos = rect.left_bottom() + egui::vec2(8.0, -6.0);
        painter.text(pos + egui::vec2(1.0, 1.0), egui::Align2::LEFT_BOTTOM, &text, egui::FontId::monospace(12.0), egui::Color32::BLACK);
        painter.text(pos, egui::Align2::LEFT_BOTTOM, &text, egui::FontId::monospace(12.0), egui::Color32::from_gray(220));
    }
    if st.player_view {
        painter.text(rect.center_top() + egui::vec2(0.0, 8.0), egui::Align2::CENTER_TOP, "PLAYERS' VIEW", egui::FontId::proportional(14.0), theme::ORANGE);
    }
}
