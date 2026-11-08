//! Painting helpers: pastel glass cards, icons, buttons and the customizable avatar.

use crate::store::*;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontFamily, FontId, Id, Painter, Pos2,
    Rect, Sense, Shape, Stroke, StrokeKind, Ui,
};
use std::f32::consts::PI;

static DARK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn set_dark(b: bool) {
    DARK.store(b, std::sync::atomic::Ordering::Relaxed);
}
pub fn is_dark() -> bool {
    DARK.load(std::sync::atomic::Ordering::Relaxed)
}
fn lum(c: &[u8; 4]) -> f32 {
    (0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32) / 255.0
}
/// Dark-mode mapping for surfaces: light pastels become deep tones of the same hue.
pub fn dm(c: Color32) -> Color32 {
    if !is_dark() {
        return c;
    }
    let u = c.to_srgba_unmultiplied();
    let key = (u[0], u[1], u[2]);
    // Formal dark palette: muted slate / teal / indigo / bronze segments.
    let fixed: Option<[u8; 3]> = match key {
        (214, 232, 216) => Some([30, 52, 56]),   // sage   -> deep teal slate
        (252, 216, 202) => Some([58, 42, 48]),   // peach  -> muted burgundy
        (253, 236, 198) => Some([58, 50, 34]),   // butter -> bronze
        (210, 226, 242) => Some([30, 46, 72]),   // sky    -> navy steel
        (228, 216, 242) => Some([46, 40, 74]),   // lav    -> indigo
        (253, 246, 236) | (255, 250, 244) | (255, 255, 255) => Some([28, 33, 43]), // cream/white -> slate
        _ => None,
    };
    if let Some(f) = fixed {
        return Color32::from_rgba_unmultiplied(f[0], f[1], f[2], u[3]);
    }
    let l = lum(&u);
    if l > 0.62 {
        let base = [20.0, 25.0, 36.0];
        let g = |i: usize| (base[i] + u[i] as f32 * 0.09) as u8;
        Color32::from_rgba_unmultiplied(g(0), g(1), g(2), u[3])
    } else if l < 0.5 {
        let kk = if l < 0.3 { 0.9 } else { 0.62 };
        let t = [232.0, 236.0, 244.0];
        let g = |i: usize| (u[i] as f32 + (t[i] - u[i] as f32) * kk) as u8;
        Color32::from_rgba_unmultiplied(g(0), g(1), g(2), u[3])
    } else {
        c
    }
}
/// Dark-mode mapping for text/icons: dark ink becomes light, bright colors stay.
pub fn dm_text(c: Color32) -> Color32 {
    if !is_dark() {
        return c;
    }
    let u = c.to_srgba_unmultiplied();
    if lum(&u) < 0.5 { dm(c) } else { c }
}

