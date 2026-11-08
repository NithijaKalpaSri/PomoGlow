//! Clock styles shared by the Timer page, the Clock gallery and the transparent overlay.

use crate::kit::*;
use egui::{pos2, vec2, Align2, Color32, Painter, Pos2, Rect, Shape, Stroke};
use std::f32::consts::PI;

pub const STYLE_NAMES: [&str; 5] = ["Ring", "Digital", "Flip", "Analog", "Minimal"];

pub struct ClockData {
    pub time: String,
    pub ratio: f32,
    pub accent: Color32,
    pub label: String,
    pub status: String,
    pub tint: Color32,
    pub show_label: bool,
    pub show_progress: bool,
}

fn big_text(p: &Painter, pos: Pos2, s: &str, size: f32, col: Color32, ghost: bool, op: f32, tint: Color32) {
    if ghost {
        let a = (op * 255.0) as u8;
        for (dx, dy, al) in [(3.0, 4.0, 0.45), (-1.5, 2.0, 0.3), (2.0, -1.5, 0.25)] {
            p.text(pos + vec2(dx, dy), Align2::CENTER_CENTER, s, font(size, true), Color32::from_rgba_unmultiplied(40, 40, 46, (a as f32 * al * 2.2).min(255.0) as u8));
        }
        p.text(pos, Align2::CENTER_CENTER, s, font(size, true), alpha(tint, a));
    } else {
        text(p, pos, Align2::CENTER_CENTER, s, size, col, true);
    }
}

fn small_text(p: &Painter, pos: Pos2, s: &str, size: f32, col: Color32, ghost: bool, op: f32, tint: Color32) {
    if ghost {
        let a = (op * 230.0) as u8;
        p.text(pos + vec2(1.5, 2.0), Align2::CENTER_CENTER, s, font(size, true), Color32::from_rgba_unmultiplied(40, 40, 46, a));
        p.text(pos, Align2::CENTER_CENTER, s, font(size, true), alpha(tint, a));
    } else {
        text(p, pos, Align2::CENTER_CENTER, s, size, col, true);
    }
}

fn track(ghost: bool, op: f32) -> Color32 {
    if ghost { Color32::from_white_alpha((op * 55.0) as u8) } else { alpha(INK2, 40) }
}

