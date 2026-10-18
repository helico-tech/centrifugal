//! The predicted path and its planning markers.
//!
//! Each part of the path is drawn relative to the body that dominates it.
//! Parts are chained so the line stays connected: a moon encounter is
//! drawn around a "ghost" of the moon where it will be at that time, and
//! an escape is drawn as the sun-centred path leaving from here.

use super::camera::Camera;
use super::text;
use crate::engine::gfx::{Frame, mix, rgb, shade};
use crate::engine::ui::OUTLINE;
use crate::engine::v2::V2;
use crate::sim::predict::{Event, Node, Prediction};
use crate::sim::system::System;

pub const PATH: u32 = rgb(88, 232, 248);
pub const PLAN: u32 = rgb(248, 168, 48);
pub const SEG: [u32; 4] = [rgb(248, 232, 96), rgb(232, 120, 232), rgb(120, 240, 120), rgb(248, 136, 136)];
pub const DANGER: u32 = rgb(248, 64, 64);

/// Where the path went on screen, for clicking on it.
#[derive(Default, Clone)]
pub struct Drawn {
    /// Screen x, y and time of each path point.
    pub pts: Vec<(f32, f32, f64)>,
    /// Screen position of each node marker.
    pub nodes: Vec<(f32, f32)>,
    /// Handles of the selected node: (x, y, prograde dir, radial dir) on screen.
    pub handles: Option<[(f32, f32); 4]>,
    /// World-space offset applied to each point (p_screen_world = p + off).
    pub offs: Vec<V2>,
}

impl Drawn {
    /// Closest path point to a screen position, within `max` pixels.
    pub fn pick(&self, x: f32, y: f32, max: f32) -> Option<(usize, f64)> {
        let mut best: Option<(usize, f32)> = None;
        for (i, &(px, py, _)) in self.pts.iter().enumerate() {
            let d = (px - x).hypot(py - y);
            if d < max && best.is_none_or(|(_, b)| d < b) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| (i, self.pts[i].2))
    }
}

pub fn label(f: &mut Frame, x: i32, y: i32, s: &str, c: u32) {
    let w = crate::engine::gfx::text_width(s, false);
    let x = x.clamp(2, f.w - w - 2);
    let y = y.clamp(2, f.h - 9);
    f.text_sh(x, y, s, c, OUTLINE, false);
}

fn marker_tri(f: &mut Frame, x: i32, y: i32, up: bool, c: u32) {
    let d = if up { -1 } else { 1 };
    let pts = [(x - 3, y), (x + 3, y), (x, y + 4 * d)];
    let o: Vec<_> = pts.iter().map(|&(a, b)| (a, b - d)).collect();
    f.fill_poly(&[(o[0].0 - 1, o[0].1), (o[1].0 + 1, o[1].1), (o[2].0, o[2].1 + d)], OUTLINE);
    f.fill_poly(&pts, c);
}

pub fn x_mark(f: &mut Frame, x: i32, y: i32, c: u32) {
    for k in -3..=3 {
        for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
            f.set(x + k + dx, y + k + dy, OUTLINE);
            f.set(x + k + dx, y - k + dy, OUTLINE);
        }
    }
    for k in -3..=3 {
        f.set(x + k, y + k, c);
        f.set(x + k, y - k, c);
    }
}

pub fn dashed_circle(f: &mut Frame, x: f64, y: f64, r: f64, c: u32) {
    let n = ((r * 0.8) as usize).clamp(16, 400);
    for k in 0..n {
        if k % 2 == 1 {
            continue;
        }
        let a0 = k as f64 / n as f64 * std::f64::consts::TAU;
        let a1 = (k + 1) as f64 / n as f64 * std::f64::consts::TAU;
        f.line_f(x + a0.cos() * r, y + a0.sin() * r, x + a1.cos() * r, y + a1.sin() * r, c);
    }
}

pub struct Input<'a> {
    pub sys: &'a System,
    pub pred: &'a Prediction,
    pub now: f64,
    pub nodes: &'a [Node],
    pub selected: Option<usize>,
    pub target: Option<usize>,
    pub tick: u32,
    /// Low over this body: draw its part of the path turning with the
    /// ground, so the impact mark sits where you will really come down.
    pub rotating: Option<usize>,
}

