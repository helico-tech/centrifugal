//! The screens: title, mission board, briefing card, flight and results.

use crate::engine::gfx::{Frame, mix, rgb, shade, text_width};
use crate::engine::input::{Input, Key};
use crate::engine::ui::{LOGO_SHADOW, OUTLINE, SUNSET, grad7};
use crate::engine::v2::V2;
use crate::game::mission::{self, MISSIONS, Mission, Outcome};
use crate::game::play::{self, BRIGHT, CYAN, DIM, GOOD, PANEL, Play, Sfx, WARN};
use crate::render::body::{self, BodyGfx};
use crate::render::camera::Camera;
use crate::render::space::{self, Sky};
use crate::sim::system::System;
use crate::sim::worldgen;
use std::f64::consts::FRAC_PI_2;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Music {
    Title,
    Flight,
    Win,
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Title,
    Board,
    Brief,
    Flight,
}

pub struct App {
    screen: Screen,
    pub sys: Rc<System>,
    gfx: Rc<Vec<BodyGfx>>,
    sky: Rc<Sky>,
    pub seed: u64,
    pub play: Option<Play>,
    sel: usize,
    pub stars: Vec<u32>,
    tick: u32,
    idle: u32,
    pub sfx: Vec<Sfx>,
    result_sel: usize,
    pub quit: bool,
}

const TICKER: &str = "WELCOME TO THE HELICO SPACE PROGRAM!   ROCKETS ARE JUST TALL CANS OF CONTROLLED PANIC.   \
    REMEMBER: PROGRADE IS FORWARD, RETROGRADE IS SORRY.   NO COWS WERE LAUNCHED IN THE MAKING OF THIS GAME (YET).   \
    ORBITING IS FALLING AND MISSING.   THE MOON IS MADE OF REGOLITH, NOT CHEESE. WE CHECKED.   \
    TO GO FASTER, BURN BACKWARDS. TO GO SLOWER, ALSO DEPENDS.   ";

impl App {
    pub fn new(seed: u64) -> Self {
        let mut a = App {
            screen: Screen::Title,
            sys: Rc::new(worldgen::generate(mission::CAMPAIGN_SEED)),
            gfx: Rc::new(vec![]),
            sky: Rc::new(space::build_sky(1)),
            seed: mission::CAMPAIGN_SEED,
            play: None,
            sel: 0,
            stars: vec![0; MISSIONS.len()],
            tick: (seed % 1000) as u32,
            idle: 0,
            sfx: vec![],
            result_sel: 0,
            quit: false,
        };
        a.load_system(mission::CAMPAIGN_SEED);
        a
    }

    fn load_system(&mut self, seed: u64) {
        self.seed = seed;
        self.sys = Rc::new(worldgen::generate(seed));
        self.gfx = Rc::new(body::build(&self.sys));
        self.sky = Rc::new(space::build_sky(seed));
    }

    pub fn music(&self) -> Option<Music> {
        match self.screen {
            Screen::Flight => match &self.play {
                Some(p) if p.tracker.outcome == Outcome::Success && p.end_timer > 30 => Some(Music::Win),
                Some(p) if matches!(p.tracker.outcome, Outcome::Failed(_)) => None,
                _ => Some(Music::Flight),
            },
            _ => Some(Music::Title),
        }
    }

    pub fn engine_volume(&self) -> f32 {
        match (&self.screen, &self.play) {
            (Screen::Flight, Some(p)) => p.engine_volume(),
            _ => 0.0,
        }
    }

    fn start_flight(&mut self) {
        let m: &'static Mission = &MISSIONS[self.sel];
        self.play = Some(Play::new(m, self.sys.clone(), self.gfx.clone(), self.sky.clone(), self.tick as u64));
        self.screen = Screen::Flight;
        self.result_sel = 0;
    }

    pub fn update(&mut self, inp: &Input) {
        self.tick += 1;
        match self.screen {
            Screen::Title => {
                if inp.confirm() || inp.mpressed[0] {
                    self.screen = Screen::Board;
                    self.sfx.push(Sfx::Confirm);
                }
                if inp.pressed(Key::Esc) {
                    self.quit = true;
                }
            }
            Screen::Board => self.update_board(inp),
            Screen::Brief => {
                if inp.confirm() || inp.mpressed[0] {
                    self.sfx.push(Sfx::Confirm);
                    self.start_flight();
                } else if inp.pressed(Key::Esc) || inp.mpressed[1] {
                    self.screen = Screen::Board;
                    self.sfx.push(Sfx::Blip);
                }
            }
            Screen::Flight => self.update_flight(inp),
        }
        if inp.any_pressed() {
            self.idle = 0;
        } else {
            self.idle += 1;
        }
    }

