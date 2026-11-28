//! Draws a `MapView` with egui: textured terrain, rock with depth, walls and
//! doors, props, light and darkness, soft-edged fog and tokens that walk
//! from square to square. No image files: everything is drawn from shapes,
//! so the same code runs on Windows and on a phone.

pub mod face;
pub mod help;
mod props;
mod tokens;

use dnd_core::model::{tile, WallKind};
use dnd_core::view::MapView;
use egui::{pos2, vec2, Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use std::collections::HashMap;

pub use tokens::TokenAnim;

/// Where the viewer is looking: the map square at the middle of the screen
/// and how many pixels one square takes.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub center: Vec2,
    pub zoom: f32,
    fitted_for: u64,
}

impl Default for Camera {
    fn default() -> Self {
        Self { center: vec2(15.0, 10.0), zoom: 32.0, fitted_for: 0 }
    }
}

impl Camera {
    pub fn to_screen(&self, rect: Rect, x: f32, y: f32) -> Pos2 {
        rect.center() + (vec2(x, y) - self.center) * self.zoom
    }
    /// Screen point to map coordinates, in squares (fractions included).
    pub fn to_map(&self, rect: Rect, p: Pos2) -> (f32, f32) {
        let v = (p - rect.center()) / self.zoom + self.center;
        (v.x, v.y)
    }
    pub fn to_cell(&self, rect: Rect, p: Pos2) -> (i32, i32) {
        let (x, y) = self.to_map(rect, p);
        (x.floor() as i32, y.floor() as i32)
    }
    pub fn pan(&mut self, screen_delta: Vec2) {
        self.center -= screen_delta / self.zoom;
    }
    /// Zoom keeping the map point under `about` where it is.
    pub fn zoom_about(&mut self, rect: Rect, about: Pos2, factor: f32) {
        let before = self.to_map(rect, about);
        self.zoom = (self.zoom * factor).clamp(6.0, 160.0);
        let after = self.to_map(rect, about);
        self.center += vec2(before.0 - after.0, before.1 - after.1);
    }
    /// Show the whole map.
    pub fn fit(&mut self, rect: Rect, width: u32, height: u32) {
        self.center = vec2(width as f32 / 2.0, height as f32 / 2.0);
        self.zoom = (rect.width() / width.max(1) as f32).min(rect.height() / height.max(1) as f32).clamp(6.0, 96.0) * 0.97;
    }
    /// Fit once for each new map (pass something that changes with the map).
    pub fn fit_once(&mut self, rect: Rect, width: u32, height: u32, key: u64) {
        if self.fitted_for != key && rect.width() > 10.0 {
            self.fit(rect, width, height);
            self.fitted_for = key;
        }
    }
    /// Scroll wheel and pinch zoom; call with the map area's response.
    pub fn zoom_input(&mut self, ui: &egui::Ui, resp: &egui::Response) {
        if let Some(p) = resp.hover_pos() {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            if scroll != 0.0 {
                self.zoom_about(resp.rect, p, (scroll * 0.004).exp());
            }
            if pinch != 1.0 {
                self.zoom_about(resp.rect, p, pinch);
            }
        } else if let Some(touch) = ui.input(|i| i.multi_touch()) {
            self.zoom_about(resp.rect, touch.start_pos, touch.zoom_delta);
            self.pan(touch.translation_delta);
        }
    }
}

pub fn key_of(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Default)]
pub struct DrawOpts<'a> {
    pub grid: bool,
    /// Token with a selection ring.
    pub selected: Option<&'a str>,
    /// Token being dragged and where it is held, in squares (its top-left).
    pub held: Option<(&'a str, (f32, f32))>,
    /// Squares to tint (area of effect, reachable squares…).
    pub highlight: &'a [(i32, i32)],
    pub highlight_color: Option<Color32>,
    /// Draw the fog solid even for the DM ("what the players see").
    pub solid_fog: bool,
}

/// Remembered between frames: where each token is on its way to.
#[derive(Default)]
pub struct Anim {
    pub tokens: HashMap<String, TokenAnim>,
    map: String,
}

impl Anim {
    /// Is anything still moving? (Water and torches always are, gently.)
    pub fn moving(&self, time: f64) -> bool {
        self.tokens.values().any(|t| t.moving(time))
    }
}

