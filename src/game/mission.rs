//! The campaign: missions, their goals and stars. Missions name bodies by
//! role (home, its moon, the neighbor planet, the gas giant), so every
//! generated system can fly the same campaign. Rocket tanks are sized
//! from the delta-v each mission needs in this system.

use crate::game::flight::{Flight, Log, Status};
use crate::sim::system::{Kind, SiteKind, System};
use crate::sim::vehicle::{self, StageKind, Vehicle, fit_fuel};
use crate::sim::worldgen;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    Home,
    Moon,
    Neighbor,
    Giant,
}

pub fn body_of(sys: &System, r: Role) -> usize {
    match r {
        Role::Home => sys.home,
        Role::Moon => worldgen::home_moon(sys).unwrap_or(sys.home),
        Role::Neighbor => worldgen::neighbor(sys).unwrap_or(sys.home),
        Role::Giant => sys.bodies.iter().position(|b| b.kind == Kind::Gas && b.parent == Some(0)).unwrap_or(sys.home),
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Goal {
    /// Climb above this altitude.
    Altitude(Role, f64),
    /// Leave the atmosphere (or go above `alt` on an airless body).
    Space(Role),
    /// Stand on the ground, within `dist` m of the site if `Some`.
    Land(Role, Option<f64>),
    /// Down safely anywhere: landed or splashed.
    Recover(Role),
    /// Orbit with periapsis above `min_pe` and apoapsis below `max_ap` (alt, m).
    Orbit(Role, f64, f64),
    /// Separate the satellite while in such an orbit.
    Deploy(Role, f64, f64),
    /// Enter this body's sphere of influence.
    Reach(Role),
    /// Fly the booster: drop the upper stage above this altitude.
    StageAbove(f64),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bonus {
    FuelLeft(f64),
    LandWithin(f64),
    Circular(f64),
    Soft(f64),
}

impl Bonus {
    pub fn text(&self) -> String {
        match *self {
            Bonus::FuelLeft(f) => format!("{:.0}% FUEL LEFT", f * 100.0),
            Bonus::LandWithin(m) => format!("LAND WITHIN {:.0} M", m),
            Bonus::Circular(m) => format!("AP-PE UNDER {:.0} KM", m / 1000.0),
            Bonus::Soft(v) => format!("TOUCH DOWN UNDER {:.0} M/S", v),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Start {
    Pad,
    /// Circular orbit at this altitude around the body.
    Orbit(Role, f64),
    /// On the landing zone of a body.
    Site(Role),
}

pub struct Mission {
    pub code: &'static str,
    pub title: &'static str,
    pub tagline: &'static str,
    pub brief: &'static [&'static str],
    pub tips: &'static [&'static str],
    pub start: Start,
    pub goals: &'static [Goal],
    pub bonus: [Bonus; 2],
    /// After staging, keep flying the bottom stage.
    pub fly_booster: bool,
    pub build: fn(&System) -> Vehicle,
}

// ------------------------------------------------------------ helpers

fn atmo_top(sys: &System, b: usize) -> f64 {
    sys.bodies[b].atmo.as_ref().map(|a| a.height).unwrap_or(sys.bodies[b].radius * 0.08)
}

fn v_circ(gm: f64, r: f64) -> f64 {
    (gm / r).sqrt()
}

/// Delta-v for a Hohmann transfer between circular orbits r1 -> r2.
pub fn hohmann(gm: f64, r1: f64, r2: f64) -> (f64, f64) {
    let a = (r1 + r2) * 0.5;
    let dv1 = (gm / r1).sqrt() * ((r2 / a).sqrt() - 1.0).abs();
    let dv2 = (gm / r2).sqrt() * (1.0 - (r1 / a).sqrt()).abs();
    (dv1, dv2)
}

/// From low orbit (or to it): a burn that gives `v_inf` at the edge of the sphere.
fn escape_dv(gm: f64, r: f64, v_inf: f64) -> f64 {
    (v_inf * v_inf + 2.0 * gm / r).sqrt() - (gm / r).sqrt()
}

fn ascent_dv(sys: &System, b: usize) -> f64 {
    let body = &sys.bodies[b];
    let r = body.radius + atmo_top(sys, b) * 1.4;
    let drag = if body.atmo.is_some() { 260.0 } else { 90.0 };
    v_circ(body.gm, r) + drag + body.surface_g() * 12.0
}

fn low_orbit(sys: &System, b: usize) -> f64 {
    sys.bodies[b].radius + atmo_top(sys, b) * 1.5
}

/// Low orbit around home to low orbit around the moon (plus capture).
fn moon_transfer_dv(sys: &System) -> f64 {
    let h = sys.home;
    let m = body_of(sys, Role::Moon);
    let hb = &sys.bodies[h];
    let mb = &sys.bodies[m];
    let r1 = low_orbit(sys, h);
    let r2 = mb.orbit.a;
    let (dv1, _) = hohmann(hb.gm, r1, r2);
    let a = (r1 + r2) * 0.5;
    let v_apo = (hb.gm * (2.0 / r2 - 1.0 / a)).sqrt();
    let v_inf = (v_circ(hb.gm, r2) - v_apo).abs();
    let capture = escape_dv(mb.gm, low_orbit(sys, m), v_inf);
    dv1 + capture
}

fn interplanetary_dv(sys: &System, to: usize) -> (f64, f64) {
    let h = sys.home;
    let star = sys.bodies[0].gm;
    let (r1, r2) = (sys.bodies[h].orbit.a, sys.bodies[to].orbit.a);
    let (dv1, dv2) = hohmann(star, r1, r2);
    let out = escape_dv(sys.bodies[h].gm, low_orbit(sys, h), dv1);
    let capture = escape_dv(sys.bodies[to].gm, low_orbit(sys, to), dv2);
    (out, capture)
}

fn landing_dv(sys: &System, b: usize) -> f64 {
    let body = &sys.bodies[b];
    v_circ(body.gm, low_orbit(sys, b)) * 1.15 + body.surface_g() * 8.0
}

// ------------------------------------------------------------ rockets

fn hopper(_: &System) -> Vehicle {
    let mut s = vehicle::booster(22.0, 130_000.0);
    s.name = "GRASSHOPPER";
    s.legs = true;
    s.fins = false;
    s.trim = vehicle::ORANGE;
    fit_fuel(&mut s, 0.0, 850.0);
    Vehicle { stages: vec![s] }
}

fn tourist(sys: &System) -> Vehicle {
    let mut cap = vehicle::capsule();
    cap.name = "TOURIST CAPSULE";
    let mut b = vehicle::booster(18.0, 200_000.0);
    b.trim = vehicle::TEAL;
    let h = &sys.bodies[sys.home];
    let up = (2.0 * h.surface_g() * atmo_top(sys, sys.home) * 1.25).sqrt();
    fit_fuel(&mut b, cap.mass(), up + 200.0);
    Vehicle { stages: vec![b, cap] }
}

fn orbiter(sys: &System, payload: vehicle::Stage, extra: f64) -> Vehicle {
    let need = ascent_dv(sys, sys.home);
    let mut up = vehicle::upper(9.0, 45_000.0);
    fit_fuel(&mut up, payload.mass(), need * 0.42 + extra);
    let mut b = vehicle::booster(24.0, 300_000.0);
    fit_fuel(&mut b, payload.mass() + up.mass(), need * 0.66);
    Vehicle { stages: vec![b, up, payload] }
}

fn first_orbit(sys: &System) -> Vehicle {
    orbiter(sys, vehicle::capsule(), 120.0)
}

fn comsat(sys: &System) -> Vehicle {
    let h = &sys.bodies[sys.home];
    let r1 = low_orbit(sys, sys.home);
    let (a, b) = hohmann(h.gm, r1, h.radius + 17_500.0);
    orbiter(sys, vehicle::satellite("COMSAT"), (a + b) * 1.4)
}

fn deorbiter(sys: &System) -> Vehicle {
    let mut up = vehicle::upper(5.0, 20_000.0);
    up.name = "SERVICE MODULE";
    let cap = vehicle::capsule();
    let _ = sys;
    fit_fuel(&mut up, cap.mass(), 160.0);
    Vehicle { stages: vec![up, cap] }
}

fn falcon(sys: &System) -> Vehicle {
    let mut v = orbiter(sys, vehicle::satellite("STARLINKY"), 60.0);
    let b = &mut v.stages[0];
    b.legs = true;
    b.name = "REUSABLE BOOSTER";
    let above: f64 = v.stages[1..].iter().map(|s| s.mass()).sum();
    let need = ascent_dv(sys, sys.home);
    // enough to boost, turn back and land
    fit_fuel(&mut v.stages[0], above, need * 0.66 + 520.0);
    v
}

fn transfer_stage(payload: vehicle::Stage, dv: f64, thrust: f64) -> Vehicle {
    let mut t = vehicle::upper(8.0, thrust);
    t.name = "TRANSFER STAGE";
    t.trim = vehicle::GOLD;
    fit_fuel(&mut t, payload.mass(), dv);
    Vehicle { stages: vec![t, payload] }
}

fn moon_orbiter(sys: &System) -> Vehicle {
    transfer_stage(vehicle::satellite("MOON PROBE"), moon_transfer_dv(sys) * 1.5, 30_000.0)
}

fn moon_lander(sys: &System) -> Vehicle {
    let m = body_of(sys, Role::Moon);
    let mut l = vehicle::lander(18_000.0);
    fit_fuel(&mut l, 0.0, landing_dv(sys, m) * 2.0);
    let mut v = transfer_stage(l.clone(), moon_transfer_dv(sys) * 1.35, 30_000.0);
    v.stages[1] = l;
    v
}

fn moon_return(sys: &System) -> Vehicle {
    let m = body_of(sys, Role::Moon);
    let cap = vehicle::capsule();
    let mut a = vehicle::lander(30_000.0);
    a.name = "ASCENT STAGE";
    let back = landing_dv(sys, m) + moon_transfer_dv(sys);
    fit_fuel(&mut a, cap.mass(), back * 1.35);
    Vehicle { stages: vec![a, cap] }
}

fn planet_lander(sys: &System) -> Vehicle {
    let n = body_of(sys, Role::Neighbor);
    let (out, capture) = interplanetary_dv(sys, n);
    let mut l = vehicle::lander(30_000.0);
    let air = sys.bodies[n].atmo.is_some();
    fit_fuel(&mut l, 0.0, landing_dv(sys, n) * if air { 0.8 } else { 1.5 } + 60.0);
    l.chute = air;
    let mut t = vehicle::upper(8.0, 40_000.0);
    t.name = "TRANSFER STAGE";
    t.trim = vehicle::GOLD;
    fit_fuel(&mut t, l.mass(), (out + capture) * 1.4);
    Vehicle { stages: vec![t, l] }
}

fn giant_probe(sys: &System) -> Vehicle {
    let g = body_of(sys, Role::Giant);
    let (out, capture) = interplanetary_dv(sys, g);
    transfer_stage(vehicle::satellite("BIG EYE PROBE"), (out + capture) * 1.3, 30_000.0)
}

// ------------------------------------------------------------ campaign

pub const MISSIONS: &[Mission] = &[
    Mission {
        code: "M-01",
        title: "GRASSHOPPER",
        tagline: "UP A BIT. DOWN A BIT. DON'T DIE.",
        brief: &["HOP TO 1 KM AND LAND", "BACK ON THE PAD.", "THE COWS ARE WATCHING."],
        tips: &[
            "UP/DOWN OR W/S: THROTTLE",
            "LEFT/RIGHT OR A/D: TURN",
            "Z: FULL THROTTLE  X: CUT",
            "WATCH V/S NEAR THE GROUND:",
            "TOUCH DOWN UNDER 7 M/S",
        ],
        start: Start::Pad,
        goals: &[Goal::Altitude(Role::Home, 1000.0), Goal::Land(Role::Home, Some(150.0))],
        bonus: [Bonus::FuelLeft(0.25), Bonus::LandWithin(20.0)],
        fly_booster: false,
        build: hopper,
    },
    Mission {
        code: "M-02",
        title: "SPACE TOURIST",
        tagline: "ONE TICKET TO SPACE. RETURN NOT GUARANTEED.",
        brief: &["TAKE A RICH TOURIST ABOVE", "THE SKY AND BRING THEM", "BACK DOWN IN ONE PIECE."],
        tips: &[
            "SPACE: DROP THE BOOSTER",
            "SPACE AGAIN: OPEN PARACHUTE",
            "THE CYAN LINE IS YOUR PATH.",
            "THE RED X IS WHERE YOU LAND.",
        ],
        start: Start::Pad,
        goals: &[Goal::Space(Role::Home), Goal::Recover(Role::Home)],
        bonus: [Bonus::Soft(6.0), Bonus::LandWithin(3000.0)],
        fly_booster: false,
        build: tourist,
    },
    Mission {
        code: "M-03",
        title: "ROUND AND ROUND",
        tagline: "FALL AROUND THE PLANET. MISS THE GROUND.",
        brief: &["REACH A STABLE ORBIT:", "PERIAPSIS ABOVE THE AIR.", "THE CAPSULE COMES BACK LATER."],
        tips: &[
            "TILT EAST (RIGHT) AFTER 1 KM.",
            "BURN UNTIL AP IS ABOVE THE AIR,",
            "THEN CUT AND COAST TO AP.",
            "CLICK THE PATH AT AP FOR A NODE,",
            "DRAG + PRO, PRESS N TO BURN IT.",
        ],
        start: Start::Pad,
        goals: &[Goal::Orbit(Role::Home, -1.0, f64::INFINITY)],
        bonus: [Bonus::FuelLeft(0.3), Bonus::Circular(3000.0)],
        fly_booster: false,
        build: first_orbit,
    },
    Mission {
        code: "M-04",
        title: "COMSAT",
        tagline: "THE PLANET NEEDS MORE CAT VIDEOS.",
        brief: &["DELIVER THE COMSAT TO AN ORBIT", "BETWEEN 15 AND 20 KM HIGH,", "THEN SEPARATE IT."],
        tips: &[
            "ORBIT LOW FIRST, THEN RAISE:",
            "A PROGRADE NODE RAISES THE",
            "FAR SIDE. CIRCULARIZE AT AP.",
            "SPACE: SEPARATE THE SATELLITE",
        ],
        start: Start::Pad,
        goals: &[Goal::Deploy(Role::Home, 15_000.0, 20_000.0)],
        bonus: [Bonus::FuelLeft(0.2), Bonus::Circular(2000.0)],
        fly_booster: false,
        build: comsat,
    },
    Mission {
        code: "M-05",
        title: "COMING HOME",
        tagline: "WHAT GOES UP MUST COME DOWN. NEAR THE BAR.",
        brief: &["YOU ARE IN ORBIT. BURN", "RETROGRADE AND BRING THE", "CAPSULE DOWN NEAR THE PAD."],
        tips: &[
            "3: HOLD RETROGRADE",
            "WATCH THE RED X MOVE AS",
            "YOU BURN. AIM IT AT THE PAD.",
            "OPEN THE CHUTE LOW AND SLOW.",
        ],
        start: Start::Orbit(Role::Home, 9_000.0),
        goals: &[Goal::Recover(Role::Home)],
        bonus: [Bonus::LandWithin(5000.0), Bonus::LandWithin(1500.0)],
        fly_booster: false,
        build: deorbiter,
    },
    Mission {
        code: "M-06",
        title: "BOOSTER BACK",
        tagline: "WHY THROW AWAY A PERFECTLY GOOD ROCKET?",
        brief: &["STAGE ABOVE 4 KM. YOU KEEP", "FLYING THE BOOSTER: LAND IT", "BACK ON THE PAD."],
        tips: &[
            "AFTER STAGING, FLIP AROUND",
            "AND BURN BACK TOWARD THE PAD.",
            "KEEP THE RED X ON THE PAD,",
            "BURN LATE AND HARD TO LAND.",
        ],
        start: Start::Pad,
        goals: &[Goal::StageAbove(4000.0), Goal::Land(Role::Home, Some(300.0))],
        bonus: [Bonus::FuelLeft(0.08), Bonus::LandWithin(30.0)],
        fly_booster: true,
        build: falcon,
    },
    Mission {
        code: "M-07",
        title: "MOON SHOT",
        tagline: "THE MOON IS RIGHT THERE. GO GET IT.",
        brief: &["FROM LOW ORBIT, FLY TO THE", "MOON AND ENTER AN ORBIT", "AROUND IT."],
        tips: &[
            "T: TARGET THE MOON",
            "PLACE A NODE, DRAG PROGRADE",
            "UNTIL THE PATH MEETS THE MOON",
            "GHOST. G: WARP TO NEXT EVENT.",
            "BRAKE RETROGRADE AT ITS PE.",
        ],
        start: Start::Orbit(Role::Home, 9_000.0),
        goals: &[Goal::Reach(Role::Moon), Goal::Orbit(Role::Moon, 500.0, 25_000.0)],
        bonus: [Bonus::FuelLeft(0.2), Bonus::Circular(4000.0)],
        fly_booster: false,
        build: moon_orbiter,
    },
    Mission {
        code: "M-08",
        title: "ONE SMALL HOP",
        tagline: "BRING A FLAG. AND A SANDWICH.",
        brief: &["FLY TO THE MOON AND LAND", "AT ITS LANDING ZONE."],
        tips: &[
            "NO AIR ON THE MOON: BRAKE",
            "WITH THE ENGINE ONLY.",
            "3: RETROGRADE TO KILL SPEED,",
            "THEN LAND UPRIGHT ON LEGS.",
        ],
        start: Start::Orbit(Role::Home, 9_000.0),
        goals: &[Goal::Reach(Role::Moon), Goal::Land(Role::Moon, Some(3000.0))],
        bonus: [Bonus::FuelLeft(0.12), Bonus::LandWithin(200.0)],
        fly_booster: false,
        build: moon_lander,
    },
    Mission {
        code: "M-09",
        title: "THERE AND BACK AGAIN",
        tagline: "THE SANDWICH WAS A MISTAKE. COME HOME.",
        brief: &["LIFT OFF FROM THE MOON,", "FLY HOME AND BRING THE", "CAPSULE DOWN SAFELY."],
        tips: &[
            "CLIMB TO ORBIT, THEN BURN",
            "SO YOUR PATH DROPS INTO",
            "THE HOME AIR. THE AIR DOES",
            "THE BRAKING. CHUTE AT THE END.",
        ],
        start: Start::Site(Role::Moon),
        goals: &[Goal::Reach(Role::Home), Goal::Recover(Role::Home)],
        bonus: [Bonus::FuelLeft(0.1), Bonus::Soft(6.0)],
        fly_booster: false,
        build: moon_return,
    },
    Mission {
        code: "M-10",
        title: "RED PLANET",
        tagline: "THE NEIGHBORS HAVE NEVER MET US. FIX THAT.",
        brief: &["LEAVE HOME, FLY TO THE", "NEIGHBOR PLANET AND LAND."],
        tips: &[
            "T: TARGET THE PLANET.",
            "BURN PROGRADE ON THE NIGHT",
            "SIDE TO ESCAPE OUTWARD.",
            "WATCH FOR THE CLOSEST MARK,",
            "THEN TUNE IT INTO AN ENCOUNTER.",
        ],
        start: Start::Orbit(Role::Home, 9_000.0),
        goals: &[Goal::Reach(Role::Neighbor), Goal::Land(Role::Neighbor, None)],
        bonus: [Bonus::FuelLeft(0.1), Bonus::Soft(4.0)],
        fly_booster: false,
        build: planet_lander,
    },
    Mission {
        code: "M-11",
        title: "BIG EYE",
        tagline: "A PROBE TO THE GIANT. BRING SNACKS.",
        brief: &["FLY TO THE GAS GIANT AND", "ENTER AN ORBIT AROUND IT."],
        tips: &[
            "A LONG TRIP: WARP HARD.",
            "ITS MOONS CAN BEND YOUR PATH",
            "FOR FREE. TRY A FLYBY!",
        ],
        start: Start::Orbit(Role::Home, 9_000.0),
        goals: &[Goal::Reach(Role::Giant), Goal::Orbit(Role::Giant, 30_000.0, 2_500_000.0)],
        bonus: [Bonus::FuelLeft(0.15), Bonus::FuelLeft(0.3)],
        fly_booster: false,
        build: giant_probe,
    },
];

pub const CAMPAIGN_SEED: u64 = 7;

// ------------------------------------------------------------ running

/// Osculating periapsis and apoapsis altitudes around body `b`.
pub fn apsides(f: &Flight, b: usize) -> Option<(f64, f64)> {
    let body = &f.sys.bodies[b];
    let (p, v) = f.pos_vel();
    let r = p - f.st[b].pos;
    let v = v - f.st[b].vel;
    let mu = body.gm;
    let e_spec = v.len2() * 0.5 - mu / r.len();
    if e_spec >= 0.0 {
        return None;
    }
    let a = -mu / (2.0 * e_spec);
    let h = r.cross(v);
    let e = (1.0 - h * h / (mu * a)).max(0.0).sqrt();
    Some((a * (1.0 - e) - body.radius, a * (1.0 + e) - body.radius))
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Outcome {
    Flying,
    Success,
    Failed(&'static str),
}

#[derive(Clone)]
pub struct Tracker {
    pub step: usize,
    pub outcome: Outcome,
    /// Orbit goal held for this many ticks.
    hold: u32,
    pub land_dist: Option<f64>,
    pub land_speed: Option<f64>,
    pub circ: Option<f64>,
    pub fuel_left: f64,
}

impl Tracker {
    pub fn new() -> Self {
        Tracker { step: 0, outcome: Outcome::Flying, hold: 0, land_dist: None, land_speed: None, circ: None, fuel_left: 1.0 }
    }

    pub fn update(&mut self, m: &Mission, f: &Flight) {
        if self.outcome != Outcome::Flying {
            return;
        }
        let sys = &f.sys;
        for l in &f.log {
            match *l {
                Log::Crashed { .. } => {
                    self.outcome = Outcome::Failed("RAPID UNSCHEDULED DISASSEMBLY");
                    return;
                }
                Log::Landed { speed, dist_to_site, .. } => {
                    self.land_speed = Some(speed);
                    self.land_dist = Some(dist_to_site);
                }
                Log::Splashdown { speed, body } => {
                    self.land_speed = Some(speed);
                    self.land_dist = f.site_distance(body).map(f64::abs);
                }
                _ => {}
            }
        }
        // fuel of the stack, before any payload is let go
        if f.craft.vehicle.stages.iter().any(|s| s.fuel_max > 0.0) {
            self.fuel_left = f.craft.vehicle.fuel_frac();
        }
        let Some(goal) = m.goals.get(self.step).copied() else { return };
        let done = match goal {
            Goal::Altitude(r, alt) => f.dominant() == body_of(sys, r) && f.altitude() > alt,
            Goal::Space(r) => {
                let b = body_of(sys, r);
                f.dominant() == b && f.alt_datum() > atmo_top(sys, b)
            }
            Goal::Land(r, dist) => match f.craft.status {
                Status::Landed { body, .. } if body == body_of(sys, r) => {
                    let ok = dist.is_none_or(|d| self.land_dist.unwrap_or(f64::INFINITY) <= d);
                    if !ok && f.log.iter().any(|l| matches!(l, Log::Landed { .. })) {
                        self.outcome = Outcome::Failed("WRONG PARKING SPOT");
                    }
                    ok
                }
                _ => false,
            },
            Goal::Recover(r) => match f.craft.status {
                Status::Landed { body, .. } | Status::Splashed { body, .. } => body == body_of(sys, r),
                _ => false,
            },
            Goal::Orbit(r, min_pe, max_ap) => {
                let b = body_of(sys, r);
                let min_pe = if min_pe < 0.0 { atmo_top(sys, b) } else { min_pe.max(atmo_top(sys, b) * 0.5) };
                let ok = f.dominant() == b
                    && !f.craft.burning
                    && apsides(f, b).is_some_and(|(pe, ap)| {
                        self.circ = Some(ap - pe);
                        pe > min_pe && ap < max_ap && f.pred.impact().is_none()
                    });
                self.hold = if ok { self.hold + 1 } else { 0 };
                self.hold > 45
            }
            Goal::Deploy(r, lo, hi) => {
                let b = body_of(sys, r);
                let sat = f.craft.vehicle.stages.len() == 1 && f.craft.vehicle.stages[0].kind == StageKind::Satellite;
                if sat {
                    match apsides(f, b) {
                        Some((pe, ap)) if f.dominant() == b && pe >= lo && ap <= hi => {
                            self.circ = Some(ap - pe);
                            true
                        }
                        _ => {
                            self.outcome = Outcome::Failed("SATELLITE IN THE WRONG ORBIT");
                            false
                        }
                    }
                } else {
                    false
                }
            }
            Goal::Reach(r) => f.dominant() == body_of(sys, r),
            Goal::StageAbove(alt) => {
                let staged = f.log.iter().any(|l| matches!(l, Log::Staged { .. }));
                if staged && f.alt_datum() < alt {
                    self.outcome = Outcome::Failed("STAGED TOO LOW");
                }
                staged
            }
        };
        if done {
            self.step += 1;
            self.hold = 0;
            if self.step >= m.goals.len() {
                self.outcome = Outcome::Success;
            }
        }
        // out of fuel, no chute and not going anywhere
        if matches!(f.craft.status, Status::Landed { .. } | Status::Splashed { .. })
            && !f.craft.vehicle.stages.iter().any(|s| s.fuel > 0.0 && s.thrust > 0.0)
            && self.outcome == Outcome::Flying
            && f.launched
        {
            self.outcome = Outcome::Failed("STRANDED. SEND SNACKS.");
        }
    }

    /// Stars earned: 1 for success, +1 per bonus met.
    pub fn stars(&self, m: &Mission) -> (u32, [bool; 2]) {
        if self.outcome != Outcome::Success {
            return (0, [false, false]);
        }
        let met = |b: &Bonus| match *b {
            Bonus::FuelLeft(x) => self.fuel_left >= x,
            Bonus::LandWithin(d) => self.land_dist.is_some_and(|x| x <= d),
            Bonus::Circular(d) => self.circ.is_some_and(|x| x <= d),
            Bonus::Soft(v) => self.land_speed.is_some_and(|x| x <= v),
        };
        let b = [met(&m.bonus[0]), met(&m.bonus[1])];
        (1 + b.iter().filter(|x| **x).count() as u32, b)
    }
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}

pub fn goal_text(sys: &System, g: &Goal) -> String {
    let name = |r: Role| sys.bodies[body_of(sys, r)].name.clone();
    match *g {
        Goal::Altitude(_, a) => format!("CLIMB ABOVE {:.0} M", a),
        Goal::Space(r) => format!("LEAVE THE AIR OF {}", name(r)),
        Goal::Land(r, Some(d)) => format!("LAND ON {} SITE (<{:.0} M)", name(r), d),
        Goal::Land(r, None) => format!("LAND ON {}", name(r)),
        Goal::Recover(r) => format!("TOUCH DOWN ON {}", name(r)),
        Goal::Orbit(r, pe, ap) => {
            if ap.is_finite() && pe > 0.0 {
                format!("ORBIT {} ({:.0}-{:.0} KM)", name(r), pe / 1000.0, ap / 1000.0)
            } else {
                format!("ORBIT {}", name(r))
            }
        }
        Goal::Deploy(_, lo, hi) => format!("DEPLOY IN {:.0}-{:.0} KM ORBIT", lo / 1000.0, hi / 1000.0),
        Goal::Reach(r) => format!("REACH {}", name(r)),
        Goal::StageAbove(a) => format!("STAGE ABOVE {:.0} KM", a / 1000.0),
    }
}

/// Set up the flight for a mission.
pub fn start(m: &Mission, sys: Rc<System>) -> Flight {
    let v = (m.build)(&sys);
    match m.start {
        Start::Pad => {
            let home = sys.home;
            let pad = sys.bodies[home].site(SiteKind::LaunchPad).map(|s| s.angle).unwrap_or(0.0);
            Flight::on_pad(sys, v, home, pad)
        }
        Start::Site(r) => {
            let b = body_of(&sys, r);
            let ang = sys.bodies[b].sites.first().map(|s| s.angle).unwrap_or(0.0);
            let mut f = Flight::on_pad(sys, v, b, ang);
            f.launched = true;
            f
        }
        Start::Orbit(r, alt) => {
            let b = body_of(&sys, r);
            let st = sys.states_at(0.0);
            let body = &sys.bodies[b];
            let rr = body.radius + alt;
            // start on the night side, going prograde
            let sun = (st[0].pos - st[b].pos).norm();
            let dir = -sun.rot(0.4);
            let pos = st[b].pos + dir * rr;
            let vel = st[b].vel - dir.perp() * v_circ(body.gm, rr);
            let heading = (-dir.perp()).angle();
            let mut f = Flight::in_flight(sys, v, pos, vel, heading, 0.0);
            f.sas = crate::game::flight::Sas::Prograde;
            f
        }
    }
}
