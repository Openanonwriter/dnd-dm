//! Creatures on the map: little figures seen from above that turn to face
//! the way they walk and glide from square to square.

use crate::{alpha, hash01, key_of, shade, Anim, Camera, DrawOpts};
use dnd_core::view::{MapView, TokenView};
use egui::{vec2, Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};

/// A token on its way from one square to another.
#[derive(Debug, Clone, Copy)]
pub struct TokenAnim {
    from: (f32, f32),
    to: (f32, f32),
    start: f64,
    duration: f64,
    /// Direction it faces, radians (0 = east).
    pub facing: f32,
}

impl TokenAnim {
    fn at(pos: (f32, f32), facing: f32) -> Self {
        Self { from: pos, to: pos, start: 0.0, duration: 0.0, facing }
    }
    pub fn moving(&self, time: f64) -> bool {
        self.duration > 0.0 && time < self.start + self.duration
    }
    /// Where to draw it now (top-left, in squares).
    pub fn shown(&self, time: f64) -> (f32, f32) {
        if self.duration <= 0.0 {
            return self.to;
        }
        let t = (((time - self.start) / self.duration) as f32).clamp(0.0, 1.0);
        let e = t * t * (3.0 - 2.0 * t);
        (self.from.0 + (self.to.0 - self.from.0) * e, self.from.1 + (self.to.1 - self.from.1) * e)
    }
    fn go(&mut self, to: (f32, f32), time: f64) {
        let here = self.shown(time);
        let (dx, dy) = (to.0 - here.0, to.1 - here.1);
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > 0.01 {
            self.facing = dy.atan2(dx);
        }
        self.from = here;
        self.to = to;
        self.start = time;
        // about six squares a second, never slower than a second and a bit
        self.duration = ((dist / 6.0) as f64).clamp(0.12, 1.3);
    }
}

fn ellipse(c: Pos2, rx: f32, ry: f32, rot: f32) -> Vec<Pos2> {
    let (sin, cos) = rot.sin_cos();
    (0..16)
        .map(|i| {
            let a = i as f32 / 16.0 * std::f32::consts::TAU;
            let (x, y) = (a.cos() * rx, a.sin() * ry);
            c + vec2(x * cos - y * sin, x * sin + y * cos)
        })
        .collect()
}

fn arc(c: Pos2, r: f32, from: f32, to: f32) -> Vec<Pos2> {
    let n = (((to - from).abs() / 0.2).ceil() as usize).max(2);
    (0..=n).map(|i| from + (to - from) * i as f32 / n as f32).map(|a| c + vec2(a.cos(), a.sin()) * r).collect()
}

/// Clothes for a class, hide for a creature: steady for the same `look`.
fn outfit(t: &TokenView) -> (Color32, Color32) {
    let k = key_of(if t.look.is_empty() { &t.name } else { &t.look });
    let pick = |n: u64| (k >> (n * 8)) as u8;
    if t.kind == "pc" {
        let cloth = match t.look.as_str() {
            "fighter" | "paladin" => Color32::from_rgb(150, 156, 170),
            "wizard" | "sorcerer" => Color32::from_rgb(70, 84, 180),
            "rogue" => Color32::from_rgb(58, 56, 66),
            "cleric" => Color32::from_rgb(214, 204, 170),
            "ranger" | "druid" => Color32::from_rgb(64, 112, 62),
            "barbarian" => Color32::from_rgb(150, 82, 50),
            "bard" => Color32::from_rgb(170, 60, 130),
            "warlock" => Color32::from_rgb(96, 50, 130),
            "monk" => Color32::from_rgb(200, 132, 50),
            _ => Color32::from_rgb(90 + pick(0) % 90, 80 + pick(1) % 90, 90 + pick(2) % 90),
        };
        let skins = [Color32::from_rgb(232, 190, 150), Color32::from_rgb(196, 146, 104), Color32::from_rgb(140, 96, 66), Color32::from_rgb(246, 214, 182)];
        (cloth, skins[(key_of(&t.name) % 4) as usize])
    } else {
        let hides = [
            Color32::from_rgb(92, 128, 62),
            Color32::from_rgb(126, 98, 70),
            Color32::from_rgb(110, 110, 118),
            Color32::from_rgb(140, 60, 52),
            Color32::from_rgb(74, 92, 120),
            Color32::from_rgb(160, 150, 120),
        ];
        let hide = hides[(k % hides.len() as u64) as usize];
        (shade(hide, 0.8), hide)
    }
}

