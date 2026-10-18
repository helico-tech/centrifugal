//! The elevation field of a solid body: one function of a point on the unit
//! sphere. The globe texture and the equator terrain both sample it.

use crate::engine::noise::{fbm3, noise3, ridged3};
use crate::engine::rng::Rng;
use crate::sim::system::Kind;

pub struct Field {
    pub kind: Kind,
    pub seed: u32,
    craters: Vec<([f32; 3], f32)>,
}

impl Field {
    pub fn new(kind: Kind, seed: u32) -> Self {
        let mut rng = Rng(seed as u64 * 7919 + 17);
        let n = match kind {
            Kind::Barren => 46,
            Kind::Ice | Kind::Desert => 10,
            Kind::Lava => 6,
            _ => 0,
        };
        let craters = (0..n)
            .map(|i| {
                // spread over the sphere, smaller ones more common, and a few
                // forced onto the equator so the side view shows them
                let z = if i % 3 == 0 { rng.range(-0.08, 0.08) } else { rng.range(-1.0, 1.0) };
                let a = rng.range(0.0, std::f32::consts::TAU);
                let r = (1.0 - z * z).sqrt();
                let size = 0.04 + rng.f().powi(3) * 0.28;
                ([r * a.cos(), r * a.sin(), z], size)
            })
            .collect();
        Field { kind, seed, craters }
    }

    /// Elevation in about 0..1 at unit-sphere point `p`.
    pub fn elev(&self, p: [f32; 3]) -> f32 {
        let s = self.seed;
        let [x, y, z] = p;
        let base = match self.kind {
            Kind::Terran => {
                // warped continents, ridged mountain chains on the high ground
                let w = fbm3(x * 1.3 + 9.1, y * 1.3, z * 1.3, 3, 0.5, s ^ 0xA5) - 0.5;
                let c = fbm3(x * 1.7 + w, y * 1.7 - w, z * 1.7, 6, 0.52, s);
                let m = ridged3(x * 2.6, y * 2.6, z * 2.6, 5, s ^ 0x77);
                c + ((c - 0.5).max(0.0) * 2.2) * m * 0.35
            }
            Kind::Desert => {
                let c = fbm3(x * 1.5, y * 1.5, z * 1.5, 6, 0.55, s);
                let canyon = 1.0 - ridged3(x * 2.2, y * 2.2, z * 2.2, 4, s ^ 0x33);
                c * 0.8 + canyon.powi(3) * -0.15 + 0.1
            }
            Kind::Ice => {
                let c = fbm3(x * 1.4, y * 1.4, z * 1.4, 5, 0.45, s);
                let crack = ridged3(x * 3.0, y * 3.0, z * 3.0, 3, s ^ 0x91);
                c * 0.85 + crack * 0.12
            }
            Kind::Lava => {
                let c = fbm3(x * 2.0, y * 2.0, z * 2.0, 6, 0.55, s);
                let r = ridged3(x * 3.0, y * 3.0, z * 3.0, 4, s ^ 0x13);
                c * 0.7 + r * 0.3
            }
            Kind::Toxic => {
                let w = noise3(x * 2.0, y * 2.0, z * 2.0, s ^ 0x55) - 0.5;
                fbm3(x * 2.2 + w, y * 2.2, z * 2.2 - w, 6, 0.5, s)
            }
            Kind::Barren => fbm3(x * 2.0, y * 2.0, z * 2.0, 6, 0.5, s) * 0.7 + 0.15,
            Kind::Star | Kind::Gas => 0.5,
        };
        let mut e = base;
        for (c, size) in &self.craters {
            let d = ((x - c[0]).powi(2) + (y - c[1]).powi(2) + (z - c[2]).powi(2)).sqrt() / size;
            if d < 1.35 {
                // bowl with a raised rim
                let bowl = if d < 1.0 { (d * d - 1.0) * 0.22 } else { 0.0 };
                let rim = (1.0 - ((d - 1.0) / 0.3).powi(2)).max(0.0) * 0.08;
                e += (bowl + rim) * size.sqrt() * 2.0;
            }
        }
        e
    }
}
