//! One flight: the rocket, its dropped stages, the clock and time warp,
//! attitude hold (SAS), maneuver nodes and the node autopilot. Pure game
//! state: the screen code in `app` turns input into calls here.

use crate::engine::v2::V2;
use crate::sim::physics::{Drive, Ship, TICK, coast_dt, step};
use crate::sim::predict::{self, Event, Node, Prediction, Setup, burn_time, frame};
use crate::sim::system::{BodyState, Kind, System};
use crate::sim::vehicle::{StageKind, Vehicle};
use std::f64::consts::{PI, TAU};
use std::rc::Rc;

pub const WARPS: [f64; 10] = [1.0, 2.0, 4.0, 10.0, 50.0, 100.0, 1_000.0, 10_000.0, 100_000.0, 1_000_000.0];
/// Warp levels allowed while burning, in air or near the ground.
pub const PHYSICS_WARP: usize = 2;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Status {
    Flying,
    /// Standing on body `body` at body-fixed angle `ang`, tilted by `tilt`.
    Landed { body: usize, ang: f64, tilt: f64 },
    Splashed { body: usize, ang: f64 },
    Crashed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sas {
    Off,
    Hold,
    Prograde,
    Retrograde,
    RadialOut,
    RadialIn,
    Node,
    Target,
}

impl Sas {
    pub fn label(self) -> &'static str {
        match self {
            Sas::Off => "FREE",
            Sas::Hold => "HOLD",
            Sas::Prograde => "PROGRADE",
            Sas::Retrograde => "RETROGRADE",
            Sas::RadialOut => "RADIAL OUT",
            Sas::RadialIn => "RADIAL IN",
            Sas::Node => "NODE",
            Sas::Target => "TARGET",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Craft {
    /// Integrator state at the last step, and the one before (the screen
    /// shows a smooth interpolation between them).
    pub ship: Ship,
    pub prev: Ship,
    /// World angle the nose points at.
    pub heading: f64,
    pub vehicle: Vehicle,
    pub throttle: f64,
    pub status: Status,
    pub legs: bool,
    pub chute: bool,
    /// Engine actually burning this tick (fuel and throttle).
    pub burning: bool,
    pub id: u32,
}

impl Craft {
    pub fn dir(&self) -> V2 {
        V2::from_angle(self.heading)
    }
    pub fn cda(&self) -> f64 {
        let w = self.vehicle.width();
        let base = 0.35 * w * w * 0.8;
        if self.chute { base + 600.0 } else { base }
    }
    pub fn drive(&self) -> Drive {
        let thrust = if self.vehicle.can_thrust() { self.vehicle.max_thrust() * self.throttle } else { 0.0 };
        Drive { thrust, dir: self.dir(), mdot: self.vehicle.mdot() * self.throttle, cda: self.cda() }
    }
}

/// Things that happened, for the mission checker, sounds and banners.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Log {
    Liftoff,
    Staged { kind: StageKind },
    Landed { body: usize, speed: f64, dist_to_site: f64 },
    Splashdown { body: usize, speed: f64 },
    Crashed { body: usize, speed: f64 },
    Soi { body: usize },
    Flameout,
    NodeDone,
    ChuteOpen,
    LegsDown,
    Debris { pos: V2 },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Auto {
    Off,
    /// Point at the node, warp close, burn it, stop.
    Node,
}

#[derive(Clone)]
pub struct Snapshot {
    t: f64,
    craft: Craft,
    debris: Vec<Craft>,
    nodes: Vec<Node>,
}

#[derive(Clone)]
pub struct Flight {
    pub sys: Rc<System>,
    pub t: f64,
    pub craft: Craft,
    pub debris: Vec<Craft>,
    pub warp: usize,
    pub warp_to: Option<f64>,
    pub sas: Sas,
    pub nodes: Vec<Node>,
    /// Remaining burn of the first node once it started (world vector).
    pub node_burn: Option<V2>,
    node_dir0: V2,
    pub auto: Auto,
    pub target: Option<usize>,
    pub pred: Prediction,
    pub log: Vec<Log>,
    pub st: Vec<BodyState>,
    pub launched: bool,
    pub max_alt: f64,
    /// Where the wreck lies: body and body-fixed angle.
    pub wreck: Option<(usize, f64)>,
    snaps: Vec<Snapshot>,
    snap_tick: u32,
    next_id: u32,
    scratch: Vec<BodyState>,
    pred_age: u32,
    pub pred_dirty: bool,
}

impl Flight {
    /// A rocket standing on a site of `body`.
    pub fn on_pad(sys: Rc<System>, vehicle: Vehicle, body: usize, ang: f64) -> Self {
        let mut st = Vec::new();
        sys.states(0.0, &mut st);
        let ship = Ship { t: 0.0, pos: V2::ZERO, vel: V2::ZERO, mass: vehicle.mass() };
        let craft = Craft {
            ship,
            prev: ship,
            heading: 0.0,
            vehicle,
            throttle: 0.0,
            status: Status::Landed { body, ang, tilt: 0.0 },
            legs: true,
            chute: false,
            burning: false,
            id: 1,
        };
        let mut f = Flight {
            sys,
            t: 0.0,
            craft,
            debris: vec![],
            warp: 0,
            warp_to: None,
            sas: Sas::Hold,
            nodes: vec![],
            node_burn: None,
            node_dir0: V2::ZERO,
            auto: Auto::Off,
            target: None,
            pred: Prediction::default(),
            log: vec![],
            st,
            launched: false,
            max_alt: 0.0,
            wreck: None,
            snaps: vec![],
            snap_tick: 0,
            next_id: 2,
            scratch: vec![],
            pred_age: 99,
            pred_dirty: true,
        };
        f.follow_ground();
        f.craft.prev = f.craft.ship;
        f
    }

    /// A rocket already flying with the given state.
    pub fn in_flight(sys: Rc<System>, vehicle: Vehicle, pos: V2, vel: V2, heading: f64, t: f64) -> Self {
        let mut f = Flight::on_pad(sys, vehicle, 0, 0.0);
        f.t = t;
        f.craft.status = Status::Flying;
        f.craft.ship = Ship { t, pos, vel, mass: f.craft.vehicle.mass() };
        f.craft.prev = f.craft.ship;
        f.craft.heading = heading;
        f.craft.legs = false;
        f.launched = true;
        f.sys.clone().states(t, &mut f.st);
        f
    }

    // ------------------------------------------------------------ readouts

    /// Smooth display position and velocity at the current time.
    pub fn pos_vel(&self) -> (V2, V2) {
        let c = &self.craft;
        if c.ship.t <= self.t + 1e-9 || c.prev.t >= c.ship.t {
            return (c.ship.pos, c.ship.vel);
        }
        predict::hermite(c.prev.t, c.prev.pos, c.prev.vel, c.ship.t, c.ship.pos, c.ship.vel, self.t)
    }

    pub fn dominant(&self) -> usize {
        match self.craft.status {
            Status::Landed { body, .. } | Status::Splashed { body, .. } => body,
            _ => self.sys.dominant(self.pos_vel().0, &self.st),
        }
    }

    /// Altitude above the ground (or sea) of the dominant body.
    pub fn altitude(&self) -> f64 {
        let d = self.dominant();
        let b = &self.sys.bodies[d];
        let rel = self.pos_vel().0 - self.st[d].pos;
        let h = self.craft.vehicle.height() * 0.5;
        rel.len() - b.surface_r(rel.angle() - b.spin_angle(self.t)) - h
    }

    pub fn alt_datum(&self) -> f64 {
        let d = self.dominant();
        (self.pos_vel().0 - self.st[d].pos).len() - self.sys.bodies[d].radius
    }

    /// Velocity relative to the dominant body's ground (air) and to its center.
    pub fn surface_vel(&self) -> V2 {
        let d = self.dominant();
        let (p, v) = self.pos_vel();
        let rel = p - self.st[d].pos;
        v - self.st[d].vel - rel.perp() * self.sys.bodies[d].spin
    }

    pub fn orbit_vel(&self) -> V2 {
        let d = self.dominant();
        self.pos_vel().1 - self.st[d].vel
    }

    pub fn up(&self) -> V2 {
        let d = self.dominant();
        (self.pos_vel().0 - self.st[d].pos).norm()
    }

    /// Low over a planet, steering is about the ground, not the orbit.
    pub fn surface_mode(&self) -> bool {
        let d = self.dominant();
        let b = &self.sys.bodies[d];
        let lim = b.atmo.as_ref().map(|a| a.height).unwrap_or(0.0).max(b.radius * 0.12);
        self.alt_datum() < lim
    }

    pub fn ref_vel(&self) -> V2 {
        if self.surface_mode() { self.surface_vel() } else { self.orbit_vel() }
    }

    pub fn in_air(&self) -> bool {
        let d = self.dominant();
        self.sys.bodies[d].atmo.as_ref().is_some_and(|a| self.alt_datum() < a.height)
    }

    pub fn twr(&self) -> f64 {
        let d = self.dominant();
        let b = &self.sys.bodies[d];
        let r = (self.pos_vel().0 - self.st[d].pos).len();
        let g = b.gm / (r * r);
        self.craft.vehicle.max_thrust() / (self.craft.vehicle.mass() * g)
    }

    pub fn max_warp(&self) -> usize {
        if !self.launched && !matches!(self.craft.status, Status::Landed { .. }) {
            return PHYSICS_WARP;
        }
        let c = &self.craft;
        match c.status {
            Status::Crashed => return 0,
            Status::Landed { .. } | Status::Splashed { .. } => return WARPS.len() - 1,
            _ => {}
        }
        if c.throttle > 0.0 && c.vehicle.can_thrust() || self.in_air() || c.chute {
            return PHYSICS_WARP;
        }
        let alt = self.alt_datum();
        let d = self.dominant();
        let r = self.sys.bodies[d].radius;
        if alt < r * 0.03 {
            return 3;
        }
        if d != 0 && alt < r * 0.6 {
            return 6;
        }
        WARPS.len() - 1
    }

    pub fn burn_info(&self) -> Option<(f64, f64, f64)> {
        let n = self.nodes.first()?;
        let v = &self.craft.vehicle;
        let dv = self.node_burn.map(|b| b.len()).unwrap_or(n.dv());
        let bt = burn_time(dv, v.mass(), v.max_thrust().max(1.0), v.mdot().max(1e-9));
        Some((n.t - self.t, dv, bt))
    }

    /// World direction to burn for the first node.
    pub fn node_dir(&self) -> Option<V2> {
        if let Some(b) = self.node_burn {
            return Some(b.norm());
        }
        let n = self.nodes.first()?;
        // direction at burn start, as the prediction flies it
        let bt = self.burn_info().map(|b| b.2).filter(|b| b.is_finite() && *b > 0.5).unwrap_or(0.0);
        let t0 = (n.t - bt * 0.5).max(self.t);
        let p = self.pred.state_at(t0).or_else(|| {
            let (pos, vel) = self.pos_vel();
            Some(predict::Point { t: self.t, pos, vel, dom: self.dominant(), burn: false })
        })?;
        let st = self.sys.states_at(p.t);
        let (pro, rad) = frame(p.pos, p.vel, &st[p.dom]);
        Some((pro * n.pro + rad * n.rad).norm())
    }

    fn sas_dir(&self) -> Option<V2> {
        let v = self.ref_vel();
        let up = self.up();
        let pro = if v.len() > 0.5 { v.norm() } else { up };
        let rad = {
            let r = pro.perp();
            if r.dot(up) < 0.0 { -r } else { r }
        };
        match self.sas {
            Sas::Off | Sas::Hold => None,
            Sas::Prograde => Some(pro),
            Sas::Retrograde => Some(-pro),
            Sas::RadialOut => Some(rad),
            Sas::RadialIn => Some(-rad),
            Sas::Node => self.node_dir(),
            Sas::Target => {
                let tg = self.target?;
                Some((self.st[tg].pos - self.pos_vel().0).norm())
            }
        }
    }

    // ------------------------------------------------------------ controls

    pub fn rotate(&mut self, dir: f64) {
        if dir != 0.0 {
            if self.auto == Auto::Node {
                self.auto = Auto::Off;
            }
            if self.sas != Sas::Off {
                self.sas = Sas::Hold;
            }
            if !matches!(self.craft.status, Status::Landed { .. }) {
                self.craft.heading += dir * 1.6 * TICK;
            }
        }
    }

    pub fn set_throttle(&mut self, v: f64) {
        let v = v.clamp(0.0, 1.0);
        if v != self.craft.throttle {
            self.craft.throttle = v;
            self.pred_dirty = true;
        }
    }

    pub fn stage(&mut self) {
        let c = &mut self.craft;
        if matches!(c.status, Status::Crashed) {
            return;
        }
        let dir = c.dir();
        let Some(dropped) = c.vehicle.separate() else {
            // last stage: open the parachute if there is one
            if c.vehicle.has_chute() && !c.chute {
                c.chute = true;
                self.log.push(Log::ChuteOpen);
                self.pred_dirty = true;
            }
            return;
        };
        let kind = dropped.stages[0].kind;
        let h_drop = dropped.height();
        let h_rest = c.vehicle.height();
        let base = c.ship.pos - dir * ((h_drop + h_rest) * 0.5);
        let mut d = c.clone();
        d.vehicle = dropped;
        d.id = self.next_id;
        self.next_id += 1;
        d.ship.pos = base + dir * (h_drop * 0.5);
        d.ship.vel -= dir * 2.0;
        d.ship.mass = d.vehicle.mass();
        d.prev = d.ship;
        d.throttle = 0.0;
        d.legs = false;
        c.ship.pos = base + dir * (h_drop + h_rest * 0.5);
        c.ship.vel += dir * 1.5;
        c.ship.mass = c.vehicle.mass();
        c.prev = c.ship;
        if matches!(c.status, Status::Landed { .. }) {
            c.status = Status::Flying;
        }
        self.debris.push(d);
        self.log.push(Log::Staged { kind });
        self.pred_dirty = true;
    }

    pub fn warp_up(&mut self) {
        self.warp = (self.warp + 1).min(self.max_warp());
        self.warp_to = None;
    }

    pub fn warp_down(&mut self) {
        self.warp = self.warp.saturating_sub(1);
        self.warp_to = None;
    }

    pub fn add_node(&mut self, t: f64) {
        let t = t.max(self.t + 1.0);
        self.nodes.push(Node { t, pro: 0.0, rad: 0.0 });
        self.nodes.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
        self.pred_dirty = true;
    }

    pub fn delete_node(&mut self, i: usize) {
        if i < self.nodes.len() {
            self.nodes.remove(i);
            if i == 0 {
                self.node_burn = None;
                if self.auto == Auto::Node {
                    self.auto = Auto::Off;
                    self.set_throttle(0.0);
                }
            }
            self.pred_dirty = true;
        }
    }

    /// Time to warp to before the next node's burn.
    pub fn node_warp_point(&self) -> Option<f64> {
        let (dt, _, bt) = self.burn_info()?;
        let t0 = self.t + dt - bt.min(600.0) * 0.5 - 8.0;
        (t0 > self.t + 2.0).then_some(t0)
    }

    pub fn rewind(&mut self) -> bool {
        // the newest snapshot is under a second old: go one further back
        let keep = if self.snaps.len() >= 2 { self.snaps.len() - 2 } else { 0 };
        let Some(s) = self.snaps.get(keep).cloned() else { return false };
        self.snaps.truncate(keep + 1);
        self.t = s.t;
        self.craft = s.craft;
        self.debris = s.debris;
        self.nodes = s.nodes;
        self.node_burn = None;
        self.auto = Auto::Off;
        self.warp = 0;
        self.warp_to = None;
        self.wreck = None;
        self.sys.clone().states(self.t, &mut self.st);
        self.pred_dirty = true;
        true
    }

    // ------------------------------------------------------------ update

    pub fn update(&mut self) {
        self.log.clear();
        let sys = self.sys.clone();

        // snapshots for rewind: one per real second, last 30
        self.snap_tick += 1;
        if self.snap_tick >= 60 && !matches!(self.craft.status, Status::Crashed) {
            self.snap_tick = 0;
            self.snaps.push(Snapshot { t: self.t, craft: self.craft.clone(), debris: self.debris.clone(), nodes: self.nodes.clone() });
            if self.snaps.len() > 30 {
                self.snaps.remove(0);
            }
        }

        self.autopilot();
        self.steer();
        self.limit_warp();

        let target_t = self.t + WARPS[self.warp] * TICK;
        let was_burning = self.craft.burning;
        self.advance(&sys, target_t);
        if was_burning && !self.craft.vehicle.can_thrust() && self.craft.throttle > 0.0 && self.craft.vehicle.active().is_some_and(|s| s.thrust > 0.0) {
            self.log.push(Log::Flameout);
        }
        self.t = target_t;
        sys.states(self.t, &mut self.st);
        self.update_debris(&sys);

        if let Some(wt) = self.warp_to {
            if self.t >= wt - 1e-6 {
                self.warp_to = None;
                self.warp = 0;
            }
        }
        if self.launched {
            self.max_alt = self.max_alt.max(self.alt_datum());
        }
        // a node in the past that was never burned: drop it
        if let Some(n) = self.nodes.first() {
            if self.node_burn.is_none() && n.t < self.t - 30.0 {
                self.nodes.remove(0);
                self.pred_dirty = true;
            }
        }
        self.refresh_prediction();
    }

    fn steer(&mut self) {
        if matches!(self.craft.status, Status::Landed { .. } | Status::Crashed) {
            return;
        }
        if let Some(d) = self.sas_dir() {
            let want = d.angle();
            let mut diff = (want - self.craft.heading).rem_euclid(TAU);
            if diff > PI {
                diff -= TAU;
            }
            // at high warp, attitude simply snaps
            let rate = if self.warp > PHYSICS_WARP { 100.0 } else { 2.2 * TICK * WARPS[self.warp] };
            self.craft.heading += diff.clamp(-rate, rate);
        }
    }

    fn limit_warp(&mut self) {
        let max = self.max_warp();
        if let Some(wt) = self.warp_to {
            // pick the largest warp that still takes half a second to arrive
            let left = wt - self.t;
            let mut w = 0;
            for (i, k) in WARPS.iter().enumerate() {
                if i <= max && k * TICK * 30.0 <= left {
                    w = i;
                }
            }
            self.warp = w;
        }
        self.warp = self.warp.min(max);
        // drop out before things happen
        if self.warp > PHYSICS_WARP {
            let horizon = self.t + WARPS[self.warp] * TICK * 45.0;
            for e in &self.pred.events {
                let t = e.t();
                let important = matches!(e, Event::Soi { .. } | Event::Impact { .. } | Event::AtmoEnter { .. });
                if important && t > self.t && t < horizon {
                    while self.warp > 0 && self.t + WARPS[self.warp] * TICK * 45.0 > t {
                        self.warp -= 1;
                    }
                }
            }
        }
    }

    fn follow_ground(&mut self) {
        let sys = self.sys.clone();
        let t = self.craft.ship.t;
        self.follow_ground_at(&sys, t);
    }

    fn advance(&mut self, sys: &System, target_t: f64) {
        let mut steps = 0;
        let mut st = std::mem::take(&mut self.scratch);
        while self.craft.ship.t < target_t - 1e-9 {
            steps += 1;
            if steps > 20_000 {
                // too much work for one tick: slow the clock down
                self.warp = self.warp.saturating_sub(1);
                self.craft.ship.t = target_t;
                break;
            }
            match self.craft.status {
                Status::Crashed => {
                    self.craft.ship.t = target_t;
                    if let Some((body, ang)) = self.wreck {
                        let st = sys.states_at(target_t);
                        let b = &sys.bodies[body];
                        let wa = ang + b.spin_angle(target_t);
                        let r = b.surface_r(ang);
                        self.craft.ship.pos = st[body].pos + V2::from_angle(wa) * r;
                        self.craft.ship.vel = st[body].vel + V2::from_angle(wa).perp() * (b.spin * r);
                    }
                    self.craft.prev = self.craft.ship;
                    break;
                }
                Status::Landed { body, .. } | Status::Splashed { body, .. } => {
                    self.craft.ship.t = target_t;
                    self.follow_ground_at(sys, target_t);
                    self.craft.prev = self.craft.ship;
                    let c = &self.craft;
                    let b = &sys.bodies[body];
                    let r = (c.ship.pos - sys.states_at(target_t)[body].pos).len();
                    let g = b.gm / (r * r);
                    let d = c.drive();
                    let legs = c.vehicle.has_legs();
                    if d.thrust > c.ship.mass * g * 1.02 {
                        self.craft.status = Status::Flying;
                        self.craft.legs = legs;
                        if !self.launched {
                            self.launched = true;
                            self.log.push(Log::Liftoff);
                        }
                    }
                    break;
                }
                Status::Flying => {}
            }
            let drive = self.craft.drive();
            sys.states(self.craft.ship.t, &mut st);
            let dt = if drive.thrust > 0.0 || self.craft.chute { TICK } else { coast_dt(sys, &st, &self.craft.ship) };
            let before = self.craft.ship;
            let mut scratch = Vec::new();
            step(sys, &mut scratch, &mut self.craft.ship, dt, &drive);
            self.craft.prev = before;
            self.craft.burning = drive.thrust > 0.0;
            if drive.thrust > 0.0 {
                if let Some(s) = self.craft.vehicle.active_mut() {
                    s.fuel = (s.fuel - drive.mdot * dt).max(0.0);
                }
                self.craft.ship.mass = self.craft.vehicle.mass();
                if let Some(b) = &mut self.node_burn {
                    *b -= drive.dir * (drive.thrust / before.mass * dt);
                }
            }
            sys.states(self.craft.ship.t, &mut st);
            self.contact(sys, &st);
            // landing legs come down by themselves near the ground
            if !self.craft.legs && self.craft.vehicle.has_legs() && matches!(self.craft.status, Status::Flying) {
                let alt = self.alt_quick(sys, &st);
                if alt < 400.0 && self.craft.ship.vel.len() > 0.0 {
                    self.craft.legs = true;
                    self.log.push(Log::LegsDown);
                }
            }
        }
        self.scratch = st;
    }

    fn follow_ground_at(&mut self, sys: &System, t: f64) {
        let st = sys.states_at(t);
        let c = &mut self.craft;
        let (body, ang, tilt) = match c.status {
            Status::Landed { body, ang, tilt } => (body, ang, tilt),
            Status::Splashed { body, ang } => (body, ang, 0.0),
            _ => return,
        };
        let b = &sys.bodies[body];
        let wa = ang + b.spin_angle(t);
        let up = V2::from_angle(wa);
        let r = b.surface_r(ang) + c.vehicle.height() * 0.5;
        c.ship.pos = st[body].pos + up * r;
        c.ship.vel = st[body].vel + up.perp() * (b.spin * r);
        c.heading = wa + tilt;
        c.ship.mass = c.vehicle.mass();
    }

    fn alt_quick(&self, sys: &System, st: &[BodyState]) -> f64 {
        let d = sys.dominant(self.craft.ship.pos, st);
        let b = &sys.bodies[d];
        let rel = self.craft.ship.pos - st[d].pos;
        rel.len() - b.surface_r(rel.angle() - b.spin_angle(self.craft.ship.t)) - self.craft.vehicle.height() * 0.5
    }

    /// Touching the ground: a gentle upright touchdown on legs lands,
    /// anything else is a rapid unscheduled disassembly.
    fn contact(&mut self, sys: &System, st: &[BodyState]) {
        let c = &self.craft;
        let d = sys.dominant(c.ship.pos, st);
        let b = &sys.bodies[d];
        let dir = c.dir();
        let half = c.vehicle.height() * 0.5;
        let t = c.ship.t;
        let alt_of = |p: V2| {
            let rel = p - st[d].pos;
            let a = rel.angle() - b.spin_angle(t);
            (rel.len() - b.surface_r(a), a)
        };
        let (base_alt, base_ang) = alt_of(c.ship.pos - dir * half);
        let (nose_alt, _) = alt_of(c.ship.pos + dir * half);
        if b.kind.solid() && base_alt > 0.0 && nose_alt > 0.0 {
            return;
        }
        if !b.kind.solid() {
            // gas giants and stars have no ground: sinking in is the end
            if (c.ship.pos - st[d].pos).len() > b.radius {
                return;
            }
        }
        let rel = c.ship.pos - st[d].pos;
        let up = rel.norm();
        let ground_v = st[d].vel + rel.perp() * b.spin;
        let v = c.ship.vel - ground_v;
        let speed = v.len();
        let tilt = {
            let mut a = (dir.angle() - up.angle()).rem_euclid(TAU);
            if a > PI {
                a -= TAU;
            }
            a
        };
        let water = b.look.sea > 0.0 && b.terrain.height(base_ang) < 0.0;
        let capsule = c.vehicle.active().is_some_and(|s| s.kind == StageKind::Capsule);
        let safe = if c.legs && c.vehicle.has_legs() { 7.0 } else if capsule { 11.0 } else { 3.5 };
        let upright = tilt.abs() < 0.4 || capsule;
        let site = b.sites.first().map(|s| {
            let mut da = (base_ang - s.angle).rem_euclid(TAU);
            if da > PI {
                da -= TAU;
            }
            (da * b.radius).abs()
        });
        if b.kind.solid() && base_alt <= nose_alt && speed < safe && upright {
            if water {
                if capsule {
                    self.craft.status = Status::Splashed { body: d, ang: base_ang };
                    self.log.push(Log::Splashdown { body: d, speed });
                } else {
                    self.crash(d, speed);
                    return;
                }
            } else {
                let keep_tilt = if capsule { 0.0 } else { tilt * 0.0 };
                self.craft.status = Status::Landed { body: d, ang: base_ang, tilt: keep_tilt };
                self.log.push(Log::Landed { body: d, speed, dist_to_site: site.unwrap_or(f64::INFINITY) });
            }
            self.craft.throttle = 0.0;
            self.craft.chute = false;
            self.auto = Auto::Off;
            self.warp = 0;
            self.follow_ground_at(sys, t);
            self.craft.prev = self.craft.ship;
            self.pred_dirty = true;
        } else {
            self.crash(d, speed);
        }
    }

    fn crash(&mut self, body: usize, speed: f64) {
        self.craft.status = Status::Crashed;
        let rel = self.craft.ship.pos - self.sys.states_at(self.craft.ship.t)[body].pos;
        self.wreck = Some((body, rel.angle() - self.sys.bodies[body].spin_angle(self.craft.ship.t)));
        self.craft.throttle = 0.0;
        self.craft.burning = false;
        self.warp = 0;
        self.warp_to = None;
        self.auto = Auto::Off;
        self.log.push(Log::Crashed { body, speed });
    }

    fn update_debris(&mut self, sys: &System) {
        let me = self.craft.ship.pos;
        let mut st = Vec::new();
        let mut scratch = Vec::new();
        let mut i = 0;
        while i < self.debris.len() {
            let d = &mut self.debris[i];
            let mut gone = false;
            let mut steps = 0;
            while d.ship.t < self.t - 1e-9 && steps < 400 {
                steps += 1;
                sys.states(d.ship.t, &mut st);
                let dt = coast_dt(sys, &st, &d.ship).min(self.t - d.ship.t).max(1e-4);
                let drive = Drive { cda: d.cda() * 2.0, ..Default::default() };
                d.prev = d.ship;
                step(sys, &mut scratch, &mut d.ship, dt, &drive);
                // tumble
                d.heading += 0.4 * dt;
                sys.states(d.ship.t, &mut st);
                let dom = sys.dominant(d.ship.pos, &st);
                let b = &sys.bodies[dom];
                let rel = d.ship.pos - st[dom].pos;
                if rel.len() < b.surface_r(rel.angle() - b.spin_angle(d.ship.t)) + 1.0 {
                    self.log.push(Log::Debris { pos: d.ship.pos });
                    gone = true;
                    break;
                }
            }
            d.ship.t = self.t;
            if gone || (d.ship.pos - me).len() > 400_000.0 {
                self.debris.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn autopilot(&mut self) {
        if self.auto != Auto::Node {
            return;
        }
        if self.nodes.is_empty() {
            self.auto = Auto::Off;
            return;
        }
        self.sas = Sas::Node;
        let Some((dt, dv, bt)) = self.burn_info() else { return };
        let start = dt - bt * 0.5;
        if self.node_burn.is_none() && start > 12.0 && self.warp_to.is_none() {
            if let Some(w) = self.node_warp_point() {
                self.warp_to = Some(w);
            }
        }
        if start <= 0.0 || self.node_burn.is_some() {
            // only burn when pointing the right way
            let aligned = self.node_dir().is_some_and(|d| d.dot(self.craft.dir()) > 0.995);
            let v = &self.craft.vehicle;
            let acc = v.max_thrust() / v.mass();
            let thr = if aligned { (dv / (acc * 0.6).max(1e-6)).clamp(0.05, 1.0) } else { 0.0 };
            let can = v.can_thrust();
            self.set_throttle(thr);
            if !can {
                self.auto = Auto::Off;
            }
        }
    }

    /// Start tracking the node's burn once the engine lights up near it,
    /// and finish it when the remaining burn points backwards.
    fn track_node(&mut self) {
        let Some(n) = self.nodes.first().copied() else { return };
        let bt = self.burn_info().map(|x| x.2).unwrap_or(0.0);
        if self.node_burn.is_none() && self.craft.burning && self.t >= n.t - bt * 0.5 - 5.0 {
            if let Some(d) = self.node_dir() {
                self.node_burn = Some(d * n.dv());
                self.node_dir0 = d;
            }
        }
        if let Some(b) = self.node_burn {
            if b.dot(self.node_dir0) <= 0.15 {
                self.nodes.remove(0);
                self.node_burn = None;
                if self.auto == Auto::Node {
                    self.auto = Auto::Off;
                    self.set_throttle(0.0);
                    self.sas = Sas::Hold;
                }
                self.log.push(Log::NodeDone);
                self.pred_dirty = true;
            }
        }
    }

    pub fn refresh_prediction(&mut self) {
        self.track_node();
        self.pred_age += 1;
        let c = &self.craft;
        let moving = c.burning || matches!(c.status, Status::Landed { .. });
        if !self.pred_dirty && !moving && self.pred_age < 20 {
            return;
        }
        if matches!(c.status, Status::Crashed) {
            self.pred = Prediction::default();
            return;
        }
        self.pred_age = 0;
        self.pred_dirty = false;
        let v = &c.vehicle;
        let fuel = v.active().map(|s| s.fuel).unwrap_or(0.0);
        let nodes: Vec<Node> = if self.node_burn.is_some() { self.nodes[1..].to_vec() } else { self.nodes.clone() };
        let set = Setup {
            cda: c.cda(),
            thrust: v.max_thrust(),
            mdot: v.mdot(),
            fuel,
            nodes: &nodes,
            target: self.target,
            max_points: 4000,
            horizon: 3.0e6,
        };
        let start = if matches!(c.status, Status::Landed { .. } | Status::Splashed { .. }) {
            // standing still: nothing to predict
            self.pred = Prediction::default();
            return;
        } else {
            c.ship
        };
        self.pred = predict::predict(&self.sys, start, &set);
    }

    /// Heading straight up from the ground.
    pub fn local_up_angle(&self) -> f64 {
        self.up().angle()
    }

    pub fn is_over(&self) -> bool {
        matches!(self.craft.status, Status::Crashed)
    }

    /// Distance along the surface to the body's first site (pad / LZ).
    pub fn site_distance(&self, body: usize) -> Option<f64> {
        let b = &self.sys.bodies[body];
        let s = b.sites.first()?;
        let rel = self.pos_vel().0 - self.st[body].pos;
        let a = rel.angle() - b.spin_angle(self.t);
        let mut da = (a - s.angle).rem_euclid(TAU);
        if da > PI {
            da -= TAU;
        }
        Some(da * b.radius)
    }

    pub fn kind_of(&self, i: usize) -> Kind {
        self.sys.bodies[i].kind
    }
}

/// Angle of a site in world space at time `t` (for markers).
pub fn site_world(sys: &System, st: &[BodyState], body: usize, ang: f64, t: f64) -> (V2, f64) {
    let b = &sys.bodies[body];
    let wa = ang + b.spin_angle(t);
    (st[body].pos + V2::from_angle(wa) * b.surface_r(ang), wa)
}