    fn update_board(&mut self, inp: &Input) {
        let n = MISSIONS.len();
        if inp.pressed(Key::Up) || inp.pressed(Key::W) {
            self.sel = (self.sel + n - 1) % n;
            self.sfx.push(Sfx::Blip);
        }
        if inp.pressed(Key::Down) || inp.pressed(Key::S) {
            self.sel = (self.sel + 1) % n;
            self.sfx.push(Sfx::Blip);
        }
        let (mx, my) = (inp.mx as i32, inp.my as i32);
        for i in 0..n {
            let (x, y, w, h) = board_row(i);
            if mx >= x && mx < x + w && my >= y && my < y + h {
                if inp.mpressed[0] {
                    if self.sel == i {
                        self.screen = Screen::Brief;
                        self.sfx.push(Sfx::Confirm);
                        return;
                    }
                    self.sel = i;
                    self.sfx.push(Sfx::Blip);
                }
            }
        }
        if inp.confirm() {
            self.screen = Screen::Brief;
            self.sfx.push(Sfx::Confirm);
        }
        if inp.pressed(Key::N) {
            // a whole new solar system
            let s = self.seed.wrapping_mul(6364136223846793005).wrapping_add(self.tick as u64 | 1) % 100_000;
            self.load_system(s);
            self.stars = vec![0; n];
            self.sfx.push(Sfx::Soi);
        }
        if inp.pressed(Key::Home) && self.seed != mission::CAMPAIGN_SEED {
            self.load_system(mission::CAMPAIGN_SEED);
            self.stars = vec![0; n];
        }
        if inp.pressed(Key::Esc) {
            self.screen = Screen::Title;
            self.sfx.push(Sfx::Blip);
        }
    }

    fn update_flight(&mut self, inp: &Input) {
        let Some(p) = &mut self.play else {
            self.screen = Screen::Board;
            return;
        };
        let ended = p.end_timer > 100;
        if ended && !p.paused && !p.help {
            // results box
            let success = p.tracker.outcome == Outcome::Success;
            let items = if success { 3 } else { 3 };
            if inp.pressed(Key::Up) || inp.pressed(Key::Left) {
                self.result_sel = (self.result_sel + items - 1) % items;
                self.sfx.push(Sfx::Blip);
            }
            if inp.pressed(Key::Down) || inp.pressed(Key::Right) {
                self.result_sel = (self.result_sel + 1) % items;
                self.sfx.push(Sfx::Blip);
            }
            let (mx, my) = (inp.mx as i32, inp.my as i32);
            let mut clicked = false;
            for i in 0..items {
                let (x, y, w, h) = result_item(p, i);
                if inp.mpressed[0] && mx >= x && mx < x + w && my >= y && my < y + h {
                    self.result_sel = i;
                    clicked = true;
                }
            }
            if inp.confirm() || clicked {
                self.sfx.push(Sfx::Confirm);
                match (success, self.result_sel) {
                    (true, 0) => {
                        self.sel = (self.sel + 1) % MISSIONS.len();
                        self.screen = Screen::Brief;
                    }
                    (_, 1) => self.start_flight(),
                    (false, 0) => {
                        p.act_rewind();
                    }
                    _ => self.screen = Screen::Board,
                }
                return;
            }
            if inp.pressed(Key::R) && !success {
                p.act_rewind();
                return;
            }
            if inp.pressed(Key::Esc) {
                self.screen = Screen::Board;
                return;
            }
        }
        p.update(inp);
        if p.end_timer == 1 {
            if let Outcome::Success = p.tracker.outcome {
                let (s, _) = p.tracker.stars(p.mission);
                let i = self.sel;
                self.stars[i] = self.stars[i].max(s);
            }
        }
        self.sfx.extend(p.sfx.drain(..));
        if p.quit {
            self.screen = Screen::Board;
        }
    }

    // ------------------------------------------------------------ draw

    pub fn draw(&mut self, f: &mut Frame) {
        match self.screen {
            Screen::Title => self.draw_title(f),
            Screen::Board => self.draw_board(f),
            Screen::Brief => self.draw_brief(f),
            Screen::Flight => {
                if let Some(p) = &mut self.play {
                    p.draw(f);
                    if p.end_timer > 100 && !p.paused && !p.help {
                        draw_results(f, p, &self.stars, self.sel, self.result_sel, self.tick);
                    }
                }
            }
        }
    }

