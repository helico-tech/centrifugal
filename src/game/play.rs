//! The flight screen: keyboard and mouse to flight controls, the camera
//! (zoom, pan, focus, auto "up"), drawing the world, the path and the HUD.

use crate::engine::gfx::{Frame, mix, rgb, shade, text_width};
use crate::engine::input::{Input, Key};
use crate::engine::ui::{LOGO_SHADOW, OUTLINE, SUNSET, grad6};
use crate::engine::v2::V2;
use crate::game::flight::{Auto, Flight, Log, PHYSICS_WARP, Sas, Status, WARPS};
use crate::game::fx::{self, Fx, P};
use crate::game::mission::{self, Mission, Outcome, Tracker};
use crate::render::body::{self, BodyGfx};
use crate::render::camera::Camera;
use crate::render::space::{self, Sky};
use crate::render::{rocket, scenery, text, traj};
use crate::sim::physics::{self, TICK};
use crate::sim::predict::Event;
use crate::sim::system::System;
use crate::sim::vehicle::StageKind;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::rc::Rc;

pub const PANEL: u32 = rgb(16, 16, 40);
pub const DIM: u32 = rgb(120, 136, 184);
pub const BRIGHT: u32 = rgb(248, 248, 248);
pub const GOOD: u32 = rgb(120, 248, 120);
pub const WARN: u32 = rgb(248, 200, 64);
pub const BAD: u32 = rgb(248, 80, 64);
pub const CYAN: u32 = rgb(88, 232, 248);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Focus {
    Craft,
    Body(usize),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    Sas(Sas),
    WarpUp,
    WarpDown,
    WarpNext,
    ExecNode,
    Stage,
    Rewind,
    Pause,
    Map,
    Target,
}

#[derive(Clone)]
pub struct Btn {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub act: Act,
    pub label: &'static str,
    pub key: &'static str,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sfx {
    Blip,
    Confirm,
    Stage,
    Boom,
    Liftoff,
    Touchdown,
    Chute,
    Beep,
    Node,
    Warn,
    Soi,
}

struct Banner {
    text: String,
    color: u32,
    t: u32,
}

pub struct Play {
    pub flight: Flight,
    pub mission: &'static Mission,
    pub tracker: Tracker,
    pub fx: Fx,
    pub gfx: Rc<Vec<BodyGfx>>,
    pub sky: Rc<Sky>,
    pub focus: Focus,
    pub mpp: f64,
    pub mpp_target: f64,
    pub pan: V2,
    pub up: f64,
    pub tick: u32,
    pub selected: Option<usize>,
    drag: Option<Drag>,
    last_mouse: (f32, f32),
    drawn: traj::Drawn,
    banners: Vec<Banner>,
    pub sfx: Vec<Sfx>,
    pub paused: bool,
    pub pause_sel: usize,
    pub quit: bool,
    pub help: bool,
    pub hint: Option<&'static str>,
    pub end_timer: u32,
    booster_swapped: bool,
    cam: Camera,
    w: i32,
    h: i32,
    shake: (i32, i32),
}

#[derive(Clone, Copy)]
enum Drag {
    Pan,
    Handle(usize, usize),
}

pub const PAUSE_ITEMS: [&str; 5] = ["RESUME", "REWIND", "RESTART", "CONTROLS", "QUIT TO MENU"];

impl Play {
    pub fn new(m: &'static Mission, sys: Rc<System>, gfx: Rc<Vec<BodyGfx>>, sky: Rc<Sky>, seed: u64) -> Self {
        let flight = mission::start(m, sys);
        // the rocket about a third of the screen tall
        let mpp = if matches!(flight.craft.status, Status::Landed { .. }) { (flight.craft.vehicle.height() / 110.0).clamp(0.12, 0.4) } else { 2.0 };
        let up = flight.local_up_angle();
        let cam = Camera::new(V2::ZERO, mpp, up, 640, 360);
        Play {
            flight,
            mission: m,
            tracker: Tracker::new(),
            fx: Fx::new(seed),
            gfx,
            sky,
            focus: Focus::Craft,
            mpp,
            mpp_target: mpp,
            pan: V2::ZERO,
            up,
            tick: 0,
            selected: None,
            drag: None,
            last_mouse: (0.0, 0.0),
            drawn: traj::Drawn::default(),
            banners: vec![],
            sfx: vec![],
            paused: false,
            pause_sel: 0,
            quit: false,
            help: false,
            hint: None,
            end_timer: 0,
            booster_swapped: false,
            cam,
            w: 640,
            h: 360,
            shake: (0, 0),
        }
    }

    pub fn frame_size(&self) -> (i32, i32) {
        (self.w, self.h)
    }

    fn banner(&mut self, s: impl Into<String>, color: u32) {
        self.banners.retain(|b| b.t < 100);
        self.banners.push(Banner { text: s.into(), color, t: 0 });
    }

    pub fn engine_volume(&self) -> f32 {
        let c = &self.flight.craft;
        if c.burning && self.flight.warp <= PHYSICS_WARP && !self.paused { (0.3 + c.throttle * 0.7) as f32 } else { 0.0 }
    }

    pub fn buttons(&self) -> Vec<Btn> {
        let (w, h) = (self.w, self.h);
        let mut v = vec![];
        let y0 = h - 32;
        let sas = [
            (Sas::Hold, "HOLD", "1"),
            (Sas::Prograde, "PRO", "2"),
            (Sas::Retrograde, "RETRO", "3"),
            (Sas::RadialOut, "RAD+", "4"),
            (Sas::RadialIn, "RAD-", "5"),
            (Sas::Node, "NODE", "6"),
        ];
        for (i, (s, l, k)) in sas.iter().enumerate() {
            let (cx, cy) = ((i % 3) as i32, (i / 3) as i32);
            v.push(Btn { x: 64 + cx * 40, y: y0 + 2 + cy * 15, w: 38, h: 13, act: Act::Sas(*s), label: l, key: k });
        }
        let cx = w / 2;
        v.push(Btn { x: cx - 64, y: y0 + 2, w: 16, h: 13, act: Act::WarpDown, label: "<", key: "," });
        v.push(Btn { x: cx + 48, y: y0 + 2, w: 16, h: 13, act: Act::WarpUp, label: ">", key: "." });
        v.push(Btn { x: cx - 64, y: y0 + 17, w: 62, h: 13, act: Act::WarpNext, label: "WARP TO", key: "G" });
        v.push(Btn { x: cx + 2, y: y0 + 17, w: 62, h: 13, act: Act::ExecNode, label: "AUTO BURN", key: "N" });
        v.push(Btn { x: cx + 70, y: y0 + 2, w: 46, h: 13, act: Act::Stage, label: "STAGE", key: "SPC" });
        v.push(Btn { x: cx + 70, y: y0 + 17, w: 46, h: 13, act: Act::Rewind, label: "REWIND", key: "R" });
        v.push(Btn { x: cx + 120, y: y0 + 2, w: 40, h: 13, act: Act::Map, label: "MAP", key: "M" });
        v.push(Btn { x: cx + 120, y: y0 + 17, w: 40, h: 13, act: Act::Target, label: "TARGET", key: "T" });
        v.push(Btn { x: w - 22, y: 27, w: 18, h: 13, act: Act::Pause, label: "II", key: "ESC" });
        v
    }

