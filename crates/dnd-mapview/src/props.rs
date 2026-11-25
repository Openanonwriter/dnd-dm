//! Furniture and scenery, drawn from simple shapes as seen from above.

use crate::{alpha, hash01, shade};
use dnd_core::model::Prop;
use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};

const WOOD: Color32 = Color32::from_rgb(132, 88, 44);
const DARK_WOOD: Color32 = Color32::from_rgb(78, 50, 26);
const STONE: Color32 = Color32::from_rgb(150, 146, 140);
const GOLD: Color32 = Color32::from_rgb(232, 196, 92);
const OUT: Color32 = Color32::from_rgb(28, 20, 14);

fn shadow(p: &Painter, c: Pos2, r: f32) {
    p.circle_filled(c + vec2(r * 0.18, r * 0.22), r, alpha(Color32::BLACK, 0.35));
}

/// A box of half-size `(w, h)` squares, turned a quarter when `rot` is odd.
fn boxed(c: Pos2, s: f32, w: f32, h: f32, rot: u8) -> Rect {
    let (w, h) = if rot % 2 == 1 { (h, w) } else { (w, h) };
    Rect::from_center_size(c, vec2(w * s * 2.0, h * s * 2.0))
}

fn outline(p: &Painter, r: Rect, round: f32, fill: Color32) {
    p.rect_filled(r.translate(vec2(r.width() * 0.06, r.height() * 0.1)), round, alpha(Color32::BLACK, 0.33));
    p.rect_filled(r.expand(1.0), round, OUT);
    p.rect_filled(r, round, fill);
}

pub fn flame(p: &Painter, c: Pos2, r: f32, time: f64, on: bool, color: Color32) {
    p.circle_filled(c, r * 0.7, Color32::from_rgb(52, 40, 32));
    if !on {
        return;
    }
    let f = 0.8 + 0.2 * (time as f32 * 9.0 + c.x * 0.1).sin();
    p.circle_filled(c, r * f, alpha(color, 0.9));
    p.circle_filled(c + vec2(0.0, r * 0.1), r * 0.5 * f, Color32::from_rgb(255, 240, 170));
}

