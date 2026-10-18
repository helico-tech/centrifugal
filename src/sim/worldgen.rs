//! Procedural solar systems: one seed gives the star, planets, moons, their
//! looks and names. Aesthetics beat physics: planets are tens of km across,
//! orbits a few thousand km, so a launch takes a minute and a trip to the
//! next planet a few seconds of time warp.
//!
//! Every system has the roles the missions need: a terran home world with
//! an atmosphere and a moon, a neighbor planet further out, and a gas giant.

use crate::engine::gfx::{mix, rgb};
use crate::engine::rng::Rng;
use crate::sim::names;
use crate::sim::surface::Field;
use crate::sim::system::*;
use crate::sim::terrain::Terrain;
use std::f64::consts::{PI, TAU};

pub const STAR_GM: f64 = 4.0e12;
pub const HOME_R: f64 = 30_000.0;
pub const HOME_G: f64 = 9.8;

/// Index of the home world's first planet slot (the third planet).
const HOME_SLOT: usize = 2;

pub fn hsv(h: f32, s: f32, v: f32) -> u32 {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let f = |u: f32| ((u + m) * 255.0) as u8;
    rgb(f(r), f(g), f(b))
}

/// Rotate a color's hue by `deg` (keeps saturation and value).
pub fn hue_shift(c: u32, deg: f32) -> u32 {
    let (r, g, b) = (((c >> 16) & 255) as f32 / 255.0, ((c >> 8) & 255) as f32 / 255.0, (c & 255) as f32 / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    if d < 1e-4 {
        return c;
    }
    let h = if mx == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if mx == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    hsv(h + deg, d / mx, mx)
}

fn pick<T: Copy>(rng: &mut Rng, xs: &[T]) -> T {
    xs[rng.int(xs.len() as i32) as usize]
}

fn look(kind: Kind, rng: &mut Rng) -> (Look, Option<Atmo>) {
    let shift = rng.range(-25.0, 25.0);
    let sh = |c: u32| hue_shift(c, shift);
    let none = Look {
        pal: [0; 8],
        sea: 0.0,
        cloud: rgb(248, 248, 248),
        cloud_cover: 0.0,
        city_lights: false,
        glow: None,
        bands: vec![],
        trees: None,
    };
    let atmo = |height: f64, rho0: f64, sky: [u32; 4], sunset: u32| Some(Atmo { height, rho0, scale_h: height / 6.5, sky, sunset });
    match kind {
        Kind::Terran => {
            let veg = rng.range(-30.0, 40.0);
            let g = |c: u32| hue_shift(c, veg);
            (
                Look {
                    pal: [
                        rgb(24, 40, 112),
                        rgb(40, 96, 184),
                        rgb(232, 216, 144),
                        g(rgb(80, 176, 64)),
                        g(rgb(40, 128, 56)),
                        rgb(128, 108, 84),
                        rgb(240, 240, 248),
                        rgb(224, 240, 248),
                    ],
                    sea: 0.5,
                    cloud: rgb(248, 248, 248),
                    cloud_cover: rng.range(0.45, 0.6),
                    city_lights: true,
                    trees: Some(g(rgb(32, 104, 48))),
                    ..none
                },
                atmo(6000.0, 1.2, [rgb(104, 168, 240), rgb(72, 128, 224), rgb(40, 72, 176), rgb(24, 32, 96)], rgb(248, 136, 64)),
            )
        }
        Kind::Desert => (
            Look {
                pal: [
                    sh(rgb(120, 56, 40)),
                    sh(rgb(152, 72, 40)),
                    sh(rgb(184, 96, 48)),
                    sh(rgb(208, 128, 64)),
                    sh(rgb(232, 160, 88)),
                    sh(rgb(176, 96, 64)),
                    sh(rgb(248, 216, 176)),
                    rgb(240, 232, 224),
                ],
                cloud: rgb(240, 208, 176),
                cloud_cover: 0.12,
                ..none
            },
            atmo(4000.0, 0.25, [sh(rgb(232, 176, 128)), sh(rgb(200, 136, 104)), sh(rgb(136, 88, 96)), rgb(48, 32, 64)], rgb(120, 160, 224)),
        ),
        Kind::Ice => (
            Look {
                pal: [
                    sh(rgb(64, 96, 152)),
                    sh(rgb(96, 136, 184)),
                    sh(rgb(144, 184, 216)),
                    sh(rgb(184, 216, 240)),
                    sh(rgb(216, 236, 248)),
                    sh(rgb(160, 184, 224)),
                    rgb(248, 248, 248),
                    rgb(248, 248, 248),
                ],
                cloud: rgb(232, 240, 248),
                cloud_cover: 0.2,
                ..none
            },
            if rng.f() < 0.6 {
                atmo(3000.0, 0.15, [sh(rgb(176, 216, 240)), sh(rgb(128, 176, 224)), sh(rgb(72, 104, 168)), rgb(24, 32, 72)], rgb(232, 184, 232))
            } else {
                None
            },
        ),
        Kind::Lava => {
            let lava = pick(rng, &[rgb(248, 120, 32), rgb(248, 72, 40), rgb(248, 168, 32)]);
            (
                Look {
                    pal: [
                        mix(lava, 0xFFFF80, 0.5),
                        lava,
                        rgb(72, 40, 40),
                        rgb(56, 40, 48),
                        rgb(80, 56, 56),
                        rgb(104, 72, 64),
                        rgb(128, 96, 80),
                        rgb(96, 80, 80),
                    ],
                    sea: 0.42,
                    cloud: rgb(88, 72, 72),
                    cloud_cover: 0.25,
                    glow: Some(lava),
                    ..none
                },
                atmo(3000.0, 0.5, [rgb(200, 96, 56), rgb(152, 64, 48), rgb(88, 40, 48), rgb(32, 16, 32)], rgb(248, 200, 64)),
            )
        }
        Kind::Barren => {
            let tint = pick(rng, &[rgb(160, 160, 168), rgb(176, 160, 144), rgb(152, 160, 176), rgb(184, 176, 160)]);
            let s = |k: f32| mix(0, tint, k);
            (
                Look {
                    pal: [s(0.35), s(0.45), s(0.55), s(0.65), s(0.75), s(0.85), s(1.0), mix(tint, 0xFFFFFF, 0.5)],
                    ..none
                },
                None,
            )
        }
        Kind::Toxic => (
            Look {
                pal: [
                    sh(rgb(64, 152, 48)),
                    sh(rgb(120, 216, 64)),
                    sh(rgb(168, 120, 168)),
                    sh(rgb(120, 64, 144)),
                    sh(rgb(88, 48, 120)),
                    sh(rgb(152, 96, 168)),
                    sh(rgb(216, 176, 232)),
                    sh(rgb(200, 232, 160)),
                ],
                sea: 0.45,
                cloud: sh(rgb(200, 232, 120)),
                cloud_cover: 0.5,
                trees: Some(sh(rgb(232, 88, 168))),
                ..none
            },
            atmo(7000.0, 2.0, [sh(rgb(168, 200, 72)), sh(rgb(120, 160, 64)), sh(rgb(72, 96, 72)), rgb(32, 40, 48)], sh(rgb(232, 96, 168))),
        ),
        Kind::Gas => {
            let sets: [&[u32]; 4] = [
                &[rgb(232, 200, 152), rgb(200, 136, 88), rgb(240, 224, 192), rgb(176, 96, 64), rgb(224, 176, 120)],
                &[rgb(104, 152, 232), rgb(64, 96, 200), rgb(152, 200, 248), rgb(48, 64, 168), rgb(120, 176, 240)],
                &[rgb(232, 144, 184), rgb(176, 88, 168), rgb(248, 200, 216), rgb(120, 64, 152), rgb(216, 120, 200)],
                &[rgb(144, 216, 200), rgb(88, 168, 160), rgb(200, 240, 224), rgb(56, 120, 136), rgb(120, 200, 184)],
            ];
            let bands: Vec<u32> = pick(rng, &sets).iter().map(|c| sh(*c)).collect();
            let sky = [bands[0], bands[1], mix(bands[3], 0, 0.4), mix(bands[3], 0, 0.75)];
            (Look { bands: bands.clone(), cloud: bands[2], ..none }, atmo(18000.0, 3.0, sky, bands[2]))
        }
        Kind::Star => (none, None),
    }
}

/// Sea level as an elevation: the `water` fraction of the globe lies below it.
fn sea_level(f: &Field, water: f32) -> f32 {
    let mut rng = Rng(f.seed as u64 + 99);
    let mut v: Vec<f32> = (0..1500)
        .map(|_| {
            let z = rng.range(-1.0, 1.0);
            let a = rng.range(0.0, std::f32::consts::TAU);
            let r = (1.0 - z * z).sqrt();
            f.elev([r * a.cos(), r * a.sin(), z])
        })
        .collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() as f32 * water) as usize).min(v.len() - 1)]
}