    fn act(&mut self, a: Act) {
        let f = &mut self.flight;
        match a {
            Act::Sas(s) => {
                f.sas = s;
                f.auto = Auto::Off;
            }
            Act::WarpUp => f.warp_up(),
            Act::WarpDown => f.warp_down(),
            Act::WarpNext => self.warp_next(),
            Act::ExecNode => {
                if f.auto == Auto::Node {
                    f.auto = Auto::Off;
                    f.set_throttle(0.0);
                } else if !f.nodes.is_empty() {
                    f.auto = Auto::Node;
                    self.banner("AUTO BURN", CYAN);
                }
            }
            Act::Stage => self.stage(),
            Act::Rewind => {
                if self.flight.rewind() {
                    self.fx.clear();
                    self.tracker.outcome = Outcome::Flying;
                    self.end_timer = 0;
                    self.banner("REWIND!", CYAN);
                }
            }
            Act::Pause => self.paused = true,
            Act::Map => {
                self.mpp_target = if self.mpp < 300.0 { 3000.0 } else { 1.5 };
                self.pan = V2::ZERO;
                self.focus = Focus::Craft;
            }
            Act::Target => self.cycle_target(),
        }
        self.sfx.push(Sfx::Blip);
    }

    fn cycle_target(&mut self) {
        let sys = self.flight.sys.clone();
        let n = sys.bodies.len();
        let dom = self.flight.dominant();
        let start = self.flight.target.map(|t| t + 1).unwrap_or(1);
        let mut next = None;
        for k in 0..n {
            let i = (start + k) % n;
            if i != 0 && i != dom {
                next = Some(i);
                break;
            }
        }
        // after the last body: no target
        if self.flight.target.is_some() && next.is_some_and(|i| i < start) {
            next = None;
        }
        self.flight.target = next;
        self.flight.pred_dirty = true;
        if let Some(t) = next {
            self.banner(format!("TARGET {}", sys.bodies[t].name), rgb(248, 200, 120));
        }
    }

    fn warp_next(&mut self) {
        let f = &mut self.flight;
        let now = f.t;
        let mut best: Option<f64> = f.node_warp_point();
        for e in &f.pred.events {
            let t = match *e {
                Event::Apoapsis { t, .. } | Event::Periapsis { t, .. } | Event::Closest { t, .. } => t - 10.0,
                Event::Soi { t, .. } | Event::AtmoEnter { t, .. } => t - 2.0,
                _ => continue,
            };
            if t > now + 5.0 && best.is_none_or(|b| t < b) {
                best = Some(t);
            }
        }
        if let Some(t) = best {
            f.warp_to = Some(t);
        }
    }

    fn stage(&mut self) {
        let before = self.flight.craft.vehicle.stages.len();
        self.flight.stage();
        if self.mission.fly_booster && !self.booster_swapped && self.flight.craft.vehicle.stages.len() < before {
            // keep flying the booster: swap it with the stack that flies on
            if let Some(mut d) = self.flight.debris.pop() {
                std::mem::swap(&mut self.flight.craft, &mut d);
                self.flight.craft.throttle = 0.0;
                self.flight.craft.legs = false;
                self.flight.sas = Sas::Hold;
                d.throttle = 0.0;
                // the upper stage leaves on its own, like the real thing
                d.ship.vel += d.dir() * 60.0;
                self.flight.debris.push(d);
                self.booster_swapped = true;
                self.flight.pred_dirty = true;
                self.banner("UPPER STAGE AWAY - BRING THE BOOSTER HOME!", WARN);
            }
        }
    }

    pub fn restart(&mut self) {
        let sys = self.flight.sys.clone();
        *self = Play::new(self.mission, sys, self.gfx.clone(), self.sky.clone(), self.tick as u64 + 7);
    }

    // ------------------------------------------------------------ update

    pub fn update(&mut self, inp: &Input) {
        self.tick += 1;
        if self.help {
            if inp.pressed(Key::F1) || inp.pressed(Key::Esc) || inp.confirm() || inp.mpressed[0] {
                self.help = false;
            }
            return;
        }
        if inp.pressed(Key::F1) {
            self.help = true;
            return;
        }
        if self.paused {
            self.update_pause(inp);
            return;
        }
        if inp.pressed(Key::Esc) {
            self.paused = true;
            self.pause_sel = 0;
            self.sfx.push(Sfx::Blip);
            return;
        }
        self.controls(inp);
        self.mouse(inp);

        let prev_status = self.flight.craft.status;
        self.flight.update();
        self.tracker.update(self.mission, &self.flight);
        self.react(prev_status);
        self.particles();
        self.camera();
        for b in &mut self.banners {
            b.t += 1;
        }
        self.banners.retain(|b| b.t < 150);
        if self.tracker.outcome != Outcome::Flying {
            self.end_timer += 1;
        }
    }

    fn update_pause(&mut self, inp: &Input) {
        if inp.pressed(Key::Esc) {
            self.paused = false;
            return;
        }
        let n = PAUSE_ITEMS.len();
        if inp.pressed(Key::Up) || inp.pressed(Key::W) {
            self.pause_sel = (self.pause_sel + n - 1) % n;
            self.sfx.push(Sfx::Blip);
        }
        if inp.pressed(Key::Down) || inp.pressed(Key::S) {
            self.pause_sel = (self.pause_sel + 1) % n;
            self.sfx.push(Sfx::Blip);
        }
        // mouse hover/click over the menu items
        let (bx, by, bw, _) = self.pause_box();
        let (mx, my) = (inp.mx as i32, inp.my as i32);
        let mut clicked = false;
        for i in 0..n {
            let y = by + 26 + i as i32 * 14;
            if mx >= bx && mx < bx + bw && my >= y - 2 && my < y + 10 {
                if inp.mpressed[0] {
                    self.pause_sel = i;
                    clicked = true;
                }
            }
        }
        if inp.confirm() || clicked {
            self.sfx.push(Sfx::Confirm);
            match self.pause_sel {
                0 => self.paused = false,
                1 => {
                    self.paused = false;
                    self.act(Act::Rewind);
                }
                2 => self.restart(),
                3 => self.help = true,
                _ => self.quit = true,
            }
        }
    }

    fn pause_box(&self) -> (i32, i32, i32, i32) {
        let (bw, bh) = (150, 26 + PAUSE_ITEMS.len() as i32 * 14 + 8);
        ((self.w - bw) / 2, (self.h - bh) / 2, bw, bh)
    }

