//! A solar system: a star, planets and moons on fixed Kepler orbits ("on
//! rails"). Bodies only move by the clock; rockets feel the gravity of all
//! of them (see `physics`).

use crate::engine::v2::{V2, v2};
use crate::sim::terrain::Terrain;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Star,
    /// Oceans, continents, forests, clouds, city lights.
    Terran,
    Desert,
    Ice,
    Lava,
    /// Grey cratered rock, no air.
    Barren,
    /// Purple/green swamp world with thick haze.
    Toxic,
    Gas,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Star => "STAR",
            Kind::Terran => "TERRAN",
            Kind::Desert => "DESERT",
            Kind::Ice => "ICE",
            Kind::Lava => "LAVA",
            Kind::Barren => "BARREN",
            Kind::Toxic => "TOXIC",
            Kind::Gas => "GAS GIANT",
        }
    }
    pub fn solid(self) -> bool {
        !matches!(self, Kind::Star | Kind::Gas)
    }
}

/// Kepler orbit around the parent. Everything orbits and spins clockwise,
/// so on the launch pad "east" (prograde) is to the right.
#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub a: f64,
    pub e: f64,
    /// Argument of periapsis (rad).
    pub argp: f64,
    /// Mean anomaly at t = 0.
    pub m0: f64,
    /// Mean motion (rad/s, negative = clockwise).
    pub n: f64,
}

impl Orbit {
    pub fn new(a: f64, e: f64, argp: f64, m0: f64, parent_gm: f64) -> Self {
        Orbit { a, e, argp, m0, n: -(parent_gm / (a * a * a)).sqrt() }
    }
    pub fn period(&self) -> f64 {
        std::f64::consts::TAU / self.n.abs()
    }
    /// Position and velocity relative to the parent at time `t`.
    pub fn state(&self, t: f64) -> (V2, V2) {
        let m = self.m0 + self.n * t;
        let e = self.e;
        let mut ea = m;
        for _ in 0..6 {
            ea -= (ea - e * ea.sin() - m) / (1.0 - e * ea.cos());
        }
        let (s, c) = ea.sin_cos();
        let b = (1.0 - e * e).sqrt();
        let edot = self.n / (1.0 - e * c);
        let p = v2(self.a * (c - e), self.a * b * s);
        let v = v2(-self.a * s * edot, self.a * b * c * edot);
        (p.rot(self.argp), v.rot(self.argp))
    }
}

#[derive(Clone, Debug)]
pub struct Atmo {
    /// Above this altitude the air is treated as vacuum (m).
    pub height: f64,
    /// Sea-level density (kg/m3) and scale height (m).
    pub rho0: f64,
    pub scale_h: f64,
    /// Sky colors from the ground up (day side).
    pub sky: [u32; 4],
    pub sunset: u32,
}

impl Atmo {
    pub fn density(&self, alt: f64) -> f64 {
        if alt >= self.height { 0.0 } else { self.rho0 * (-alt.max(0.0) / self.scale_h).exp() }
    }
}

/// Colors a planet is painted with (see `render::planet`).
#[derive(Clone, Debug)]
pub struct Look {
    /// Low to high: deep sea, shallow sea, shore, lowland, upland,
    /// highland, peak (snow), polar.
    pub pal: [u32; 8],
    /// Elevation (0..1) of the sea surface; 0 = no sea.
    pub sea: f32,
    pub cloud: u32,
    /// 0 = clear sky, 1 = overcast.
    pub cloud_cover: f32,
    pub city_lights: bool,
    pub glow: Option<u32>,
    /// Gas giants: band colors from the pole out.
    pub bands: Vec<u32>,
    /// Vegetation on the ground near the surface (tree color), if any.
    pub trees: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Rings {
    pub inner: f64,
    pub outer: f64,
    pub colors: [u32; 3],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SiteKind {
    LaunchPad,
    LandingZone,
}

/// A place on a body's surface, at a body-fixed angle.
#[derive(Clone, Debug)]
pub struct Site {
    pub name: String,
    pub kind: SiteKind,
    pub angle: f64,
}

#[derive(Clone, Debug)]
pub struct Body {
    pub name: String,
    pub kind: Kind,
    pub parent: Option<usize>,
    pub orbit: Orbit,
    pub radius: f64,
    pub gm: f64,
    /// Sphere of influence (Laplace radius) around the parent; the star's is infinite.
    pub soi: f64,
    /// Spin (rad/s, negative = clockwise) and spin angle at t = 0.
    pub spin: f64,
    pub spin0: f64,
    pub atmo: Option<Atmo>,
    pub terrain: Terrain,
    pub look: Look,
    pub rings: Option<Rings>,
    pub sites: Vec<Site>,
    pub seed: u32,
}

impl Body {
    pub fn spin_angle(&self, t: f64) -> f64 {
        self.spin0 + self.spin * t
    }
    pub fn surface_g(&self) -> f64 {
        self.gm / (self.radius * self.radius)
    }
    /// Surface radius at body-fixed angle `a` (the sea counts as surface).
    pub fn surface_r(&self, a: f64) -> f64 {
        self.radius + self.terrain.height(a).max(0.0)
    }
    pub fn site(&self, kind: SiteKind) -> Option<&Site> {
        self.sites.iter().find(|s| s.kind == kind)
    }
}

#[derive(Clone, Debug)]
pub struct System {
    pub name: String,
    pub seed: u64,
    /// Parents always come before their children; body 0 is the star.
    pub bodies: Vec<Body>,
    pub star_color: u32,
    /// Roles the missions use.
    pub home: usize,
}

/// Where every body is at one moment.
#[derive(Clone, Copy, Debug, Default)]
pub struct BodyState {
    pub pos: V2,
    pub vel: V2,
}

impl System {
    pub fn states(&self, t: f64, out: &mut Vec<BodyState>) {
        out.clear();
        for b in &self.bodies {
            let s = match b.parent {
                None => BodyState::default(),
                Some(p) => {
                    let (rp, rv) = b.orbit.state(t);
                    BodyState { pos: out[p].pos + rp, vel: out[p].vel + rv }
                }
            };
            out.push(s);
        }
    }

    /// Position of one body at time `t` (walks up its parents only).
    pub fn pos_of(&self, i: usize, t: f64) -> V2 {
        let b = &self.bodies[i];
        match b.parent {
            None => V2::ZERO,
            Some(p) => self.pos_of(p, t) + b.orbit.state(t).0,
        }
    }

    pub fn states_at(&self, t: f64) -> Vec<BodyState> {
        let mut v = Vec::with_capacity(self.bodies.len());
        self.states(t, &mut v);
        v
    }

    /// The deepest body whose sphere of influence holds `p`.
    pub fn dominant(&self, p: V2, st: &[BodyState]) -> usize {
        let mut best = 0;
        for (i, b) in self.bodies.iter().enumerate().skip(1) {
            // a body can only win inside its parent's sphere, and children come later
            if b.parent == Some(best) || self.is_ancestor(best, i) {
                if (p - st[i].pos).len() < b.soi {
                    best = i;
                }
            }
        }
        best
    }

    pub fn is_ancestor(&self, a: usize, mut i: usize) -> bool {
        while let Some(p) = self.bodies[i].parent {
            if p == a {
                return true;
            }
            i = p;
        }
        false
    }

    pub fn moons(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        self.bodies.iter().enumerate().filter(move |(_, b)| b.parent == Some(i)).map(|(k, _)| k)
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.bodies.iter().position(|b| b.name == name)
    }
}