/// Meters of height per unit of elevation.
fn relief(kind: Kind, radius: f64) -> f32 {
    let k = match kind {
        Kind::Terran => 5200.0,
        Kind::Desert => 4200.0,
        Kind::Ice => 3000.0,
        Kind::Lava => 4500.0,
        Kind::Barren => 4000.0,
        Kind::Toxic => 3500.0,
        _ => 0.0,
    };
    // small moons get smaller mountains
    k * (radius / HOME_R).clamp(0.35, 1.2) as f32
}

fn terrain(kind: Kind, field: &Field, sea: f32, radius: f64) -> Terrain {
    if !kind.solid() {
        return Terrain::flat();
    }
    let n = ((TAU * radius / 8.0) as usize).clamp(512, 40_000);
    let rel = relief(kind, radius);
    let mut h: Vec<f32> = (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            field.elev([a.cos(), a.sin(), 0.0])
        })
        .collect();
    if sea > 0.0 {
        for x in &mut h {
            *x = (*x - sea) * rel;
        }
    } else {
        let lo = h.iter().cloned().fold(f32::MAX, f32::min);
        for x in &mut h {
            *x = (*x - lo) * rel;
        }
    }
    Terrain { h }
}

/// Find a body-fixed angle with gentle land around it, near `prefer`.
fn find_site(t: &Terrain, radius: f64, prefer: f64, need_land: bool) -> f64 {
    let mut best = (f64::MAX, prefer);
    for k in 0..720 {
        let a = prefer + (k as f64 / 720.0) * TAU;
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        for j in -6..=6 {
            let h = t.height(a + j as f64 * 40.0 / radius);
            lo = lo.min(h);
            hi = hi.max(h);
        }
        if need_land && lo < 40.0 {
            continue;
        }
        let score = (hi - lo) + (k as f64 / 720.0) * 300.0 + if lo > 450.0 { 1500.0 + lo } else { 0.0 };
        if score < best.0 {
            best = (score, a);
        }
    }
    best.1
}