pub fn draw(p: &Painter, c: Pos2, s: f32, prop: &Prop, time: f64, dm: bool) {
    let seed = (prop.x * 7.0 + prop.y * 13.0) as i32;
    let v = |k: u32| hash01(seed, k as i32, 7);
    let line = |a: Vec2, b: Vec2, w: f32, col: Color32| {
        p.line_segment([c + a * s, c + b * s], Stroke::new((w * s).max(1.0), col));
    };
    match prop.kind.as_str() {
        "chest" => {
            let r = boxed(c, s, 0.32, 0.22, prop.rot);
            outline(p, r, 2.0, WOOD);
            p.rect_filled(Rect::from_center_size(r.center(), vec2(r.width() * 0.16, r.height())), 0.0, GOLD);
            p.rect_filled(Rect::from_center_size(r.center(), vec2(r.width(), r.height() * 0.14)), 0.0, DARK_WOOD);
            p.circle_filled(r.center(), s * 0.05, GOLD);
        }
        "barrel" => {
            shadow(p, c, s * 0.3);
            p.circle_filled(c, s * 0.3, OUT);
            p.circle_filled(c, s * 0.27, WOOD);
            p.circle_stroke(c, s * 0.2, Stroke::new((s * 0.03).max(1.0), DARK_WOOD));
            line(vec2(-0.19, 0.0), vec2(0.19, 0.0), 0.02, DARK_WOOD);
            line(vec2(0.0, -0.19), vec2(0.0, 0.19), 0.02, DARK_WOOD);
        }
        "crate" => {
            let r = boxed(c, s, 0.28, 0.28, 0);
            outline(p, r, 1.0, shade(WOOD, 1.1));
            p.line_segment([r.left_top(), r.right_bottom()], Stroke::new((s * 0.04).max(1.0), DARK_WOOD));
            p.line_segment([r.right_top(), r.left_bottom()], Stroke::new((s * 0.04).max(1.0), DARK_WOOD));
        }
        "table" => {
            let r = boxed(c, s, 0.75, 0.38, prop.rot);
            outline(p, r, 3.0, shade(WOOD, 1.15));
            for i in 1..4 {
                let y = r.min.y + r.height() * i as f32 / 4.0;
                p.line_segment([pos2(r.min.x, y), pos2(r.max.x, y)], Stroke::new(1.0, shade(WOOD, 0.8)));
            }
        }
        "chair" => {
            let r = boxed(c, s, 0.2, 0.2, 0);
            outline(p, r, 2.0, WOOD);
            let back = match prop.rot % 4 {
                0 => [r.left_top(), r.right_top()],
                1 => [r.right_top(), r.right_bottom()],
                2 => [r.left_bottom(), r.right_bottom()],
                _ => [r.left_top(), r.left_bottom()],
            };
            p.line_segment(back, Stroke::new((s * 0.08).max(2.0), DARK_WOOD));
        }
        "bed" => {
            let r = boxed(c, s, 0.42, 0.85, prop.rot);
            outline(p, r, 3.0, Color32::from_rgb(150, 44, 40));
            let pillow = if prop.rot % 2 == 0 {
                Rect::from_min_max(r.min + vec2(r.width() * 0.12, r.height() * 0.05), pos2(r.max.x - r.width() * 0.12, r.min.y + r.height() * 0.26))
            } else {
                Rect::from_min_max(r.min + vec2(r.width() * 0.05, r.height() * 0.12), pos2(r.min.x + r.width() * 0.26, r.max.y - r.height() * 0.12))
            };
            p.rect_filled(pillow, 3.0, Color32::from_rgb(236, 230, 214));
        }
        "bookshelf" => {
            let r = boxed(c, s, 0.9, 0.22, prop.rot);
            outline(p, r, 1.0, DARK_WOOD);
            let cols = [Color32::from_rgb(150, 50, 44), Color32::from_rgb(52, 92, 150), Color32::from_rgb(60, 124, 70), Color32::from_rgb(190, 160, 70)];
            let n = 9;
            for i in 0..n {
                let f = (i as f32 + 0.5) / n as f32;
                let (q, size) = if prop.rot % 2 == 0 {
                    (pos2(r.min.x + r.width() * f, r.center().y), vec2(r.width() / n as f32 * 0.7, r.height() * 0.7))
                } else {
                    (pos2(r.center().x, r.min.y + r.height() * f), vec2(r.width() * 0.7, r.height() / n as f32 * 0.7))
                };
                p.rect_filled(Rect::from_center_size(q, size), 0.0, cols[(i + seed.unsigned_abs() as usize) % cols.len()]);
            }
        }
        "pillar" => {
            shadow(p, c, s * 0.36);
            p.circle_filled(c, s * 0.36, Color32::from_rgb(40, 38, 40));
            p.circle_filled(c, s * 0.32, STONE);
            p.circle_filled(c + vec2(-s * 0.05, -s * 0.05), s * 0.2, shade(STONE, 1.2));
        }
        "statue" => {
            let r = boxed(c, s, 0.34, 0.34, 0);
            outline(p, r, 1.0, shade(STONE, 0.8));
            p.circle_filled(c, s * 0.24, STONE);
            p.circle_filled(c + vec2(0.0, -s * 0.03), s * 0.11, shade(STONE, 1.25));
        }
        "altar" => {
            let r = boxed(c, s, 0.7, 0.34, prop.rot);
            outline(p, r, 2.0, shade(STONE, 1.15));
            p.rect_filled(Rect::from_center_size(r.center(), r.size() * vec2(0.3, 1.0)), 0.0, Color32::from_rgb(120, 30, 40));
            p.circle_filled(r.center(), s * 0.08, GOLD);
        }
        "stairs" => {
            let r = boxed(c, s, 0.46, 0.46, 0);
            p.rect_filled(r.expand(1.0), 0.0, OUT);
            let n = 6;
            for i in 0..n {
                let f = i as f32 / n as f32;
                let col = shade(STONE, 1.1 - 0.75 * f);
                let step = match prop.rot % 4 {
                    0 => Rect::from_min_max(pos2(r.min.x, r.min.y + r.height() * f), pos2(r.max.x, r.min.y + r.height() * (f + 1.0 / n as f32))),
                    1 => Rect::from_min_max(pos2(r.max.x - r.width() * (f + 1.0 / n as f32), r.min.y), pos2(r.max.x - r.width() * f, r.max.y)),
                    2 => Rect::from_min_max(pos2(r.min.x, r.max.y - r.height() * (f + 1.0 / n as f32)), pos2(r.max.x, r.max.y - r.height() * f)),
                    _ => Rect::from_min_max(pos2(r.min.x + r.width() * f, r.min.y), pos2(r.min.x + r.width() * (f + 1.0 / n as f32), r.max.y)),
                };
                p.rect_filled(step.shrink(0.5), 0.0, col);
            }
        }
        "campfire" => {
            for i in 0..7 {
                let a = i as f32 / 7.0 * std::f32::consts::TAU;
                p.circle_filled(c + vec2(a.cos(), a.sin()) * s * 0.3, s * 0.07, shade(STONE, 0.8));
            }
            line(vec2(-0.2, -0.14), vec2(0.2, 0.14), 0.08, DARK_WOOD);
            line(vec2(-0.2, 0.14), vec2(0.2, -0.14), 0.08, DARK_WOOD);
            flame(p, c, s * 0.2, time, true, Color32::from_rgb(255, 130, 30));
        }
        "tree" => {
            let r = s * (0.55 + 0.2 * v(1));
            p.circle_filled(c + vec2(r * 0.25, r * 0.3), r, alpha(Color32::BLACK, 0.35));
            p.circle_filled(c, s * 0.12, DARK_WOOD);
            let sway = (time as f32 * 0.8 + v(2) * 6.0).sin() * s * 0.015;
            let green = Color32::from_rgb(40, 96 + (40.0 * v(3)) as u8, 44);
            for (i, (dx, dy, k)) in [(-0.3, 0.1, 0.62), (0.28, 0.18, 0.6), (0.0, -0.3, 0.64), (0.0, 0.05, 0.72)].iter().enumerate() {
                let q = c + vec2(*dx * r + sway * (i as f32 - 1.5), *dy * r);
                p.circle_filled(q, r * k, shade(green, 0.62 + 0.14 * i as f32));
                p.circle_filled(q + vec2(-r * 0.12, -r * 0.14), r * k * 0.55, shade(green, 0.86 + 0.14 * i as f32));
            }
        }
        "bush" => {
            let green = Color32::from_rgb(58, 118, 50);
            shadow(p, c, s * 0.3);
            for (dx, dy, k) in [(-0.14, 0.05, 0.2), (0.14, 0.06, 0.2), (0.0, -0.1, 0.22)] {
                p.circle_filled(c + vec2(dx, dy) * s, s * k, shade(green, 0.8));
                p.circle_filled(c + vec2(dx - 0.03, dy - 0.04) * s, s * k * 0.6, shade(green, 1.15));
            }
        }
        "boulder" => {
            let r = s * (0.3 + 0.1 * v(1));
            let pts: Vec<Pos2> = (0..8).map(|i| c + vec2((i as f32 * 0.785).cos(), (i as f32 * 0.785).sin()) * r * (0.8 + 0.3 * v(10 + i))).collect();
            p.add(Shape::convex_polygon(pts.iter().map(|q| *q + vec2(r * 0.2, r * 0.25)).collect(), alpha(Color32::BLACK, 0.35), Stroke::NONE));
            p.add(Shape::convex_polygon(pts, shade(STONE, 0.75), Stroke::new(1.5, OUT)));
            p.circle_filled(c + vec2(-r * 0.2, -r * 0.25), r * 0.4, shade(STONE, 0.98));
        }
        "well" => {
            shadow(p, c, s * 0.38);
            p.circle_filled(c, s * 0.38, shade(STONE, 0.6));
            p.circle_filled(c, s * 0.33, STONE);
            p.circle_filled(c, s * 0.22, Color32::from_rgb(22, 44, 78));
            p.circle_stroke(c, s * (0.08 + 0.05 * (time as f32).sin().abs()), Stroke::new(1.0, Color32::from_rgb(110, 160, 210)));
        }
        "trap" => {
            p.circle_filled(c, s * 0.3, Color32::from_rgb(44, 36, 36));
            for i in 0..8 {
                let a = i as f32 * 0.785;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * s * 0.1, c + d * s * 0.3], Stroke::new((s * 0.04).max(1.0), Color32::from_rgb(200, 200, 210)));
            }
            p.circle_filled(c, s * 0.08, Color32::from_rgb(190, 40, 40));
        }
        "rug" => {
            let r = boxed(c, s, 0.9, 0.55, prop.rot);
            p.rect_filled(r, 2.0, GOLD);
            p.rect_filled(r.shrink(s * 0.07), 1.0, Color32::from_rgb(134, 34, 44));
            p.rect_filled(Rect::from_center_size(r.center(), r.size() * 0.35), 1.0, Color32::from_rgb(170, 64, 60));
        }
        "bones" => {
            let bone = Color32::from_rgb(226, 220, 200);
            line(vec2(-0.22, -0.1), vec2(0.18, 0.14), 0.05, bone);
            line(vec2(-0.16, 0.16), vec2(0.2, -0.12), 0.05, bone);
            p.circle_filled(c + vec2(-0.05, -0.2) * s, s * 0.1, bone);
            p.circle_filled(c + vec2(-0.08, -0.21) * s, s * 0.025, OUT);
            p.circle_filled(c + vec2(-0.02, -0.21) * s, s * 0.025, OUT);
        }
        "portal" => {
            for i in 0..3 {
                let phase = (time as f32 * 0.9 + i as f32 * 0.33).fract();
                p.circle_stroke(c, s * (0.12 + 0.3 * phase), Stroke::new((s * 0.05).max(1.5), alpha(Color32::from_rgb(170, 90, 255), 1.0 - phase)));
            }
            p.circle_filled(c, s * 0.14, Color32::from_rgb(214, 170, 255));
        }
        other => {
            p.circle_filled(c, s * 0.26, Color32::from_rgb(70, 60, 80));
            p.text(c, egui::Align2::CENTER_CENTER, other.chars().take(2).collect::<String>(), egui::FontId::proportional((s * 0.3).max(8.0)), Color32::WHITE);
        }
    }
    if dm && prop.hidden {
        let ring: Vec<Pos2> = (0..=16).map(|i| c + vec2((i as f32 * 0.3927).cos(), (i as f32 * 0.3927).sin()) * s * 0.46).collect();
        p.extend(Shape::dashed_line(&ring, Stroke::new(1.5, Color32::from_rgb(214, 120, 255)), 4.0, 3.0));
    }
}
