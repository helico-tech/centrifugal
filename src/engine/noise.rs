//! Value noise for procedural planets: hashed lattice values, smoothly
//! interpolated, summed over octaves (fbm). Deterministic per seed.

#[inline]
fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F)
        ^ seed.wrapping_mul(0x9E37_79B9);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Integer hash to 0..1, for scattering things (trees, craters, stars).
pub fn hash(a: i32, b: i32, seed: u32) -> f32 {
    hash3(a, b, 0x51ED, seed)
}

#[inline]
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 3D value noise in 0..1.
pub fn noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (fx, fy, fz) = (smooth(x - xi as f32), smooth(y - yi as f32), smooth(z - zi as f32));
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c = |dx, dy, dz| hash3(xi + dx, yi + dy, zi + dz, seed);
    let x00 = l(c(0, 0, 0), c(1, 0, 0), fx);
    let x10 = l(c(0, 1, 0), c(1, 1, 0), fx);
    let x01 = l(c(0, 0, 1), c(1, 0, 1), fx);
    let x11 = l(c(0, 1, 1), c(1, 1, 1), fx);
    l(l(x00, x10, fy), l(x01, x11, fy), fz)
}

/// Fractal noise in roughly 0..1: `oct` octaves, each twice the frequency
/// and `gain` times the amplitude of the last.
pub fn fbm3(x: f32, y: f32, z: f32, oct: u32, gain: f32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..oct {
        sum += noise3(x * f, y * f, z * f, seed.wrapping_add(o * 101)) * amp;
        norm += amp;
        amp *= gain;
        f *= 2.03;
    }
    sum / norm
}

/// Ridged fractal noise (sharp crests), roughly 0..1: mountain ranges.
pub fn ridged3(x: f32, y: f32, z: f32, oct: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..oct {
        let n = 1.0 - (noise3(x * f, y * f, z * f, seed.wrapping_add(o * 131)) * 2.0 - 1.0).abs();
        sum += n * n * amp;
        norm += amp;
        amp *= 0.5;
        f *= 2.1;
    }
    sum / norm
}
