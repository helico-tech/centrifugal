//! Scripted pilots for tests, balance checks and screenshots: they press
//! the same controls a player does.

use crate::engine::v2::V2;
use crate::game::flight::{Auto, Flight, Sas, Status};
use crate::game::mission::{self, apsides};
use crate::sim::predict::Node;
use std::f64::consts::FRAC_PI_2;

/// Gravity-turn ascent to a low orbit, then a planned circularization burn.
pub struct Ascent {
    pub target_ap: f64,
    phase: u8,
}

impl Ascent {
    pub fn new(target_ap: f64) -> Self {
        Ascent { target_ap, phase: 0 }
    }

    pub fn done(&self) -> bool {
        self.phase >= 3
    }

    pub fn step(&mut self, f: &mut Flight) {
        let dom = f.dominant();
        let alt = f.alt_datum();
        let up = f.up();
        match self.phase {
            0 => {
                // straight up, then tip east (clockwise, to the right)
                f.set_throttle(1.0);
                let k = ((alt - 300.0) / 7000.0).clamp(0.0, 1.0).powf(0.55);
                let pitch = k * 1.35;
                let want = up.angle() - pitch;
                f.sas = Sas::Off;
                f.craft.heading = want;
                if !f.craft.vehicle.can_thrust() && f.craft.vehicle.stages.len() > 1 {
                    f.stage();
                }
                if apsides(f, dom).is_some_and(|(_, ap)| ap > self.target_ap) {
                    f.set_throttle(0.0);
                    self.phase = 1;
                }
            }
            1 => {
                // above the air: plan the circularization at apoapsis
                if f.in_air() {
                    // drag eats the apoapsis: top it up
                    if apsides(f, dom).is_some_and(|(_, ap)| ap < self.target_ap) {
                        f.sas = Sas::Prograde;
                        f.set_throttle(0.3);
                    } else {
                        f.set_throttle(0.0);
                    }
                    return;
                }
                f.set_throttle(0.0);
                if let Some((t, _)) = f.pred.apsis(dom, true) {
                    let st = f.sys.states_at(t);
                    if let Some(p) = f.pred.state_at(t) {
                        let b = &f.sys.bodies[dom];
                        let r = (p.pos - st[dom].pos).len();
                        let v = (p.vel - st[dom].vel).len();
                        let dv = (b.gm / r).sqrt() - v;
                        f.nodes.clear();
                        f.nodes.push(Node { t, pro: dv, rad: 0.0 });
                        f.pred_dirty = true;
                        f.auto = Auto::Node;
                        self.phase = 2;
                    }
                }
            }
            2 => {
                if f.nodes.is_empty() {
                    self.phase = 3;
                }
                if !f.craft.vehicle.can_thrust() && f.craft.vehicle.stages.len() > 1 && f.craft.throttle > 0.0 {
                    f.stage();
                }
            }
            _ => {}
        }
        let _ = (V2::ZERO, FRAC_PI_2, Status::Flying, mission::MISSIONS.len());
    }
}

/// Hover-style lander: fly up to `peak`, then descend onto the site.
pub struct Hop {
    pub peak: f64,
    up: bool,
}

impl Hop {
    pub fn new(peak: f64) -> Self {
        Hop { peak, up: true }
    }
    pub fn step(&mut self, f: &mut Flight, tick: u32) {
        let alt = f.altitude();
        let vs = f.surface_vel().dot(f.up());
        let hs = f.surface_vel().dot(f.up().perp());
        let dom = f.dominant();
        let b = &f.sys.bodies[dom];
        let g = b.surface_g();
        let acc = f.craft.vehicle.max_thrust() / f.craft.vehicle.mass().max(1.0);
        if self.up {
            f.set_throttle(1.0);
            if alt > self.peak * 0.75 || vs * vs / (2.0 * g) + alt > self.peak {
                self.up = false;
            }
        } else {
            // descend no faster than half the engine's braking can stop
            let a_net = (acc - g).max(0.5);
            let want = -(a_net * alt.max(0.0) * 0.6).sqrt().clamp(1.5, 120.0);
            f.set_throttle(((g + (want - vs) * 2.0) / acc).clamp(0.0, 1.0));
        }
        // stay upright, lean against sideways drift
        f.sas = Sas::Off;
        f.craft.heading = f.up().angle() - (hs * 0.02).clamp(-0.2, 0.2);
        let _ = tick;
    }
}

