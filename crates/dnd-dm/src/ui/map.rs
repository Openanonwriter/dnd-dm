//! Battle map: a grid with draggable tokens. Positions live on the
//! combatants, so the encounter tools and the player apps see the same thing.

use crate::app::DmApp;
use crate::ui::encounter::area_cells;
use dnd_core::grid::Pos;
use dnd_core::model::CombatantKind;
use eframe::egui;

#[derive(Default)]
pub struct MapViewState {
    pub cell_px: f32,
    pub drag: Option<String>,
    pub mark_text: String,
    pub hover: Option<Pos>,
}

pub fn show(app: &mut DmApp, ui: &mut egui::Ui) {
    let world = app.world.clone();
    let mut w = world.write().unwrap();
    let w = &mut *w;
    let st = &mut app.map;
    if st.cell_px == 0.0 {
        st.cell_px = 28.0;
    }
    let before = (w.campaign.map.clone(), w.campaign.encounter.clone());

    ui.horizontal(|ui| {
        ui.label("Map");
        ui.add(egui::TextEdit::singleline(&mut w.campaign.map.name).desired_width(140.0));
        ui.label("W");
        ui.add(egui::DragValue::new(&mut w.campaign.map.width).range(5..=200));
        ui.label("H");
        ui.add(egui::DragValue::new(&mut w.campaign.map.height).range(5..=200));
        ui.label("ft/square");
        ui.add(egui::DragValue::new(&mut w.campaign.map.feet_per_square).range(1..=100));
        ui.label("zoom");
        ui.add(egui::Slider::new(&mut st.cell_px, 12.0..=60.0).show_value(false));
        ui.separator();
        ui.label("Mark");
        ui.add(egui::TextEdit::singleline(&mut st.mark_text).desired_width(90.0).hint_text("wall / water…"));
        ui.label(egui::RichText::new("right-click a square to mark/unmark").weak().small());
        if let Some(h) = st.hover {
            ui.label(egui::RichText::new(format!("({}, {})", h.x, h.y)).monospace());
        }
    });
    ui.label(egui::RichText::new("Drag tokens to move. Left-click an empty square with a combatant selected in Encounter to place it there. Blue = PCs, red = creatures, grey = custom. Yellow = area of effect from the Encounter tab.").weak().small());

    let highlight = area_cells(&app.encounter, w);
    let selected = app.encounter.selected.clone();

    egui::ScrollArea::both().show(ui, |ui| {
        let cell = st.cell_px;
        let (mw, mh) = (w.campaign.map.width as f32, w.campaign.map.height as f32);
        let size = egui::vec2(mw * cell + 1.0, mh * cell + 1.0);
        let (resp, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
        let origin = resp.rect.min;
        let to_screen = |p: Pos| egui::pos2(origin.x + p.x as f32 * cell, origin.y + p.y as f32 * cell);
        let to_cell = |sp: egui::Pos2| Pos::new(((sp.x - origin.x) / cell).floor() as i32, ((sp.y - origin.y) / cell).floor() as i32);

        painter.rect_filled(resp.rect, 0.0, egui::Color32::from_rgb(40, 42, 46));
        // marks
        for (k, v) in &w.campaign.map.marks {
            if let Some((x, y)) = k.split_once(',') {
                if let (Ok(x), Ok(y)) = (x.parse::<i32>(), y.parse::<i32>()) {
                    let r = egui::Rect::from_min_size(to_screen(Pos::new(x, y)), egui::vec2(cell, cell));
                    let col = match v.as_str() {
                        "wall" => egui::Color32::from_rgb(90, 90, 100),
                        "water" => egui::Color32::from_rgb(40, 80, 140),
                        "difficult" => egui::Color32::from_rgb(90, 70, 40),
                        _ => egui::Color32::from_rgb(70, 60, 80),
                    };
                    painter.rect_filled(r, 0.0, col);
                    if cell >= 20.0 {
                        painter.text(r.center(), egui::Align2::CENTER_CENTER, v.chars().take(3).collect::<String>(), egui::FontId::proportional(cell * 0.35), egui::Color32::from_gray(200));
                    }
                }
            }
        }
        // highlight
        for p in &highlight {
            if p.x >= 0 && p.y >= 0 && (p.x as f32) < mw && (p.y as f32) < mh {
                let r = egui::Rect::from_min_size(to_screen(*p), egui::vec2(cell, cell));
                painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(240, 200, 60, 70));
            }
        }
        // grid lines
        let line = egui::Stroke::new(1.0, egui::Color32::from_gray(70));
        for x in 0..=w.campaign.map.width {
            let sx = origin.x + x as f32 * cell;
            painter.line_segment([egui::pos2(sx, origin.y), egui::pos2(sx, origin.y + mh * cell)], line);
        }
        for y in 0..=w.campaign.map.height {
            let sy = origin.y + y as f32 * cell;
            painter.line_segment([egui::pos2(origin.x, sy), egui::pos2(origin.x + mw * cell, sy)], line);
        }
        // tokens
        let rules = &w.library.rules;
        let active_turn = if w.campaign.encounter.active { w.campaign.encounter.current().map(|c| c.id.clone()) } else { None };
        for c in &w.campaign.encounter.combatants {
            let Some(p) = c.pos else { continue };
            let side = rules.size_squares(&c.size) as f32;
            let r = egui::Rect::from_min_size(to_screen(p), egui::vec2(cell * side, cell * side));
            let col = match c.kind {
                CombatantKind::Pc { .. } => egui::Color32::from_rgb(70, 130, 220),
                CombatantKind::Creature { .. } => egui::Color32::from_rgb(200, 70, 70),
                CombatantKind::Custom => egui::Color32::from_gray(140),
            };
            let col = if c.is_down() { col.gamma_multiply(0.4) } else { col };
            painter.circle_filled(r.center(), r.width() * 0.45, col);
            if selected.as_deref() == Some(c.id.as_str()) {
                painter.circle_stroke(r.center(), r.width() * 0.45, egui::Stroke::new(2.0, egui::Color32::WHITE));
            }
            if active_turn.as_deref() == Some(c.id.as_str()) {
                painter.circle_stroke(r.center(), r.width() * 0.5, egui::Stroke::new(2.0, egui::Color32::from_rgb(240, 200, 60)));
            }
            let initials: String = c.name.split_whitespace().filter_map(|s| s.chars().next()).take(2).collect();
            painter.text(r.center(), egui::Align2::CENTER_CENTER, initials, egui::FontId::proportional(cell * 0.45 * side.max(1.0).sqrt()), egui::Color32::WHITE);
            if cell >= 24.0 {
                painter.text(r.center_bottom() + egui::vec2(0.0, -2.0), egui::Align2::CENTER_BOTTOM, format!("{}", c.hp), egui::FontId::proportional(cell * 0.3), egui::Color32::from_gray(230));
            }
        }

        // interaction
        st.hover = resp.hover_pos().map(to_cell);
        if resp.drag_started() {
            if let Some(sp) = resp.interact_pointer_pos() {
                let cp = to_cell(sp);
                st.drag = w
                    .campaign
                    .encounter
                    .combatants
                    .iter()
                    .find(|c| c.pos.map(|p| dnd_core::grid::footprint(p, rules.size_squares(&c.size)).contains(&cp)).unwrap_or(false))
                    .map(|c| c.id.clone());
            }
        }
        if resp.dragged() {
            if let (Some(id), Some(sp)) = (&st.drag, resp.interact_pointer_pos()) {
                let cp = to_cell(sp);
                if let Some(c) = w.campaign.encounter.find_mut(id) {
                    let side = rules.size_squares(&c.size) as i32;
                    let np = Pos::new((cp.x - side / 2).clamp(0, w.campaign.map.width as i32 - side), (cp.y - side / 2).clamp(0, w.campaign.map.height as i32 - side));
                    c.pos = Some(np);
                }
            }
        }
        if resp.drag_stopped() {
            st.drag = None;
        }
        if resp.clicked() {
            if let Some(sp) = resp.interact_pointer_pos() {
                let cp = to_cell(sp);
                let occupied = w.campaign.encounter.combatants.iter().any(|c| c.pos == Some(cp));
                if let Some(id) = &selected {
                    if !occupied {
                        if let Some(c) = w.campaign.encounter.find_mut(id) {
                            c.pos = Some(cp);
                        }
                    }
                }
            }
        }
        if resp.secondary_clicked() {
            if let Some(sp) = resp.interact_pointer_pos() {
                let cp = to_cell(sp);
                let key = format!("{},{}", cp.x, cp.y);
                if w.campaign.map.marks.contains_key(&key) {
                    w.campaign.map.marks.remove(&key);
                } else {
                    let text = if st.mark_text.trim().is_empty() { "wall".to_string() } else { st.mark_text.trim().to_string() };
                    w.campaign.map.marks.insert(key, text);
                }
            }
        }
    });

    if (w.campaign.map.clone(), w.campaign.encounter.clone()) != before {
        w.touch();
    }
}
