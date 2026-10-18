//! Draws a planet or moon per pixel: the lit globe seen from the pole, the
//! side-view ground (grass, dirt, rock, sea) along its rim, the sky and
//! clouds of its atmosphere, and rings. The same code works from a close-up
//! of the launch pad to a dot on the system map.

use super::camera::Camera;
use super::globe::{self, CITY, DARK, GH, GLOW, GW, Globe};
use crate::engine::gfx::{Frame, bayer, mix, rgb, shade};
use crate::engine::noise::{hash, noise3};
use crate::engine::ui::OUTLINE;
use crate::engine::v2::{V2, v2};
use crate::sim::system::{Body, Kind, System};
use std::f64::consts::{FRAC_PI_2, TAU};

pub struct BodyGfx {
    pub globe: Globe,
    pub max_h: f64,
    /// Average color, for dots on the map.
    pub avg: u32,
}

pub fn build(sys: &System) -> Vec<BodyGfx> {
    sys.bodies
        .iter()
        .map(|b| {
            let globe = globe::build(b);
            let max_h = b.terrain.h.iter().cloned().fold(0.0f32, f32::max) as f64;
            let avg = match b.kind {
                Kind::Star => sys.star_color,
                Kind::Gas => b.look.bands[0],
                _ => b.look.pal[if b.look.sea > 0.0 { 1 } else { 3 }],
            };
            BodyGfx { globe, max_h, avg }
        })
        .collect()
}

const NIGHT: u32 = rgb(8, 12, 40);

/// Five light levels from night to full sun.
#[inline]
fn lit(c: u32, level: usize) -> u32 {
    match level {
        0 => mix(shade(c, 1, 4), NIGHT, 0.45),
        1 => mix(shade(c, 1, 2), NIGHT, 0.25),
        2 => shade(c, 3, 4),
        3 => shade(c, 7, 8),
        _ => c,
    }
}

#[inline]
fn level(q: f64, x: i32, y: i32) -> usize {
    band(q.clamp(0.0, 1.0) * 4.0, x, y, 0.7).min(4)
}

/// Band index for a continuous value: flat bands with a dithered seam
/// (`spread` 1 = fully dithered, 0 = hard edges).
#[inline]
fn band(v: f64, x: i32, y: i32, spread: f32) -> usize {
    (v as f32 + (bayer(x, y) - 0.5) * spread).max(0.0) as usize
}

pub struct Ctx<'a> {
    pub sys: &'a System,
    pub gfx: &'a [BodyGfx],
    /// Simulation time (s) and animation tick.
    pub t: f64,
    pub tick: u32,
}

/// Rough color of the ground at the rim (body-fixed angle `a`).
pub fn ground_index(b: &Body, g: &Globe, a: f64) -> u8 {
    let h = b.terrain.height(a);
    if b.look.sea > 0.0 {
        if h < 0.0 {
            return 1;
        }
        if h < 22.0 {
            return 2;
        }
    }
    let ai = ((a.rem_euclid(TAU) / TAU) * GW as f64) as usize % GW;
    let i = g.at(ai, 0) & 7;
    if b.look.sea > 0.0 && i < 3 { 3 } else { i }
}