/// Search node times over one orbit for a transfer that enters `target`'s
/// sphere with a periapsis near `want_pe`. Returns the best node.
pub fn plan_transfer(f: &Flight, target: usize, want_pe: f64) -> Option<Node> {
    use crate::sim::predict::{Setup, predict};
    let dom = f.dominant();
    let sys = &f.sys;
    let b = &sys.bodies[dom];
    let tb = &sys.bodies[target];
    let (p, v) = f.pos_vel();
    let r1 = (p - f.st[dom].pos).len();
    let v1 = (v - f.st[dom].vel).len();
    let period = std::f64::consts::TAU * (r1.powi(3) / b.gm).sqrt();
    // aim the far point at the target's orbit (or the target's distance from the star)
    let r2 = if tb.parent == Some(dom) { tb.orbit.a } else { 0.0 };
    let base_dv = if r2 > 0.0 {
        (b.gm * (2.0 / r1 - 2.0 / (r1 + r2))).sqrt() - v1
    } else {
        // escape with the right excess speed (Hohmann around the star)
        let star = sys.bodies[0].gm;
        let (ra, rb) = (b.orbit.a, tb.orbit.a);
        let a = (ra + rb) * 0.5;
        let v_inf = ((star * (2.0 / ra - 1.0 / a)).sqrt() - (star / ra).sqrt()).abs();
        (v_inf * v_inf + 2.0 * b.gm / r1).sqrt() - v1
    };
    let v = &f.craft.vehicle;
    let mut best: Option<(f64, Node)> = None;
    let steps = 90;
    for k in 0..steps {
        let t = f.t + 30.0 + period * k as f64 / steps as f64;
        for dk in [-0.04, -0.02, 0.0, 0.02, 0.04, 0.07] {
            let n = Node { t, pro: base_dv * (1.0 + dk), rad: 0.0 };
            let set = Setup {
                cda: f.craft.cda(),
                thrust: v.max_thrust(),
                mdot: v.mdot(),
                fuel: v.active().map(|s| s.fuel).unwrap_or(0.0),
                nodes: &[n],
                target: Some(target),
                max_points: 5000,
                horizon: 6.0e6,
            };
            let pred = predict(sys, f.craft.ship, &set);
            let score = score(&pred, target, want_pe);
            if best.as_ref().is_none_or(|(s, _)| score < *s) {
                best = Some((score, n));
            }
        }
    }
    best.map(|(_, n)| n)
}

/// How far a predicted path is from meeting `target` at periapsis `want_pe`.
pub fn score(pred: &crate::sim::predict::Prediction, target: usize, want_pe: f64) -> f64 {
    use crate::sim::predict::Event;
    let mut entered = false;
    for e in &pred.events {
        match *e {
            Event::Soi { body, .. } if body == target => entered = true,
            Event::Periapsis { body, alt, .. } if body == target && entered => {
                return (alt - want_pe).abs() + if alt < 0.0 { 1e6 } else { 0.0 };
            }
            Event::Impact { body, .. } if body == target => return 1e6,
            _ => {}
        }
    }
    pred.events.iter().find_map(|e| match *e { Event::Closest { dist, .. } => Some(1e7 + dist), _ => None }).unwrap_or(1e12)
}

/// Small burn soon that best fixes the encounter.
pub fn plan_correction(f: &Flight, target: usize, want_pe: f64) -> Option<(f64, Node)> {
    use crate::sim::predict::{Setup, predict};
    let v = &f.craft.vehicle;
    let mut best: Option<(f64, Node)> = None;
    let t = f.t + 40.0;
    for pi in -10..=10 {
        for ri in -5..=5 {
            let n = Node { t, pro: pi as f64 * 1.5, rad: ri as f64 * 3.0 };
            let set = Setup {
                cda: f.craft.cda(),
                thrust: v.max_thrust(),
                mdot: v.mdot(),
                fuel: v.active().map(|s| s.fuel).unwrap_or(0.0),
                nodes: &[n],
                target: Some(target),
                max_points: 5000,
                horizon: 6.0e6,
            };
            let pred = predict(&f.sys, f.craft.ship, &set);
            let sc = score(&pred, target, want_pe) + n.dv() * 20.0;
            if best.as_ref().is_none_or(|(s, _)| sc < *s) {
                best = Some((sc, n));
            }
        }
    }
    best
}