fn draw_one(p: &Painter, c: Pos2, s: f32, t: &TokenView, facing: f32, walking: bool, time: f64, selected: bool, held: bool) {
    let seed = key_of(&t.id);
    let wob = hash01(seed as i32, (seed >> 32) as i32, 3) * 6.28;
    let breathe = 1.0 + 0.018 * (time as f32 * 2.2 + wob).sin();
    let step = if walking { (time as f32 * 14.0).sin() } else { 0.0 };
    let r = s * t.size as f32 * 0.43 * breathe * if held { 1.12 } else { 1.0 };
    let fade = if t.hidden { 0.45 } else { 1.0 };
    let team = match t.kind.as_str() {
        "pc" => Color32::from_rgb(64, 138, 236),
        "creature" => Color32::from_rgb(206, 62, 52),
        _ => Color32::from_rgb(150, 150, 156),
    };
    let dead = t.conditions.iter().any(|c| c == dnd_core::model::DEAD);
    // down is darkened; dead is darker still and drained of colour
    let dim = |col: Color32| {
        if dead {
            let g = ((col.r() as u16 + col.g() as u16 + col.b() as u16) / 3) as u8;
            alpha(shade(Color32::from_gray(g), 0.4), fade)
        } else if t.down {
            alpha(shade(col, 0.45), fade)
        } else {
            alpha(col, fade)
        }
    };

    // shadow, further away while it is held in the air
    let lift = if held { 0.24 } else { 0.1 };
    p.add(Shape::convex_polygon(ellipse(c + vec2(r * lift, r * (lift + 0.06)), r, r * 0.92, 0.0), alpha(Color32::BLACK, 0.4 * fade), Stroke::NONE));
    // base
    p.circle_filled(c, r, dim(shade(team, 0.42)));
    p.circle_stroke(c, r, Stroke::new((s * 0.06).max(2.0), dim(team)));

    // the figure: shoulders across the way it faces, head, a weapon arm
    let (cloth, skin) = outfit(t);
    let fwd = vec2(facing.cos(), facing.sin());
    let side = vec2(-fwd.y, fwd.x);
    let body = c + fwd * (r * 0.04 * step);
    if t.kind == "creature" {
        // a longer body with a tail end, eyes forward
        p.add(Shape::convex_polygon(ellipse(body - fwd * r * 0.12, r * 0.62, r * 0.46, facing), dim(cloth), Stroke::new(1.5, dim(Color32::from_rgb(24, 18, 16)))));
        p.add(Shape::convex_polygon(ellipse(body + fwd * r * 0.34, r * 0.3, r * 0.3, facing), dim(skin), Stroke::new(1.5, dim(Color32::from_rgb(24, 18, 16)))));
        for sgn in [-1.0, 1.0] {
            p.circle_filled(body + fwd * r * 0.46 + side * r * 0.13 * sgn, (r * 0.06).max(1.0), dim(Color32::from_rgb(255, 226, 90)));
            p.line_segment([body + side * r * 0.4 * sgn, body + side * r * 0.62 * sgn + fwd * r * (0.2 + 0.12 * step * sgn)], Stroke::new((r * 0.12).max(1.5), dim(skin)));
        }
    } else {
        let hand = body + side * r * 0.56 + fwd * r * (0.18 + 0.1 * step);
        p.line_segment([hand - fwd * r * 0.1, hand + fwd * r * 0.62], Stroke::new((r * 0.09).max(1.5), dim(Color32::from_rgb(208, 212, 220))));
        p.line_segment([body - side * r * 0.56, body - side * r * 0.56 + fwd * r * (0.2 - 0.1 * step)], Stroke::new((r * 0.16).max(2.0), dim(shade(cloth, 0.8))));
        p.add(Shape::convex_polygon(ellipse(body, r * 0.3, r * 0.62, facing), dim(cloth), Stroke::new(1.5, dim(Color32::from_rgb(24, 18, 16)))));
        p.circle_filled(hand, (r * 0.11).max(1.5), dim(skin));
        p.circle_filled(body + fwd * r * 0.06, r * 0.27, dim(Color32::from_rgb(24, 18, 16)));
        p.circle_filled(body + fwd * r * 0.06, r * 0.23, dim(skin));
        // hair at the back of the head
        p.add(Shape::convex_polygon(ellipse(body - fwd * r * 0.03, r * 0.16, r * 0.22, facing), dim(shade(cloth, 0.5)), Stroke::NONE));
    }

    // health ring
    if let Some(hp) = t.hp {
        let rr = r + (s * 0.07).max(2.5);
        let w = (s * 0.055).max(2.0);
        p.add(Shape::line(arc(c, rr, 0.0, std::f32::consts::TAU), Stroke::new(w, alpha(Color32::BLACK, 0.55 * fade))));
        let col = if hp > 0.5 {
            Color32::from_rgb(94, 204, 96)
        } else if hp > 0.25 {
            Color32::from_rgb(236, 190, 60)
        } else {
            Color32::from_rgb(232, 70, 60)
        };
        if hp > 0.0 {
            let top = -std::f32::consts::FRAC_PI_2;
            p.add(Shape::line(arc(c, rr, top, top + std::f32::consts::TAU * hp), Stroke::new(w, alpha(col, fade))));
        }
    }
    if t.current {
        let pulse = 0.5 + 0.5 * (time as f32 * 4.0).sin();
        p.circle_stroke(c, r + s * (0.16 + 0.05 * pulse), Stroke::new((s * 0.06).max(2.0), alpha(Color32::from_rgb(255, 214, 70), 0.6 + 0.4 * pulse)));
    }
    if selected {
        p.circle_stroke(c, r + s * 0.13, Stroke::new(2.0, Color32::WHITE));
    }
    if t.down || dead {
        let x = Stroke::new((s * if dead { 0.12 } else { 0.08 }).max(2.0), if dead { Color32::from_rgb(150, 20, 20) } else { Color32::from_rgb(220, 50, 50) });
        p.line_segment([c + vec2(-r, -r) * 0.6, c + vec2(r, r) * 0.6], x);
        p.line_segment([c + vec2(r, -r) * 0.6, c + vec2(-r, r) * 0.6], x);
    }
    if t.hidden {
        p.extend(Shape::dashed_line(&arc(c, r + s * 0.2, 0.0, std::f32::consts::TAU), Stroke::new(1.5, Color32::from_rgb(214, 120, 255)), 4.0, 3.0));
    }
    let conditions: Vec<&str> = t.conditions.iter().map(|c| c.as_str()).filter(|c| *c != dnd_core::model::DEAD).collect();
    if s < 14.0 {
        // too small for words: a dot for each condition
        for i in 0..conditions.len().min(4) {
            p.circle_filled(c + vec2(r * 0.9 - i as f32 * s * 0.16, -r * 0.95), (s * 0.07).max(2.5), Color32::from_rgb(190, 110, 255));
        }
        return;
    }
    // name, then how it is doing, then its conditions, each on a dark plate
    let font = egui::FontId::proportional((s * 0.24).clamp(10.0, 15.0));
    let small = egui::FontId::proportional((s * 0.2).clamp(9.0, 12.5));
    let mut at = c + vec2(0.0, r + s * 0.14);
    let mut plate = |text: String, font: &egui::FontId, bg: Color32, fg: Color32| {
        let size = vec2(text.chars().count() as f32 * font.size * 0.56 + 6.0, font.size + 3.0);
        p.rect_filled(Rect::from_center_size(at + vec2(0.0, size.y * 0.5), size), 3.0, bg);
        p.text(at, egui::Align2::CENTER_TOP, text, font.clone(), fg);
        at.y += size.y + 1.0;
    };
    let black = alpha(Color32::BLACK, 0.55 * fade);
    plate(t.name.chars().take(16).collect(), &font, black, if t.mine && t.kind == "pc" { Color32::from_rgb(255, 196, 110) } else { Color32::WHITE });
    let red = Color32::from_rgb(170, 28, 28);
    if dead {
        plate("DEAD".into(), &small, red, Color32::WHITE);
    } else if t.down {
        plate("DOWN".into(), &small, red, Color32::WHITE);
    } else if t.hp.map(|hp| hp <= 0.5).unwrap_or(false) {
        plate("bloodied".into(), &small, alpha(Color32::BLACK, 0.7), Color32::from_rgb(255, 120, 100));
    }
    if !conditions.is_empty() {
        let mut text = conditions.join(", ");
        if text.chars().count() > 30 {
            text = text.chars().take(29).chain(std::iter::once('…')).collect();
        }
        plate(text, &small, alpha(Color32::from_rgb(52, 22, 84), 0.9), Color32::from_rgb(226, 190, 255));
    }
}

