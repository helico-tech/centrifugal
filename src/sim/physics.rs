//! Newtonian flight: gravity from every body (n-body, bodies on rails),
//! engine thrust and air drag, integrated with RK4.
//!
//! One rule keeps the predicted path honest: flight and prediction use the
//! same `step` with the same `step_dt`, and the step size depends only on
//! the state (never on the frame rate or time warp). So while coasting, the
//! drawn path is exactly the path the rocket will fly.

use crate::engine::v2::V2;
use crate::sim::system::{BodyState, System};

/// The part of a rocket the integrator moves.
#[derive(Clone, Copy, Debug)]
pub struct Ship {
    pub t: f64,
    pub pos: V2,
    pub vel: V2,
    pub mass: f64,
}

/// Engine and air for one step: constant during the step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Drive {
    /// Thrust (N) along `dir` (unit vector).
    pub thrust: f64,
    pub dir: V2,
    /// Fuel use (kg/s) at this thrust.
    pub mdot: f64,
    /// Drag coefficient times area (m2).
    pub cda: f64,
}

pub const TICK: f64 = 1.0 / 60.0;

#[inline]
fn pull(gm: f64, radius: f64, from: V2, at: V2) -> V2 {
    let d = from - at;
    let r2 = d.len2();
    // inside a body (only while crashing): don't blow up
    let r3 = r2.max(radius * radius * 0.25) * r2.sqrt();
    d * (gm / r3)
}

/// Gravity at `p` from all bodies, seen from the frame of the planet (or
/// star) whose neighborhood `p` is in.
///
/// Bodies are on rails, so a planet never feels its moons pull it. The
/// "indirect term" fixes that: subtract the pull the frame body would feel
/// from the bodies below it. Without it a moon drags low orbits around.
pub fn gravity(sys: &System, st: &[BodyState], p: V2) -> V2 {
    let mut a = V2::ZERO;
    for (b, s) in sys.bodies.iter().zip(st) {
        a += pull(b.gm, b.radius, s.pos, p);
    }
    let mut frame = sys.dominant(p, st);
    while let Some(parent) = sys.bodies[frame].parent {
        if parent == 0 {
            break;
        }
        frame = parent;
    }
    for (i, b) in sys.bodies.iter().enumerate() {
        if i != frame && (frame == 0 || sys.is_ancestor(frame, i)) {
            a -= pull(b.gm, b.radius, st[i].pos, st[frame].pos);
        }
    }
    a
}

/// Air density and the air's velocity at `p` (air turns with its planet).
pub fn air(sys: &System, st: &[BodyState], t: f64, p: V2) -> Option<(f64, V2)> {
    for (b, s) in sys.bodies.iter().zip(st) {
        let Some(atmo) = &b.atmo else { continue };
        let d = p - s.pos;
        let r = d.len();
        if r < b.radius + atmo.height {
            let alt = r - b.surface_r(d.angle() - b.spin_angle(t));
            let rho = atmo.density(alt.max(0.0));
            return Some((rho, s.vel + d.perp() * b.spin));
        }
    }
    None
}

pub fn accel(sys: &System, scratch: &mut Vec<BodyState>, t: f64, p: V2, v: V2, m: f64, d: &Drive) -> V2 {
    sys.states(t, scratch);
    let mut a = gravity(sys, scratch, p);
    if d.thrust > 0.0 {
        a += d.dir * (d.thrust / m);
    }
    if d.cda > 0.0 {
        if let Some((rho, wind)) = air(sys, scratch, t, p) {
            let rel = v - wind;
            a -= rel * (0.5 * rho * rel.len() * d.cda / m);
        }
    }
    a
}

/// One RK4 step of `dt` seconds.
pub fn step(sys: &System, scratch: &mut Vec<BodyState>, s: &mut Ship, dt: f64, d: &Drive) {
    let (t, p, v, m) = (s.t, s.pos, s.vel, s.mass);
    let mass = |h: f64| (m - d.mdot * h).max(1.0);
    let a1 = accel(sys, scratch, t, p, v, mass(0.0), d);
    let (p2, v2) = (p + v * (dt * 0.5), v + a1 * (dt * 0.5));
    let a2 = accel(sys, scratch, t + dt * 0.5, p2, v2, mass(dt * 0.5), d);
    let (p3, v3) = (p + v2 * (dt * 0.5), v + a2 * (dt * 0.5));
    let a3 = accel(sys, scratch, t + dt * 0.5, p3, v3, mass(dt * 0.5), d);
    let (p4, v4) = (p + v3 * dt, v + a3 * dt);
    let a4 = accel(sys, scratch, t + dt, p4, v4, mass(dt), d);
    s.pos = p + (v + v2 * 2.0 + v3 * 2.0 + v4) * (dt / 6.0);
    s.vel = v + (a1 + a2 * 2.0 + a3 * 2.0 + a4) * (dt / 6.0);
    s.mass = mass(dt);
    s.t = t + dt;
}

/// Step size for a coasting ship: a small fraction of the shortest orbital
/// time scale around any body, smaller still near the ground and in air.
pub fn coast_dt(sys: &System, st: &[BodyState], s: &Ship) -> f64 {
    let mut tau = f64::INFINITY;
    let mut near = f64::INFINITY;
    let mut rel_speed = 0.0;
    let mut in_air = false;
    for (b, bs) in sys.bodies.iter().zip(st) {
        let d = s.pos - bs.pos;
        let r = d.len();
        tau = tau.min((r * r * r / b.gm).sqrt());
        let alt = r - b.radius;
        if alt < near {
            near = alt;
            rel_speed = (s.vel - bs.vel).len();
            in_air = b.atmo.as_ref().is_some_and(|a| alt < a.height * 1.05);
        }
    }
    let mut dt = 0.025 * tau;
    if in_air {
        dt = dt.min(0.05);
    }
    // never cross more than a tenth of the height above the ground in one step
    dt = dt.min((0.1 * near.max(50.0) / rel_speed.max(1.0)).max(0.05));
    dt.clamp(TICK, 1800.0)
}
