//! Rockets drawn from their stages as polygons in the rocket's own frame,
//! rendered into a small offscreen buffer and outlined in navy, so they
//! look like sprites at any angle and zoom. Far away they become an icon.

use super::camera::Camera;
use super::local::Local;
use crate::engine::gfx::{Frame, mix, rgb, shade};
use crate::engine::ui::OUTLINE;
use crate::engine::v2::V2;
use crate::sim::vehicle::{StageKind, Vehicle};

const KEY: u32 = 1;

pub struct Look {
    pub legs: bool,
    pub chute: bool,
    pub throttle: f64,
    pub burning: bool,
    /// 0..1: how much air there is (flame shape).
    pub air: f64,
    pub tick: u32,
    /// Plasma glow from reentry, 0..1.
    pub heat: f64,
}

fn shaded_quad(l: &Local, f: &mut Frame, x0: f64, y0: f64, x1: f64, y1: f64, c: u32) {
    let w = x1 - x0;
    l.quad(f, x0, y0, x1, y1, c);
    l.quad(f, x0, y0, x0 + w * 0.22, y1, mix(c, 0xFFFFFF, 0.35));
    l.quad(f, x1 - w * 0.25, y0, x1, y1, shade(c, 3, 4));
}

/// Draw the vehicle body (no flame) in local frame `l` (origin at the
/// stack's center, y up along the nose).
fn body(f: &mut Frame, l: &Local, v: &Vehicle, look: &Look) {
    let h = v.height();
    let mut y = -h * 0.5;
    let n = v.stages.len();
    let s_px = l.scale();
    for (i, s) in v.stages.iter().enumerate() {
        let hw = s.width * 0.5;
        let top = y + s.height;
        let is_top = i + 1 == n;
        match s.kind {
            StageKind::Booster | StageKind::Upper => {
                let bell = if s.kind == StageKind::Booster { 1.6 } else { 1.2 };
                // engine bell
                if i == 0 {
                    l.poly(f, &[(-hw * 0.55, y + bell), (hw * 0.55, y + bell), (hw * 0.75, y), (-hw * 0.75, y)], rgb(72, 72, 88));
                    l.quad(f, -hw * 0.75, y, hw * 0.75, y + 0.3, rgb(40, 40, 56));
                }
                let y0 = y + if i == 0 { bell } else { 0.0 };
                let body_top = if is_top { top - s.width * 1.1 } else { top };
                shaded_quad(l, f, -hw, y0, hw, body_top, s.color);
                // trim bands and a stripe of windows/logo blocks
                let band = (s.height * 0.06).max(0.6);
                shaded_quad(l, f, -hw, body_top - band * 1.5, hw, body_top - band * 0.5, s.trim);
                shaded_quad(l, f, -hw, y0 + band * 0.8, hw, y0 + band * 1.8, s.trim);
                if s.height > 10.0 && s_px > 1.5 {
                    let mid = (y0 + body_top) * 0.5;
                    l.quad(f, -hw * 0.25, mid - s.height * 0.12, hw * 0.25, mid + s.height * 0.12, shade(s.trim, 7, 8));
                }
                if is_top {
                    // nose cone
                    l.poly(f, &[(-hw, body_top), (hw, body_top), (hw * 0.55, body_top + s.width * 0.7), (0.0, top), (-hw * 0.55, body_top + s.width * 0.7)], s.trim);
                    l.poly(f, &[(-hw, body_top), (-hw * 0.4, body_top), (-hw * 0.15, top - 0.3), (-hw * 0.55, body_top + s.width * 0.7)], mix(s.trim, 0xFFFFFF, 0.35));
                }
                if s.fins {
                    let fy = y0;
                    for side in [-1.0, 1.0] {
                        l.poly(f, &[(side * hw, fy + 4.0), (side * (hw + 2.2), fy - 0.2), (side * (hw + 2.2), fy - 1.2), (side * hw, fy + 0.5)], s.trim);
                    }
                }
                if s.legs {
                    for side in [-1.0, 1.0] {
                        if look.legs {
                            l.poly(f, &[(side * hw, y0 + 3.5), (side * (hw + 0.6), y0 + 3.5), (side * (hw + 3.2), y - 0.2), (side * (hw + 2.4), y - 0.2)], rgb(64, 64, 80));
                            l.quad(f, side * (hw + 2.0), y - 0.3, side * (hw + 3.6), y + 0.2, rgb(48, 48, 64));
                        } else {
                            l.quad(f, side * hw - 0.3 * side, y0 + 0.3, side * hw + 0.3 * side, y0 + 6.0, rgb(64, 64, 80));
                        }
                    }
                }
            }
            StageKind::Capsule => {
                l.quad(f, -hw, y, hw, y + 0.5, rgb(88, 56, 40));
                l.poly(f, &[(-hw, y + 0.5), (hw, y + 0.5), (hw * 0.4, top), (-hw * 0.4, top)], s.color);
                l.poly(f, &[(-hw, y + 0.5), (-hw * 0.45, y + 0.5), (-hw * 0.2, top), (-hw * 0.4, top)], mix(s.color, 0xFFFFFF, 0.4));
                l.quad(f, -hw * 0.2, y + 1.6, hw * 0.25, y + 2.4, rgb(64, 160, 232));
                if look.chute {
                    let ch = top + 9.0;
                    for k in [-1.0, -0.4, 0.4, 1.0] {
                        l.line(f, k * hw * 0.3, top, k * 7.0, ch, rgb(200, 200, 200));
                    }
                    l.poly(f, &[(-8.0, ch), (-6.0, ch + 3.0), (-2.5, ch + 4.2), (2.5, ch + 4.2), (6.0, ch + 3.0), (8.0, ch)], rgb(240, 96, 64));
                    l.poly(f, &[(-2.5, ch), (-2.5, ch + 4.2), (2.5, ch + 4.2), (2.5, ch)], rgb(248, 240, 232));
                }
            }
            StageKind::Satellite => {
                if is_top && i > 0 {
                    // inside its fairing
                    let fw = hw * 1.05;
                    shaded_quad(l, f, -fw, y, fw, top - s.width * 0.4, rgb(232, 232, 240));
                    l.poly(f, &[(-fw, top - s.width * 0.4), (fw, top - s.width * 0.4), (0.0, top + s.width * 0.5)], rgb(232, 232, 240));
                    l.poly(f, &[(-fw, top - s.width * 0.4), (-fw * 0.3, top - s.width * 0.4), (-fw * 0.1, top + s.width * 0.4)], rgb(248, 248, 255));
                    l.quad(f, -fw, y + s.height * 0.3, fw, y + s.height * 0.4, s.trim);
                } else {
                    // deployed: gold box, solar wings, dish
                    for side in [-1.0, 1.0] {
                        l.quad(f, side * hw, y + s.height * 0.45, side * (hw + 0.6), y + s.height * 0.55, rgb(160, 160, 176));
                        l.quad(f, side * (hw + 0.6), y + s.height * 0.3, side * (hw + 7.0), y + s.height * 0.7, rgb(48, 72, 168));
                        for k in 1..4 {
                            let x = side * (hw + 0.6 + k as f64 * 1.6);
                            l.line(f, x, y + s.height * 0.3, x, y + s.height * 0.7, rgb(120, 160, 232));
                        }
                    }
                    shaded_quad(l, f, -hw * 0.8, y, hw * 0.8, top - 1.5, s.color);
                    l.poly(f, &[(-1.6, top - 1.5), (1.6, top - 1.5), (0.8, top), (-0.8, top)], s.trim);
                }
            }
            StageKind::Lander => {
                for side in [-1.0, 1.0] {
                    l.poly(f, &[(side * hw * 0.6, y + 2.6), (side * hw * 0.8, y + 2.6), (side * (hw + 1.4), y - 0.1), (side * (hw + 0.9), y - 0.1)], rgb(96, 96, 112));
                    l.quad(f, side * (hw + 0.5), y - 0.3, side * (hw + 1.9), y + 0.1, rgb(64, 64, 80));
                }
                l.poly(f, &[(-0.9, y + 1.2), (0.9, y + 1.2), (1.3, y), (-1.3, y)], rgb(72, 72, 88));
                shaded_quad(l, f, -hw * 0.7, y + 1.2, hw * 0.7, top - 1.2, s.color);
                l.poly(f, &[(-hw * 0.7, top - 1.2), (hw * 0.7, top - 1.2), (hw * 0.4, top), (-hw * 0.4, top)], s.trim);
                l.quad(f, -0.5, top - 0.9, 0.6, top - 0.4, rgb(64, 160, 232));
            }
        }
        y = top;
    }
}