pub fn draw_all(p: &Painter, rect: Rect, cam: &Camera, view: &MapView, anim: &mut Anim, time: f64, opts: &DrawOpts) {
    anim.tokens.retain(|id, _| view.tokens.iter().any(|t| &t.id == id));
    // the ones standing still first, so a walking figure passes over them
    let mut order: Vec<&TokenView> = view.tokens.iter().collect();
    order.sort_by_key(|t| (t.kind == "pc", t.y));
    for t in order {
        let target = (t.x as f32, t.y as f32);
        let facing0 = if t.kind == "pc" { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::FRAC_PI_2 };
        let a = anim.tokens.entry(t.id.clone()).or_insert_with(|| TokenAnim::at(target, facing0));
        let held = opts.held.filter(|(id, _)| *id == t.id).map(|(_, at)| at);
        if let Some(at) = held {
            // carried by the pointer: turn toward where it is going
            let (dx, dy): (f32, f32) = (at.0 - a.to.0, at.1 - a.to.1);
            if dx.abs() + dy.abs() > 0.3 {
                a.facing = dy.atan2(dx);
            }
            *a = TokenAnim { from: at, to: at, start: time, duration: 0.0, facing: a.facing };
        } else if a.to != target {
            a.go(target, time);
        }
        let (x, y) = a.shown(time);
        let half = t.size as f32 / 2.0;
        let c: Pos2 = cam.to_screen(rect, x + half, y + half);
        if !rect.expand(cam.zoom * 2.0).contains(c) {
            continue;
        }
        let _: Vec2 = c.to_vec2();
        draw_one(p, c, cam.zoom, t, a.facing, a.moving(time), time, opts.selected == Some(t.id.as_str()), held.is_some());
    }
}