struct Spec {
    kind: Kind,
    radius: f64,
    g: f64,
    a: f64,
    e: f64,
    spin_period: f64,
    parent: usize,
}

fn make_body(sys: &mut System, spec: Spec, name: String, rng: &mut Rng) -> usize {
    let parent_gm = sys.bodies[spec.parent].gm;
    let gm = spec.g * spec.radius * spec.radius;
    let orbit = Orbit::new(spec.a, spec.e, rng.range(0.0, TAU as f32) as f64, rng.range(0.0, TAU as f32) as f64, parent_gm);
    let seed = rng.next();
    let (mut look, atmo) = look(spec.kind, rng);
    let field = Field::new(spec.kind, seed);
    if look.sea > 0.0 {
        let water = match spec.kind {
            Kind::Terran => 0.58,
            Kind::Lava => 0.3,
            _ => 0.35,
        };
        look.sea = sea_level(&field, water);
    }
    let terrain = terrain(spec.kind, &field, look.sea, spec.radius);
    let soi = spec.a * (gm / parent_gm).powf(0.4);
    let spin = if spec.spin_period > 0.0 { -TAU / spec.spin_period } else { orbit.n };
    let rings = if spec.kind == Kind::Gas && rng.f() < 0.7 {
        let b = &look.bands;
        Some(Rings { inner: spec.radius * 1.35, outer: spec.radius * rng.range(1.9, 2.4) as f64, colors: [b[2], b[0], b[4]] })
    } else {
        None
    };
    sys.bodies.push(Body {
        name,
        kind: spec.kind,
        parent: Some(spec.parent),
        orbit,
        radius: spec.radius,
        gm,
        soi,
        spin,
        spin0: rng.range(0.0, TAU as f32) as f64,
        atmo,
        terrain,
        look,
        rings,
        sites: vec![],
        seed,
    });
    sys.bodies.len() - 1
}