pub fn draw_clock(p: &Painter, r: Rect, style: u8, d: &ClockData, ghost: bool, op: f32) {
    let c = r.center();
    let s = r.height().min(r.width());
    let accent = if ghost { alpha(d.accent, (op * 255.0) as u8) } else { d.accent };
    match style {
        0 => {
            let rad = s * 0.4;
            let w = (s * 0.055).max(5.0);
            if !ghost {
                p.circle_filled(c, rad - 4.0, dm(Color32::from_white_alpha(120)));
            }
            p.circle_stroke(c, rad, Stroke::new(w, track(ghost, op)));
            if d.ratio > 0.002 {
                let pts = arc_points(c, rad, -PI * 0.5, -PI * 0.5 + 2.0 * PI * d.ratio.min(1.0));
                if !ghost {
                    p.add(Shape::line(pts.clone(), Stroke::new(w + 8.0, alpha(d.accent, 40))));
                } else {
                    p.add(Shape::line(pts.clone(), Stroke::new(w + 3.0, Color32::from_rgb(40, 40, 46))));
                }
                p.add(Shape::line(pts.clone(), Stroke::new(w, accent)));
                p.circle_filled(pts[0], w * 0.5, accent);
                p.circle_filled(*pts.last().unwrap(), w * 0.5, accent);
            }
            if d.show_label {
                small_text(p, c + vec2(0.0, -rad * 0.46), &d.label, (s * 0.04).max(11.0), if ghost { Color32::WHITE } else { INK2 }, ghost, op, d.tint);
            }
            big_text(p, c + vec2(0.0, -s * 0.01), &d.time, s * 0.21, INK, ghost, op, d.tint);
            if !ghost {
                small_text(p, c + vec2(0.0, rad * 0.46), &d.status, 14.0, INK2, false, op, d.tint);
            }
        }
        1 => {
            let size = (r.height() * 0.55).min(r.width() / 2.8);
            if d.show_label {
                small_text(p, c + vec2(0.0, -size * 0.78), &d.label, (size * 0.14).max(11.0), if ghost { Color32::WHITE } else { INK2 }, ghost, op, d.tint);
            }
            big_text(p, c, &d.time, size, INK, ghost, op, d.tint);
            if !d.show_progress {
                return;
            }
            let bw = size * 2.3;
            let bar = Rect::from_center_size(c + vec2(0.0, size * 0.72), vec2(bw, (size * 0.07).max(6.0)));
            p.rect_filled(bar, radius(bar.height() / 2.0), track(ghost, op));
            let w = (bw * d.ratio.clamp(0.0, 1.0)).max(bar.height());
            p.rect_filled(Rect::from_min_size(bar.min, vec2(w, bar.height())), radius(bar.height() / 2.0), accent);
        }
        2 => {
            let digits: Vec<char> = d.time.chars().filter(|c| c.is_ascii_digit()).collect();
            let cw = (r.height() * 0.5).min(r.width() / 5.4);
            let ch = cw * 1.35;
            let gap = cw * 0.1;
            let total = cw * 4.0 + gap * 5.0 + cw * 0.4;
            let x0 = c.x - total / 2.0;
            if d.show_label {
                small_text(p, pos2(c.x, c.y - ch * 0.5 - cw * 0.3), &d.label, (cw * 0.2).max(11.0), if ghost { Color32::WHITE } else { INK2 }, ghost, op, d.tint);
            }
            for (i, ch_d) in digits.iter().enumerate() {
                let extra = if i >= 2 { cw * 0.4 + gap } else { 0.0 };
                let x = x0 + i as f32 * (cw + gap) + extra;
                let card = Rect::from_min_size(pos2(x, c.y - ch / 2.0 + cw * 0.15), vec2(cw, ch));
                let fill = if ghost { Color32::from_rgb(44, 44, 52) } else { Color32::from_rgb(64, 50, 48) };
                p.rect_filled(card.translate(vec2(0.0, 4.0)), radius(cw * 0.14), Color32::from_black_alpha(if ghost { 0 } else { 40 }));
                p.rect_filled(card, radius(cw * 0.14), fill);
                if ghost {
                    p.rect_stroke(card, radius(cw * 0.14), Stroke::new(1.5, Color32::from_white_alpha((op * 70.0) as u8)), egui::StrokeKind::Inside);
                }
                let col = Color32::from_rgb(253, 246, 236);
                p.text(card.center(), Align2::CENTER_CENTER, ch_d.to_string(), font(cw * 1.05, true), if ghost { alpha(d.tint, (op * 255.0) as u8) } else { col });
                p.line_segment([pos2(card.left(), card.center().y), pos2(card.right(), card.center().y)], Stroke::new(2.0, Color32::from_rgb(20, 20, 24)));
            }
            let cx = x0 + 2.0 * (cw + gap) + cw * 0.2 - gap * 0.2 + gap * 0.5;
            for dy in [-0.18, 0.18] {
                p.circle_filled(pos2(cx, c.y + cw * 0.15 + dy * ch), cw * 0.06, if ghost { Color32::from_white_alpha((op * 230.0) as u8) } else { INK });
            }
            if !d.show_progress {
                return;
            }
            let bar = Rect::from_center_size(pos2(c.x, c.y + ch / 2.0 + cw * 0.45), vec2(total, 6.0));
            p.rect_filled(bar, radius(3.0), track(ghost, op));
            p.rect_filled(Rect::from_min_size(bar.min, vec2((total * d.ratio.clamp(0.0, 1.0)).max(6.0), 6.0)), radius(3.0), accent);
        }
        3 => {
            let rad = s * 0.4;
            if !ghost {
                p.circle_filled(c, rad, dm(Color32::from_white_alpha(150)));
            } else {
                p.circle_filled(c, rad, Color32::from_rgb(24, 24, 30));
            }
            p.circle_stroke(c, rad, Stroke::new(3.0, if ghost { Color32::from_white_alpha((op * 140.0) as u8) } else { alpha(INK2, 90) }));
            // remaining sector
            let sweep = 2.0 * PI * d.ratio.clamp(0.0, 1.0);
            if sweep > 0.01 {
                let mut pts = vec![c];
                pts.extend(arc_points(c, rad - 4.0, -PI * 0.5, -PI * 0.5 + sweep));
                p.add(Shape::convex_polygon(pts, alpha(d.accent, if ghost { (op * 120.0) as u8 } else { 90 }), Stroke::NONE));
            }
            for i in 0..12 {
                let a = i as f32 * PI / 6.0;
                let dir = vec2(a.cos(), a.sin());
                let long = i % 3 == 0;
                let col = if ghost { Color32::from_white_alpha((op * 220.0) as u8) } else { alpha(INK, 160) };
                p.line_segment([c + dir * rad * if long { 0.82 } else { 0.9 }, c + dir * rad * 0.97], Stroke::new(if long { 3.5 } else { 2.0 }, col));
            }
            let a = -PI * 0.5 + sweep;
            let tip = c + vec2(a.cos(), a.sin()) * rad * 0.82;
            p.line_segment([c, tip], Stroke::new(4.0, accent));
            p.circle_filled(c, 7.0, accent);
            small_text(p, c + vec2(0.0, rad * 0.5), &d.time, (s * 0.075).max(13.0), INK, ghost, op, d.tint);
        }
        _ => {
            let size = (r.height() * 0.62).min(r.width() / 2.9);
            big_text(p, c, &d.time, size, INK, ghost, op, d.tint);
            if d.show_label {
                small_text(p, c + vec2(0.0, size * 0.66), &d.label, (size * 0.1).max(11.0), if ghost { Color32::WHITE } else { INK2 }, ghost, op, d.tint);
            }
        }
    }
}