pub(crate) fn hash01(x: i32, y: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ salt.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

pub(crate) fn shade(c: Color32, f: f32) -> Color32 {
    let m = |v: u8| (v as f32 * f).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(m(c.r()), m(c.g()), m(c.b()), c.a())
}

pub(crate) fn alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

pub(crate) fn hex(s: &str) -> Color32 {
    let s = s.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(s.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255);
    if s.len() >= 6 {
        Color32::from_rgb(p(0), p(2), p(4))
    } else {
        Color32::from_rgb(255, 154, 46)
    }
}

const VOID: Color32 = Color32::from_rgb(9, 10, 14);
const FOG: Color32 = Color32::from_rgb(7, 8, 12);

fn base_color(t: char) -> Color32 {
    match t {
        tile::FLOOR => Color32::from_rgb(121, 114, 104),
        tile::WOOD => Color32::from_rgb(134, 92, 52),
        tile::DIRT => Color32::from_rgb(112, 86, 58),
        tile::GRASS => Color32::from_rgb(74, 122, 58),
        tile::SAND => Color32::from_rgb(196, 172, 118),
        tile::SNOW => Color32::from_rgb(226, 232, 238),
        tile::WATER => Color32::from_rgb(42, 96, 158),
        tile::RUBBLE => Color32::from_rgb(108, 102, 94),
        tile::LAVA => Color32::from_rgb(214, 82, 20),
        tile::ROCK => Color32::from_rgb(34, 32, 38),
        _ => VOID,
    }
}

fn draw_tile(p: &Painter, r: Rect, t: char, x: i32, y: i32, time: f64, detail: bool) {
    let s = r.width();
    let v = hash01(x, y, 1);
    let base = shade(base_color(t), 0.9 + 0.2 * v);
    p.rect_filled(r, 0.0, base);
    if !detail {
        return;
    }
    let at = |fx: f32, fy: f32| pos2(r.min.x + fx * s, r.min.y + fy * s);
    match t {
        tile::FLOOR => {
            // a flagstone: light on the top-left edges, dark on the others
            let (hi, lo) = (Stroke::new(1.0, shade(base, 1.14)), Stroke::new(1.0, shade(base, 0.72)));
            p.line_segment([at(0.03, 0.03), at(0.97, 0.03)], hi);
            p.line_segment([at(0.03, 0.03), at(0.03, 0.97)], hi);
            p.line_segment([at(0.0, 1.0), at(1.0, 1.0)], lo);
            p.line_segment([at(1.0, 0.0), at(1.0, 1.0)], lo);
            if v > 0.82 {
                let a = hash01(x, y, 2);
                p.add(Shape::line(vec![at(0.2 + 0.3 * a, 0.15), at(0.45, 0.45 + 0.2 * a), at(0.4 + 0.3 * a, 0.85)], Stroke::new(1.0, shade(base, 0.62))));
            }
        }
        tile::WOOD => {
            for i in 0..3 {
                let top = i as f32 / 3.0;
                let tint = 0.9 + 0.2 * hash01(x, y, 10 + i);
                p.rect_filled(Rect::from_min_max(at(0.0, top), at(1.0, top + 1.0 / 3.0)), 0.0, shade(base_color(t), tint));
                p.line_segment([at(0.0, top), at(1.0, top)], Stroke::new(1.0, shade(base, 0.6)));
                let joint = 0.15 + 0.7 * hash01(x, y, 20 + i);
                p.line_segment([at(joint, top), at(joint, top + 1.0 / 3.0)], Stroke::new(1.0, shade(base, 0.6)));
            }
        }
        tile::DIRT | tile::SAND | tile::SNOW => {
            let dot = if t == tile::SNOW { Color32::from_rgb(190, 206, 226) } else { shade(base, 0.78) };
            for i in 0..4 {
                p.circle_filled(at(0.1 + 0.8 * hash01(x, y, 30 + i), 0.1 + 0.8 * hash01(x, y, 40 + i)), (s * 0.035).max(0.8), dot);
            }
        }
        tile::GRASS => {
            for i in 0..5 {
                let (gx, gy) = (0.1 + 0.8 * hash01(x, y, 50 + i), 0.2 + 0.7 * hash01(x, y, 60 + i));
                let lean = hash01(x, y, 70 + i) - 0.5;
                let col = if i % 2 == 0 { shade(base, 1.25) } else { shade(base, 0.72) };
                p.line_segment([at(gx, gy), at(gx + lean * 0.12, gy - 0.14)], Stroke::new((s * 0.03).max(1.0), col));
            }
        }
        tile::WATER => {
            for i in 0..2 {
                let phase = time as f32 * (0.5 + 0.2 * i as f32) + hash01(x, y, 80 + i) * 6.28;
                let wy = 0.3 + 0.4 * i as f32 + 0.06 * phase.sin();
                let wx = 0.15 + 0.3 * hash01(x, y, 90 + i) + 0.05 * (phase * 0.7).cos();
                let pts = (0..5).map(|k| at(wx + 0.1 * k as f32, wy + 0.03 * ((k as f32) * 1.6 + phase).sin())).collect();
                p.add(Shape::line(pts, Stroke::new((s * 0.03).max(1.0), alpha(Color32::from_rgb(170, 214, 245), 0.55))));
            }
        }
        tile::RUBBLE => {
            for i in 0..5 {
                let c = at(0.15 + 0.7 * hash01(x, y, 100 + i), 0.15 + 0.7 * hash01(x, y, 110 + i));
                let rad = s * (0.06 + 0.07 * hash01(x, y, 120 + i));
                p.circle_filled(c + vec2(rad * 0.3, rad * 0.35), rad, shade(base, 0.55));
                p.circle_filled(c, rad, shade(base, 1.2));
            }
        }
        tile::LAVA => {
            let pulse = 0.5 + 0.5 * (time as f32 * 1.3 + v * 6.28).sin();
            p.rect_filled(r, 0.0, alpha(Color32::from_rgb(255, 196, 60), 0.18 + 0.2 * pulse));
            for i in 0..3 {
                let c = at(0.2 + 0.6 * hash01(x, y, 130 + i), 0.2 + 0.6 * hash01(x, y, 140 + i));
                p.circle_filled(c, s * (0.07 + 0.05 * pulse), Color32::from_rgb(255, 222, 110));
            }
            p.line_segment([at(0.1, 0.2 + 0.6 * v), at(0.9, 0.8 - 0.6 * v)], Stroke::new((s * 0.05).max(1.0), Color32::from_rgb(92, 26, 12)));
        }
        _ => {}
    }
}

/// A quad whose corners can each have their own colour (soft gradients).
fn gradient_quad(mesh: &mut Mesh, corners: [Pos2; 4], colors: [Color32; 4]) {
    let i = mesh.vertices.len() as u32;
    for k in 0..4 {
        mesh.colored_vertex(corners[k], colors[k]);
    }
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
}

/// Draw the map into `rect`. Returns nothing: hit-testing uses the camera.
pub fn draw(painter: &Painter, rect: Rect, cam: &Camera, view: &MapView, anim: &mut Anim, time: f64, opts: &DrawOpts) {
    let p = painter.with_clip_rect(rect);
    p.rect_filled(rect, 0.0, VOID);
    if anim.map != view.id {
        anim.tokens.clear();
        anim.map = view.id.clone();
    }
    let s = cam.zoom;
    let detail = s >= 14.0;
    let (x0, y0) = cam.to_cell(rect, rect.min);
    let (x1, y1) = cam.to_cell(rect, rect.max);
    let (x0, y0) = (x0.max(0), y0.max(0));
    let (x1, y1) = (x1.min(view.width as i32 - 1), y1.min(view.height as i32 - 1));
    let cell = |x: i32, y: i32| Rect::from_min_size(cam.to_screen(rect, x as f32, y as f32), vec2(s + 0.5, s + 0.5));
    let open = |x: i32, y: i32| {
        let t = view.tile(x, y);
        t != tile::ROCK && t != tile::UNKNOWN
    };

    // ---- terrain
    for y in y0..=y1 {
        for x in x0..=x1 {
            let t = view.tile(x, y);
            if t != tile::UNKNOWN {
                draw_tile(&p, cell(x, y), t, x, y, time, detail);
            }
        }
    }
    // ---- rock depth: a lit face where rock meets floor below it, and shadows on the floor
    let mut shadows = Mesh::default();
    let dark = |a: f32| alpha(Color32::BLACK, a);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let r = cell(x, y);
            if view.tile(x, y) == tile::ROCK {
                if open(x, y + 1) {
                    let face = Rect::from_min_max(pos2(r.min.x, r.max.y - s * 0.3), r.max);
                    p.rect_filled(face, 0.0, Color32::from_rgb(92, 84, 80));
                    if detail {
                        let line = Stroke::new(1.0, Color32::from_rgb(58, 52, 52));
                        p.line_segment([pos2(face.min.x, face.center().y), pos2(face.max.x, face.center().y)], line);
                        let off = if (x + y) % 2 == 0 { 0.3 } else { 0.7 };
                        p.line_segment([pos2(face.min.x + s * off, face.min.y), pos2(face.min.x + s * off, face.center().y)], line);
                        p.line_segment([pos2(face.min.x + s * (1.0 - off), face.center().y), pos2(face.min.x + s * (1.0 - off), face.max.y)], line);
                    }
                    p.line_segment([face.left_top(), face.right_top()], Stroke::new(1.5, Color32::from_rgb(128, 118, 110)));
                }
                let edge = Stroke::new((s * 0.05).max(1.0), Color32::from_rgb(14, 13, 17));
                if open(x, y - 1) {
                    p.line_segment([r.left_top(), r.right_top()], edge);
                }
                if open(x - 1, y) {
                    p.line_segment([r.left_top(), r.left_bottom()], edge);
                }
                if open(x + 1, y) {
                    p.line_segment([r.right_top(), r.right_bottom()], edge);
                }
            } else if open(x, y) {
                if view.tile(x, y - 1) == tile::ROCK {
                    let d = s * 0.42;
                    gradient_quad(&mut shadows, [r.left_top(), r.right_top(), pos2(r.max.x, r.min.y + d), pos2(r.min.x, r.min.y + d)], [dark(0.5), dark(0.5), dark(0.0), dark(0.0)]);
                }
                if view.tile(x - 1, y) == tile::ROCK {
                    let d = s * 0.28;
                    gradient_quad(&mut shadows, [r.left_top(), pos2(r.min.x + d, r.min.y), pos2(r.min.x + d, r.max.y), r.left_bottom()], [dark(0.32), dark(0.0), dark(0.0), dark(0.32)]);
                }
            }
        }
    }
    p.add(Shape::mesh(shadows));

    // ---- grid
    if opts.grid && s >= 10.0 {
        let line = Stroke::new(1.0, alpha(Color32::BLACK, 0.16));
        for y in y0..=y1 {
            for x in x0..=x1 {
                if open(x, y) {
                    let r = cell(x, y);
                    p.line_segment([r.right_top(), r.right_bottom()], line);
                    p.line_segment([r.left_bottom(), r.right_bottom()], line);
                }
            }
        }
    }
    // ---- highlighted squares
    let hl = opts.highlight_color.unwrap_or(Color32::from_rgba_unmultiplied(240, 200, 60, 70));
    for (x, y) in opts.highlight {
        if *x >= x0 && *x <= x1 && *y >= y0 && *y <= y1 {
            p.rect_filled(cell(*x, *y).shrink(1.0), 2.0, hl);
        }
    }

    // ---- props, walls, doors
    for prop in &view.props {
        props::draw(&p, cam.to_screen(rect, prop.x, prop.y), s, prop, time, view.dm);
    }
    for w in &view.walls {
        draw_wall(&p, rect, cam, w, view.dm);
    }
    if let (Some(st), true) = (view.start, view.dm) {
        let c = cam.to_screen(rect, st.x as f32 + 0.5, st.y as f32 + 0.5);
        p.circle_stroke(c, s * 0.42, Stroke::new(2.0, Color32::from_rgb(90, 220, 140)));
        p.text(c, egui::Align2::CENTER_CENTER, "START", egui::FontId::proportional((s * 0.2).max(8.0)), Color32::from_rgb(90, 220, 140));
    }

    // ---- warm light on the floor. Worked out square by square from what is
    // actually lit, so it stops at walls, then blended at the corners.
    let fps = view.feet_per_square.max(1) as f32;
    let (gx0, gy0) = (x0 - 1, y0 - 1);
    let (gw, gh) = ((x1 - x0 + 3).max(1) as usize, (y1 - y0 + 3).max(1) as usize);
    let mut warm = vec![0f32; gw * gh];
    let night = view.darkness.clamp(0.2, 1.0);
    let mut shine = |cx: f32, cy: f32, radius: f32, seed: f32| {
        let flicker = 0.86 + 0.14 * ((time as f32 * 7.0 + seed).sin() * (time as f32 * 3.1 + seed * 2.0).cos());
        let r = radius.ceil() as i32 + 1;
        for y in (cy as i32 - r).max(gy0)..=(cy as i32 + r).min(gy0 + gh as i32 - 1) {
            for x in (cx as i32 - r).max(gx0)..=(cx as i32 + r).min(gx0 + gw as i32 - 1) {
                let t = view.tile(x, y);
                if t == tile::ROCK || t == tile::UNKNOWN || view.light_at(x, y) == 0 || view.seen_at(x, y) != 2 {
                    continue;
                }
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                if d < radius {
                    warm[(y - gy0) as usize * gw + (x - gx0) as usize] += (1.0 - d / radius).powf(1.4) * flicker * night;
                }
            }
        }
    };
    for l in view.lights.iter().filter(|l| l.on) {
        shine(l.x, l.y, l.dim.max(l.bright).max(5) as f32 / fps, l.x * 3.1 + l.y);
    }
    for t in view.tokens.iter().filter(|t| t.light > 0) {
        let c = anim.tokens.get(&t.id).map(|a| a.shown(time)).unwrap_or((t.x as f32, t.y as f32));
        let half = t.size as f32 / 2.0;
        shine(c.0 + half, c.1 + half, t.light as f32 * 2.0 / fps, t.x as f32);
    }
    let warm_at = |x: i32, y: i32| -> f32 {
        if x < gx0 || y < gy0 || x >= gx0 + gw as i32 || y >= gy0 + gh as i32 {
            0.0
        } else {
            warm[(y - gy0) as usize * gw + (x - gx0) as usize]
        }
    };
    let mut glow = Mesh::default();
    let ember = |x: i32, y: i32| alpha(Color32::from_rgb(255, 168, 72), ((warm_at(x - 1, y - 1) + warm_at(x, y - 1) + warm_at(x - 1, y) + warm_at(x, y)) / 4.0 * 0.26).min(0.32));
    for y in y0..=y1 {
        for x in x0..=x1 {
            let cols = [ember(x, y), ember(x + 1, y), ember(x + 1, y + 1), ember(x, y + 1)];
            if cols.iter().all(|c| c.a() == 0) {
                continue;
            }
            let r = cell(x, y);
            gradient_quad(&mut glow, [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], cols);
        }
    }
    p.add(Shape::mesh(glow));
    for l in &view.lights {
        props::flame(&p, cam.to_screen(rect, l.x, l.y), s * 0.16, time, l.on, hex(&l.color));
    }

    // ---- darkness and fog, blended at the corners of the squares so edges are soft
    let fog_strength = if view.dm && !opts.solid_fog { 0.45 } else { 1.0 };
    let cover = |x: i32, y: i32| -> f32 {
        let (x, y) = (x.clamp(0, view.width as i32 - 1), y.clamp(0, view.height as i32 - 1));
        match view.seen_at(x, y) {
            0 => 1.0 * fog_strength,
            1 => 0.62 * fog_strength,
            _ => {
                fog_strength.max(0.6)
                    * match view.light_at(x, y) {
                        2 => 0.0,
                        1 => 0.26,
                        _ => 0.5,
                    }
            }
        }
    };
    let mut fog = Mesh::default();
    let corner = |x: i32, y: i32| alpha(FOG, (cover(x - 1, y - 1) + cover(x, y - 1) + cover(x - 1, y) + cover(x, y)) / 4.0);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let cols = [corner(x, y), corner(x + 1, y), corner(x + 1, y + 1), corner(x, y + 1)];
            if cols.iter().all(|c| c.a() == 0) {
                continue;
            }
            let r = cell(x, y);
            gradient_quad(&mut fog, [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], cols);
        }
    }
    p.add(Shape::mesh(fog));

    // ---- notes
    for m in &view.markers {
        let c = cam.to_screen(rect, m.x as f32 + 0.5, m.y as f32 + 0.5);
        let col = if m.hidden { Color32::from_rgb(214, 120, 255) } else { Color32::from_rgb(255, 214, 90) };
        p.circle_filled(c + vec2(0.0, -s * 0.12), s * 0.16, col);
        p.add(Shape::convex_polygon(vec![c + vec2(-s * 0.13, -s * 0.05), c + vec2(s * 0.13, -s * 0.05), c + vec2(0.0, s * 0.22)], col, Stroke::NONE));
        p.circle_filled(c + vec2(0.0, -s * 0.12), s * 0.06, Color32::from_rgb(40, 30, 20));
        if s >= 22.0 && !m.text.is_empty() {
            let text: String = m.text.chars().take(28).collect();
            let font = egui::FontId::proportional((s * 0.26).clamp(10.0, 15.0));
            p.text(c + vec2(1.0, s * 0.3 + 1.0), egui::Align2::CENTER_TOP, &text, font.clone(), Color32::BLACK);
            p.text(c + vec2(0.0, s * 0.3), egui::Align2::CENTER_TOP, &text, font, col);
        }
    }

    // ---- tokens
    tokens::draw_all(&p, rect, cam, view, anim, time, opts);
}

