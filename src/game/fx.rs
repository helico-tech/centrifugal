//! Particles: exhaust, launch smoke, reentry plasma and explosions. Pure
//! decoration in world coordinates; they never touch the physics.

use crate::engine::gfx::{Frame, bayer, mix, rgb};
use crate::engine::rng::Rng;
use crate::engine::ui::OUTLINE;
use crate::engine::v2::V2;
use crate::render::camera::Camera;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Fire,
    Smoke,
    Spark,
    Plasma,
    Flash,
}

#[derive(Clone, Copy)]
pub struct P {
    pub pos: V2,
    pub vel: V2,
    pub age: f32,
    pub life: f32,
    /// Radius in meters at birth and at death.
    pub r0: f32,
    pub r1: f32,
    pub kind: Kind,
}

#[derive(Clone)]
pub struct Fx {
    pub ps: Vec<P>,
    rng: std::cell::Cell<Rng>,
    /// Screen shake in pixels, decays.
    pub shake: f32,
}

impl Fx {
    pub fn new(seed: u64) -> Self {
        Fx { ps: Vec::new(), rng: std::cell::Cell::new(Rng(seed | 1)), shake: 0.0 }
    }

    /// Random number in a..b (works while `self` is borrowed for `add`).
    pub fn r(&self, a: f32, b: f32) -> f32 {
        let mut g = self.rng.get();
        let v = g.range(a, b);
        self.rng.set(g);
        v
    }

    pub fn int(&self, n: i32) -> i32 {
        let mut g = self.rng.get();
        let v = g.int(n);
        self.rng.set(g);
        v
    }

    pub fn clear(&mut self) {
        self.ps.clear();
    }

    pub fn add(&mut self, p: P) {
        if self.ps.len() < 2500 {
            self.ps.push(p);
        }
    }

    /// Advance `dt` seconds; `air` gives the air velocity and density at a point.
    pub fn update(&mut self, dt: f32, air: impl Fn(V2) -> Option<(V2, f64)>) {
        self.shake *= 0.9;
        for p in &mut self.ps {
            p.age += dt;
            if p.kind == Kind::Smoke || p.kind == Kind::Fire {
                if let Some((wind, rho)) = air(p.pos) {
                    // smoke slows down to the wind
                    let k = (rho * 2.5 * dt as f64).min(1.0);
                    p.vel = p.vel + (wind - p.vel) * k;
                }
            }
            p.pos += p.vel * dt as f64;
        }
        self.ps.retain(|p| p.age < p.life);
    }

    pub fn explode(&mut self, pos: V2, vel: V2, size: f32) {
        self.shake = 8.0 * size.min(2.0);
        self.add(P { pos, vel, age: 0.0, life: 0.35, r0: 4.0 * size, r1: 30.0 * size, kind: Kind::Flash });
        for _ in 0..(60.0 * size) as usize {
            let a = self.r(0.0, std::f32::consts::TAU) as f64;
            let sp = self.r(10.0, 70.0) as f64 * size as f64;
            let d = V2::from_angle(a);
            self.add(P { pos, vel: vel + d * sp, age: 0.0, life: self.r(0.5, 1.6), r0: 0.8, r1: 0.4, kind: Kind::Spark });
        }
        for _ in 0..(30.0 * size) as usize {
            let a = self.r(0.0, std::f32::consts::TAU) as f64;
            let sp = self.r(2.0, 25.0) as f64 * size as f64;
            let d = V2::from_angle(a);
            let k = if self.r(0.0, 1.0) < 0.5 { Kind::Fire } else { Kind::Smoke };
            self.add(P { pos: pos + d * 3.0, vel: vel + d * sp, age: 0.0, life: self.r(1.0, 4.0), r0: 3.0 * size, r1: 12.0 * size, kind: k });
        }
    }

    pub fn draw(&self, f: &mut Frame, cam: &Camera, tick: u32) {
        for (i, p) in self.ps.iter().enumerate() {
            let u = (p.age / p.life).clamp(0.0, 1.0);
            let (x, y) = cam.to_screen(p.pos);
            let rm = p.r0 + (p.r1 - p.r0) * u;
            let r = (rm as f64 / cam.mpp).max(0.6);
            if x + r < 0.0 || y + r < 0.0 || x - r > cam.w || y - r > cam.h {
                continue;
            }
            let (xi, yi) = (x as i32, y as i32);
            let ri = r.round() as i32;
            match p.kind {
                Kind::Fire => {
                    let c = if u < 0.15 {
                        rgb(255, 248, 200)
                    } else if u < 0.35 {
                        rgb(248, 200, 64)
                    } else if u < 0.6 {
                        rgb(240, 112, 40)
                    } else {
                        rgb(152, 64, 56)
                    };
                    disc_dither(f, xi, yi, ri, c, 1.0 - u * 0.6);
                }
                Kind::Smoke => {
                    let c = mix(rgb(232, 232, 232), rgb(120, 120, 136), u);
                    let shadow = mix(c, OUTLINE, 0.25);
                    let a = (1.0 - u * u) * 0.95;
                    disc_dither(f, xi + 1, yi + 1, ri, shadow, a);
                    disc_dither(f, xi, yi, ri, c, a);
                }
                Kind::Spark => {
                    let c = if (tick as usize + i) % 4 < 2 { rgb(255, 232, 120) } else { rgb(248, 128, 40) };
                    f.set(xi, yi, c);
                    if ri > 1 {
                        f.set(xi + 1, yi, c);
                    }
                }
                Kind::Plasma => {
                    let c = if u < 0.3 { rgb(255, 232, 200) } else if u < 0.6 { rgb(248, 136, 64) } else { rgb(216, 64, 96) };
                    disc_dither(f, xi, yi, ri.max(1), c, 1.0 - u);
                }
                Kind::Flash => {
                    disc_dither(f, xi, yi, ri, rgb(255, 255, 232), 1.0 - u);
                    disc_dither(f, xi, yi, (ri as f32 * 0.6) as i32, rgb(255, 255, 255), 1.0 - u * 0.5);
                }
            }
        }
    }
}

fn disc_dither(f: &mut Frame, cx: i32, cy: i32, r: i32, c: u32, a: f32) {
    if r <= 0 {
        if a > 0.4 {
            f.set(cx, cy, c);
        }
        return;
    }
    for dy in -r..=r {
        let w = (((r * r - dy * dy) as f32).max(0.0)).sqrt() as i32;
        let y = cy + dy;
        if y < 0 || y >= f.h {
            continue;
        }
        for x in (cx - w).max(0)..=(cx + w).min(f.w - 1) {
            if a >= 1.0 || a > bayer(x, y) {
                f.px[(y * f.w + x) as usize] = c;
            }
        }
    }
}
