//! The map as this player's character sees it. Drag your own token to walk
//! (the server checks the way is clear), tap a door next to you to open it,
//! drag anywhere else to look around, pinch or scroll to zoom.

use crate::app::PlayerApp;
use crate::client::Cmd;
use crate::theme;
use dnd_core::view::MapView;
use dnd_mapview::{Anim, Camera, DrawOpts};

#[derive(Default)]
pub struct MapUi {
    pub cam: Camera,
    pub anim: Anim,
    /// Own token in hand: id and where it is held (top-left, in squares).
    pub held: Option<(String, (f32, f32))>,
    /// Map we last centred on our own token for.
    pub centred: String,
    pub last_walk: String,
}

fn turn_strip(app: &PlayerApp, ui: &mut egui::Ui) {
    let Some(enc) = &app.encounter else { return };
    if enc.combatants.is_empty() {
        return;
    }
    egui::ScrollArea::horizontal().id_salt("turns").show(ui, |ui| {
        ui.horizontal(|ui| {
            if enc.active {
                ui.label(theme::accent(format!("Round {}", enc.round)));
            }
            for c in &enc.combatants {
                let mut text = egui::RichText::new(if enc.active { format!("{} {}", c.initiative, c.name) } else { c.name.clone() }).small();
                text = if c.is_current { text.color(egui::Color32::BLACK).strong() } else { text.color(if c.is_pc { theme::PC_BLUE } else { theme::TEXT }) };
                let frame = egui::Frame::new().fill(if c.is_current { theme::ORANGE } else { theme::CARD }).corner_radius(6.0).inner_margin(egui::Margin::symmetric(8, 4));
                frame.show(ui, |ui| {
                    ui.label(text);
                    if let (Some(hp), Some(max)) = (c.hp, c.hp_max) {
                        ui.label(egui::RichText::new(format!("{hp}/{max}")).small().color(if c.is_current { egui::Color32::BLACK } else { theme::MUTED }));
                    }
                });
            }
        });
    });
}

pub fn show(app: &mut PlayerApp, ui: &mut egui::Ui) {
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    turn_strip(app, ui);
    let Some(view) = app.map.clone() else {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| ui.label(theme::muted("No map yet. It appears here when the DM puts one on the table.")));
        return;
    };
    let view: MapView = view;
    let st = &mut app.map_ui;
    let hint_h = 22.0;
    let size = egui::vec2(ui.available_width(), (ui.available_height() - hint_h).max(120.0));
    let (resp, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
    let rect = resp.rect;
    let time = ui.input(|i| i.time);
    let me = view.tokens.iter().find(|t| t.kind == "pc" && (t.mine || app.config.character_id == t.id)).or_else(|| view.tokens.iter().find(|t| t.mine));
    let me_pos = me.map(|t| (t.x as f32 + t.size as f32 / 2.0, t.y as f32 + t.size as f32 / 2.0));

    // first look at a map: stand the camera on our own token
    if st.centred != view.id && rect.width() > 10.0 {
        st.cam.fit(rect, view.width, view.height);
        if let Some((x, y)) = me_pos {
            st.cam.center = egui::vec2(x, y);
            st.cam.zoom = st.cam.zoom.max((rect.width() / 14.0).clamp(24.0, 64.0));
        }
        st.centred = view.id.clone();
    }
    st.cam.zoom_input(ui, &resp);
    let cam = st.cam;
    let pointer = resp.interact_pointer_pos();
    let at = pointer.map(|p| cam.to_map(rect, p));
    let two_fingers = ui.input(|i| i.multi_touch().is_some());

    if resp.drag_started() && !two_fingers {
        if let Some((fx, fy)) = at {
            let hit = view.tokens.iter().find(|t| t.mine && fx >= t.x as f32 && fy >= t.y as f32 && fx < (t.x + t.size as i32) as f32 && fy < (t.y + t.size as i32) as f32);
            st.held = hit.map(|t| (t.id.clone(), (t.x as f32, t.y as f32)));
        }
    }
    if resp.dragged() && !two_fingers {
        match (&mut st.held, at) {
            (Some((id, held)), Some((fx, fy))) => {
                let side = view.tokens.iter().find(|t| &t.id == id).map(|t| t.size).unwrap_or(1) as f32;
                *held = (fx - side / 2.0, fy - side / 2.0);
            }
            _ => st.cam.pan(resp.drag_delta()),
        }
    }
    if resp.drag_stopped() {
        if let Some((id, held)) = st.held.take() {
            let (x, y) = (held.0.round() as i32, held.1.round() as i32);
            if view.tokens.iter().any(|t| t.id == id && (t.x, t.y) != (x, y)) {
                let _ = app.client.tx.send(Cmd::MapMove { x, y });
            }
        }
    }
    if resp.clicked() {
        if let Some((fx, fy)) = at {
            let door = view.walls.iter().any(|w| w.is_door() && dnd_core::model::point_seg_distance((fx, fy), (w.x1, w.y1), (w.x2, w.y2)) < 0.45);
            if door {
                let _ = app.client.tx.send(Cmd::MapDoor { x: fx, y: fy });
            } else if let Some(t) = me.filter(|t| t.mine) {
                // tap a square to walk there
                let (x, y) = (fx.floor() as i32, fy.floor() as i32);
                if (t.x, t.y) != (x, y) && view.seen_at(x, y) > 0 && view.tile(x, y) != dnd_core::model::tile::ROCK {
                    let _ = app.client.tx.send(Cmd::MapMove { x, y });
                }
            }
        }
    }

    let held = st.held.as_ref().map(|(id, at)| (id.as_str(), *at));
    let opts = DrawOpts { grid: true, selected: None, held, highlight: &[], highlight_color: None, solid_fog: true };
    dnd_mapview::draw(&painter, rect, &cam, &view, &mut st.anim, time, &opts);

    // "find me" button, top right of the map
    let b = egui::Rect::from_min_size(rect.right_top() + egui::vec2(-46.0, 8.0), egui::vec2(38.0, 38.0));
    let find = ui.put(b, egui::Button::new(egui::RichText::new("◎").size(20.0)).fill(theme::CARD)).on_hover_text("Centre on my character");
    if find.clicked() {
        if let Some((x, y)) = me_pos {
            st.cam.center = egui::vec2(x, y);
        } else {
            st.cam.fit(rect, view.width, view.height);
        }
    }

    let can_walk = me.map(|t| t.mine).unwrap_or(false);
    let hint = if me.is_none() {
        "You are not on the map yet.".to_string()
    } else if !st.last_walk.is_empty() {
        st.last_walk.clone()
    } else if can_walk {
        "Drag your character or tap a square to walk. Tap a door beside you to open it.".to_string()
    } else if view.encounter_active {
        "Wait for your turn to move.".to_string()
    } else {
        "The DM moves the tokens.".to_string()
    };
    ui.label(theme::muted(format!("{} · {}", view.name, hint)).small());
}