    /// A spinning planet as a backdrop.
    fn backdrop(&self, f: &mut Frame, body: usize, cx: f64, cy: f64, radius_px: f64, spin_speed: f64) {
        let sys = &self.sys;
        let t = self.tick as f64 * spin_speed;
        let st = sys.states_at(t);
        let mpp = sys.bodies[body].radius / radius_px;
        // place the body at screen (cx, cy)
        let center = st[body].pos + V2 { x: (f.w as f64 * 0.5 - cx) * mpp, y: (cy - f.h as f64 * 0.5) * mpp };
        let cam = Camera::new(center, mpp, FRAC_PI_2, f.w, f.h);
        space::draw_sky(f, &cam, &self.sky, self.tick);
        let ctx = body::Ctx { sys, gfx: &self.gfx, t, tick: self.tick };
        // light from the upper left, whatever the real sun does
        let fake_star = st[body].pos + V2 { x: -1.0, y: 0.6 }.norm() * 1e9;
        body::draw(f, &cam, &ctx, body, st[body].pos, fake_star);
        for m in sys.moons(body) {
            body::draw(f, &cam, &ctx, m, st[m].pos, fake_star);
        }
    }

    fn draw_title(&self, f: &mut Frame) {
        let (w, h) = (f.w, f.h);
        self.backdrop(f, self.sys.home, w as f64 * 0.5, h as f64 * 1.05, h as f64 * 0.62, 40.0);
        let scale = ((w - 40) / (11 * 6 + 4)).clamp(3, 7);
        f.logo(w / 2, 14, "CENTRIFUGAL", scale, &SUNSET, OUTLINE, LOGO_SHADOW);
        let sub_y = 14 + scale * 7 + 14;
        f.rect(0, sub_y - 3, w, 13, PANEL);
        f.hline(0, w, sub_y - 4, rgb(248, 176, 32));
        f.hline(0, w, sub_y + 10, rgb(176, 32, 88));
        f.text_grad_center(w / 2, sub_y, "ORBITAL MECHANICS FOR EVERYBODY", &grad7(rgb(120, 216, 248)), OUTLINE, false);
        if self.tick / 30 % 2 == 0 {
            f.text_grad_center(w / 2, h / 2 + 8, "PUSH START", &grad7(rgb(248, 248, 248)), OUTLINE, true);
        }
        f.text_center(w / 2, sub_y + 18, &format!("WELCOME TO THE {} SYSTEM", self.sys.name), BRIGHT, OUTLINE, false);
        // ticker and credit
        f.rect(0, h - 22, w, 22, PANEL);
        f.hline(0, w, h - 23, rgb(248, 176, 32));
        let tw = text_width(TICKER, false);
        let off = (self.tick as i32) % tw;
        f.text_sh(w - off, h - 19, TICKER, WARN, OUTLINE, false);
        f.text_sh(w - off + tw, h - 19, TICKER, WARN, OUTLINE, false);
        f.text_center(w / 2, h - 9, "(C) 1992 HELICO SOFT", BRIGHT, OUTLINE, false);
    }

