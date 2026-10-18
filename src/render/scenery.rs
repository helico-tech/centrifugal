//! Things standing on the ground near the camera: trees by biome, the
//! launch complex, landing zones and a few cows. Only drawn when zoomed in.

use super::body::{BodyGfx, ground_index};
use super::camera::Camera;
use super::local::Local;
use crate::engine::gfx::{Frame, mix, rgb, shade};
use crate::engine::noise::hash;
use crate::engine::ui::OUTLINE;
use crate::engine::v2::V2;
use crate::sim::system::{Body, BodyState, Kind, SiteKind};
use std::f64::consts::TAU;

fn tree(f: &mut Frame, l: &Local, kind: u8, size: f64, leaf: u32, seed: f32) {
    let trunk = rgb(104, 64, 40);
    match kind {
        // round tree
        0 => {
            l.quad_outline(f, -0.5, 0.0, 0.5, size * 0.5, OUTLINE);
            l.quad(f, -0.5, 0.0, 0.5, size * 0.5, trunk);
            let r = size * 0.32;
            l.disc(f, 0.0, size * 0.62, r + 1.0 / l.scale(), OUTLINE);
            l.disc(f, 0.0, size * 0.62, r, leaf);
            l.disc(f, -r * 0.3, size * 0.7, r * 0.45, mix(leaf, 0xFFFFFF, 0.25));
        }
        // pine
        1 => {
            l.quad(f, -0.4, 0.0, 0.4, size * 0.3, trunk);
            for k in 0..3 {
                let y0 = size * (0.2 + k as f64 * 0.22);
                let w = size * (0.32 - k as f64 * 0.08);
                let pts = [(-w, y0), (w, y0), (0.0, y0 + size * 0.38)];
                l.poly_outline(f, &pts, OUTLINE);
                l.poly(f, &pts, if k == 1 { shade(leaf, 7, 8) } else { leaf });
            }
        }
        // palm
        2 => {
            let lean = if seed > 0.5 { 1.0 } else { -1.0 };
            l.poly(f, &[(-0.4, 0.0), (0.4, 0.0), (lean * 1.5 + 0.3, size), (lean * 1.5 - 0.3, size)], rgb(144, 104, 56));
            for a in [-1.0, -0.4, 0.4, 1.0] {
                let tip = (lean * 1.5 + a * size * 0.4, size * 0.85 - a.abs() * 1.5);
                l.poly(f, &[(lean * 1.5, size + 0.4), (lean * 1.5, size - 0.4), tip], leaf);
            }
        }
        // mushroom (toxic worlds)
        _ => {
            l.quad(f, -0.6, 0.0, 0.6, size * 0.6, rgb(232, 216, 200));
            let cap = [(-size * 0.4, size * 0.55), (size * 0.4, size * 0.55), (size * 0.25, size * 0.85), (-size * 0.25, size * 0.85)];
            l.poly_outline(f, &cap, OUTLINE);
            l.poly(f, &cap, leaf);
            l.disc(f, size * 0.1, size * 0.72, size * 0.06, rgb(248, 240, 232));
        }
    }
}

fn cow(f: &mut Frame, l: &Local, flip: f64, tick: u32, seed: f32) {
    let s = flip;
    let body = [(-1.4 * s, 0.9), (1.2 * s, 0.9), (1.2 * s, 1.9), (-1.4 * s, 1.9)];
    l.poly_outline(f, &body, OUTLINE);
    l.poly(f, &body, rgb(248, 248, 240));
    l.quad(f, -0.6 * s, 1.2, 0.1 * s, 1.8, OUTLINE);
    let bob = if (tick / 20 + (seed * 10.0) as u32) % 6 == 0 { -0.3 } else { 0.0 };
    l.quad(f, 1.2 * s, 1.3 + bob, 2.0 * s, 2.1 + bob, rgb(248, 248, 240));
    l.quad(f, 1.7 * s, 1.3 + bob, 2.1 * s, 1.6 + bob, rgb(240, 168, 168));
    for x in [-1.2, -0.7, 0.6, 1.0] {
        l.quad(f, x * s, 0.0, x * s + 0.25 * s, 0.9, OUTLINE);
    }
}