    fn controls(&mut self, inp: &Input) {
        let f = &mut self.flight;
        let rot = (inp.down(Key::Left) || inp.down(Key::A)) as i32 - (inp.down(Key::Right) || inp.down(Key::D)) as i32;
        f.rotate(rot as f64);
        let thr_up = inp.down(Key::Up) || inp.down(Key::W) || inp.down(Key::Shift);
        let thr_dn = inp.down(Key::Down) || inp.down(Key::S) || inp.down(Key::Ctrl);
        if thr_up || thr_dn {
            if f.auto == Auto::Node {
                f.auto = Auto::Off;
            }
            let t = f.craft.throttle + if thr_up { 0.9 } else { -0.9 } * TICK;
            f.set_throttle(t);
        }
        if inp.pressed(Key::Z) {
            f.set_throttle(1.0);
            f.auto = Auto::Off;
        }
        if inp.pressed(Key::X) {
            f.set_throttle(0.0);
            f.auto = Auto::Off;
        }
        let keys = [
            (Key::Num1, Sas::Hold),
            (Key::Num2, Sas::Prograde),
            (Key::Num3, Sas::Retrograde),
            (Key::Num4, Sas::RadialOut),
            (Key::Num5, Sas::RadialIn),
            (Key::Num6, Sas::Node),
        ];
        for (k, s) in keys {
            if inp.pressed(k) {
                self.act(Act::Sas(s));
            }
        }
        let acts = [
            (Key::Space, Act::Stage),
            (Key::Comma, Act::WarpDown),
            (Key::Period, Act::WarpUp),
            (Key::G, Act::WarpNext),
            (Key::N, Act::ExecNode),
            (Key::R, Act::Rewind),
            (Key::M, Act::Map),
            (Key::T, Act::Target),
        ];
        for (k, a) in acts {
            if inp.pressed(k) {
                self.act(a);
            }
        }
        if inp.pressed(Key::F) {
            let sys = self.flight.sys.clone();
            self.focus = match self.focus {
                Focus::Craft => Focus::Body(self.flight.dominant()),
                Focus::Body(i) => match self.flight.target {
                    Some(t) if t != i => Focus::Body(t),
                    _ if i != 0 => Focus::Body(0),
                    _ => Focus::Craft,
                },
            };
            self.pan = V2::ZERO;
            if let Focus::Body(i) = self.focus {
                let r = if i == 0 { sys.bodies.iter().map(|b| b.orbit.a).fold(0.0, f64::max) * 0.6 } else { sys.bodies[i].radius * 3.0 };
                self.mpp_target = r / (self.h as f64 * 0.5);
            } else {
                self.mpp_target = 2.0;
            }
        }
        if inp.pressed(Key::Home) || inp.pressed(Key::C) {
            self.focus = Focus::Craft;
            self.pan = V2::ZERO;
        }
        if inp.pressed(Key::Delete) || inp.pressed(Key::Backspace) {
            if let Some(i) = self.selected.take() {
                self.flight.delete_node(i);
                self.sfx.push(Sfx::Blip);
            }
        }
    }

    fn mouse(&mut self, inp: &Input) {
        let (mx, my) = (inp.mx, inp.my);
        let (lx, ly) = self.last_mouse;
        self.last_mouse = (mx, my);
        if inp.wheel != 0.0 {
            // over a node handle: fine tune it
            if let (Some(sel), Some(hs)) = (self.selected, self.drawn.handles) {
                for (k, &(hx, hy)) in hs.iter().enumerate() {
                    if (hx - mx).hypot(hy - my) < 8.0 {
                        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
                        let d = inp.wheel.signum() as f64 * sign;
                        if let Some(n) = self.flight.nodes.get_mut(sel) {
                            if k < 2 { n.pro += d } else { n.rad += d }
                        }
                        self.flight.pred_dirty = true;
                        return;
                    }
                }
            }
            // zoom around the mouse
            let before = self.cam.to_world(mx as f64, my as f64);
            let k = 1.25f64.powf(-inp.wheel as f64);
            self.mpp_target = (self.mpp_target * k).clamp(0.12, 400_000.0);
            let c2 = self.cam.with(self.cam.center, self.mpp_target, self.cam.up);
            let after = c2.to_world(mx as f64, my as f64);
            self.pan += before - after;
        }
        if inp.mpressed[0] {
            // HUD buttons
            for b in self.buttons() {
                if mx as i32 >= b.x && (mx as i32) < b.x + b.w && my as i32 >= b.y && (my as i32) < b.y + b.h {
                    self.act(b.act);
                    return;
                }
            }
            if my < 26.0 || my > self.h as f32 - 34.0 {
                return;
            }
            if let (Some(sel), Some(hs)) = (self.selected, self.drawn.handles) {
                for (k, &(hx, hy)) in hs.iter().enumerate() {
                    if (hx - mx).hypot(hy - my) < 7.0 {
                        self.drag = Some(Drag::Handle(sel, k));
                        return;
                    }
                }
            }
            for (i, &(nx, ny)) in self.drawn.nodes.iter().enumerate() {
                if (nx - mx).hypot(ny - my) < 7.0 {
                    self.selected = Some(i);
                    self.sfx.push(Sfx::Blip);
                    return;
                }
            }
            if let Some((_, t)) = self.drawn.pick(mx, my, 5.0) {
                if t > self.flight.t + 2.0 && self.flight.nodes.len() < 4 {
                    self.flight.add_node(t);
                    self.selected = self.flight.nodes.iter().position(|n| n.t == t.max(self.flight.t + 1.0));
                    self.sfx.push(Sfx::Node);
                    return;
                }
            }
            // a body: make it the target, again: focus it
            let sys = self.flight.sys.clone();
            for (i, b) in sys.bodies.iter().enumerate().skip(1) {
                let (x, y) = self.cam.to_screen(self.flight.st[i].pos);
                let r = (b.radius / self.cam.mpp).max(6.0) as f32;
                if (x as f32 - mx).hypot(y as f32 - my) < r && i != self.flight.dominant() {
                    if self.flight.target == Some(i) {
                        self.focus = Focus::Body(i);
                        self.pan = V2::ZERO;
                    } else {
                        self.flight.target = Some(i);
                        self.flight.pred_dirty = true;
                        self.banner(format!("TARGET {}", b.name), rgb(248, 200, 120));
                    }
                    return;
                }
            }
            self.selected = None;
            self.drag = Some(Drag::Pan);
        }
        if inp.mpressed[1] {
            for (i, &(nx, ny)) in self.drawn.nodes.iter().enumerate() {
                if (nx - mx).hypot(ny - my) < 7.0 {
                    self.flight.delete_node(i);
                    self.selected = None;
                    self.sfx.push(Sfx::Blip);
                    return;
                }
            }
            self.drag = Some(Drag::Pan);
        }
        if !inp.mdown[0] && !inp.mdown[1] {
            self.drag = None;
        }
        match self.drag {
            Some(Drag::Pan) => {
                let a = self.cam.to_world(lx as f64, ly as f64);
                let b = self.cam.to_world(mx as f64, my as f64);
                self.pan += a - b;
            }
            Some(Drag::Handle(i, k)) => {
                if let Some(hs) = self.drawn.handles {
                    // movement along the handle's axis changes delta-v
                    let (nx, ny) = self.drawn.nodes.get(i).copied().unwrap_or((0.0, 0.0));
                    let (hx, hy) = hs[k];
                    let (ax, ay) = (hx - nx, hy - ny);
                    let l = ax.hypot(ay).max(1.0);
                    let along = ((mx - lx) * ax + (my - ly) * ay) / l;
                    let fine = if inp.down(Key::Shift) { 0.1 } else { 1.0 };
                    let d = along as f64 * 0.6 * fine;
                    if let Some(n) = self.flight.nodes.get_mut(i) {
                        match k {
                            0 => n.pro += d,
                            1 => n.pro -= d,
                            2 => n.rad += d,
                            _ => n.rad -= d,
                        }
                    }
                    if along != 0.0 {
                        self.flight.pred_dirty = true;
                    }
                }
            }
            None => {}
        }
    }