    fn draw_board(&self, f: &mut Frame) {
        let (w, h) = (f.w, f.h);
        let m = &MISSIONS[self.sel];
        let target = mission_body(&self.sys, m);
        self.backdrop(f, target, w as f64 * 0.72, h as f64 * 0.52, (h as f64 * 0.26).min(w as f64 * 0.16), 30.0);
        f.rect(0, 0, w, 22, PANEL);
        f.hline(0, w, 22, rgb(248, 176, 32));
        f.text_grad(8, 7, "MISSION BOARD", &grad7(rgb(248, 200, 64)), OUTLINE, true);
        let sys_txt = format!("SYSTEM {}  #{}", self.sys.name, self.seed);
        f.text_sh(w - text_width(&sys_txt, false) - 8, 8, &sys_txt, CYAN, OUTLINE, false);
        for (i, ms) in MISSIONS.iter().enumerate() {
            let (x, y, bw, bh) = board_row(i);
            let sel = i == self.sel;
            f.shade_rect(x, y, bw, bh, 1, 2);
            if sel {
                f.rect(x, y, bw, bh, rgb(40, 56, 128));
                f.rect_outline(x, y, bw, bh, WARN);
            }
            f.text_sh(x + 4, y + 3, ms.code, if sel { WARN } else { DIM }, OUTLINE, false);
            f.text_sh(x + 34, y + 3, ms.title, if sel { BRIGHT } else { rgb(200, 208, 232) }, OUTLINE, false);
            for s in 0..3 {
                let c = if (s as u32) < self.stars[i] { rgb(248, 216, 64) } else { rgb(64, 64, 96) };
                f.text_sh(x + bw - 26 + s * 8, y + 3, "*", c, OUTLINE, false);
            }
        }
        // briefing preview
        let px = w / 2 - 10;
        let body_name = &self.sys.bodies[target].name;
        let py = h - 74;
        f.shade_rect(px - 4, py - 4, w - px, 56, 1, 2);
        f.text_sh(px, py, &format!("{}  -  {}", m.title, body_name), WARN, OUTLINE, true);
        f.text_sh(px, py + 13, m.tagline, CYAN, OUTLINE, false);
        for (k, l) in m.brief.iter().enumerate() {
            f.text_sh(px, py + 25 + k as i32 * 9, l, BRIGHT, OUTLINE, false);
        }
        f.rect(0, h - 14, w, 14, PANEL);
        f.text_center(w / 2, h - 11, "ENTER: BRIEFING   N: NEW SYSTEM   HOME: CAMPAIGN SYSTEM   ESC: TITLE", DIM, OUTLINE, false);
    }

    fn draw_brief(&self, f: &mut Frame) {
        let (w, h) = (f.w, f.h);
        let m = &MISSIONS[self.sel];
        let target = mission_body(&self.sys, m);
        self.backdrop(f, target, w as f64 * 0.8, h as f64 * 0.6, h as f64 * 0.3, 30.0);
        f.shade_rect(0, 0, w, h, 2, 3);
        let scale = ((w - 60) / (m.title.len() as i32 * 6 + 4)).clamp(2, 4);
        f.logo(w / 2, 8, m.title, scale, &SUNSET, OUTLINE, LOGO_SHADOW);
        let mut y = 8 + scale * 7 + 16;
        f.text_center(w / 2, y, m.tagline, CYAN, OUTLINE, false);
        y += 14;
        let col = w / 2 - 150;
        f.text_sh(col, y, "OBJECTIVES", WARN, OUTLINE, true);
        y += 12;
        for g in m.goals {
            f.text_sh(col, y, &format!("- {}", mission::goal_text(&self.sys, g)), BRIGHT, OUTLINE, false);
            y += 10;
        }
        f.text_sh(col, y + 2, "BONUS STARS", WARN, OUTLINE, true);
        y += 14;
        for b in &m.bonus {
            f.text_sh(col, y, &format!("* {}", b.text()), rgb(248, 216, 64), OUTLINE, false);
            y += 10;
        }
        y += 6;
        // tips box
        let th = 10 + m.tips.len() as i32 * 10;
        f.rect(col - 6, y - 4, 300, th + 6, PANEL);
        f.rect_outline(col - 6, y - 4, 300, th + 6, rgb(88, 104, 168));
        f.text_sh(col, y, "FLIGHT TIPS", CYAN, OUTLINE, false);
        for (k, t) in m.tips.iter().enumerate() {
            f.text_sh(col, y + 11 + k as i32 * 10, t, BRIGHT, OUTLINE, false);
        }
        y += th + 10;
        // the rocket
        let v = (m.build)(&self.sys);
        let names: Vec<&str> = v.stages.iter().map(|s| s.name).collect();
        f.text_sh(col, y, &format!("ROCKET: {}", names.join(" + ")), DIM, OUTLINE, false);
        f.text_sh(col, y + 10, &format!("DELTA-V {:.0} M/S   MASS {:.1} T", v.dv(), v.mass() / 1000.0), DIM, OUTLINE, false);
        if self.tick / 30 % 2 == 0 {
            f.text_grad_center(w / 2, h - 18, "PUSH START", &grad7(rgb(248, 248, 248)), OUTLINE, true);
        }
    }
}

impl Play {
    pub fn act_rewind(&mut self) {
        if self.flight.rewind() {
            self.fx.clear();
            self.tracker.outcome = Outcome::Flying;
            self.end_timer = 0;
        }
    }
}

fn board_row(i: usize) -> (i32, i32, i32, i32) {
    (8, 30 + i as i32 * 15, 230, 13)
}