fn draw_wall(p: &Painter, rect: Rect, cam: &Camera, w: &dnd_core::model::Wall, dm: bool) {
    let s = cam.zoom;
    let (a, b) = (cam.to_screen(rect, w.x1, w.y1), cam.to_screen(rect, w.x2, w.y2));
    match w.kind {
        WallKind::Wall => {
            p.line_segment([a, b], Stroke::new((s * 0.2).max(3.0), Color32::from_rgb(20, 18, 22)));
            p.line_segment([a, b], Stroke::new((s * 0.1).max(1.5), Color32::from_rgb(98, 90, 86)));
        }
        WallKind::Window => {
            p.line_segment([a, b], Stroke::new((s * 0.16).max(3.0), Color32::from_rgb(20, 18, 22)));
            p.line_segment([a, b], Stroke::new((s * 0.08).max(1.5), Color32::from_rgb(150, 212, 240)));
        }
        WallKind::Door | WallKind::SecretDoor => {
            let secret = w.kind == WallKind::SecretDoor;
            let d = b - a;
            let len = d.length().max(1.0);
            let (u, n) = (d / len, vec2(-d.y, d.x) / len);
            let th = (s * 0.11).max(2.0);
            // door posts
            for q in [a, b] {
                p.circle_filled(q, th * 0.9, Color32::from_rgb(52, 44, 40));
            }
            let wood = if secret { Color32::from_rgb(112, 100, 122) } else { Color32::from_rgb(142, 92, 46) };
            let (h0, h1, half) = if w.open {
                // swung open on its first post
                (a + n * (len * 0.08), a + n * (len * 0.92), u * th)
            } else {
                (a + u * (len * 0.06), a + u * (len * 0.94), n * th)
            };
            let quad = vec![h0 - half, h1 - half, h1 + half, h0 + half];
            p.add(Shape::convex_polygon(quad, wood, Stroke::new(1.5, Color32::from_rgb(40, 26, 16))));
            if !w.open {
                p.circle_filled(a + u * (len * 0.8) + n * (th * 0.2), (th * 0.5).max(1.5), Color32::from_rgb(232, 196, 92));
            }
            if dm && w.locked {
                let c = a + u * (len * 0.5);
                p.circle_filled(c, (s * 0.13).max(4.0), Color32::from_rgb(196, 48, 48));
                p.text(c, egui::Align2::CENTER_CENTER, "L", egui::FontId::proportional((s * 0.18).max(8.0)), Color32::WHITE);
            }
            if dm && secret {
                p.extend(Shape::dashed_line(&[a - n * th * 2.0, b - n * th * 2.0], Stroke::new(1.5, Color32::from_rgb(214, 120, 255)), 4.0, 3.0));
            }
        }
    }
}