pub fn draw(f: &mut Frame, cam: &Camera, inp: &Input) -> Drawn {
    let mut out = Drawn::default();
    let pts = &inp.pred.points;
    if pts.len() < 2 {
        return out;
    }
    let sys = inp.sys;
    // anchor chain
    let mut dom = pts[0].dom;
    let mut anchor = sys.pos_of(dom, inp.now);
    let mut seg = 0usize;
    let mut segs = vec![(0usize, dom, anchor)];
    out.offs.reserve(pts.len());
    for (i, p) in pts.iter().enumerate() {
        if p.dom != dom {
            let a_new = sys.pos_of(p.dom, p.t) - sys.pos_of(dom, p.t) + anchor;
            anchor = a_new;
            dom = p.dom;
            seg += 1;
            segs.push((i, dom, anchor));
        }
        let dpos = sys.pos_of(dom, p.t);
        let off = anchor - dpos;
        out.offs.push(off);
        let mut world = p.pos + off;
        if inp.rotating == Some(dom) && seg == 0 {
            let spin = sys.bodies[dom].spin;
            world = anchor + (p.pos - dpos).rot(-spin * (p.t - inp.now));
        }
        let (x, y) = cam.to_screen(world);
        out.pts.push((x as f32, y as f32, p.t));
    }
    let first_node_t = inp.nodes.first().map(|n| n.t).unwrap_or(f64::INFINITY);
    let fade = inp.pred.fade_from.unwrap_or(usize::MAX);
    let mut seg_i = 0;
    for i in 1..pts.len() {
        while seg_i + 1 < segs.len() && segs[seg_i + 1].0 <= i {
            seg_i += 1;
        }
        let p = &pts[i];
        let mut c = if seg_i == 0 { PATH } else { SEG[(seg_i - 1) % SEG.len()] };
        if p.t > first_node_t {
            c = if seg_i == 0 { PLAN } else { mix(PLAN, c, 0.5) };
        }
        if p.burn {
            c = rgb(255, 255, 255);
        }
        if i > fade {
            if (i / 3) % 2 == 0 {
                continue;
            }
            c = shade(c, 5, 8);
        }
        let (x0, y0, _) = out.pts[i - 1];
        let (x1, y1, _) = out.pts[i];
        // a dark edge under the line keeps it readable over planets
        f.line_f(x0 as f64 + 1.0, y0 as f64 + 1.0, x1 as f64 + 1.0, y1 as f64 + 1.0, OUTLINE);
        f.line_f(x0 as f64, y0 as f64, x1 as f64, y1 as f64, c);
    }

    // ghosts of the bodies we will meet
    for &(i, d, a) in segs.iter().skip(1) {
        let parent_side = pts[i - 1].dom;
        if sys.is_ancestor(d, parent_side) || d == 0 {
            continue;
        }
        let (x, y) = cam.to_screen(a);
        let r = (sys.bodies[d].radius / cam.mpp).max(4.0);
        dashed_circle(f, x, y, r, rgb(232, 232, 248));
        let dt = pts[i].t - inp.now;
        label(f, x as i32 + r as i32 + 3, y as i32 - 8, &sys.bodies[d].name, rgb(232, 232, 248));
        label(f, x as i32 + r as i32 + 3, y as i32 + 1, &format!("IN {}", text::dur(dt)), rgb(176, 176, 200));
    }

    let at = |t: f64| -> Option<(f32, f32, usize)> {
        let i = pts.partition_point(|p| p.t < t).min(pts.len() - 1);
        out.pts.get(i).map(|&(x, y, _)| (x, y, i))
    };
    let mut seen_ap = std::collections::HashSet::new();
    let mut seen_pe = std::collections::HashSet::new();
    for e in &inp.pred.events {
        match *e {
            Event::Apoapsis { t, body, alt } => {
                if !seen_ap.insert(body) {
                    continue;
                }
                if let Some((x, y, _)) = at(t) {
                    marker_tri(f, x as i32, y as i32 - 2, true, rgb(120, 200, 248));
                    label(f, x as i32 - 14, y as i32 - 14, &format!("AP {}", text::dist(alt)), rgb(120, 200, 248));
                }
            }
            Event::Periapsis { t, body, alt } => {
                if !seen_pe.insert(body) {
                    continue;
                }
                if let Some((x, y, _)) = at(t) {
                    let c = if alt < 0.0 { DANGER } else { rgb(120, 248, 168) };
                    marker_tri(f, x as i32, y as i32 + 2, false, c);
                    label(f, x as i32 - 14, y as i32 + 8, &format!("PE {}", text::dist(alt)), c);
                }
            }
            Event::Impact { t, body, .. } => {
                if let Some((x, y, _)) = at(t) {
                    let blink = inp.tick / 10 % 2 == 0;
                    x_mark(f, x as i32, y as i32, if blink { DANGER } else { rgb(255, 232, 120) });
                    let what = if sys.bodies[body].look.sea > 0.0 { "IMPACT" } else { "IMPACT" };
                    label(f, x as i32 + 6, y as i32 - 3, &format!("{what} {}", text::dur(t - inp.now)), DANGER);
                }
            }
            Event::Closest { t, body, dist } => {
                if let Some((x, y, i)) = at(t) {
                    let tp = sys.pos_of(body, t) + out.offs[i];
                    let (tx, ty) = cam.to_screen(tp);
                    f.line_f(x as f64, y as f64, tx, ty, rgb(200, 200, 216));
                    let r = (sys.bodies[body].radius / cam.mpp).max(3.0);
                    dashed_circle(f, tx, ty, r, rgb(248, 200, 120));
                    f.fill_ellipse(x as i32, y as i32, 2, 2, rgb(248, 200, 120));
                    label(f, x as i32 + 5, y as i32 + 4, &format!("CLOSEST {}", text::dist(dist - sys.bodies[body].radius)), rgb(248, 200, 120));
                }
            }
            Event::Soi { .. } | Event::AtmoEnter { .. } | Event::Node { .. } => {}
        }
    }

    // maneuver nodes
    for (k, n) in inp.nodes.iter().enumerate() {
        let Some((x, y, i)) = at(n.t) else {
            out.nodes.push((-99.0, -99.0));
            continue;
        };
        let (xi, yi) = (x as i32, y as i32);
        let sel = inp.selected == Some(k);
        let c = if sel { rgb(255, 255, 255) } else { PLAN };
        f.fill_poly(&[(xi, yi - 5), (xi + 5, yi), (xi, yi + 5), (xi - 5, yi)], OUTLINE);
        f.fill_poly(&[(xi, yi - 3), (xi + 3, yi), (xi, yi + 3), (xi - 3, yi)], c);
        out.nodes.push((x, y));
        let dt = n.t - inp.now;
        label(f, xi + 7, yi - 3, &format!("{} M/S IN {}", n.dv().round(), text::dur(dt)), c);
        if sel {
            let p = &pts[i];
            let st = sys.states_at(p.t);
            let (pro, rad) = crate::sim::predict::frame(p.pos, p.vel, &st[p.dom]);
            let sp = cam.dir(pro);
            let sr = cam.dir(rad);
            let norm = |d: (f64, f64)| {
                let l = d.0.hypot(d.1).max(1e-9);
                (d.0 / l, d.1 / l)
            };
            let (px, py) = norm(sp);
            let (rx, ry) = norm(sr);
            let k = 26.0;
            let hs = [
                (x + (px * k) as f32, y + (py * k) as f32),
                (x - (px * k) as f32, y - (py * k) as f32),
                (x + (rx * k) as f32, y + (ry * k) as f32),
                (x - (rx * k) as f32, y - (ry * k) as f32),
            ];
            let cols = [rgb(120, 248, 120), rgb(96, 176, 96), rgb(96, 200, 248), rgb(72, 136, 200)];
            for (j, &(hx, hy)) in hs.iter().enumerate() {
                f.line(xi, yi, hx as i32, hy as i32, shade(cols[j], 3, 4));
                f.fill_ellipse(hx as i32, hy as i32, 4, 4, OUTLINE);
                f.fill_ellipse(hx as i32, hy as i32, 3, 3, cols[j]);
                let sym = ["+", "-", "+", "-"][j];
                f.text(hx as i32 - 2, hy as i32 - 3, sym, OUTLINE);
            }
            label(f, hs[0].0 as i32 + 6, hs[0].1 as i32 - 3, &format!("PRO {:+.0}", n.pro), cols[0]);
            label(f, hs[2].0 as i32 + 6, hs[2].1 as i32 - 3, &format!("RAD {:+.0}", n.rad), cols[2]);
            out.handles = Some(hs);
        }
    }
    let _ = inp.target;
    out
}
