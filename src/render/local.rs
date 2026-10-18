//! Drawing in a rotated local frame (meters, x right, y up), mapped onto
//! the screen. Rockets, trees and buildings use it, so they stand upright
//! on any part of a planet and scale with the zoom.

use super::camera::Camera;
use crate::engine::gfx::Frame;
use crate::engine::v2::V2;

#[derive(Clone, Copy)]
pub struct Local {
    pub ox: f64,
    pub oy: f64,
    /// Screen pixels per local meter along local x and y.
    pub rx: (f64, f64),
    pub ry: (f64, f64),
}

impl Local {
    /// Frame at world point `origin` with local up along world angle `up`.
    pub fn new(cam: &Camera, origin: V2, up: f64) -> Self {
        let (ox, oy) = cam.to_screen(origin);
        let ry = cam.dir(V2::from_angle(up));
        let rx = cam.dir(V2::from_angle(up - std::f64::consts::FRAC_PI_2));
        Local { ox, oy, rx, ry }
    }

    /// Screen pixels per meter.
    pub fn scale(&self) -> f64 {
        self.ry.0.hypot(self.ry.1)
    }

    #[inline]
    pub fn pt(&self, x: f64, y: f64) -> (i32, i32) {
        (
            (self.ox + self.rx.0 * x + self.ry.0 * y).floor() as i32,
            (self.oy + self.rx.1 * x + self.ry.1 * y).floor() as i32,
        )
    }

    pub fn ptf(&self, x: f64, y: f64) -> (f64, f64) {
        (self.ox + self.rx.0 * x + self.ry.0 * y, self.oy + self.rx.1 * x + self.ry.1 * y)
    }

    /// Filled polygon; with `pad` > 0 it is first drawn grown by `pad`
    /// pixels in `outline`, which gives every shape the house outline.
    pub fn poly(&self, f: &mut Frame, pts: &[(f64, f64)], c: u32) {
        let p: Vec<(i32, i32)> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        if p.len() >= 3 {
            f.fill_poly(&p, c);
        }
    }

    pub fn quad(&self, f: &mut Frame, x0: f64, y0: f64, x1: f64, y1: f64, c: u32) {
        self.poly(f, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], c);
    }

    /// Outline pass: the polygon grown by one screen pixel all around.
    pub fn poly_outline(&self, f: &mut Frame, pts: &[(f64, f64)], c: u32) {
        let p: Vec<(i32, i32)> = pts.iter().map(|&(x, y)| self.pt(x, y)).collect();
        if p.len() < 3 {
            return;
        }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let q: Vec<(i32, i32)> = p.iter().map(|&(x, y)| (x + dx, y + dy)).collect();
            f.fill_poly(&q, c);
        }
    }

    pub fn quad_outline(&self, f: &mut Frame, x0: f64, y0: f64, x1: f64, y1: f64, c: u32) {
        self.poly_outline(f, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], c);
    }

    pub fn disc(&self, f: &mut Frame, x: f64, y: f64, r: f64, c: u32) {
        let (sx, sy) = self.pt(x, y);
        let rr = (r * self.scale()).round().max(1.0) as i32;
        f.fill_ellipse(sx, sy, rr, rr, c);
    }

    pub fn line(&self, f: &mut Frame, x0: f64, y0: f64, x1: f64, y1: f64, c: u32) {
        let (a, b) = self.pt(x0, y0);
        let (cx, cy) = self.pt(x1, y1);
        f.line(a, b, cx, cy, c);
    }
}