/// Launch complex: pad slab, service tower, hangar, flag and a sign.
fn launch_complex(f: &mut Frame, l: &Local, tick: u32) {
    let concrete = rgb(168, 168, 176);
    l.quad_outline(f, -26.0, -2.0, 26.0, 0.6, OUTLINE);
    l.quad(f, -26.0, -2.0, 26.0, 0.6, concrete);
    l.quad(f, -26.0, 0.2, 26.0, 0.6, mix(concrete, 0xFFFFFF, 0.3));
    l.quad(f, -6.0, 0.0, 6.0, 0.6, rgb(232, 200, 64));
    // tower
    let (x0, x1) = (-19.0, -12.0);
    l.quad_outline(f, x0, 0.6, x1, 44.0, OUTLINE);
    l.quad(f, x0, 0.6, x1, 44.0, rgb(200, 56, 48));
    let mut y = 2.0;
    while y < 43.0 {
        l.line(f, x0, y, x1, y + 3.5, rgb(120, 32, 32));
        l.line(f, x1, y, x0, y + 3.5, rgb(120, 32, 32));
        y += 3.5;
    }
    l.quad(f, x0 - 1.0, 44.0, x1 + 1.0, 46.0, rgb(232, 232, 232));
    if (tick / 30) % 2 == 0 {
        l.disc(f, (x0 + x1) * 0.5, 47.0, 0.8, rgb(255, 64, 64));
    }
    // swing arms
    for ay in [14.0, 26.0] {
        l.quad(f, x1, ay, -4.0, ay + 1.2, rgb(176, 176, 184));
    }
    // hangar
    let hx = 38.0;
    let roof = [(hx - 14.0, 0.0), (hx + 14.0, 0.0), (hx + 14.0, 9.0), (hx, 13.0), (hx - 14.0, 9.0)];
    l.poly_outline(f, &roof, OUTLINE);
    l.poly(f, &roof, rgb(224, 224, 232));
    l.poly(f, &[(hx - 14.0, 9.0), (hx, 13.0), (hx + 14.0, 9.0), (hx + 14.0, 10.0), (hx, 14.0), (hx - 14.0, 10.0)], rgb(48, 96, 200));
    l.quad(f, hx - 6.0, 0.0, hx + 6.0, 7.0, rgb(88, 88, 104));
    for k in 0..4 {
        l.line(f, hx - 6.0, 1.5 + k as f64 * 1.6, hx + 6.0, 1.5 + k as f64 * 1.6, rgb(120, 120, 136));
    }
    // flag
    l.quad(f, 60.0, 0.0, 60.3, 12.0, rgb(200, 200, 208));
    let wave = ((tick as f64) * 0.15).sin() * 0.4;
    l.poly(f, &[(60.3, 12.0), (65.0, 11.5 + wave), (65.0, 9.0 + wave), (60.3, 9.5)], rgb(232, 64, 48));
    l.poly(f, &[(60.3, 10.6), (65.0, 10.1 + wave), (65.0, 9.6 + wave), (60.3, 10.1)], rgb(248, 248, 248));
    // water tower
    l.quad(f, -40.0, 0.0, -39.4, 10.0, rgb(160, 160, 168));
    l.quad(f, -35.0, 0.0, -34.4, 10.0, rgb(160, 160, 168));
    l.quad_outline(f, -41.0, 10.0, -33.4, 16.0, OUTLINE);
    l.quad(f, -41.0, 10.0, -33.4, 16.0, rgb(232, 232, 240));
    l.quad(f, -41.0, 12.0, -33.4, 13.5, rgb(232, 64, 48));
}