/// Flame below the engine: nested flickering cones, longer with throttle,
/// wide and pale in vacuum.
fn flame(f: &mut Frame, l: &Local, v: &Vehicle, look: &Look) {
    let Some(s) = v.active() else { return };
    if !look.burning || s.thrust <= 0.0 {
        return;
    }
    let base = -v.height() * 0.5;
    let fl = (look.tick as f64 * 1.7).sin() * 0.12 + ((look.tick * 7 % 5) as f64) * 0.04;
    let len = (4.0 + 12.0 * look.throttle) * (1.0 + fl) * (s.width / 4.0);
    let spread = 1.0 + (1.0 - look.air) * 1.6;
    let w = s.width * 0.38;
    let layers = [
        (1.0, 1.0, if look.air > 0.3 { rgb(232, 72, 40) } else { rgb(120, 104, 232) }),
        (0.78, 0.75, if look.air > 0.3 { rgb(248, 152, 32) } else { rgb(176, 168, 248) }),
        (0.5, 0.5, rgb(248, 232, 104)),
        (0.25, 0.3, rgb(255, 255, 240)),
    ];
    for (lk, wk, c) in layers {
        let ll = len * lk;
        let ww = w * wk * spread;
        l.poly(f, &[(-w * wk, base), (w * wk, base), (ww * 1.3, base - ll * 0.35), (0.0, base - ll), (-ww * 1.3, base - ll * 0.35)], c);
    }
}

