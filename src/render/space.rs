//! Deep space: the star field and nebula behind everything, the sun, and
//! the orbit lines of the planets and moons.

use super::camera::Camera;
use crate::engine::gfx::{Frame, bayer, mix, rgb, shade};
use crate::engine::noise::{fbm3, hash, noise3};
use crate::engine::rng::Rng;
use crate::engine::ui::OUTLINE;
use crate::engine::v2::V2;
use crate::sim::worldgen::hsv;
use crate::sim::system::{BodyState, System};
use std::f64::consts::{FRAC_PI_2, TAU};

pub const SKY: usize = 1024;
pub const SPACE: u32 = rgb(8, 8, 24);

/// The sky as a big square texture, sampled with the camera's rotation.
pub struct Sky {
    pub px: Vec<u32>,
    pub twinkle: Vec<(u16, u16, u32)>,
}

pub fn build_sky(seed: u64) -> Sky {
    let mut rng = Rng(seed ^ 0x5EED_5CA1);
    rng.next();
    let h1 = rng.range(0.0, 360.0);
    let h2 = h1 + rng.range(90.0, 200.0);
    let ca = [hsv(h1, 0.7, 0.16), hsv(h1, 0.65, 0.26), hsv(h1 + 15.0, 0.5, 0.38)];
    let cb = [hsv(h2, 0.7, 0.15), hsv(h2, 0.6, 0.24), hsv(h2 + 10.0, 0.45, 0.36)];
    let s = rng.next();
    let mut px = vec![SPACE; SKY * SKY];
    let half = SKY / 2;
    // nebula at half resolution, posterized into dithered bands
    for y in 0..half {
        for x in 0..half {
            let (fx, fy) = (x as f32 / half as f32 * 3.0, y as f32 / half as f32 * 3.0);
            let w = fbm3(fx * 0.8, fy * 0.8, 1.0, 3, 0.5, s) - 0.5;
            let n = fbm3(fx + w * 1.6, fy - w, 2.0, 5, 0.55, s ^ 1);
            let m = fbm3(fx * 1.3 - w, fy * 1.3 + w, 4.0, 4, 0.5, s ^ 2);
            let band = |v: f32, lo: f32| ((v - lo) / 0.05).max(0.0);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (px_x, px_y) = (x * 2 + dx, y * 2 + dy);
                let th = bayer(px_x as i32, px_y as i32);
                let mut c = SPACE;
                let a = band(n, 0.5);
                if a > th {
                    c = ca[((a - th) as usize).min(2)];
                }
                let b = band(m, 0.56);
                if b > th + 0.3 {
                    c = mix(c, cb[((b - th) as usize).min(2)], 0.7);
                }
                px[px_y * SKY + px_x] = c;
            }
        }
    }
    let mut twinkle = vec![];
    for _ in 0..2600 {
        let (x, y) = (rng.int(SKY as i32) as usize, rng.int(SKY as i32) as usize);
        let k = rng.f();
        let hue = [rgb(255, 255, 255), rgb(200, 216, 255), rgb(255, 232, 200), rgb(255, 200, 216)][rng.int(4) as usize];
        let c = if k < 0.75 { mix(px[y * SKY + x], hue, 0.4) } else { hue };
        px[y * SKY + x] = c;
        if k > 0.985 {
            twinkle.push((x as u16, y as u16, hue));
        }
    }
    let _ = (noise3, hash);
    Sky { px, twinkle }
}

pub fn draw_sky(f: &mut Frame, cam: &Camera, sky: &Sky, tick: u32) {
    let rot = cam.up - FRAC_PI_2;
    let (s, c) = (rot.sin() as f32, rot.cos() as f32);
    let (hw, hh) = (f.w as f32 * 0.5, f.h as f32 * 0.5);
    let m = SKY as f32 * 0.5;
    for y in 0..f.h {
        let dy = y as f32 - hh;
        for x in 0..f.w {
            let dx = x as f32 - hw;
            let u = (dx * c - dy * s + m) as i32;
            let v = (dx * s + dy * c + m) as i32;
            let col = if u >= 0 && v >= 0 && (u as usize) < SKY && (v as usize) < SKY { sky.px[v as usize * SKY + u as usize] } else { SPACE };
            f.px[(y * f.w + x) as usize] = col;
        }
    }
    // a few big stars twinkle as plus signs
    for (k, &(u, v, col)) in sky.twinkle.iter().enumerate() {
        let phase = (tick as usize / 6 + k * 7) % 40;
        if phase > 8 {
            continue;
        }
        let (du, dv) = (u as f32 - m, v as f32 - m);
        let x = (du * c + dv * s + hw) as i32;
        let y = (-du * s + dv * c + hh) as i32;
        let arm = if phase < 3 || phase > 6 { 1 } else { 2 };
        for d in 1..=arm {
            f.set(x + d, y, col);
            f.set(x - d, y, col);
            f.set(x, y + d, col);
            f.set(x, y - d, col);
        }
        f.set(x, y, 0xFFFFFF);
    }
}