/// Fly from low orbit into an orbit around `target`.
pub struct Transfer {
    pub target: usize,
    pub want_pe: f64,
    phase: u8,
    corrections: u8,
    wait: u32,
}

impl Transfer {
    pub fn new(target: usize, want_pe: f64) -> Self {
        Transfer { target, want_pe, phase: 0, corrections: 0, wait: 0 }
    }
    pub fn done(&self) -> bool {
        self.phase >= 4
    }
    pub fn step(&mut self, f: &mut Flight) {
        match self.phase {
            0 => {
                f.target = Some(self.target);
                if let Some(n) = plan_transfer(f, self.target, self.want_pe) {
                    f.nodes = vec![n];
                    f.pred_dirty = true;
                    f.auto = Auto::Node;
                    self.phase = 1;
                }
            }
            1 => {
                if f.nodes.is_empty() {
                    self.phase = 2;
                }
            }
            2 => {
                // coast until inside the target's sphere, then warp to periapsis
                if f.dominant() == self.target {
                    if let Some((t, _)) = f.pred.apsis(self.target, false) {
                        let st = f.sys.states_at(t);
                        if let Some(p) = f.pred.state_at(t) {
                            let b = &f.sys.bodies[self.target];
                            let r = (p.pos - st[self.target].pos).len();
                            let v = (p.vel - st[self.target].vel).len();
                            f.nodes = vec![Node { t, pro: (b.gm / r).sqrt() - v, rad: 0.0 }];
                            f.pred_dirty = true;
                            f.auto = Auto::Node;
                            self.phase = 3;
                        }
                    }
                } else if !f.nodes.is_empty() {
                    // a correction is being flown
                } else {
                    self.wait += 1;
                    let off = score(&f.pred, self.target, self.want_pe);
                    if self.corrections < 3 && self.wait > 30 && off > self.want_pe * 0.5 + 800.0 {
                        if let Some((sc, n)) = plan_correction(f, self.target, self.want_pe) {
                            if sc < off {
                                f.nodes = vec![n];
                                f.pred_dirty = true;
                                f.auto = Auto::Node;
                            }
                        }
                        self.corrections += 1;
                        self.wait = 0;
                    } else if f.warp_to.is_none() {
                        f.warp = (f.warp + 1).min(f.max_warp());
                    }
                }
            }
            3 => {
                if f.nodes.is_empty() {
                    self.phase = 4;
                }
            }
            _ => {}
        }
    }
}

/// From a low orbit: brake against the ground speed, then come down
/// vertically with a suicide-burn style profile.
pub struct Descent {
    braking: bool,
}

impl Descent {
    pub fn new() -> Self {
        Descent { braking: true }
    }
    pub fn step(&mut self, f: &mut Flight) {
        let up = f.up();
        let sv = f.surface_vel();
        let vs = sv.dot(up);
        let hs = sv.dot(up.perp());
        let alt = f.altitude();
        let dom = f.dominant();
        let g = f.sys.bodies[dom].surface_g();
        let acc = f.craft.vehicle.max_thrust() / f.craft.vehicle.mass().max(1.0);
        f.sas = Sas::Off;
        if self.braking {
            // point against the ground speed and burn it off
            f.craft.heading = (-sv).angle();
            f.set_throttle(1.0);
            if hs.abs() < 8.0 {
                self.braking = false;
            }
            return;
        }
        let a_net = (acc - g).max(0.3);
        let want = -(a_net * alt.max(0.0) * 0.6).sqrt().clamp(1.5, 150.0);
        f.set_throttle(((g + (want - vs) * 2.0) / acc).clamp(0.0, 1.0));
        f.craft.heading = up.angle() - (hs * 0.05).clamp(-0.4, 0.4);
    }
}

impl Default for Descent {
    fn default() -> Self {
        Self::new()
    }
}