    /// Sounds, banners and explosions from what happened this tick.
    fn react(&mut self, prev: Status) {
        let logs = self.flight.log.clone();
        let sys = self.flight.sys.clone();
        for l in logs {
            match l {
                Log::Liftoff => {
                    self.banner("LIFTOFF!", WARN);
                    self.sfx.push(Sfx::Liftoff);
                }
                Log::Staged { kind } => {
                    self.sfx.push(Sfx::Stage);
                    if kind == StageKind::Upper || kind == StageKind::Booster {
                        self.banner("STAGING", BRIGHT);
                    }
                    let (p, v) = self.flight.pos_vel();
                    for _ in 0..12 {
                        let a = self.fx.r(0.0, 6.28) as f64;
                        self.fx.add(P { pos: p, vel: v + V2::from_angle(a) * 6.0, age: 0.0, life: 1.0, r0: 1.0, r1: 3.0, kind: fx::Kind::Smoke });
                    }
                }
                Log::Landed { speed, .. } => {
                    self.sfx.push(Sfx::Touchdown);
                    self.banner(format!("TOUCHDOWN! {}", text::speed(speed)), GOOD);
                }
                Log::Splashdown { .. } => {
                    self.sfx.push(Sfx::Touchdown);
                    self.banner("SPLASHDOWN!", GOOD);
                }
                Log::Crashed { .. } => {
                    self.sfx.push(Sfx::Boom);
                    let (p, v) = self.flight.pos_vel();
                    let size = (self.flight.craft.vehicle.height() / 20.0).clamp(0.6, 2.0) as f32;
                    self.fx.explode(p, v, size);
                }
                Log::Debris { pos } => {
                    if self.cam.visible(pos, 50.0) {
                        let v = self.flight.st[self.flight.sys.dominant(pos, &self.flight.st)].vel;
                        self.fx.explode(pos, v, 0.8);
                        self.sfx.push(Sfx::Boom);
                    }
                }
                Log::Soi { body } => {
                    self.sfx.push(Sfx::Soi);
                    self.banner(format!("ENTERING {}", sys.bodies[body].name), CYAN);
                }
                Log::NodeDone => {
                    self.sfx.push(Sfx::Node);
                    self.banner("BURN COMPLETE", GOOD);
                    self.selected = None;
                }
                Log::Flameout => {
                    self.sfx.push(Sfx::Warn);
                    self.banner("FLAMEOUT - SPACE TO STAGE", WARN);
                }
                Log::ChuteOpen => {
                    self.sfx.push(Sfx::Chute);
                    self.banner("CHUTE OUT", BRIGHT);
                }
                Log::LegsDown => self.sfx.push(Sfx::Stage),
            }
        }
        // dominant body change shows as an SOI banner
        let _ = prev;
        match self.tracker.outcome {
            Outcome::Success if self.end_timer == 0 => self.banner("MISSION COMPLETE!", GOOD),
            Outcome::Failed(why) if self.end_timer == 0 => self.banner(why, BAD),
            _ => {}
        }
    }

    fn particles(&mut self) {
        let warp = WARPS[self.flight.warp];
        if warp > 4.0 {
            self.fx.clear();
            return;
        }
        let sys = self.flight.sys.clone();
        let st = self.flight.st.clone();
        let t = self.flight.t;
        let dt = (TICK * warp) as f32;
        self.fx.update(dt, |p| physics::air(&sys, &st, t, p).map(|(rho, wind)| (wind, rho)));
        let c = &self.flight.craft;
        if c.burning {
            let (p, v) = self.flight.pos_vel();
            let dir = c.dir();
            let h = c.vehicle.height();
            let nozzle = p - dir * (h * 0.5 + 1.0);
            let air = self.flight.in_air();
            let n = (1.0 + c.throttle * 3.0) as usize;
            for _ in 0..n {
                let spread = self.fx.r(-0.25, 0.25) as f64;
                let ev = v - dir.rot(spread) * (80.0 + self.fx.r(0.0, 60.0) as f64);
                self.fx.add(P { pos: nozzle, vel: ev, age: 0.0, life: self.fx.r(0.15, 0.4), r0: 1.2, r1: 2.5, kind: fx::Kind::Fire });
                if air {
                    let sv = v - dir.rot(spread * 2.0) * (30.0 + self.fx.r(0.0, 30.0) as f64);
                    self.fx.add(P { pos: nozzle - dir * 4.0, vel: sv, age: 0.0, life: self.fx.r(2.0, 6.0), r0: 2.0, r1: 9.0, kind: fx::Kind::Smoke });
                }
            }
            // launch clouds rolling along the ground
            if air && self.flight.altitude() < 60.0 {
                let up = self.flight.up();
                for _ in 0..3 {
                    let side = if self.fx.r(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 };
                    let sv = self.flight.st[self.flight.dominant()].vel + up.perp() * (side * self.fx.r(15.0, 45.0) as f64) + up * 3.0;
                    let ground = p - up * (self.flight.altitude() + h * 0.5);
                    self.fx.add(P { pos: ground, vel: sv, age: 0.0, life: self.fx.r(3.0, 7.0), r0: 3.0, r1: 12.0, kind: fx::Kind::Smoke });
                }
            }
        }
        // reentry plasma
        let heat = self.heat();
        if heat > 0.05 {
            let (p, v) = self.flight.pos_vel();
            let sv = self.flight.surface_vel();
            let lead = sv.norm();
            for _ in 0..(heat * 6.0) as usize {
                let off = lead.perp() * self.fx.r(-3.0, 3.0) as f64 + lead * 4.0;
                self.fx.add(P {
                    pos: p + off,
                    vel: v - sv * 0.3 + lead.perp() * self.fx.r(-8.0, 8.0) as f64,
                    age: 0.0,
                    life: self.fx.r(0.2, 0.5),
                    r0: 1.5,
                    r1: 0.5,
                    kind: fx::Kind::Plasma,
                });
            }
        }
    }