fn landing_zone(f: &mut Frame, l: &Local, tick: u32) {
    l.quad_outline(f, -14.0, -1.5, 14.0, 0.4, OUTLINE);
    l.quad(f, -14.0, -1.5, 14.0, 0.4, rgb(120, 120, 128));
    l.quad(f, -12.0, 0.1, 12.0, 0.4, rgb(232, 200, 64));
    l.quad(f, -2.0, 0.1, 2.0, 0.4, rgb(248, 248, 248));
    for x in [-13.0, 13.0] {
        l.quad(f, x - 0.3, 0.4, x + 0.3, 3.0, rgb(176, 176, 184));
        if (tick / 20) % 2 == 0 {
            l.disc(f, x, 3.4, 0.6, rgb(64, 255, 96));
        }
    }
}

pub fn draw(f: &mut Frame, cam: &Camera, b: &Body, g: &BodyGfx, bs: &BodyState, t: f64, tick: u32) {
    if cam.mpp > 3.5 || !b.kind.solid() {
        return;
    }
    let rel = cam.center - bs.pos;
    let dist = rel.len();
    let view = cam.radius_m();
    if (dist - b.radius).abs() > view + 3000.0 {
        return;
    }
    let spin = b.spin_angle(t);
    let ac = rel.angle() - spin;
    let half = (view / b.radius).min(TAU / 2.0);
    let spacing = 7.0;
    let k0 = ((ac - half) * b.radius / spacing).floor() as i64;
    let k1 = ((ac + half) * b.radius / spacing).ceil() as i64;
    let pad_angles: Vec<(f64, SiteKind)> = b.sites.iter().map(|s| (s.angle, s.kind)).collect();
    let near_site = |a: f64| {
        pad_angles.iter().any(|(pa, k)| {
            let mut d = (a - pa).rem_euclid(TAU);
            if d > TAU / 2.0 {
                d -= TAU;
            }
            (d * b.radius).abs() < if *k == SiteKind::LaunchPad { 90.0 } else { 40.0 }
        })
    };
    if let Some(leaf) = b.look.trees {
        for k in k0..=k1 {
            let h = hash(k as i32, 3, b.seed);
            let gi = {
                let a = k as f64 * spacing / b.radius;
                ground_index(b, &g.globe, a)
            };
            let dens = match gi {
                3 => 0.35,
                4 => 0.6,
                2 => 0.12,
                5 => 0.08,
                _ => 0.0,
            };
            if h > dens {
                continue;
            }
            let a = (k as f64 + hash(k as i32, 4, b.seed) as f64 * 0.8) * spacing / b.radius;
            if near_site(a) {
                continue;
            }
            let ht = b.terrain.height(a);
            if b.look.sea > 0.0 && ht < 0.0 {
                continue;
            }
            let wa = a + spin;
            let base = bs.pos + V2::from_angle(wa) * (b.surface_r(a) - 0.4);
            if !cam.visible(base, 60.0) {
                continue;
            }
            let l = Local::new(cam, base, wa);
            let kind = match (b.kind, gi) {
                (Kind::Toxic, _) => 3,
                (_, 2) => 2,
                (_, 4) | (_, 5) => 1,
                _ => 0,
            };
            let size = 7.0 + hash(k as i32, 5, b.seed) as f64 * 9.0;
            let leaf = if hash(k as i32, 6, b.seed) > 0.5 { leaf } else { shade(leaf, 7, 8) };
            tree(f, &l, kind, size, leaf, h);
        }
    }
    for s in &b.sites {
        let wa = s.angle + spin;
        let base = bs.pos + V2::from_angle(wa) * b.surface_r(s.angle);
        if !cam.visible(base, 200.0 / cam.mpp + 40.0) {
            continue;
        }
        let l = Local::new(cam, base, wa);
        match s.kind {
            SiteKind::LaunchPad => {
                launch_complex(f, &l, tick);
                if b.look.trees.is_some() {
                    for (i, x) in [78.0, 86.0, 101.0].iter().enumerate() {
                        let lc = Local::new(cam, bs.pos + V2::from_angle(wa + x / b.radius) * b.surface_r(s.angle + x / b.radius), wa + x / b.radius);
                        cow(f, &lc, if i == 1 { -1.0 } else { 1.0 }, tick, i as f32 * 0.37);
                    }
                }
            }
            SiteKind::LandingZone => landing_zone(f, &l, tick),
        }
    }
}
