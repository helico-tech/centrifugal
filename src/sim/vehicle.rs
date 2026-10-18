//! Rockets as a stack of stages. Only the bottom stage's engine burns;
//! staging drops it. Numbers are game numbers: tiny planets need little
//! delta-v, so engines have a low Isp and tanks still look rocket-sized.

use crate::engine::gfx::rgb;

pub const G0: f64 = 9.81;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StageKind {
    Booster,
    Upper,
    /// Crew capsule: heat shield and parachute, no engine.
    Capsule,
    /// Payload that counts as delivered when separated in the right orbit.
    Satellite,
    /// Lander with legs and its own engine.
    Lander,
}

#[derive(Clone, Debug)]
pub struct Stage {
    pub name: &'static str,
    pub kind: StageKind,
    /// Mass without fuel, and fuel (kg).
    pub dry: f64,
    pub fuel: f64,
    pub fuel_max: f64,
    /// Thrust (N) and specific impulse (s); 0 thrust = no engine.
    pub thrust: f64,
    pub isp: f64,
    /// Size in meters.
    pub width: f64,
    pub height: f64,
    pub legs: bool,
    pub fins: bool,
    pub chute: bool,
    pub color: u32,
    pub trim: u32,
}

impl Stage {
    pub fn mass(&self) -> f64 {
        self.dry + self.fuel
    }
}

#[derive(Clone, Debug)]
pub struct Vehicle {
    /// Bottom (burning) stage first.
    pub stages: Vec<Stage>,
}

impl Vehicle {
    pub fn mass(&self) -> f64 {
        self.stages.iter().map(Stage::mass).sum()
    }
    pub fn active(&self) -> Option<&Stage> {
        self.stages.first()
    }
    pub fn active_mut(&mut self) -> Option<&mut Stage> {
        self.stages.first_mut()
    }
    pub fn can_thrust(&self) -> bool {
        self.active().is_some_and(|s| s.thrust > 0.0 && s.fuel > 0.0)
    }
    pub fn max_thrust(&self) -> f64 {
        self.active().map(|s| if s.fuel > 0.0 { s.thrust } else { 0.0 }).unwrap_or(0.0)
    }
    /// Fuel burned per second at full throttle.
    pub fn mdot(&self) -> f64 {
        self.active().map(|s| if s.thrust > 0.0 { s.thrust / (s.isp * G0) } else { 0.0 }).unwrap_or(0.0)
    }
    pub fn height(&self) -> f64 {
        self.stages.iter().map(|s| s.height).sum()
    }
    pub fn width(&self) -> f64 {
        self.stages.iter().map(|s| s.width).fold(0.0, f64::max)
    }
    /// Distance from the bottom of the stack to its center of mass.
    pub fn com(&self) -> f64 {
        let mut y = 0.0;
        let mut m = 0.0;
        let mut acc = 0.0;
        for s in &self.stages {
            acc += s.mass() * (y + s.height * 0.5);
            m += s.mass();
            y += s.height;
        }
        if m > 0.0 { acc / m } else { 0.0 }
    }
    pub fn has_legs(&self) -> bool {
        self.active().is_some_and(|s| s.legs)
    }
    pub fn has_chute(&self) -> bool {
        self.stages.iter().any(|s| s.chute)
    }
    /// Delta-v of stage `i` and everything above it riding along.
    pub fn stage_dv(&self, i: usize) -> f64 {
        let s = &self.stages[i];
        if s.thrust <= 0.0 {
            return 0.0;
        }
        let above: f64 = self.stages[i + 1..].iter().map(Stage::mass).sum();
        let m0 = above + s.mass();
        let m1 = above + s.dry;
        s.isp * G0 * (m0 / m1).ln()
    }
    pub fn dv(&self) -> f64 {
        (0..self.stages.len()).map(|i| self.stage_dv(i)).sum()
    }
    pub fn fuel_frac(&self) -> f64 {
        let (f, m): (f64, f64) = self.stages.iter().fold((0.0, 0.0), |(f, m), s| (f + s.fuel, m + s.fuel_max));
        if m > 0.0 { f / m } else { 0.0 }
    }
    /// Drop the bottom stage; returns it as its own vehicle.
    pub fn separate(&mut self) -> Option<Vehicle> {
        if self.stages.len() < 2 {
            return None;
        }
        Some(Vehicle { stages: vec![self.stages.remove(0)] })
    }
}

/// Size a stage's tank so it gives `dv` with `above` kg riding on top.
pub fn fit_fuel(stage: &mut Stage, above: f64, dv: f64) {
    let ratio = (dv / (stage.isp * G0)).exp();
    let fuel = (above + stage.dry) * (ratio - 1.0);
    stage.fuel = fuel;
    stage.fuel_max = fuel;
}

pub const WHITE: u32 = rgb(232, 232, 240);
pub const RED: u32 = rgb(216, 48, 48);
pub const ORANGE: u32 = rgb(240, 136, 32);
pub const BLUE: u32 = rgb(48, 96, 200);
pub const TEAL: u32 = rgb(40, 168, 160);
pub const GOLD: u32 = rgb(232, 184, 48);
pub const DARK: u32 = rgb(56, 56, 72);

pub fn booster(height: f64, thrust: f64) -> Stage {
    Stage {
        name: "BOOSTER",
        kind: StageKind::Booster,
        dry: 5000.0,
        fuel: 0.0,
        fuel_max: 0.0,
        thrust,
        isp: 140.0,
        width: 4.0,
        height,
        legs: false,
        fins: true,
        chute: false,
        color: WHITE,
        trim: RED,
    }
}

pub fn upper(height: f64, thrust: f64) -> Stage {
    Stage {
        name: "UPPER STAGE",
        kind: StageKind::Upper,
        dry: 1200.0,
        fuel: 0.0,
        fuel_max: 0.0,
        thrust,
        isp: 190.0,
        width: 4.0,
        height,
        legs: false,
        fins: false,
        chute: false,
        color: WHITE,
        trim: BLUE,
    }
}

pub fn capsule() -> Stage {
    Stage {
        name: "CAPSULE",
        kind: StageKind::Capsule,
        dry: 1500.0,
        fuel: 0.0,
        fuel_max: 0.0,
        thrust: 0.0,
        isp: 1.0,
        width: 3.6,
        height: 4.0,
        legs: false,
        fins: false,
        chute: true,
        color: rgb(216, 216, 224),
        trim: DARK,
    }
}

pub fn satellite(name: &'static str) -> Stage {
    Stage {
        name,
        kind: StageKind::Satellite,
        dry: 800.0,
        fuel: 0.0,
        fuel_max: 0.0,
        thrust: 0.0,
        isp: 1.0,
        width: 4.0,
        height: 6.0,
        legs: false,
        fins: false,
        chute: false,
        color: GOLD,
        trim: rgb(200, 200, 216),
    }
}

pub fn lander(thrust: f64) -> Stage {
    Stage {
        name: "LANDER",
        kind: StageKind::Lander,
        dry: 1400.0,
        fuel: 0.0,
        fuel_max: 0.0,
        thrust,
        isp: 180.0,
        width: 4.6,
        height: 5.0,
        legs: true,
        fins: false,
        chute: false,
        color: GOLD,
        trim: rgb(176, 176, 192),
    }
}