    /// Reentry glow 0..1 from air density and speed (looks only).
    pub fn heat(&self) -> f64 {
        let f = &self.flight;
        if !f.in_air() || matches!(f.craft.status, Status::Landed { .. } | Status::Crashed) {
            return 0.0;
        }
        let Some((rho, _)) = physics::air(&f.sys, &f.st, f.t, f.pos_vel().0) else { return 0.0 };
        let v = f.surface_vel().len();
        ((rho.sqrt() * v * v * v / 2.0e7) - 0.25).clamp(0.0, 1.0)
    }

    fn focus_pos(&self) -> V2 {
        match self.focus {
            Focus::Craft => self.flight.pos_vel().0,
            Focus::Body(i) => self.flight.st[i].pos,
        }
    }

    fn camera(&mut self) {
        let k = 0.2;
        self.mpp = (self.mpp.ln() + (self.mpp_target.ln() - self.mpp.ln()) * k).exp();
        // near a planet the ground is down; zoomed out, north is up
        let want = match self.focus {
            Focus::Craft => {
                let local = self.flight.local_up_angle();
                let z = ((self.mpp.log10() - 1.3) / 1.0).clamp(0.0, 1.0);
                let mut d = (FRAC_PI_2 - local).rem_euclid(TAU);
                if d > PI {
                    d -= TAU;
                }
                local + d * z
            }
            Focus::Body(_) => FRAC_PI_2,
        };
        let mut d = (want - self.up).rem_euclid(TAU);
        if d > PI {
            d -= TAU;
        }
        self.up += d * 0.15;
        self.shake = if self.fx.shake > 0.5 {
            let s = self.fx.shake as i32;
            (self.fx.int(s * 2 + 1) - s, self.fx.int(s * 2 + 1) - s)
        } else {
            (0, 0)
        };
    }

    // ------------------------------------------------------------ draw

    pub fn draw(&mut self, fr: &mut Frame) {
        self.w = fr.w;
        self.h = fr.h;
        let center = self.focus_pos() + self.pan;
        let mut cam = Camera::new(center, self.mpp, self.up, fr.w, fr.h);
        if self.shake != (0, 0) {
            cam = cam.with(cam.to_world(fr.w as f64 * 0.5 + self.shake.0 as f64, fr.h as f64 * 0.5 + self.shake.1 as f64), self.mpp, self.up);
        }
        self.cam = cam;
        let f = &self.flight;
        let sys = f.sys.clone();
        let st = f.st.clone();
        space::draw_sky(fr, &cam, &self.sky, self.tick);
        if self.mpp > 40.0 {
            space::draw_orbits(fr, &cam, &sys, &st, f.target);
        }
        space::draw_star(fr, &cam, &sys, st[0].pos, self.tick);
        let ctx = body::Ctx { sys: &sys, gfx: &self.gfx, t: f.t, tick: self.tick };
        for i in 1..sys.bodies.len() {
            body::draw(fr, &cam, &ctx, i, st[i].pos, st[0].pos);
        }
        let dom = f.dominant();
        scenery::draw(fr, &cam, &sys.bodies[dom], &self.gfx[dom], &st[dom], f.t, self.tick);
        self.fx.draw(fr, &cam, self.tick);
        let air = if f.in_air() {
            physics::air(&sys, &st, f.t, f.pos_vel().0).map(|(r, _)| (r / 0.3).min(1.0)).unwrap_or(0.0)
        } else {
            0.0
        };
        for d in &f.debris {
            let look = rocket::Look { legs: false, chute: false, throttle: 0.0, burning: false, air, tick: self.tick, heat: 0.0 };
            let (p, _) = crate::sim::predict::hermite(d.prev.t, d.prev.pos, d.prev.vel, d.ship.t, d.ship.pos, d.ship.vel, f.t);
            rocket::draw(fr, &cam, p, d.heading, &d.vehicle, &look);
        }
        if !matches!(f.craft.status, Status::Crashed) {
            let c = &f.craft;
            let look = rocket::Look {
                legs: c.legs,
                chute: c.chute,
                throttle: c.throttle,
                burning: c.burning && c.throttle > 0.0,
                air,
                tick: self.tick,
                heat: self.heat(),
            };
            rocket::draw(fr, &cam, f.pos_vel().0, c.heading, &c.vehicle, &look);
        }
        let inp = traj::Input {
            sys: &sys,
            pred: &f.pred,
            now: f.t,
            nodes: &f.nodes,
            selected: self.selected,
            target: f.target,
            tick: self.tick,
            rotating: f.surface_mode().then(|| f.dominant()),
        };
        self.drawn = traj::draw(fr, &cam, &inp);
        self.body_labels(fr, &cam);
        self.site_marker(fr, &cam);
        self.hud(fr);
        if self.paused {
            self.draw_pause(fr);
        }
        if self.help {
            draw_help(fr);
        }
    }

