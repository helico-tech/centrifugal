//! Trajectory prediction: fly a copy of the ship forward with the same
//! step function as the real flight, including planned maneuver nodes, and
//! note the events a pilot plans with (Ap/Pe, encounters, closest
//! approach, impact).

use crate::engine::v2::V2;
use crate::sim::physics::{Drive, Ship, coast_dt, step};
use crate::sim::system::{BodyState, System};
use std::f64::consts::TAU;

/// A planned burn: at time `t`, change velocity by `pro` m/s along the
/// direction of travel and `rad` m/s away from the body it orbits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub t: f64,
    pub pro: f64,
    pub rad: f64,
}

impl Node {
    pub fn dv(&self) -> f64 {
        self.pro.hypot(self.rad)
    }
}

/// Prograde and radial-out unit vectors of a state around body `b`.
pub fn frame(pos: V2, vel: V2, bs: &BodyState) -> (V2, V2) {
    let rel_v = vel - bs.vel;
    let pro = if rel_v.len2() > 1e-9 { rel_v.norm() } else { (pos - bs.pos).norm().perp() };
    let mut rad = pro.perp();
    if rad.dot(pos - bs.pos) < 0.0 {
        rad = -rad;
    }
    (pro, rad)
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub t: f64,
    pub pos: V2,
    pub vel: V2,
    /// Dominant body here.
    pub dom: usize,
    /// Planned burn in progress.
    pub burn: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Periapsis { t: f64, body: usize, alt: f64 },
    Apoapsis { t: f64, body: usize, alt: f64 },
    /// Entering `body`'s sphere of influence (or leaving `from`'s).
    Soi { t: f64, body: usize, from: usize },
    Impact { t: f64, body: usize, speed: f64 },
    /// Closest approach to the target body.
    Closest { t: f64, body: usize, dist: f64 },
    Node { t: f64, index: usize },
    AtmoEnter { t: f64, body: usize },
}