pub fn visuals() -> egui::Visuals {
    let mut v = if is_dark() { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = if is_dark() { Color32::from_rgb(13, 17, 23) } else { CREAM };
    v.extreme_bg_color = if is_dark() { Color32::from_rgb(22, 27, 36) } else { Color32::from_rgb(255, 252, 247) };
    v.override_text_color = Some(dm_text(INK));
    v.selection.bg_fill = CORAL;
    let base = if is_dark() { Color32::from_rgb(38, 45, 58) } else { Color32::from_rgb(244, 232, 218) };
    v.widgets.inactive.bg_fill = base;
    v.widgets.inactive.weak_bg_fill = base;
    v.widgets.hovered.bg_fill = dm(PEACH);
    v.widgets.active.bg_fill = CORAL;
    v
}

pub const CREAM: Color32 = Color32::from_rgb(253, 246, 236);
pub const INK: Color32 = Color32::from_rgb(70, 50, 44);
pub const INK2: Color32 = Color32::from_rgb(132, 110, 98);
pub const CORAL: Color32 = Color32::from_rgb(234, 120, 88);
pub const SAGE: Color32 = Color32::from_rgb(214, 232, 216);
pub const PEACH: Color32 = Color32::from_rgb(252, 216, 202);
pub const BUTTER: Color32 = Color32::from_rgb(253, 236, 198);
pub const SKY: Color32 = Color32::from_rgb(210, 226, 242);
pub const LAVENDER: Color32 = Color32::from_rgb(228, 216, 242);

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub fn font(size: f32, bold: bool) -> FontId {
    if bold {
        FontId::new(size, FontFamily::Name("bold".into()))
    } else {
        FontId::proportional(size)
    }
}

pub fn text(p: &Painter, pos: Pos2, a: Align2, s: &str, size: f32, color: Color32, bold: bool) -> Rect {
    p.text(pos, a, s, font(size, bold), dm_text(color))
}

pub fn radius(r: f32) -> CornerRadius {
    CornerRadius::same(r.clamp(0.0, 255.0) as u8)
}

/// Frosted glass card with soft shadow and a specular rim.
pub fn glass(p: &Painter, r: Rect, tint: Color32, rad: f32) {
    for i in (1..=7).rev() {
        let e = i as f32 * 1.6;
        p.rect_filled(r.expand(e).translate(vec2(0.0, 5.0)), radius(rad + e), alpha(Color32::from_rgb(120, 80, 50), 5));
    }
    let tint = dm(tint);
    p.rect_filled(r, radius(rad), alpha(tint, if is_dark() { 225 } else { 215 }));
    // inner sheen on the upper half
    let top = Rect::from_min_max(r.min + vec2(2.0, 2.0), pos2(r.max.x - 2.0, r.center().y));
    p.rect_filled(top, CornerRadius { nw: rad as u8, ne: rad as u8, sw: 0, se: 0 }, Color32::from_white_alpha(if is_dark() { 6 } else { 40 }));
    p.rect_stroke(r, radius(rad), Stroke::new(1.5, Color32::from_white_alpha(if is_dark() { 30 } else { 190 })), StrokeKind::Inside);
}

/// Soft ambient blob to give the glass something to refract.
pub fn blob(p: &Painter, c: Pos2, r: f32, color: Color32) {
    for i in 0..14 {
        let t = i as f32 / 14.0;
        p.circle_filled(c, r * (1.0 - t * 0.85), alpha(color, if is_dark() { 5 } else { 7 }));
    }
}

pub fn pill_button(ui: &mut Ui, id: &str, rect: Rect, label: &str, fill: Color32, fg: Color32, size: f32) -> bool {
    let fill = dm(fill);
    let resp = ui.interact(rect, Id::new(id), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let f = if resp.is_pointer_button_down_on() { mix(fill, Color32::BLACK, 0.12) } else if resp.hovered() { mix(fill, Color32::WHITE, 0.18) } else { fill };
    let p = ui.painter();
    p.rect_filled(rect.translate(vec2(0.0, 3.0)), radius(rect.height() / 2.0), alpha(mix(fill, Color32::BLACK, 0.4), 60));
    p.rect_filled(rect, radius(rect.height() / 2.0), f);
    let hl = Rect::from_min_max(rect.min + vec2(6.0, 2.0), pos2(rect.max.x - 6.0, rect.center().y - 2.0));
    p.rect_filled(hl, radius(rect.height() / 4.0), Color32::from_white_alpha(46));
    text(p, rect.center(), Align2::CENTER_CENTER, label, size, fg, true);
    resp.clicked()
}

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Play,
    Pause,
    Reset,
    Skip,
    Dashboard,
    Timer,
    User,
    Gear,
    Plus,
    Minus,
    Flame,
    Note,
    Heart,
    Clock,
    Pin,
    Moon,
    Sun,
}

pub fn icon(p: &Painter, kind: Icon, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new((s * 0.14).max(1.6), col);
    match kind {
        Icon::Play => {
            p.add(Shape::convex_polygon(
                vec![c + vec2(-s * 0.28, -s * 0.4), c + vec2(-s * 0.28, s * 0.4), c + vec2(s * 0.42, 0.0)],
                col,
                Stroke::new(s * 0.1, col),
            ));
        }
        Icon::Pause => {
            for dx in [-0.22, 0.22] {
                let r = Rect::from_center_size(c + vec2(s * dx, 0.0), vec2(s * 0.22, s * 0.8));
                p.rect_filled(r, radius(s * 0.08), col);
            }
        }
        Icon::Reset => {
            let pts: Vec<Pos2> = (0..=22).map(|i| {
                let a = -PI * 0.2 + (i as f32 / 22.0) * PI * 1.55;
                c + vec2(a.cos(), a.sin()) * s * 0.4
            }).collect();
            let end = *pts.last().unwrap();
            p.add(Shape::line(pts, st));
            p.add(Shape::convex_polygon(vec![end + vec2(-s * 0.2, -s * 0.12), end + vec2(s * 0.2, -s * 0.12), end + vec2(0.0, s * 0.22)], col, Stroke::NONE));
        }
        Icon::Skip => {
            p.add(Shape::convex_polygon(vec![c + vec2(-s * 0.35, -s * 0.38), c + vec2(-s * 0.35, s * 0.38), c + vec2(s * 0.25, 0.0)], col, Stroke::NONE));
            p.rect_filled(Rect::from_center_size(c + vec2(s * 0.36, 0.0), vec2(s * 0.14, s * 0.76)), radius(2.0), col);
        }
        Icon::Dashboard => {
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                p.rect_filled(Rect::from_center_size(c + vec2(dx, dy) * s * 0.23, vec2(s * 0.38, s * 0.38)), radius(s * 0.1), col);
            }
        }
        Icon::Timer => {
            p.circle_stroke(c + vec2(0.0, s * 0.05), s * 0.4, st);
            p.line_segment([c + vec2(0.0, s * 0.05), c + vec2(0.0, -s * 0.18)], st);
            p.line_segment([c + vec2(0.0, s * 0.05), c + vec2(s * 0.15, s * 0.14)], st);
            p.line_segment([c + vec2(-s * 0.1, -s * 0.46), c + vec2(s * 0.1, -s * 0.46)], st);
        }
        Icon::User => {
            p.circle_filled(c + vec2(0.0, -s * 0.15), s * 0.2, col);
            let pts: Vec<Pos2> = (0..=16).map(|i| {
                let a = PI + (i as f32 / 16.0) * PI;
                c + vec2(0.0, s * 0.5) + vec2(a.cos() * s * 0.38, a.sin() * s * 0.4)
            }).collect();
            p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
        }
        Icon::Gear => {
            for i in 0..8 {
                let a = i as f32 * PI / 4.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * s * 0.34, c + d * s * 0.48], Stroke::new(s * 0.16, col));
            }
            p.circle_stroke(c, s * 0.3, Stroke::new(s * 0.14, col));
        }
        Icon::Plus | Icon::Minus => {
            p.line_segment([c + vec2(-s * 0.3, 0.0), c + vec2(s * 0.3, 0.0)], st);
            if kind == Icon::Plus {
                p.line_segment([c + vec2(0.0, -s * 0.3), c + vec2(0.0, s * 0.3)], st);
            }
        }
        Icon::Flame => {
            let pts: Vec<Pos2> = (0..=20).map(|i| {
                let t = i as f32 / 20.0 * 2.0 * PI;
                let x = t.sin() * (0.32 - 0.16 * t.cos().max(0.0)) ;
                let y = -t.cos() * 0.46 + 0.04;
                c + vec2(x, y) * s * 1.1
            }).collect();
            p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
            p.circle_filled(c + vec2(0.0, s * 0.22), s * 0.13, Color32::from_white_alpha(170));
        }
        Icon::Note => {
            p.circle_filled(c + vec2(-s * 0.12, s * 0.28), s * 0.17, col);
            p.line_segment([c + vec2(s * 0.04, s * 0.26), c + vec2(s * 0.04, -s * 0.4)], st);
            p.line_segment([c + vec2(s * 0.04, -s * 0.4), c + vec2(s * 0.34, -s * 0.22)], st);
        }
        Icon::Heart => {
            let pts: Vec<Pos2> = (0..=32).map(|i| {
                let t = i as f32 / 32.0 * 2.0 * PI;
                let x = 16.0 * t.sin().powi(3);
                let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
                c + vec2(x, y) * s * 0.028 + vec2(0.0, s * 0.02)
            }).collect();
            p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
        }
        Icon::Clock => {
            p.circle_stroke(c, s * 0.4, st);
            p.line_segment([c, c + vec2(0.0, -s * 0.24)], st);
            p.line_segment([c, c + vec2(s * 0.18, s * 0.08)], st);
        }
        Icon::Moon => {
            p.circle_filled(c, s * 0.38, col);
        }
        Icon::Sun => {
            p.circle_filled(c, s * 0.22, col);
            for i in 0..8 {
                let a = i as f32 * PI / 4.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * s * 0.34, c + d * s * 0.5], st);
            }
        }
        Icon::Pin => {
            p.circle_filled(c + vec2(0.0, -s * 0.12), s * 0.26, col);
            p.line_segment([c + vec2(0.0, s * 0.1), c + vec2(0.0, s * 0.46)], st);
        }
    };
}