pub fn generate(seed: u64) -> System {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    for _ in 0..4 {
        rng.next();
    }
    let star_color = pick(&mut rng, &[rgb(248, 232, 136), rgb(248, 200, 104), rgb(248, 168, 96), rgb(216, 232, 248)]);
    let name = names::star(&mut rng);
    let mut sys = System { name: name.clone(), seed, bodies: vec![], star_color, home: 0 };
    sys.bodies.push(Body {
        name,
        kind: Kind::Star,
        parent: None,
        orbit: Orbit { a: 0.0, e: 0.0, argp: 0.0, m0: 0.0, n: 0.0 },
        radius: 260_000.0,
        gm: STAR_GM,
        soi: f64::INFINITY,
        spin: 0.0,
        spin0: 0.0,
        atmo: None,
        terrain: Terrain::flat(),
        look: look(Kind::Star, &mut rng).0,
        rings: None,
        sites: vec![],
        seed: rng.next(),
    });

    let count = 5 + rng.int(2) as usize;
    let mut a = 1.5e6 * rng.range(1.0, 1.15) as f64;
    let outer_kinds = [Kind::Desert, Kind::Toxic, Kind::Ice];
    for slot in 0..count {
        let kind = match slot {
            0 => Kind::Lava,
            1 => pick(&mut rng, &[Kind::Barren, Kind::Desert, Kind::Toxic]),
            HOME_SLOT => Kind::Terran,
            3 => pick(&mut rng, &outer_kinds),
            4 => Kind::Gas,
            _ => pick(&mut rng, &[Kind::Gas, Kind::Ice]),
        };
        let (radius, g) = match kind {
            Kind::Terran => (HOME_R, HOME_G),
            Kind::Gas => (rng.range(90_000.0, 130_000.0) as f64, rng.range(14.0, 20.0) as f64),
            _ => (rng.range(16_000.0, 34_000.0) as f64, rng.range(4.0, 10.0) as f64),
        };
        let spin_period = rng.range(5_000.0, 16_000.0) as f64;
        let pname = names::planet(&mut rng);
        let e = if slot == HOME_SLOT { 0.0 } else { rng.range(0.0, 0.05) as f64 };
        let p = make_body(&mut sys, Spec { kind, radius, g, a, e, spin_period, parent: 0 }, pname, &mut rng);
        if slot == HOME_SLOT {
            sys.home = p;
        }

        // moons, out to a safe fraction of the Hill sphere
        let hill = a * (sys.bodies[p].gm / (3.0 * STAR_GM)).cbrt();
        let n_moons = match (slot, kind) {
            (HOME_SLOT, _) => 1,
            (_, Kind::Gas) => 2 + rng.int(2) as usize,
            (0, _) => 0,
            _ => rng.int(2) as usize,
        };
        let mut ma = radius * if kind == Kind::Gas { 4.0 } else { 4.5 };
        for m in 0..n_moons {
            let (mkind, mr, mg) = if slot == HOME_SLOT {
                (Kind::Barren, 11_000.0, 2.07)
            } else {
                let k = pick(&mut rng, &[Kind::Barren, Kind::Barren, Kind::Ice, Kind::Lava, Kind::Desert]);
                (k, rng.range(6_000.0, 16_000.0) as f64, rng.range(1.2, 3.0) as f64)
            };
            if slot == HOME_SLOT {
                ma = 180_000.0;
            } else {
                ma *= rng.range(1.5, 2.1) as f64;
            }
            if ma > hill * 0.45 && slot != HOME_SLOT {
                break;
            }
            let mname = names::moon(&mut rng);
            let _ = m;
            make_body(
                &mut sys,
                Spec { kind: mkind, radius: mr, g: mg, a: ma, e: 0.0, spin_period: 0.0, parent: p },
                mname,
                &mut rng,
            );
        }
        a *= rng.range(1.5, 1.72) as f64;
    }

    // launch pad on the home world, landing zones on the rest
    for i in 1..sys.bodies.len() {
        let b = &sys.bodies[i];
        if !b.kind.solid() {
            continue;
        }
        let home = i == sys.home;
        let pref = rng.range(0.0, TAU as f32) as f64;
        let ang = find_site(&b.terrain, b.radius, pref, b.look.sea > 0.0);
        let (kind, label, half) = if home { (SiteKind::LaunchPad, "LAUNCH COMPLEX 1", 70.0) } else { (SiteKind::LandingZone, "LANDING ZONE", 45.0) };
        let r = b.radius;
        let b = &mut sys.bodies[i];
        b.terrain.flatten(ang, half, r, 30.0);
        b.sites.push(Site { name: label.into(), kind, angle: ang });
    }
    // start with the pad in the morning sun
    let home = sys.home;
    let st = sys.states_at(0.0);
    let sun_dir = (st[0].pos - st[home].pos).norm().angle();
    let pad = sys.bodies[home].sites[0].angle;
    sys.bodies[home].spin0 = sun_dir - pad + PI * 0.3;
    sys
}

/// The neighbor planet missions fly to: the first solid planet outside home.
pub fn neighbor(sys: &System) -> Option<usize> {
    sys.bodies
        .iter()
        .enumerate()
        .find(|(i, b)| b.parent == Some(0) && *i > sys.home && b.kind.solid())
        .map(|(i, _)| i)
}

pub fn home_moon(sys: &System) -> Option<usize> {
    sys.moons(sys.home).next()
}