impl Event {
    pub fn t(&self) -> f64 {
        match *self {
            Event::Periapsis { t, .. }
            | Event::Apoapsis { t, .. }
            | Event::Soi { t, .. }
            | Event::Impact { t, .. }
            | Event::Closest { t, .. }
            | Event::Node { t, .. }
            | Event::AtmoEnter { t, .. } => t,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Prediction {
    pub points: Vec<Point>,
    pub events: Vec<Event>,
    /// Index of the first point after which the path is no longer
    /// trustworthy (after a close encounter).
    pub fade_from: Option<usize>,
}

impl Prediction {
    pub fn impact(&self) -> Option<(f64, usize, f64)> {
        self.events.iter().find_map(|e| match *e {
            Event::Impact { t, body, speed } => Some((t, body, speed)),
            _ => None,
        })
    }
    /// Next periapsis/apoapsis around `body`.
    pub fn apsis(&self, body: usize, apo: bool) -> Option<(f64, f64)> {
        self.events.iter().find_map(|e| match *e {
            Event::Apoapsis { t, body: b, alt } if apo && b == body => Some((t, alt)),
            Event::Periapsis { t, body: b, alt } if !apo && b == body => Some((t, alt)),
            _ => None,
        })
    }
    pub fn state_at(&self, t: f64) -> Option<Point> {
        let i = self.points.partition_point(|p| p.t <= t);
        if i == 0 || i >= self.points.len() {
            return None;
        }
        let (a, b) = (&self.points[i - 1], &self.points[i]);
        let (pos, vel) = hermite(a.t, a.pos, a.vel, b.t, b.pos, b.vel, t);
        Some(Point { t, pos, vel, dom: a.dom, burn: a.burn })
    }
}

/// Cubic Hermite interpolation of a state between two steps.
pub fn hermite(t0: f64, p0: V2, v0: V2, t1: f64, p1: V2, v1: V2, t: f64) -> (V2, V2) {
    let h = t1 - t0;
    if h <= 0.0 {
        return (p1, v1);
    }
    let s = ((t - t0) / h).clamp(0.0, 1.0);
    let (s2, s3) = (s * s, s * s * s);
    let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
    let h10 = s3 - 2.0 * s2 + s;
    let h01 = -2.0 * s3 + 3.0 * s2;
    let h11 = s3 - s2;
    let p = p0 * h00 + v0 * (h10 * h) + p1 * h01 + v1 * (h11 * h);
    let d00 = (6.0 * s2 - 6.0 * s) / h;
    let d10 = 3.0 * s2 - 4.0 * s + 1.0;
    let d01 = (-6.0 * s2 + 6.0 * s) / h;
    let d11 = 3.0 * s2 - 2.0 * s;
    let v = p0 * d00 + v0 * d10 + p1 * d01 + v1 * d11;
    (p, v)
}

pub struct Setup<'a> {
    pub cda: f64,
    /// Thrust (N) and fuel use (kg/s) for planned burns, and fuel left (kg).
    pub thrust: f64,
    pub mdot: f64,
    pub fuel: f64,
    pub nodes: &'a [Node],
    pub target: Option<usize>,
    pub max_points: usize,
    /// Stop after this much time at most (s).
    pub horizon: f64,
}

/// Burn time for `dv` with the given engine and mass.
pub fn burn_time(dv: f64, mass: f64, thrust: f64, mdot: f64) -> f64 {
    if thrust <= 0.0 || mdot <= 0.0 {
        return f64::INFINITY;
    }
    let ve = thrust / mdot;
    let m1 = mass * (-dv / ve).exp();
    (mass - m1) / mdot
}

pub fn predict(sys: &System, start: Ship, set: &Setup) -> Prediction {
    let mut out = Prediction::default();
    let mut scratch = Vec::with_capacity(sys.bodies.len());
    let mut st = Vec::with_capacity(sys.bodies.len());
    let mut s = start;
    sys.states(s.t, &mut st);
    let mut dom = sys.dominant(s.pos, &st);
    out.points.push(Point { t: s.t, pos: s.pos, vel: s.vel, dom, burn: false });

    let mut sweep = 0.0;
    let mut last_ang = (s.pos - st[dom].pos).angle();
    let mut segments = 0;
    // radial speed sign around the dominant body, for apsides
    let radial = |s: &Ship, st: &[BodyState], d: usize| (s.pos - st[d].pos).dot(s.vel - st[d].vel);
    let mut last_rad = radial(&s, &st, dom);
    let mut closest: Option<(f64, f64)> = None;
    let mut last_dist = f64::INFINITY;
    let mut approaching = false;
    let mut fuel = set.fuel;
    let mut node_i = 0;
    let mut burn_dir: Option<V2> = None;
    let mut in_air = false;
    let mut nodes: Vec<Node> = set.nodes.iter().copied().filter(|n| n.t >= s.t - 1.0).collect();
    nodes.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());

    while out.points.len() < set.max_points && s.t - start.t < set.horizon {
        // planned burn: centred on the node time, fixed direction
        let mut drive = Drive { cda: set.cda, ..Default::default() };
        let mut burning = false;
        if let Some(n) = nodes.get(node_i) {
            let bt = burn_time(n.dv(), s.mass, set.thrust, set.mdot).min(n.dv() * 10.0);
            let t0 = n.t - bt * 0.5;
            if s.t >= t0 - 1e-9 {
                if out.events.iter().all(|e| !matches!(e, Event::Node { index, .. } if *index == node_i)) {
                    out.events.push(Event::Node { t: n.t, index: node_i });
                }
                // the burn holds one direction: the node's, at burn start
                let dir = *burn_dir.get_or_insert_with(|| {
                    let (pro, rad) = frame(s.pos, s.vel, &st[dom]);
                    (pro * n.pro + rad * n.rad).norm()
                });
                if bt.is_finite() && bt > 0.5 && fuel > 0.0 {
                    let left = t0 + bt - s.t;
                    if left > 0.0 {
                        let dt = left.min(0.25);
                        drive.thrust = set.thrust;
                        drive.mdot = set.mdot;
                        drive.dir = dir;
                        burning = true;
                        step(sys, &mut scratch, &mut s, dt, &drive);
                        fuel -= set.mdot * dt;
                    } else {
                        node_i += 1;
                        burn_dir = None;
                    }
                } else {
                    s.vel += dir * n.dv();
                    node_i += 1;
                    burn_dir = None;
                }
            }
        }
        if !burning {
            let mut dt = coast_dt(sys, &st, &s);
            // land exactly on the next node's burn start
            if let Some(n) = nodes.get(node_i) {
                let bt = burn_time(n.dv(), s.mass, set.thrust, set.mdot).min(n.dv() * 10.0);
                let t0 = n.t - if bt.is_finite() && bt > 0.5 { bt * 0.5 } else { 0.0 };
                if t0 > s.t && t0 < s.t + dt {
                    dt = t0 - s.t;
                }
            }
            step(sys, &mut scratch, &mut s, dt.max(1e-6), &drive);
        }
        sys.states(s.t, &mut st);

        // what is under us
        let new_dom = sys.dominant(s.pos, &st);
        if new_dom != dom {
            out.events.push(Event::Soi { t: s.t, body: new_dom, from: dom });
            if sys.is_ancestor(dom, new_dom) && out.fade_from.is_none() {
                // fly-bys are chaotic: trust the path less after the first
                out.fade_from = Some(out.points.len() + 40);
            }
            dom = new_dom;
            segments += 1;
            sweep = 0.0;
            last_ang = (s.pos - st[dom].pos).angle();
            last_rad = radial(&s, &st, dom);
            if segments > 4 {
                break;
            }
        }
        let b = &sys.bodies[dom];
        let rel = s.pos - st[dom].pos;
        let r = rel.len();
        let body_ang = rel.angle() - b.spin_angle(s.t);
        let ground = b.surface_r(body_ang);
        if b.kind.solid() && r <= ground || !b.kind.solid() && r <= b.radius {
            let wind = st[dom].vel + rel.perp() * b.spin;
            out.points.push(Point { t: s.t, pos: s.pos, vel: s.vel, dom, burn: burning });
            out.events.push(Event::Impact { t: s.t, body: dom, speed: (s.vel - wind).len() });
            break;
        }
        if let Some(a) = &b.atmo {
            let air = r - b.radius < a.height;
            if air && !in_air && out.points.len() > 1 {
                out.events.push(Event::AtmoEnter { t: s.t, body: dom });
            }
            in_air = air;
        }
        let rad = radial(&s, &st, dom);
        if last_rad < 0.0 && rad >= 0.0 {
            out.events.push(Event::Periapsis { t: s.t, body: dom, alt: r - b.radius });
        } else if last_rad > 0.0 && rad <= 0.0 && dom != 0 && r < b.soi {
            out.events.push(Event::Apoapsis { t: s.t, body: dom, alt: r - b.radius });
        } else if last_rad > 0.0 && rad <= 0.0 && dom == 0 {
            out.events.push(Event::Apoapsis { t: s.t, body: 0, alt: r - b.radius });
        }
        last_rad = rad;

        if let Some(tg) = set.target {
            let dist = (s.pos - st[tg].pos).len();
            if dist < last_dist {
                approaching = true;
            } else if approaching {
                approaching = false;
                if closest.is_none_or(|(_, d)| last_dist < d) {
                    closest = Some((s.t, last_dist));
                }
            }
            last_dist = dist;
        }

        out.points.push(Point { t: s.t, pos: s.pos, vel: s.vel, dom, burn: burning });

        // one full turn around the same body (with no burns left) is enough
        let ang = rel.angle();
        let mut da = ang - last_ang;
        if da > TAU / 2.0 {
            da -= TAU;
        } else if da < -TAU / 2.0 {
            da += TAU;
        }
        sweep += da;
        last_ang = ang;
        if sweep.abs() > TAU * 1.02 && node_i >= nodes.len() {
            break;
        }
    }
    if let (Some(tg), Some((t, dist))) = (set.target, closest) {
        out.events.push(Event::Closest { t, body: tg, dist });
    }
    out
}