    fn body_labels(&self, fr: &mut Frame, cam: &Camera) {
        let f = &self.flight;
        let sys = &f.sys;
        for (i, b) in sys.bodies.iter().enumerate() {
            let rpx = b.radius / cam.mpp;
            if rpx > 60.0 {
                continue;
            }
            // moons only once their orbit is big enough to tell apart
            if let Some(p) = b.parent {
                if p != 0 && b.orbit.a / cam.mpp < 14.0 {
                    continue;
                }
            }
            let (x, y) = cam.to_screen(f.st[i].pos);
            if x < -20.0 || y < -20.0 || x > cam.w + 20.0 || y > cam.h + 20.0 {
                continue;
            }
            let r = rpx.max(2.0) as i32;
            if rpx < 2.5 && i != 0 {
                body::dot(fr, x as i32, y as i32, self.gfx[i].avg, 2);
            }
            let c = if f.target == Some(i) { rgb(248, 200, 120) } else if i == sys.home { CYAN } else { DIM };
            let name = &b.name;
            let w = text_width(name, false);
            fr.text_sh(x as i32 - w / 2, y as i32 + r + 4, name, c, OUTLINE, false);
            if f.target == Some(i) {
                let s = r + 4;
                for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let (cx, cy) = (x as i32 + dx * s, y as i32 + dy * s);
                    fr.hline(cx - dx.min(0) * 0 - if dx < 0 { 0 } else { 3 }, cx + if dx < 0 { 3 } else { 0 }, cy, c);
                    fr.line(cx, cy, cx, cy - dy * 3, c);
                }
            }
        }
    }

    /// Arrow at the screen edge pointing to the landing site when it is off screen.
    fn site_marker(&self, fr: &mut Frame, cam: &Camera) {
        let f = &self.flight;
        if !f.surface_mode() || cam.mpp > 60.0 {
            return;
        }
        let dom = f.dominant();
        let b = &f.sys.bodies[dom];
        let Some(s) = b.sites.first() else { return };
        let (p, _) = crate::game::flight::site_world(&f.sys, &f.st, dom, s.angle, f.t);
        let (x, y) = cam.to_screen(p);
        let (cx, cy) = (cam.w * 0.5, cam.h * 0.5);
        let inside = x > 10.0 && y > 30.0 && x < cam.w - 10.0 && y < cam.h - 40.0;
        let dist = f.site_distance(dom).unwrap_or(0.0).abs();
        if dist < 40.0 && !matches!(f.craft.status, Status::Flying) || dist < 15.0 {
            return;
        }
        if inside {
            let (xi, yi) = (x as i32, y as i32);
            if self.tick / 15 % 2 == 0 {
                fr.fill_poly(&[(xi - 4, yi - 14), (xi + 4, yi - 14), (xi, yi - 8)], GOOD);
            }
            traj::label(fr, xi - 12, yi - 24, &text::dist(dist), GOOD);
        } else {
            let a = (y - cy).atan2(x - cx);
            let r = (cam.w * 0.5 - 16.0).min(cam.h * 0.5 - 44.0);
            let (ex, ey) = ((cx + a.cos() * r) as i32, (cy + a.sin() * r) as i32);
            let (dx, dy) = (a.cos(), a.sin());
            let tip = (ex + (dx * 6.0) as i32, ey + (dy * 6.0) as i32);
            let l = (ex + (-dy * 4.0) as i32, ey + (dx * 4.0) as i32);
            let rr = (ex - (-dy * 4.0) as i32, ey - (dx * 4.0) as i32);
            fr.fill_poly(&[tip, l, rr], GOOD);
            traj::label(fr, ex - 12, ey + 8, &format!("{} {}", if b.sites[0].kind == crate::sim::system::SiteKind::LaunchPad { "PAD" } else { "LZ" }, text::dist(dist)), GOOD);
        }
    }

    fn hud(&self, fr: &mut Frame) {
        let (w, h) = (fr.w, fr.h);
        let f = &self.flight;
        let sys = &f.sys;
        // top band
        fr.rect(0, 0, w, 25, PANEL);
        fr.hline(0, w, 25, rgb(248, 176, 32));
        fr.hline(0, w, 26, rgb(176, 32, 88));
        let dom = f.dominant();
        let alt = f.altitude().max(0.0);
        let srf = f.surface_mode();
        let v = f.ref_vel();
        let up = f.up();
        let vs = f.surface_vel().dot(up);
        let hs = -f.surface_vel().dot(up.perp());
        let col = |fr: &mut Frame, x: i32, y: i32, lab: &str, val: &str, c: u32| {
            fr.text(x, y, lab, DIM);
            fr.text_sh(x + text_width(lab, false) + 4, y, val, c, OUTLINE, false);
        };
        col(fr, 4, 4, "", &text::met(f.t), BRIGHT);
        let status = match f.craft.status {
            Status::Landed { .. } => "LANDED",
            Status::Splashed { .. } => "SPLASHED",
            Status::Crashed => "LOST",
            Status::Flying => {
                if srf {
                    "FLYING"
                } else if mission::apsides(f, dom).is_some_and(|(pe, _)| pe > 0.0) {
                    "ORBITING"
                } else {
                    "COASTING"
                }
            }
        };
        let x1 = 122.min(w / 5);
        let mut line = format!("{} {}", status, sys.bodies[dom].name);
        while text_width(&line, false) > x1 - 8 && line.len() > 4 {
            line.pop();
        }
        fr.text_sh(4, 14, &line, CYAN, OUTLINE, false);
        col(fr, x1, 4, "ALT", &text::dist(alt), BRIGHT);
        let vsc = if vs < -8.0 && alt < 1500.0 { BAD } else if vs < -5.0 && alt < 500.0 { WARN } else { BRIGHT };
        col(fr, x1, 14, "V/S", &format!("{:+.1}", vs), vsc);
        let x2 = x1 + 86;
        col(fr, x2, 4, if srf { "SRF" } else { "ORB" }, &text::speed(v.len()), BRIGHT);
        col(fr, x2, 14, "H/S", &format!("{:+.1}", hs), BRIGHT);
        let x3 = x2 + 86;
        let grounded = matches!(f.craft.status, Status::Landed { .. } | Status::Splashed { .. } | Status::Crashed);
        let (ap, pe) = match mission::apsides(f, dom) {
            _ if grounded => ("-".into(), "-".into()),
            Some((pe, ap)) => (text::dist(ap), text::dist(pe)),
            None => ("ESCAPE".into(), f.pred.apsis(dom, false).map(|(_, a)| text::dist(a)).unwrap_or("-".into())),
        };
        col(fr, x3, 4, "AP", &ap, rgb(120, 200, 248));
        let pec = if !grounded && mission::apsides(f, dom).is_some_and(|(p, _)| p < 0.0) { BAD } else { rgb(120, 248, 168) };
        col(fr, x3, 14, "PE", &pe, pec);
        let x4 = x3 + 92;
        if x4 + 80 < w - 90 {
            col(fr, x4, 4, "DV", &format!("{:.0}", f.craft.vehicle.dv()), BRIGHT);
            col(fr, x4, 14, "TWR", &format!("{:.2}", f.twr()), BRIGHT);
        }
        // throttle and fuel gauges
        let gx = w - 92;
        fr.text(gx, 4, "THR", DIM);
        let tw = 24;
        fr.rect(gx + 20, 4, tw, 6, OUTLINE);
        fr.rect(gx + 20, 4, (tw as f64 * f.craft.throttle) as i32, 6, WARN);
        fr.text(gx, 14, "FUEL", DIM);
        let n = f.craft.vehicle.stages.iter().filter(|s| s.fuel_max > 0.0).count().max(1) as i32;
        let mut sx = gx + 26;
        let each = (40 / n).max(5);
        for s in f.craft.vehicle.stages.iter().filter(|s| s.fuel_max > 0.0) {
            let frac = s.fuel / s.fuel_max;
            fr.rect(sx, 14, each - 2, 6, OUTLINE);
            let c = if frac < 0.15 { BAD } else if frac < 0.35 { WARN } else { GOOD };
            fr.rect(sx, 14, ((each - 2) as f64 * frac).ceil() as i32, 6, c);
            sx += each;
        }
        let wtxt = format!("{}X", WARPS[f.warp]);
        fr.text_sh(w - 4 - text_width(&wtxt, false), 4, &wtxt, if f.warp > 0 { WARN } else { DIM }, OUTLINE, false);

        // objectives
        self.objectives(fr);

        // bottom panel
        let y0 = h - 33;
        fr.rect(0, y0, w, 33, PANEL);
        fr.hline(0, w, y0, rgb(176, 32, 88));
        self.dial(fr, 32, y0 - 2);
        for b in self.buttons() {
            let on = match b.act {
                Act::Sas(s) => f.sas == s,
                Act::ExecNode => f.auto == Auto::Node,
                Act::WarpNext => f.warp_to.is_some(),
                _ => false,
            };
            if b.act == Act::Pause {
                continue;
            }
            let bg = if on { rgb(48, 112, 200) } else { rgb(40, 40, 80) };
            fr.rect(b.x, b.y, b.w, b.h, bg);
            fr.rect_outline(b.x, b.y, b.w, b.h, if on { CYAN } else { rgb(72, 72, 128) });
            let lw = text_width(b.label, false);
            fr.text_sh(b.x + (b.w - lw) / 2, b.y + 3, b.label, if on { BRIGHT } else { rgb(200, 208, 232) }, OUTLINE, false);
        }
        // warp readout between the arrows
        let cx = w / 2;
        let wl = if let Some(wt) = f.warp_to { format!("TO {}", text::dur(wt - f.t)) } else { format!("WARP {}X", WARPS[f.warp]) };
        fr.text_center(cx - 8, y0 + 5, &wl, if f.warp > 0 { WARN } else { BRIGHT }, OUTLINE, false);
        // node / burn info on the right
        let rx = cx + 168;
        if rx + 100 < w {
            if let Some((dt, dv, bt)) = f.burn_info() {
                let start = dt - bt * 0.5;
                let c = if start < 5.0 && start > -bt { WARN } else { PLANC };
                fr.text_sh(rx, y0 + 4, &format!("NODE {:.0} M/S", dv), c, OUTLINE, false);
                let when = if start > 0.0 { format!("BURN IN {}", text::dur(start)) } else { "BURN NOW!".into() };
                fr.text_sh(rx, y0 + 13, &when, c, OUTLINE, false);
                fr.text_sh(rx, y0 + 22, &format!("LASTS {}", text::dur(bt)), DIM, OUTLINE, false);
            } else if let Some(hint) = self.context_hint() {
                fr.text_sh(rx, y0 + 4, hint.0, DIM, OUTLINE, false);
                fr.text_sh(rx, y0 + 13, hint.1, DIM, OUTLINE, false);
                fr.text_sh(rx, y0 + 22, hint.2, DIM, OUTLINE, false);
            }
        }
        // landing helper
        self.landing_panel(fr);
        // pause button
        fr.rect(w - 22, 27, 18, 13, rgb(40, 40, 80));
        fr.rect_outline(w - 22, 27, 18, 13, rgb(72, 72, 128));
        fr.text(w - 17, 30, "II", BRIGHT);
        // banners
        for (i, b) in self.banners.iter().enumerate() {
            let slide = if b.t < 10 { (10 - b.t as i32) * 30 } else { 0 };
            let y = 46 + i as i32 * 20;
            if b.t > 130 && (b.t / 3) % 2 == 0 {
                continue;
            }
            let tw = text_width(&b.text, true);
            let bx = w / 2 - tw / 2 - 8 + slide;
            fr.shade_rect(bx, y - 4, tw + 16, 16, 1, 3);
            fr.text_grad(w / 2 - tw / 2 + slide, y, &b.text, &crate::engine::ui::grad7(b.color), OUTLINE, true);
        }
        let _ = (SUNSET, LOGO_SHADOW, grad6);
    }

    fn context_hint(&self) -> Option<(&'static str, &'static str, &'static str)> {
        let f = &self.flight;
        match f.craft.status {
            Status::Landed { .. } if !f.launched => Some(("Z: FULL THROTTLE", "UP/DOWN: THROTTLE", "F1: ALL CONTROLS")),
            Status::Crashed => Some(("R: REWIND", "ESC: MENU", "")),
            _ if f.nodes.is_empty() && !f.surface_mode() => Some(("CLICK THE PATH", "TO PLAN A BURN", "WHEEL: ZOOM")),
            _ if !f.craft.vehicle.can_thrust() && f.craft.vehicle.stages.len() > 1 => Some(("OUT OF FUEL:", "SPACE TO STAGE", "")),
            _ => Some(("WHEEL: ZOOM", "DRAG: PAN  F: FOCUS", "F1: CONTROLS")),
        }
    }

    fn objectives(&self, fr: &mut Frame) {
        let m = self.mission;
        let sys = &self.flight.sys;
        let lines: Vec<(String, u32)> = m
            .goals
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let done = i < self.tracker.step;
                let cur = i == self.tracker.step;
                let mark = if done { "[*]" } else if cur { "[>]" } else { "[ ]" };
                (format!("{mark} {}", mission::goal_text(sys, g)), if done { GOOD } else if cur { BRIGHT } else { DIM })
            })
            .collect();
        let wmax = lines.iter().map(|(s, _)| text_width(s, false)).max().unwrap_or(0).max(text_width(m.title, false) + 30);
        let x = fr.w - wmax - 30;
        let y = 30;
        fr.shade_rect(x - 4, y - 2, wmax + 8, 12 + lines.len() as i32 * 10, 1, 2);
        fr.text_sh(x, y, &format!("{} {}", m.code, m.title), WARN, OUTLINE, false);
        for (i, (s, c)) in lines.iter().enumerate() {
            fr.text_sh(x, y + 11 + i as i32 * 10, s, *c, OUTLINE, false);
        }
    }

    fn landing_panel(&self, fr: &mut Frame) {
        let f = &self.flight;
        if !f.surface_mode() || !matches!(f.craft.status, Status::Flying) {
            return;
        }
        let up = f.up();
        let sv = f.surface_vel();
        let vs = sv.dot(up);
        let alt = f.altitude();
        if vs > -3.0 || alt > 6000.0 {
            return;
        }
        let dom = f.dominant();
        let b = &f.sys.bodies[dom];
        let r = (f.pos_vel().0 - f.st[dom].pos).len();
        let g = b.gm / (r * r);
        let v = &f.craft.vehicle;
        let acc = v.max_thrust() / v.mass();
        let x = 6;
        let y = fr.h - 33 - 34;
        fr.shade_rect(x - 3, y - 3, 132, 30, 1, 2);
        let impact = f.pred.impact().map(|(t, _, _)| t - f.t);
        if let Some(t) = impact {
            fr.text_sh(x, y, &format!("IMPACT IN {}", text::dur(t)), if t < 10.0 { WARN } else { BRIGHT }, OUTLINE, false);
        }
        if acc > g * 1.05 && !f.craft.chute {
            // stop distance for a full-throttle retro burn
            let a_net = acc - g;
            let stop = sv.len2() / (2.0 * a_net);
            let left = alt - stop;
            let tleft = left / (-vs).max(0.1);
            let (msg, c) = if left < 0.0 {
                ("BURN NOW!!".to_string(), BAD)
            } else if tleft < 3.0 {
                (format!("BURN IN {:.1}S", tleft), WARN)
            } else {
                (format!("BURN IN {}", text::dur(tleft)), GOOD)
            };
            if left > 0.0 || self.tick / 6 % 2 == 0 {
                fr.text_sh(x, y + 10, &msg, c, OUTLINE, false);
            }
            // a bar: how much of the stopping distance we still have
            let frac = (alt / stop.max(1.0) / 3.0).clamp(0.0, 1.0);
            fr.rect(x, y + 20, 120, 4, OUTLINE);
            fr.rect(x, y + 20, (120.0 * frac) as i32, 4, c);
        } else if v.has_chute() && !f.craft.chute {
            fr.text_sh(x, y + 10, "SPACE: OPEN CHUTE", WARN, OUTLINE, false);
        }
    }

    /// Attitude dial: where the nose points, and the prograde/retrograde,
    /// node and target markers, as seen on screen.
    fn dial(&self, fr: &mut Frame, cx: i32, cy: i32) {
        let f = &self.flight;
        let r = 26;
        fr.fill_ellipse(cx, cy, r + 2, r + 2, OUTLINE);
        fr.fill_ellipse(cx, cy, r, r, rgb(24, 32, 72));
        // sky half / ground half, relative to local up
        let up_a = self.cam.screen_angle(f.up().angle());
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y > r * r {
                    continue;
                }
                let a = (y as f64).atan2(x as f64);
                let mut d = (a - up_a).rem_euclid(TAU);
                if d > PI {
                    d -= TAU;
                }
                if d.abs() > FRAC_PI_2 {
                    fr.set(cx + x, cy + y, rgb(72, 48, 40));
                } else if (x + y) & 1 == 0 {
                    fr.set(cx + x, cy + y, rgb(32, 56, 112));
                }
            }
        }
        fr.circle_outline(cx, cy, r, rgb(88, 104, 168));
        let mark = |fr: &mut Frame, a: f64, rr: f64, c: u32, kind: u8| {
            let (x, y) = (cx + (a.cos() * rr) as i32, cy + (a.sin() * rr) as i32);
            match kind {
                0 => {
                    fr.circle_outline(x, y, 3, OUTLINE);
                    fr.circle_outline(x, y, 2, c);
                    fr.set(x, y, c);
                }
                1 => {
                    fr.circle_outline(x, y, 2, c);
                    fr.set(x - 2, y - 2, c);
                    fr.set(x + 2, y + 2, c);
                    fr.set(x + 2, y - 2, c);
                    fr.set(x - 2, y + 2, c);
                }
                _ => {
                    fr.fill_poly(&[(x, y - 3), (x + 3, y), (x, y + 3), (x - 3, y)], c);
                }
            }
        };
        let v = f.ref_vel();
        if v.len() > 1.0 {
            let pa = self.cam.screen_angle(v.angle());
            mark(fr, pa, 18.0, GOOD, 0);
            mark(fr, pa + PI, 18.0, rgb(232, 120, 64), 1);
        }
        if let Some(d) = f.node_dir() {
            mark(fr, self.cam.screen_angle(d.angle()), 18.0, PLANC, 2);
        }
        if let Some(t) = f.target {
            let d = f.st[t].pos - f.pos_vel().0;
            mark(fr, self.cam.screen_angle(d.angle()), 22.0, rgb(248, 200, 120), 2);
        }
        // the rocket needle
        let ha = self.cam.screen_angle(f.craft.heading);
        let (dx, dy) = (ha.cos(), ha.sin());
        let tip = (cx + (dx * 14.0) as i32, cy + (dy * 14.0) as i32);
        let back = (cx - (dx * 8.0) as i32, cy - (dy * 8.0) as i32);
        fr.thick_line(back.0, back.1, tip.0, tip.1, 3, OUTLINE);
        fr.line(back.0, back.1, tip.0, tip.1, BRIGHT);
        fr.fill_ellipse(tip.0, tip.1, 1, 1, WARN);
        let label = if f.auto == Auto::Node { "AUTO" } else { f.sas.label() };
        let lw = text_width(label, false);
        fr.text_sh(cx - lw / 2, cy + r - 6, label, CYAN, OUTLINE, false);
    }

    fn draw_pause(&self, fr: &mut Frame) {
        fr.shade_rect(0, 0, fr.w, fr.h, 1, 2);
        let (bx, by, bw, bh) = self.pause_box();
        fr.rect(bx, by, bw, bh, PANEL);
        fr.rect_outline(bx, by, bw, bh, rgb(248, 176, 32));
        fr.text_grad_center(fr.w / 2, by + 7, "PAUSED", &crate::engine::ui::grad7(rgb(248, 200, 64)), OUTLINE, true);
        for (i, s) in PAUSE_ITEMS.iter().enumerate() {
            let y = by + 26 + i as i32 * 14;
            let sel = i == self.pause_sel;
            if sel {
                fr.rect(bx + 6, y - 2, bw - 12, 11, rgb(48, 64, 136));
                fr.text(bx + 10, y, ">", WARN);
            }
            fr.text_center(fr.w / 2, y, s, if sel { BRIGHT } else { DIM }, OUTLINE, false);
        }
    }
}

