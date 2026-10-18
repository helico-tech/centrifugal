//! Surface heights around a body's equator (the 2D world's ground), from
//! the same elevation field that paints the planet, so the side view and
//! the globe agree.

use std::f64::consts::TAU;

#[derive(Clone, Debug, Default)]
pub struct Terrain {
    /// Height above the datum radius (m) at evenly spaced angles; the sea
    /// floor is negative.
    pub h: Vec<f32>,
}

impl Terrain {
    pub fn flat() -> Self {
        Terrain { h: vec![0.0; 16] }
    }

    fn idx(&self, a: f64) -> (usize, usize, f64) {
        let n = self.h.len();
        let u = a.rem_euclid(TAU) / TAU * n as f64;
        let i = (u.floor() as usize) % n;
        (i, (i + 1) % n, u.fract())
    }

    /// Height at body-fixed angle `a` (rad).
    pub fn height(&self, a: f64) -> f64 {
        let (i, j, f) = self.idx(a);
        self.h[i] as f64 * (1.0 - f) + self.h[j] as f64 * f
    }

    /// Ground slope at `a`: rise per meter along the surface, for a datum radius `r`.
    pub fn slope(&self, a: f64, r: f64) -> f64 {
        let da = 4.0 / r;
        (self.height(a + da).max(0.0) - self.height(a - da).max(0.0)) / 8.0
    }

    /// Flatten a pad of `half` meters each side of angle `a` (datum radius
    /// `r`) to the average height there, blending into the land around it.
    pub fn flatten(&mut self, a: f64, half: f64, r: f64, min_h: f32) {
        let n = self.h.len();
        let per = TAU / n as f64 * r;
        let k = (half / per).ceil() as i64;
        let blend = k * 2 + 4;
        let c = (a.rem_euclid(TAU) / TAU * n as f64).round() as i64;
        let at = |o: i64| (c + o).rem_euclid(n as i64) as usize;
        let level = ((-k..=k).map(|o| self.h[at(o)]).sum::<f32>() / (2 * k + 1) as f32).max(min_h);
        for o in -(k + blend)..=(k + blend) {
            let i = at(o);
            let d = o.abs();
            let t = if d <= k { 1.0 } else { 1.0 - (d - k) as f32 / blend as f32 };
            let t = t * t * (3.0 - 2.0 * t);
            self.h[i] = self.h[i] * (1.0 - t) + level * t;
        }
    }
}
