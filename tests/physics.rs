use centrifugal::engine::v2::V2;
use centrifugal::game::flight::{Flight, Log, Status};
use centrifugal::sim::physics::TICK;
use centrifugal::sim::system::SiteKind;
use centrifugal::sim::vehicle::{self, Vehicle, fit_fuel};
use centrifugal::sim::worldgen;
use std::rc::Rc;

fn hopper() -> Vehicle {
    let mut s = vehicle::booster(20.0, 120_000.0);
    s.legs = true;
    fit_fuel(&mut s, 0.0, 600.0);
    Vehicle { stages: vec![s] }
}

fn orbiter() -> Vehicle {
    let mut up = vehicle::upper(8.0, 40_000.0);
    fit_fuel(&mut up, 1000.0, 400.0);
    let sat = vehicle::satellite("SAT");
    Vehicle { stages: vec![up, sat] }
}

#[test]
fn kepler_velocity_is_derivative() {
    let sys = worldgen::generate(1);
    for b in sys.bodies.iter().skip(1) {
        let (p0, v) = b.orbit.state(1000.0);
        let (p1, _) = b.orbit.state(1000.5);
        let (pm, _) = b.orbit.state(999.5);
        let num = (p1 - pm) / 1.0;
        assert!((num - v).len() < v.len() * 1e-4 + 1e-6, "{}: {:?} vs {:?}", b.name, num, v);
        let _ = p0;
    }
}

fn circular(sys: &centrifugal::sim::system::System, body: usize, alt: f64) -> (V2, V2) {
    let st = sys.states_at(0.0);
    let b = &sys.bodies[body];
    let r = b.radius + alt;
    let v = (b.gm / r).sqrt();
    (st[body].pos + V2 { x: r, y: 0.0 }, st[body].vel + V2 { x: 0.0, y: -v })
}

#[test]
fn low_orbit_stays_put_for_an_orbit() {
    let sys = Rc::new(worldgen::generate(1));
    let home = sys.home;
    let (p, v) = circular(&sys, home, 9000.0);
    let mut f = Flight::in_flight(sys.clone(), orbiter(), p, v, 0.0, 0.0);
    f.warp = 4; // 50x
    let r0 = sys.bodies[home].radius + 9000.0;
    let period = std::f64::consts::TAU * (r0.powi(3) / sys.bodies[home].gm).sqrt();
    let mut lo = f64::MAX;
    let mut hi = 0.0f64;
    while f.t < period {
        f.update();
        let r = (f.pos_vel().0 - f.st[home].pos).len();
        lo = lo.min(r);
        hi = hi.max(r);
    }
    assert!(matches!(f.craft.status, Status::Flying));
    // the moon's tide moves it a little
    assert!(hi - lo < 800.0, "orbit wandered {} m (lo {lo} hi {hi})", hi - lo);
}

#[test]
fn prediction_matches_flight_while_coasting() {
    let sys = Rc::new(worldgen::generate(3));
    let home = sys.home;
    let (p, v) = circular(&sys, home, 12000.0);
    // slightly eccentric
    let mut f = Flight::in_flight(sys.clone(), orbiter(), p, v * 1.08, 0.0, 0.0);
    f.update();
    let pred = f.pred.clone();
    f.warp = 5;
    for _ in 0..600 {
        f.update();
    }
    let c = &f.craft;
    // compare the flight's last step with the prediction's point at the same time
    let pt = pred.points.iter().find(|q| (q.t - c.ship.t).abs() < 1e-6).expect("same step times");
    assert!((pt.pos - c.ship.pos).len() < 1e-3, "drift {}", (pt.pos - c.ship.pos).len());
}

#[test]
fn hop_lifts_off_and_crashes_without_throttle() {
    let sys = Rc::new(worldgen::generate(1));
    let home = sys.home;
    let pad = sys.bodies[home].site(SiteKind::LaunchPad).unwrap().angle;
    let mut f = Flight::on_pad(sys.clone(), hopper(), home, pad);
    for _ in 0..60 {
        f.update();
    }
    assert!(matches!(f.craft.status, Status::Landed { .. }), "stands on the pad");
    let a0 = f.altitude();
    assert!(a0.abs() < 1.0, "on the ground: {a0}");
    f.set_throttle(1.0);
    let mut lifted = false;
    for _ in 0..(8.0 / TICK) as usize {
        f.update();
        lifted |= f.log.contains(&Log::Liftoff);
    }
    assert!(lifted);
    assert!(f.altitude() > 100.0, "climbed: {}", f.altitude());
    f.set_throttle(0.0);
    let mut crashed = false;
    for _ in 0..(60.0 / TICK) as usize {
        f.update();
        crashed |= f.log.iter().any(|l| matches!(l, Log::Crashed { .. }));
        if crashed {
            break;
        }
    }
    assert!(crashed, "falls and crashes; status {:?} alt {}", f.craft.status, f.altitude());
}

#[test]
fn hop_lands_with_a_simple_controller() {
    let sys = Rc::new(worldgen::generate(1));
    let home = sys.home;
    let pad = sys.bodies[home].site(SiteKind::LaunchPad).unwrap().angle;
    let mut f = Flight::on_pad(sys.clone(), hopper(), home, pad);
    f.set_throttle(1.0);
    let mut landed = false;
    for i in 0..(120.0 / TICK) as usize {
        let alt = f.altitude();
        let vs = f.surface_vel().dot(f.up());
        // climb for 4 s, then hold a descent speed that shrinks with altitude
        let thr = if i < 240 { 1.0 } else {
            let want = -(alt * 0.15).clamp(1.5, 40.0);
            let g = 9.8;
            let acc = f.craft.vehicle.max_thrust() / f.craft.vehicle.mass();
            ((g + (want - vs) * 1.5) / acc).clamp(0.0, 1.0)
        };
        f.set_throttle(thr);
        f.update();
        if f.log.iter().any(|l| matches!(l, Log::Landed { .. })) {
            landed = true;
            break;
        }
        assert!(!f.is_over(), "crashed at alt {alt} vs {vs}");
    }
    assert!(landed);
}

