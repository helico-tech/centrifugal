//! Headless tools: scripted screenshots of every screen (`--shots`) and
//! bot flights that check each mission can be flown with its rocket
//! (`--bots`).

use crate::engine::gfx::Frame;
use crate::engine::input::{Input, Key};
use crate::engine::png::save_png;
use crate::game::app::App;
use crate::game::bot::{Ascent, Hop};

use crate::game::flight::{Auto, Status};
use crate::game::mission::{self, MISSIONS, Outcome};
use crate::game::play::Play;
use crate::render::{body, space};
use crate::sim::predict::Node;
use crate::sim::worldgen;
use std::path::Path;
use std::rc::Rc;

const W: i32 = 640;
const H: i32 = 360;

fn save(f: &Frame, dir: &str, name: &str) {
    save_png(f, &Path::new(dir).join(format!("{name}.png")), 2);
    println!("{dir}/{name}.png");
}

fn press(k: Key) -> Input {
    let mut i = Input::default();
    i.set(k, true, true);
    i
}

fn new_play(m: usize) -> Play {
    let sys = Rc::new(worldgen::generate(mission::CAMPAIGN_SEED));
    let gfx = Rc::new(body::build(&sys));
    let sky = Rc::new(space::build_sky(mission::CAMPAIGN_SEED));
    Play::new(&MISSIONS[m], sys, gfx, sky, 3)
}

fn run(p: &mut Play, ticks: usize, mut each: impl FnMut(&mut Play, usize)) {
    let idle = Input::default();
    for i in 0..ticks {
        each(p, i);
        p.update(&idle);
    }
}

pub fn shots(dir: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let mut f = Frame::new(W, H);
    // menus
    let mut app = App::new(1);
    let idle = Input::default();
    for _ in 0..40 {
        app.update(&idle);
    }
    app.draw(&mut f);
    save(&f, dir, "title");
    app.update(&press(Key::Enter));
    app.update(&idle);
    app.draw(&mut f);
    save(&f, dir, "board");
    app.update(&press(Key::Down));
    app.update(&press(Key::Down));
    app.update(&press(Key::Enter));
    app.update(&idle);
    app.draw(&mut f);
    save(&f, dir, "brief");

    // on the pad and lifting off
    let mut p = new_play(0);
    run(&mut p, 30, |_, _| {});
    p.draw(&mut f);
    save(&f, dir, "pad");
    run(&mut p, 150, |p, _| p.flight.set_throttle(1.0));
    p.draw(&mut f);
    save(&f, dir, "liftoff");

    // gravity turn
    let mut p = new_play(2);
    let mut bot = Ascent::new(10_000.0);
    run(&mut p, 60 * 35, |p, _| bot.step(&mut p.flight));
    p.mpp_target = 12.0;
    run(&mut p, 30, |p, _| bot.step(&mut p.flight));
    p.draw(&mut f);
    save(&f, dir, "ascent");
    p.mpp_target = 120.0;
    run(&mut p, 40, |p, _| bot.step(&mut p.flight));
    p.draw(&mut f);
    save(&f, dir, "ascent-map");

    // in orbit, a node planned toward the moon
    let mut p = new_play(6);
    run(&mut p, 10, |_, _| {});
    let moon = mission::body_of(&p.flight.sys, mission::Role::Moon);
    p.flight.target = Some(moon);
    let t = p.flight.t + 200.0;
    p.flight.nodes.push(Node { t, pro: 150.0, rad: 0.0 });
    p.selected = Some(0);
    p.flight.pred_dirty = true;
    p.mpp_target = 500.0;
    run(&mut p, 40, |_, _| {});
    p.draw(&mut f);
    save(&f, dir, "node");
    p.mpp_target = 1500.0;
    run(&mut p, 60, |_, _| {});
    p.draw(&mut f);
    save(&f, dir, "node-far");
    p.mpp_target = 30.0;
    run(&mut p, 40, |_, _| {});
    p.draw(&mut f);
    save(&f, dir, "orbit-close");
    p.mpp_target = 30_000.0;
    run(&mut p, 60, |_, _| {});
    p.draw(&mut f);
    save(&f, dir, "system");

    // landing on the moon
    let mut p = new_play(8);
    run(&mut p, 30, |_, _| {});
    p.draw(&mut f);
    save(&f, dir, "moon-site");

    // crash
    let mut p = new_play(0);
    run(&mut p, 60 * 5, |p, i| p.flight.set_throttle(if i < 200 { 1.0 } else { 0.0 }));
    run(&mut p, 60 * 30, |p, _| if matches!(p.flight.craft.status, Status::Crashed) { p.flight.warp = 0 });
    let mut n = 0;
    run(&mut p, 1, |_, _| n += 1);
    p.draw(&mut f);
    save(&f, dir, "crash");

    // help and results
    let mut p = new_play(0);
    p.help = true;
    run(&mut p, 2, |_, _| {});
    p.draw(&mut f);
    save(&f, dir, "help");
}