pub fn icon_button(ui: &mut Ui, id: &str, c: Pos2, r: f32, kind: Icon, fill: Color32, fg: Color32) -> bool {
    let (fill, fg) = (dm(fill), dm_text(fg));
    let rect = Rect::from_center_size(c, vec2(r * 2.0, r * 2.0));
    let resp = ui.interact(rect, Id::new(id), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let f = if resp.is_pointer_button_down_on() { mix(fill, Color32::BLACK, 0.1) } else if resp.hovered() { mix(fill, Color32::WHITE, 0.25) } else { fill };
    let p = ui.painter();
    p.circle_filled(c + vec2(0.0, 3.0), r, alpha(mix(fill, Color32::BLACK, 0.4), 55));
    p.circle_filled(c, r, f);
    p.circle_stroke(c, r - 0.8, Stroke::new(1.5, Color32::from_white_alpha(150)));
    icon(p, kind, c, r * 1.0, fg);
    if kind == Icon::Moon {
        p.circle_filled(c + vec2(r * 0.22, -r * 0.16), r * 0.34, f);
    }
    resp.clicked()
}

/// Rounded square icon tile used on stat cards.
pub fn tile(p: &Painter, c: Pos2, size: f32, fill: Color32, kind: Icon) {
    let r = Rect::from_center_size(c, vec2(size, size));
    p.rect_filled(r.translate(vec2(0.0, 3.0)), radius(size * 0.3), alpha(mix(fill, Color32::BLACK, 0.45), 70));
    p.rect_filled(r, radius(size * 0.3), fill);
    p.rect_stroke(r, radius(size * 0.3), Stroke::new(1.5, Color32::from_white_alpha(120)), StrokeKind::Inside);
    icon(p, kind, c, size * 0.56, Color32::WHITE);
}

pub fn arc_points(c: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let n = (((a1 - a0).abs() * 28.0) as usize).max(4);
    (0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        c + vec2(a.cos(), a.sin()) * r
    }).collect()
}