pub fn draw(f: &mut Frame, cam: &Camera, ctx: &Ctx, i: usize, pos: V2, star: V2) {
    let b = &ctx.sys.bodies[i];
    let g = &ctx.gfx[i];
    let atmo_h = b.atmo.as_ref().map(|a| a.height).unwrap_or(0.0);
    let ring_r = b.rings.as_ref().map(|r| r.outer).unwrap_or(0.0);
    let rmax = (b.radius + g.max_h + 2.0).max(b.radius + atmo_h).max(ring_r);
    let rpx = rmax / cam.mpp;
    let (cx, cy) = cam.to_screen(pos);
    if cx + rpx < 0.0 || cy + rpx < 0.0 || cx - rpx > cam.w || cy - rpx > cam.h {
        return;
    }
    if b.radius / cam.mpp < 1.5 {
        let (x, y) = (cx.floor() as i32, cy.floor() as i32);
        f.rect(x - 1, y - 1, 3, 3, OUTLINE);
        f.set(x, y, g.avg);
        return;
    }
    let x0 = (cx - rpx).floor().max(0.0) as i32;
    let x1 = (cx + rpx).ceil().min(cam.w - 1.0) as i32;
    let y0 = (cy - rpx).floor().max(0.0) as i32;
    let y1 = (cy + rpx).ceil().min(cam.h - 1.0) as i32;

    let sun = (star - pos).norm();
    let lz: f64 = 0.55;
    let ln = (1.0 + lz * lz).sqrt();
    let (lx, ly, lz) = (sun.x / ln, sun.y / ln, lz / ln);
    let spin = b.spin_angle(ctx.t);
    let cloud_spin = spin + ctx.t * b.spin * 0.12;
    let sea_body = b.look.sea > 0.0;
    let strata_px = (b.radius * 0.012 / cam.mpp).min(34.0);
    let pal = &b.look.pal;
    let mpp = cam.mpp;
    let o = cam.to_world(0.5, 0.5) - pos;
    let ax = cam.to_world(1.5, 0.5) - pos - o;
    let ay = cam.to_world(0.5, 1.5) - pos - o;
    let r_ground_max = b.radius + g.max_h + 1.0;
    let cloud_lo = atmo_h * 0.16;
    let cloud_hi = atmo_h * 0.42;
    let near_detail = mpp < 40.0;

    for sy in y0..=y1 {
        let row = o + ay * sy as f64;
        for sx in x0..=x1 {
            let d = row + ax * sx as f64;
            let r2 = d.len2();
            if r2 > rmax * rmax {
                continue;
            }
            let r = r2.sqrt();
            let (ux, uy) = (d.x / r, d.y / r);
            let day = ux * sun.x + uy * sun.y;
            let wa = d.y.atan2(d.x);
            let ba = wa - spin;
            let h = if r < r_ground_max { b.terrain.height(ba) } else { 0.0 };
            let surf = b.radius + if sea_body { h.max(0.0) } else { h };

            if b.kind != Kind::Gas && r <= surf {
                let depth_px = (surf - r) / mpp;
                let daylvl = level(day * 1.6 + 0.55, sx, sy);
                if depth_px < strata_px {
                    let c = if sea_body && h < 0.0 {
                        // the sea in side view, with a bright surface line
                        let below = (b.radius - r) / mpp;
                        if below < 1.0 {
                            mix(pal[1], 0xFFFFFF, 0.35)
                        } else if below < 3.0 + ((sx as f64 * 0.7 + ctx.tick as f64 * 0.1).sin() * 1.2) {
                            pal[1]
                        } else if (b.radius - r) < -h * 0.5 {
                            if (sx + sy) & 1 == 0 && below < 6.0 { pal[1] } else { mix(pal[1], pal[0], 0.5) }
                        } else {
                            pal[0]
                        }
                    } else {
                        let gi = ground_index(b, &g.globe, ba) as usize;
                        let col = (ba * b.radius).floor() as i32;
                        let wob = (hash(col >> 1, 7, b.seed) * 2.0) as f64;
                        let top = match gi {
                            3 | 4 => mix(pal[gi], 0xFFFFFF, 0.2),
                            _ => mix(pal[gi], 0xFFFFFF, 0.1),
                        };
                        if depth_px < 1.5 {
                            top
                        } else if depth_px < 4.0 + wob {
                            pal[gi]
                        } else {
                            strata(b, surf - r, depth_px, col, mpp, wob)
                        }
                    };
                    f.px[(sy * f.w + sx) as usize] = lit(c, daylvl);
                    continue;
                }
                if r > b.radius {
                    // deep under a mountain: rock layers all the way down
                    let col = (ba * b.radius).floor() as i32;
                    f.px[(sy * f.w + sx) as usize] = lit(strata(b, surf - r, depth_px, col, mpp, 1.0), daylvl);
                    continue;
                }
            }

            if r <= b.radius {
                // the globe, seen from the pole
                let rho = (r / b.radius).min(1.0);
                let nz = (1.0 - rho * rho).sqrt();
                let lum = ux * rho * lx + uy * rho * ly + nz * lz;
                let ring = ((nz.min(1.0).asin() / FRAC_PI_2) * (GH - 1) as f64) as usize;
                let ai = ((ba.rem_euclid(TAU) / TAU) * GW as f64) as usize % GW;
                let lv = level(lum * 1.35 + 0.1, sx, sy);
                let mut c;
                let mut emissive = None;
                if b.kind == Kind::Gas {
                    c = g.globe.gas[ring * GW + ai];
                } else {
                    let t = g.globe.at(ai, ring);
                    c = pal[(t & 7) as usize];
                    if t & DARK != 0 {
                        c = shade(c, 7, 8);
                    }
                    if lv <= 1 && t & CITY != 0 {
                        emissive = Some(if (ai + ring) % 3 == 0 { rgb(255, 248, 200) } else { rgb(248, 200, 96) });
                    }
                    if t & GLOW != 0 {
                        emissive = b.look.glow.map(|gl| if lv <= 1 { gl } else { mix(gl, 0xFFFF88, 0.3) });
                    }
                }
                let mut px = lit(c, lv);
                if let Some(e) = emissive {
                    px = e;
                }
                if b.look.cloud_cover > 0.0 {
                    let ca = (((wa - cloud_spin).rem_euclid(TAU) / TAU) * GW as f64) as usize % GW;
                    let cv = g.globe.cloud[ring * GW + ca] as f32 / 255.0;
                    if cv > bayer(sx, sy) {
                        px = lit(b.look.cloud, lv);
                    }
                }
                f.px[(sy * f.w + sx) as usize] = px;
                continue;
            }

            // outside the solid body: rings, then air
            if let Some(rings) = &b.rings {
                if r > rings.inner && r < rings.outer {
                    let u = (r - rings.inner) / (rings.outer - rings.inner);
                    let n = noise3((u * 16.0) as f32, 0.5, 0.0, b.seed ^ 77) * 0.8 + noise3((u * 60.0) as f32, 0.5, 0.0, b.seed ^ 78) * 0.2;
                    if n > 0.35 {
                        let band = ((u * 6.0) as usize + (n * 3.0) as usize) % 3;
                        let mut c = rings.colors[band];
                        // planet shadow on the night side
                        let along = d.dot(sun);
                        if along < 0.0 && (d.cross(sun)).abs() < b.radius {
                            c = lit(c, 0);
                        }
                        if n > 0.45 || (sx + sy) & 1 == 0 {
                            f.px[(sy * f.w + sx) as usize] = c;
                        }
                    }
                }
            }
            if let Some(a) = &b.atmo {
                let alt = r - b.radius;
                if alt < a.height {
                    let u = (alt / a.height).max(0.0);
                    let dk = ((day + 0.22) / 0.55).clamp(0.0, 1.0);
                    let si = band(u * 3.8, sx, sy, 0.6).min(3);
                    let mut c = a.sky[si];
                    let dusk = 1.0 - ((day - 0.02).abs() / 0.25).min(1.0);
                    if dusk > 0.0 && (dusk * (1.0 - u) * 1.3) as f32 > bayer(sx + 1, sy) {
                        c = a.sunset;
                    }
                    let lv = level(dk, sx, sy);
                    c = lit(c, lv);
                    let mut alpha = ((1.0 - u) * 1.8).min(1.0) * (0.35 + 0.65 * dk);
                    if near_detail && alt > cloud_lo && alt < cloud_hi && b.look.cloud_cover > 0.0 {
                        let ca = ((wa - cloud_spin).rem_euclid(TAU) / TAU * GW as f64) as usize % GW;
                        let cov = g.globe.cloud[ca] as f64 / 255.0;
                        let sarc = (wa - cloud_spin).rem_euclid(TAU) * b.radius;
                        let env = 1.0 - ((alt - (cloud_lo + cloud_hi) * 0.5) / ((cloud_hi - cloud_lo) * 0.5)).abs();
                        let n = noise3((sarc / 520.0) as f32, (alt / 260.0) as f32, 3.0, b.seed ^ 0xC10) as f64;
                        let dens = n * env.powf(0.5) + cov * 0.55 - 0.72;
                        if dens > 0.0 {
                            let shadeb = if dens < 0.05 { 3 } else { 4 };
                            let lower = (alt - cloud_lo) / (cloud_hi - cloud_lo) < 0.35 && dens < 0.12;
                            let cc = if lower { shade(b.look.cloud, 13, 16) } else { b.look.cloud };
                            c = lit(cc, lv.min(shadeb));
                            alpha = 1.0;
                        }
                    }
                    if alpha >= 1.0 || alpha as f32 > bayer(sx, sy) {
                        f.px[(sy * f.w + sx) as usize] = c;
                    }
                }
            }
        }
    }
}

