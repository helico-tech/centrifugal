//! Planet textures, built once per body. The 2D world sees each planet from
//! above its pole: the disc center is the pole, the rim is the equator (the
//! ground the rockets stand on), and the planet spins in the plane. So the
//! texture is polar: `w` steps of body-fixed angle by `h` latitude rings
//! from the equator (ring 0) to the pole.

use crate::engine::gfx::mix;
use crate::engine::noise::{fbm3, hash, noise3};
use crate::sim::surface::Field;
use crate::sim::system::{Body, Kind};

pub const GW: usize = 512;
pub const GH: usize = 128;

/// Bit flags next to the palette index in `Globe::tex`.
pub const DARK: u8 = 8;
pub const CITY: u8 = 16;
pub const GLOW: u8 = 32;

pub struct Globe {
    /// Palette index (low 3 bits) plus flags.
    pub tex: Vec<u8>,
    /// Cloud cover 0..255.
    pub cloud: Vec<u8>,
    /// Final colors for gas giants (bands), else empty.
    pub gas: Vec<u32>,
}

impl Globe {
    #[inline]
    pub fn at(&self, ai: usize, ring: usize) -> u8 {
        self.tex[ring * GW + ai]
    }
}

fn point(ai: usize, ring: usize) -> [f32; 3] {
    let lon = ai as f32 / GW as f32 * std::f32::consts::TAU;
    let lat = ring as f32 / (GH - 1) as f32 * std::f32::consts::FRAC_PI_2;
    let c = lat.cos();
    [c * lon.cos(), c * lon.sin(), lat.sin()]
}

/// Palette index for an elevation; shared by the globe and the side view.
pub fn classify(kind: Kind, e: f32, sea: f32, lat: f32, wobble: f32) -> u8 {
    let polar = match kind {
        Kind::Terran => 1.42,
        Kind::Ice => 1.2,
        Kind::Desert => 1.45,
        _ => 9.0,
    };
    if lat + wobble * 0.25 > polar {
        return 7;
    }
    if sea > 0.0 {
        if e < sea - 0.035 {
            return 0;
        }
        if e < sea {
            return 1;
        }
        let u = e - sea;
        return match u {
            u if u < 0.012 => 2,
            u if u < 0.07 + wobble * 0.03 => 3,
            u if u < 0.14 => 4,
            u if u < 0.2 => 5,
            _ => 6,
        };
    }
    match e + wobble * 0.04 {
        e if e < 0.32 => 0,
        e if e < 0.4 => 1,
        e if e < 0.46 => 2,
        e if e < 0.53 => 3,
        e if e < 0.6 => 4,
        e if e < 0.68 => 5,
        _ => 6,
    }
}

pub fn build(b: &Body) -> Globe {
    let mut tex = vec![0u8; GW * GH];
    let mut cloud = vec![0u8; GW * GH];
    let mut gas = vec![];
    let s = b.seed;
    if b.kind == Kind::Gas {
        gas = vec![0u32; GW * GH];
        let bands = &b.look.bands;
        for ring in 0..GH {
            for ai in 0..GW {
                let [x, y, z] = point(ai, ring);
                // bands follow latitude, warped into swirls; storms are blobs
                let warp = fbm3(x * 3.0, y * 3.0, z * 3.0, 4, 0.5, s) - 0.5;
                let lat = z + warp * 0.18;
                let k = (lat * 9.0 + noise3(z * 2.0, 0.0, 0.0, s ^ 3) * 2.0).rem_euclid(bands.len() as f32);
                let i = k as usize % bands.len();
                let j = (i + 1) % bands.len();
                let f = k.fract();
                let mut c = if f > 0.82 { mix(bands[i], bands[j], 0.5) } else { bands[i] };
                let storm = noise3(x * 5.0, y * 5.0, z * 5.0, s ^ 9);
                if storm > 0.8 {
                    c = mix(c, bands[(i + 2) % bands.len()], 0.7);
                }
                if hash(ai as i32, ring as i32, s) < 0.04 {
                    c = mix(c, 0xFFFFFF, 0.15);
                }
                gas[ring * GW + ai] = c;
            }
        }
        return Globe { tex, cloud, gas };
    }
    let field = Field::new(b.kind, s);
    let look = &b.look;
    for ring in 0..GH {
        let lat = ring as f32 / (GH - 1) as f32 * std::f32::consts::FRAC_PI_2;
        for ai in 0..GW {
            let p = point(ai, ring);
            let e = field.elev(p);
            let wobble = noise3(p[0] * 6.0, p[1] * 6.0, p[2] * 6.0, s ^ 0x1234) - 0.5;
            let mut t = classify(b.kind, e, look.sea, lat, wobble);
            let detail = noise3(p[0] * 14.0, p[1] * 14.0, p[2] * 14.0, s ^ 0x777);
            if detail > 0.62 {
                t |= DARK;
            }
            let idx = t & 7;
            if look.city_lights && (idx == 2 || idx == 3) && hash(ai as i32, ring as i32, s ^ 0xC17) < 0.09 && detail > 0.45 {
                t |= CITY;
            }
            if look.glow.is_some() && idx <= 1 {
                t |= GLOW;
            }
            tex[ring * GW + ai] = t;
            if look.cloud_cover > 0.0 {
                let [x, y, z] = p;
                let w = fbm3(x * 2.0, y * 2.0, z * 2.0, 3, 0.5, s ^ 0xC10D) - 0.5;
                let c = fbm3(x * 3.2 + w * 1.5, y * 3.2 - w, z * 3.2 + w, 5, 0.55, s ^ 0xC1);
                let v = ((c - (1.0 - look.cloud_cover) * 0.75 - 0.2) * 6.0).clamp(0.0, 1.0);
                cloud[ring * GW + ai] = (v * 255.0) as u8;
            }
        }
    }
    Globe { tex, cloud, gas }
}