/// Draws the customizable avatar (hair, skin, headphones, background).
pub fn avatar(p: &Painter, c: Pos2, r: f32, base: &Profile) {
    let mut pf = base.clone();
    if base.gender == 1 {
        (pf.skin, pf.hair, pf.phones, pf.bg, pf.style) = (1, 1, 3, 3, 2);
    } else {
        (pf.skin, pf.hair, pf.phones, pf.bg, pf.style) = (1, 0, 0, 0, 0);
    }
    let pf = &pf;
    let skin = rgb(SKINS[pf.skin % 6]);
    let hair = rgb(HAIRS[pf.hair % 6]);
    let phones = rgb(PHONES[pf.phones % 6]);
    let bg = rgb(BGS[pf.bg % 6]);
    let u = |x: f32, y: f32| c + vec2(x * r, y * r);

    p.circle_filled(c + vec2(0.0, r * 0.05), r + 1.0, alpha(Color32::from_rgb(120, 80, 50), 28));
    p.circle_filled(c, r, bg);
    // everything below is clipped to the badge circle
    let clipped = p.with_clip_rect(Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)));
    let p = &clipped;

    // body / hoodie
    let hoodie = rgb(THEMES[pf.theme % 4].2);
    p.circle_filled(u(0.0, 1.12), r * 0.72, mix(hoodie, Color32::WHITE, 0.45));
    // hair behind head
    match pf.style % 3 {
        0 => {
            p.circle_filled(u(0.0, -0.03), r * 0.56, hair);
            p.circle_filled(u(0.0, -0.66), r * 0.22, hair);
        }
        1 => {
            p.circle_filled(u(0.0, -0.03), r * 0.56, hair);
            p.rect_filled(Rect::from_min_max(u(-0.56, -0.03), u(0.56, 0.78)), radius(r * 0.22), hair);
        }
        _ => {}
    }
    // face
    p.circle_filled(u(0.0, 0.06), r * 0.44, skin);
    // fringe
    let rr = if pf.style % 3 == 2 { 0.47 } else { 0.5 };
    let fringe: Vec<Pos2> = arc_points(u(0.0, 0.0), r * rr, PI, 2.0 * PI);
    p.add(Shape::convex_polygon(fringe, hair, Stroke::NONE));
    // eyes, cheeks, smile
    for dx in [-0.17, 0.17] {
        p.circle_filled(u(dx, 0.12), r * 0.055, Color32::from_rgb(60, 40, 36));
        p.circle_filled(u(dx * 1.5, 0.27), r * 0.07, Color32::from_rgba_unmultiplied(240, 120, 120, 90));
    }
    p.add(Shape::line(arc_points(u(0.0, 0.2), r * 0.1, 0.25 * PI, 0.75 * PI), Stroke::new(r * 0.035, Color32::from_rgb(150, 70, 60))));
    // headphones
    p.add(Shape::line(arc_points(u(0.0, 0.04), r * 0.56, PI * 1.07, PI * 1.93), Stroke::new(r * 0.075, phones)));
    for dx in [-0.57, 0.57] {
        p.rect_filled(Rect::from_center_size(u(dx, 0.1), vec2(r * 0.2, r * 0.36)), radius(r * 0.1), phones);
        p.rect_filled(Rect::from_center_size(u(dx, 0.1), vec2(r * 0.1, r * 0.22)), radius(r * 0.05), Color32::from_white_alpha(80));
    }
    p.circle_stroke(c, r - 1.0, Stroke::new(2.0, Color32::from_white_alpha(200)));
}