/// The sun: a boiling disc with a dithered corona. Small on screen, a glint.
pub fn draw_star(f: &mut Frame, cam: &Camera, sys: &System, pos: V2, tick: u32) {
    let b = &sys.bodies[0];
    let col = sys.star_color;
    let (cx, cy) = cam.to_screen(pos);
    let rp = b.radius / cam.mpp;
    let corona = rp * 2.6 + 6.0;
    if cx + corona < 0.0 || cy + corona < 0.0 || cx - corona > cam.w || cy - corona > cam.h {
        return;
    }
    let pal = [mix(col, 0xFFFFFF, 0.75), mix(col, 0xFFFFFF, 0.4), col, mix(col, rgb(248, 96, 32), 0.45), mix(col, rgb(200, 40, 40), 0.6)];
    if rp < 3.0 {
        let (x, y) = (cx as i32, cy as i32);
        let arm = 3 + ((tick / 8) % 2) as i32;
        for d in 1..=arm {
            let c = if d <= 1 { pal[0] } else { pal[2] };
            f.set(x + d, y, c);
            f.set(x - d, y, c);
            f.set(x, y + d, c);
            f.set(x, y - d, c);
        }
        f.fill_ellipse(x, y, 1, 1, pal[0]);
        return;
    }
    let x0 = (cx - corona).max(0.0) as i32;
    let x1 = (cx + corona).min(cam.w - 1.0) as i32;
    let y0 = (cy - corona).max(0.0) as i32;
    let y1 = (cy + corona).min(cam.h - 1.0) as i32;
    let t = tick as f32 * 0.02;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
            let r = dx.hypot(dy);
            if r > corona {
                continue;
            }
            let ang = dy.atan2(dx);
            if r <= rp {
                // granulation: noise in disc coordinates, posterized
                let u = (dx / rp) as f32;
                let v = (dy / rp) as f32;
                let n = noise3(u * 6.0 + t, v * 6.0, t * 0.7, 3) * 0.6 + noise3(u * 13.0, v * 13.0 - t, t, 4) * 0.4;
                let limb = (r / rp) as f32;
                let q = n * 1.2 - limb * limb * 0.9 + 0.35;
                let i = ((1.0 - q) * 3.0 + bayer(x, y) - 0.5).clamp(0.0, 4.0) as usize;
                f.px[(y * f.w + x) as usize] = pal[i];
            } else {
                // corona: flickering rays fading out in dithered rings
                let u = ((r - rp) / (corona - rp)) as f32;
                let ray = noise3((ang * 5.0) as f32, t * 0.5, 0.0, 9) * 0.5 + noise3((ang * 13.0) as f32, t, 1.0, 11) * 0.3;
                let a = (1.0 - u).powf(2.2) * (0.6 + ray);
                if a > bayer(x, y) {
                    let i = if a > 0.8 { 1 } else if a > 0.45 { 2 } else { 3 };
                    let c = f.px[(y * f.w + x) as usize];
                    f.px[(y * f.w + x) as usize] = if i == 3 { mix(c, pal[3], 0.6) } else { pal[i] };
                }
            }
        }
    }
}

/// Dotted orbit lines around each body's parent.
pub fn draw_orbits(f: &mut Frame, cam: &Camera, sys: &System, st: &[BodyState], highlight: Option<usize>) {
    for (i, b) in sys.bodies.iter().enumerate() {
        let Some(p) = b.parent else { continue };
        let rpx = b.orbit.a / cam.mpp;
        if rpx < 6.0 {
            continue;
        }
        // bright for the highlighted body, otherwise muted
        let base = if b.kind == crate::sim::system::Kind::Gas { b.look.bands[0] } else { b.look.pal[3] };
        let c = if Some(i) == highlight { mix(base, 0xFFFFFF, 0.4) } else { shade(mix(base, rgb(96, 112, 160), 0.6), 3, 4) };
        let n = ((rpx * 0.9) as usize).clamp(64, 900);
        let period = b.orbit.period();
        let mut last: Option<(f64, f64)> = None;
        for k in 0..=n {
            let t = k as f64 / n as f64 * period;
            let (rel, _) = b.orbit.state(t);
            let q = cam.to_screen(st[p].pos + rel);
            if let Some(l) = last {
                if k % 2 == 0 || Some(i) == highlight {
                    f.line_f(l.0, l.1, q.0, q.1, c);
                }
            }
            last = Some(q);
        }
    }
    let _ = (OUTLINE, TAU);
}