/// Compose: flame first, then the body through an offscreen mask with an outline.
pub fn draw(f: &mut Frame, cam: &Camera, pos: V2, heading: f64, v: &Vehicle, look: &Look) {
    let l = Local::new(cam, pos, heading);
    let s = l.scale();
    let h = v.height();
    if h * s < 6.0 {
        icon(f, cam, pos, heading, look);
        return;
    }
    flame(f, &l, v, look);
    // offscreen buffer around the rocket
    let ext = h * 0.5 + 12.0 + if look.chute { 14.0 } else { 0.0 };
    let r = (ext * s).ceil() as i32 + 3;
    let (cx, cy) = l.pt(0.0, 0.0);
    if cx + r < 0 || cy + r < 0 || cx - r > f.w || cy - r > f.h {
        return;
    }
    let size = r * 2 + 1;
    if size > 2048 {
        return;
    }
    let mut buf = Frame::new(size, size);
    buf.clear(KEY);
    let mut lo = l;
    lo.ox -= (cx - r) as f64;
    lo.oy -= (cy - r) as f64;
    body(&mut buf, &lo, v, look);
    for y in 0..size {
        for x in 0..size {
            let c = buf.px[(y * size + x) as usize];
            let (sx, sy) = (cx - r + x, cy - r + y);
            if c != KEY {
                let mut c = c;
                if look.heat > 0.0 && (x + y + look.tick as i32) % 3 == 0 {
                    c = mix(c, rgb(248, 120, 48), look.heat as f32 * 0.6);
                }
                f.set(sx, sy, c);
            } else {
                let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    nx >= 0 && ny >= 0 && nx < size && ny < size && buf.px[(ny * size + nx) as usize] != KEY
                });
                if near {
                    f.set(sx, sy, OUTLINE);
                }
            }
        }
    }
}

/// Map icon: a small arrow pointing where the nose points.
pub fn icon(f: &mut Frame, cam: &Camera, pos: V2, heading: f64, look: &Look) {
    let (x, y) = cam.to_screen(pos);
    let a = cam.screen_angle(heading);
    let (dx, dy) = (a.cos(), a.sin());
    let (px, py) = (-dy, dx);
    let pt = |u: f64, v: f64| ((x + dx * u + px * v).round() as i32, (y + dy * u + py * v).round() as i32);
    let tri = [pt(6.0, 0.0), pt(-4.0, 4.0), pt(-2.0, 0.0), pt(-4.0, -4.0)];
    for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let t: Vec<(i32, i32)> = tri.iter().map(|&(a, b)| (a + ox, b + oy)).collect();
        f.fill_poly(&t, OUTLINE);
    }
    f.fill_poly(&tri, rgb(248, 248, 248));
    if look.burning {
        let (fx, fy) = pt(-6.0 - (look.tick % 3) as f64, 0.0);
        f.fill_ellipse(fx, fy, 1, 1, rgb(248, 160, 32));
    }
}