/// Neutral "no one" placeholder avatar.
pub fn avatar_empty(p: &Painter, c: Pos2, r: f32) {
    p.circle_filled(c + vec2(0.0, r * 0.05), r + 1.0, alpha(Color32::from_rgb(120, 80, 50), 28));
    p.circle_filled(c, r, Color32::from_rgb(234, 226, 216));
    let p = &p.with_clip_rect(Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)));
    let col = Color32::from_rgb(196, 184, 172);
    p.circle_filled(c + vec2(0.0, -r * 0.14), r * 0.32, col);
    p.circle_filled(c + vec2(0.0, r * 0.95), r * 0.58, col);
    p.circle_stroke(c, r - 1.0, Stroke::new(2.0, Color32::from_white_alpha(200)));
}

/// Circular crop of a user picture.
pub fn avatar_photo(p: &Painter, c: Pos2, r: f32, tex: egui::TextureId) {
    p.circle_filled(c + vec2(0.0, r * 0.05), r + 1.0, alpha(Color32::from_rgb(120, 80, 50), 28));
    let mut m = egui::Mesh::with_texture(tex);
    let n = 64u32;
    m.vertices.push(egui::epaint::Vertex { pos: c, uv: pos2(0.5, 0.5), color: Color32::WHITE });
    for i in 0..=n {
        let a = i as f32 / n as f32 * 2.0 * PI;
        let (s, co) = a.sin_cos();
        m.vertices.push(egui::epaint::Vertex { pos: c + vec2(co, s) * r, uv: pos2(0.5 + 0.5 * co, 0.5 + 0.5 * s), color: Color32::WHITE });
    }
    for i in 0..n {
        m.indices.extend([0, i + 1, i + 2]);
    }
    p.add(Shape::mesh(m));
    p.circle_stroke(c, r - 1.0, Stroke::new(2.0, Color32::from_white_alpha(200)));
}

/// A wide line slider: thin track, filled part, round knob. Returns true when changed.
pub fn line_slider(ui: &mut Ui, id: &str, rect: Rect, value: &mut f32, min: f32, max: f32, accent: Color32) -> bool {
    let track = Rect::from_center_size(rect.center(), vec2(rect.width(), 8.0));
    let resp = ui.interact(rect.expand2(vec2(0.0, 10.0)), Id::new(id), Sense::click_and_drag()).on_hover_cursor(CursorIcon::PointingHand);
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let t = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let v = min + t * (max - min);
            if (v - *value).abs() > f32::EPSILON {
                *value = v;
                changed = true;
            }
        }
    }
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let p = ui.painter();
    p.rect_filled(track, radius(4.0), alpha(INK2, 55));
    let filled = Rect::from_min_max(track.min, pos2(track.left() + track.width() * t, track.bottom()));
    p.rect_filled(filled, radius(4.0), accent);
    let k = pos2(track.left() + track.width() * t, track.center().y);
    p.circle_filled(k + vec2(0.0, 2.0), 13.0, Color32::from_black_alpha(40));
    p.circle_filled(k, if resp.hovered() || resp.dragged() { 14.0 } else { 12.0 }, Color32::WHITE);
    p.circle_stroke(k, 12.0, Stroke::new(3.0, accent));
    changed
}