pub const PLANC: u32 = traj::PLAN;

pub const HELP: &[(&str, &str)] = &[
    ("UP / DOWN  (W/S)", "THROTTLE"),
    ("LEFT/RIGHT (A/D)", "TURN"),
    ("Z / X", "FULL THROTTLE / CUT"),
    ("SPACE", "STAGE / PARACHUTE"),
    ("1-6", "HOLD PRO RETRO RAD+ RAD- NODE"),
    (", / .", "TIME WARP SLOWER / FASTER"),
    ("G", "WARP TO NEXT EVENT"),
    ("CLICK PATH", "ADD BURN NODE"),
    ("DRAG HANDLES", "PLAN THE BURN (SHIFT: FINE)"),
    ("N", "AUTO BURN THE NODE"),
    ("DEL / R-CLICK", "DELETE NODE"),
    ("T / CLICK BODY", "SET TARGET"),
    ("WHEEL / DRAG", "ZOOM / PAN"),
    ("F / C / M", "FOCUS / CENTER / MAP"),
    ("R", "REWIND A FEW SECONDS"),
    ("ESC", "PAUSE"),
];

pub fn draw_help(fr: &mut Frame) {
    fr.shade_rect(0, 0, fr.w, fr.h, 1, 3);
    let bw = 330;
    let bh = 34 + HELP.len() as i32 * 11;
    let (bx, by) = ((fr.w - bw) / 2, ((fr.h - bh) / 2).max(2));
    fr.rect(bx, by, bw, bh, PANEL);
    fr.rect_outline(bx, by, bw, bh, rgb(248, 176, 32));
    fr.text_grad_center(fr.w / 2, by + 7, "CONTROLS", &crate::engine::ui::grad7(rgb(88, 232, 248)), OUTLINE, true);
    for (i, (k, v)) in HELP.iter().enumerate() {
        let y = by + 26 + i as i32 * 11;
        fr.text_sh(bx + 10, y, k, WARN, OUTLINE, false);
        fr.text_sh(bx + 120, y, v, BRIGHT, OUTLINE, false);
    }
    let _ = (mix, shade);
}