/// Fly each mission that has a bot and report the outcome and fuel left.
pub fn bots() {
    // M-01: hop
    let mut p = new_play(0);
    let mut hop = Hop::new(1200.0);
    let mut tick = 0;
    run(&mut p, 60 * 180, |p, _| {
        if p.tracker.outcome == Outcome::Flying {
            hop.step(&mut p.flight, tick);
            tick += 1;
        }
    });
    report(&p);

    // M-02: straight up, drop the booster, chute down
    let mut p = new_play(1);
    run(&mut p, 60 * 400, |p, _| {
        let f = &mut p.flight;
        if p.tracker.outcome != Outcome::Flying {
            return;
        }
        f.sas = crate::game::flight::Sas::Off;
        if f.craft.vehicle.stages.len() > 1 {
            f.set_throttle(1.0);
            if !f.craft.vehicle.can_thrust() && f.launched {
                f.stage();
            }
        } else if !f.craft.chute && f.surface_vel().dot(f.up()) < 0.0 && f.altitude() < 2500.0 {
            f.stage();
        }
        if f.warp < 2 && f.craft.chute {
            f.warp = 2;
        }
    });
    println!("   max alt {:.0}", p.flight.max_alt);
    report(&p);

    // M-03: orbit
    for m in [2, 3] {
        let mut p = new_play(m);
        let mut bot = Ascent::new(if m == 3 { 9_500.0 } else { 9_500.0 });
        run(&mut p, 60 * 400, |p, _| {
            if p.tracker.outcome == Outcome::Flying {
                if bot.done() && m == 3 {
                    // raise to 17.5 km with a Hohmann pair, then deploy
                    comsat_raise(p);
                } else {
                    bot.step(&mut p.flight);
                }
                if p.flight.auto == Auto::Node {
                    p.flight.warp = p.flight.warp.max(0);
                }
            }
        });
        report(&p);
    }
    transfer_bot(6, mission::Role::Moon, 4000.0);
    transfer_bot(10, mission::Role::Giant, 300_000.0);

    // M-08: moon transfer, drop the transfer stage, brake and land
    let mut p = new_play(7);
    let target = mission::body_of(&p.flight.sys, mission::Role::Moon);
    let mut tr = crate::game::bot::Transfer::new(target, 3000.0);
    let mut de = crate::game::bot::Descent::new();
    run(&mut p, 60 * 900, |p, _| {
        if p.tracker.outcome != Outcome::Flying {
            return;
        }
        if !tr.done() {
            tr.step(&mut p.flight);
        } else {
            if p.flight.craft.vehicle.stages.len() > 1 {
                p.flight.stage();
            }
            p.flight.warp = 0;
            de.step(&mut p.flight);
        }
    });
    println!("   land dist {:?} speed {:?}", p.tracker.land_dist, p.tracker.land_speed);
    report(&p);
}

fn transfer_bot(m: usize, role: mission::Role, want_pe: f64) {
    let mut p = new_play(m);
    let target = mission::body_of(&p.flight.sys, role);
    let mut bot = crate::game::bot::Transfer::new(target, want_pe);
    let mut last = String::new();
    run(&mut p, 60 * 600, |p, _| {
        if p.tracker.outcome == Outcome::Flying && !bot.done() {
            bot.step(&mut p.flight);
        }
        for l in &p.flight.log {
            last = format!("{:?} at t {:.0} dom {}", l, p.flight.t, p.flight.dominant());
            println!("   log {last}");
        }
    });
    report(&p);
}

fn comsat_raise(p: &mut Play) {
    let f = &mut p.flight;
    if !f.nodes.is_empty() || f.auto == Auto::Node {
        return;
    }
    let dom = f.dominant();
    let b = f.sys.bodies[dom].clone();
    let Some((pe, ap)) = mission::apsides(f, dom) else { return };
    let _ = pe;
    let target = b.radius + 17_500.0;
    if ap + b.radius < target - 500.0 {
        // raise apoapsis at periapsis
        if let Some((t, _)) = f.pred.apsis(dom, false) {
            let st = f.sys.states_at(t);
            let Some(pt) = f.pred.state_at(t) else { return };
            let r1 = (pt.pos - st[dom].pos).len();
            let v = (pt.vel - st[dom].vel).len();
            // vis-viva: speed here for an orbit reaching out to the target
            let want = (b.gm * (2.0 / r1 - 2.0 / (r1 + target))).sqrt();
            f.nodes.push(Node { t, pro: want - v, rad: 0.0 });
            f.auto = Auto::Node;
            f.pred_dirty = true;
        }
    } else if pe + b.radius < target - 1500.0 {
        if let Some((t, _)) = f.pred.apsis(dom, true) {
            let st = f.sys.states_at(t);
            if let Some(pt) = f.pred.state_at(t) {
                let r = (pt.pos - st[dom].pos).len();
                let v = (pt.vel - st[dom].vel).len();
                f.nodes.push(Node { t, pro: (b.gm / r).sqrt() - v, rad: 0.0 });
                f.auto = Auto::Node;
                f.pred_dirty = true;
            }
        }
    } else {
        f.stage();
    }
}

fn report(p: &Play) {
    let dom = p.flight.dominant();
    if let Some((pe, ap)) = mission::apsides(&p.flight, dom) {
        println!("   orbit pe {:.0} ap {:.0} stages {}", pe, ap, p.flight.craft.vehicle.stages.len());
    }
    println!(
        "{} {:<20} {:?} step {} fuel {:.0}% stars {:?} t {:.0}s dv left {:.0}",
        p.mission.code,
        p.mission.title,
        p.tracker.outcome,
        p.tracker.step,
        p.flight.craft.vehicle.fuel_frac() * 100.0,
        p.tracker.stars(p.mission),
        p.flight.t,
        p.flight.craft.vehicle.dv()
    );
}