pub fn mission_body(sys: &System, m: &Mission) -> usize {
    use mission::{Goal, Role};
    let role = m
        .goals
        .iter()
        .rev()
        .find_map(|g| match *g {
            Goal::Land(r, _) | Goal::Orbit(r, _, _) | Goal::Reach(r) | Goal::Recover(r) | Goal::Deploy(r, _, _) => Some(r),
            _ => None,
        })
        .unwrap_or(Role::Home);
    mission::body_of(sys, role)
}

fn result_box(f_w: i32, f_h: i32) -> (i32, i32, i32, i32) {
    let (bw, bh) = (300, 150);
    ((f_w - bw) / 2, (f_h - bh) / 2, bw, bh)
}

fn result_item(p: &Play, i: usize) -> (i32, i32, i32, i32) {
    let (fw, fh) = p.frame_size();
    let (bx, by, bw, bh) = result_box(fw, fh);
    let iw = (bw - 20) / 3;
    (bx + 10 + i as i32 * iw, by + bh - 22, iw - 4, 14)
}

fn draw_results(f: &mut Frame, p: &Play, _stars: &[u32], _sel: usize, item: usize, tick: u32) {
    let (bx, by, bw, bh) = result_box(f.w, f.h);
    f.shade_rect(0, 0, f.w, f.h, 2, 3);
    f.rect(bx, by, bw, bh, PANEL);
    f.rect_outline(bx, by, bw, bh, rgb(248, 176, 32));
    let success = p.tracker.outcome == Outcome::Success;
    let (title, col) = if success { ("MISSION COMPLETE", GOOD) } else { ("MISSION FAILED", rgb(248, 80, 64)) };
    f.text_grad_center(f.w / 2, by + 8, title, &grad7(col), OUTLINE, true);
    let m = p.mission;
    if success {
        let (s, bonus) = p.tracker.stars(m);
        // stars pop in one by one
        let shown = ((p.end_timer.saturating_sub(100)) / 20).min(3);
        for k in 0..3u32 {
            let lit = k < s && k < shown;
            let c = if lit { rgb(248, 216, 64) } else { rgb(64, 64, 96) };
            let x = f.w / 2 - 30 + k as i32 * 30;
            f.logo(x, by + 24, "*", 3, &if lit { SUNSET.to_vec() } else { vec![c; 6] }, OUTLINE, LOGO_SHADOW);
        }
        let mut y = by + 62;
        f.text_sh(bx + 14, y, "* MISSION DONE", GOOD, OUTLINE, false);
        y += 11;
        for (k, b) in m.bonus.iter().enumerate() {
            let c = if bonus[k] { GOOD } else { DIM };
            f.text_sh(bx + 14, y, &format!("{} {}", if bonus[k] { "*" } else { "-" }, b.text()), c, OUTLINE, false);
            y += 11;
        }
        let t = &p.tracker;
        f.text_sh(bx + 14, y + 2, &format!("FUEL LEFT {:.0}%", t.fuel_left * 100.0), DIM, OUTLINE, false);
        if let Some(d) = t.land_dist.filter(|d| d.is_finite()) {
            f.text_sh(bx + 150, y + 2, &format!("FROM SITE {}", crate::render::text::dist(d)), DIM, OUTLINE, false);
        }
    } else {
        let why = match p.tracker.outcome {
            Outcome::Failed(w) => w,
            _ => "",
        };
        f.text_center(f.w / 2, by + 40, why, WARN, OUTLINE, false);
        let quips = ["THE COWS SAW EVERYTHING.", "WE'LL CALL IT A TEST FLIGHT.", "SCIENCE HAPPENED. LOUDLY.", "THE INSURANCE PEOPLE ARE CRYING."];
        f.text_center(f.w / 2, by + 60, quips[(tick as usize / 600) % quips.len()], DIM, OUTLINE, false);
        f.text_center(f.w / 2, by + 84, "R: REWIND A FEW SECONDS", BRIGHT, OUTLINE, false);
    }
    let labels = if success { ["NEXT", "RETRY", "MENU"] } else { ["REWIND", "RETRY", "MENU"] };
    for (i, l) in labels.iter().enumerate() {
        let (x, y, w, h) = result_item(p, i);
        let sel = i == item;
        f.rect(x, y, w, h, if sel { rgb(48, 112, 200) } else { rgb(40, 40, 80) });
        f.rect_outline(x, y, w, h, if sel { CYAN } else { rgb(72, 72, 128) });
        f.text_center(x + w / 2, y + 4, l, BRIGHT, OUTLINE, false);
    }
    let _ = (mix, shade, play::DIM);
}