/// Underground in side view: a dirt band, then wavy rock layers with
/// pebbles, in meters so they turn with the planet.
fn strata(b: &Body, depth_m: f64, depth_px: f64, col: i32, mpp: f64, wob: f64) -> u32 {
    let pal = &b.look.pal;
    let dirt = mix(pal[5], 0, 0.25);
    if depth_px < 12.0 + wob * 2.0 {
        let row = (depth_m / mpp.max(0.5)) as i32;
        return if hash(col, row >> 1, b.seed) < 0.08 { mix(dirt, 0xFFFFFF, 0.2) } else { dirt };
    }
    let rock = mix(pal[5], 0, 0.45);
    let tones = [rock, mix(rock, pal[2], 0.3), mix(rock, 0, 0.3), mix(dirt, 0, 0.35)];
    let layer_h = (6.0f64).max(mpp * 5.0);
    let wave = (noise3(col as f32 * 0.02, 0.0, 1.0, b.seed ^ 0x5A) as f64 - 0.5) * layer_h * 1.2;
    let layer = ((depth_m + wave) / layer_h).floor() as i32;
    let c = tones[(hash(layer, 11, b.seed) * 3.99) as usize];
    // pebbles: 2x2 blocks in world meters
    let (px, py) = ((col as f64 / mpp.max(0.25) * 0.0) as i32 + col / 2, (depth_m / mpp.max(0.25)) as i32 / 2);
    let h = hash(px, py, b.seed ^ 0xBEEF);
    if h < 0.035 {
        mix(c, 0xFFFFFF, 0.25)
    } else if h < 0.07 {
        mix(c, 0, 0.35)
    } else {
        c
    }
}

/// A small marker for bodies too small to see, with the outline.
pub fn dot(f: &mut Frame, x: i32, y: i32, c: u32, r: i32) {
    f.fill_ellipse(x, y, r + 1, r + 1, OUTLINE);
    f.fill_ellipse(x, y, r, r, c);
    f.set(x - r / 2, y - r / 2, mix(c, 0xFFFFFF, 0.5));
}

/// Unit vector of local "up" at body-fixed angle `a` of body `i` at time `t`.
pub fn up_at(sys: &System, i: usize, a: f64, t: f64) -> V2 {
    let w = a + sys.bodies[i].spin_angle(t);
    v2(w.cos(), w.sin())
}
