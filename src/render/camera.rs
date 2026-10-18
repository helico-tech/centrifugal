//! World-to-screen mapping: center, meters per pixel and a rotation, so
//! near a planet the local "up" can point up the screen.

use crate::engine::v2::{V2, v2};
use std::f64::consts::FRAC_PI_2;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub center: V2,
    /// Meters per screen pixel.
    pub mpp: f64,
    /// World direction (angle) that points up on screen.
    pub up: f64,
    pub w: f64,
    pub h: f64,
    sin: f64,
    cos: f64,
}

impl Camera {
    pub fn new(center: V2, mpp: f64, up: f64, w: i32, h: i32) -> Self {
        let (sin, cos) = (FRAC_PI_2 - up).sin_cos();
        Camera { center, mpp, up, w: w as f64, h: h as f64, sin, cos }
    }

    pub fn with(self, center: V2, mpp: f64, up: f64) -> Self {
        Camera::new(center, mpp, up, self.w as i32, self.h as i32)
    }

    /// Screen pixel coordinates (float) of a world point.
    #[inline]
    pub fn to_screen(&self, p: V2) -> (f64, f64) {
        let d = p - self.center;
        let x = d.x * self.cos - d.y * self.sin;
        let y = d.x * self.sin + d.y * self.cos;
        (self.w * 0.5 + x / self.mpp, self.h * 0.5 - y / self.mpp)
    }

    #[inline]
    pub fn to_screen_i(&self, p: V2) -> (i32, i32) {
        let (x, y) = self.to_screen(p);
        (x.floor() as i32, y.floor() as i32)
    }

    /// World point under a screen pixel coordinate.
    #[inline]
    pub fn to_world(&self, sx: f64, sy: f64) -> V2 {
        let x = (sx - self.w * 0.5) * self.mpp;
        let y = (self.h * 0.5 - sy) * self.mpp;
        // inverse rotation
        self.center + v2(x * self.cos + y * self.sin, -x * self.sin + y * self.cos)
    }

    /// Screen-space direction (dx, dy per meter) of a world direction.
    pub fn dir(&self, d: V2) -> (f64, f64) {
        let x = d.x * self.cos - d.y * self.sin;
        let y = d.x * self.sin + d.y * self.cos;
        (x / self.mpp, -y / self.mpp)
    }

    /// Screen angle (radians, y down) of a world direction angle.
    pub fn screen_angle(&self, world_angle: f64) -> f64 {
        let (dx, dy) = self.dir(V2::from_angle(world_angle));
        dy.atan2(dx)
    }

    pub fn visible(&self, p: V2, margin: f64) -> bool {
        let (x, y) = self.to_screen(p);
        x > -margin && y > -margin && x < self.w + margin && y < self.h + margin
    }

    /// Half the screen diagonal, in meters.
    pub fn radius_m(&self) -> f64 {
        (self.w.hypot(self.h)) * 0.5 * self.mpp
    }
}
