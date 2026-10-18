// release builds on Windows open no console window next to the game
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use centrifugal::engine::gfx::Frame;
use centrifugal::engine::input::{Input, Key};
use centrifugal::engine::synth::wav;
use centrifugal::game::app::App;
use centrifugal::game::audio;
use centrifugal::sim::physics::TICK;
use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound};
use macroquad::prelude::*;

fn conf() -> Conf {
    Conf {
        window_title: "CENTRIFUGAL".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--shots") => centrifugal::headless::shots(args.get(2).map(String::as_str).unwrap_or("shots")),
        Some("--bots") => centrifugal::headless::bots(),
        Some(x) => {
            eprintln!("unknown option {x}; usage: centrifugal [--shots <dir>] [--bots]");
            std::process::exit(2);
        }
        None => macroquad::Window::from_config(conf(), run()),
    }
}

const KEYS: &[(Key, &[KeyCode])] = &[
    (Key::Left, &[KeyCode::Left]),
    (Key::Right, &[KeyCode::Right]),
    (Key::Up, &[KeyCode::Up]),
    (Key::Down, &[KeyCode::Down]),
    (Key::W, &[KeyCode::W]),
    (Key::A, &[KeyCode::A]),
    (Key::S, &[KeyCode::S]),
    (Key::D, &[KeyCode::D]),
    (Key::Z, &[KeyCode::Z]),
    (Key::X, &[KeyCode::X]),
    (Key::C, &[KeyCode::C]),
    (Key::Q, &[KeyCode::Q]),
    (Key::E, &[KeyCode::E]),
    (Key::R, &[KeyCode::R]),
    (Key::F, &[KeyCode::F]),
    (Key::T, &[KeyCode::T]),
    (Key::M, &[KeyCode::M]),
    (Key::N, &[KeyCode::N]),
    (Key::P, &[KeyCode::P]),
    (Key::H, &[KeyCode::H]),
    (Key::G, &[KeyCode::G]),
    (Key::Space, &[KeyCode::Space]),
    (Key::Enter, &[KeyCode::Enter, KeyCode::KpEnter]),
    (Key::Esc, &[KeyCode::Escape]),
    (Key::Tab, &[KeyCode::Tab]),
    (Key::Shift, &[KeyCode::LeftShift, KeyCode::RightShift]),
    (Key::Ctrl, &[KeyCode::LeftControl, KeyCode::RightControl]),
    (Key::Comma, &[KeyCode::Comma]),
    (Key::Period, &[KeyCode::Period]),
    (Key::Delete, &[KeyCode::Delete]),
    (Key::Backspace, &[KeyCode::Backspace]),
    (Key::Home, &[KeyCode::Home]),
    (Key::Num1, &[KeyCode::Key1, KeyCode::Kp1]),
    (Key::Num2, &[KeyCode::Key2, KeyCode::Kp2]),
    (Key::Num3, &[KeyCode::Key3, KeyCode::Kp3]),
    (Key::Num4, &[KeyCode::Key4, KeyCode::Kp4]),
    (Key::Num5, &[KeyCode::Key5, KeyCode::Kp5]),
    (Key::Num6, &[KeyCode::Key6, KeyCode::Kp6]),
    (Key::F1, &[KeyCode::F1]),
    (Key::F2, &[KeyCode::F2]),
];

/// Keys and mouse for one tick; presses since the last tick are kept in `acc`.
fn read_input(acc: &mut Input, scale: f32) {
    for (k, codes) in KEYS {
        let down = codes.iter().any(|c| is_key_down(*c));
        let pressed = codes.iter().any(|c| is_key_pressed(*c));
        acc.set(*k, down, pressed);
    }
    let (mx, my) = mouse_position();
    acc.mx = mx / scale;
    acc.my = my / scale;
    for (i, b) in [MouseButton::Left, MouseButton::Right, MouseButton::Middle].iter().enumerate() {
        acc.mdown[i] = is_mouse_button_down(*b);
        acc.mpressed[i] |= is_mouse_button_pressed(*b);
        acc.mreleased[i] |= is_mouse_button_released(*b);
    }
    let (_, wy) = mouse_wheel();
    if wy != 0.0 {
        // browsers report pixels, desktops report notches
        let w = if wy.abs() >= 50.0 { wy / 100.0 } else { wy };
        acc.wheel += w.clamp(-3.0, 3.0);
    }
}

async fn load(samples: &[f32]) -> Option<Sound> {
    load_sound_from_bytes(&wav(samples)).await.ok()
}

async fn run() {
    let seed = (macroquad::miniquad::date::now() * 1000.0) as u64;
    let mut app = App::new(seed);
    let mut sfx = Vec::new();
    for s in audio::SFX {
        let (samples, vol) = audio::sfx(s);
        sfx.push(load(&samples).await.map(|x| (x, vol)));
    }
    let mut songs: Vec<Option<(Sound, f32)>> = vec![None, None, None];
    let engine = load(&audio::engine_loop()).await;
    if let Some(e) = &engine {
        play_sound(e, PlaySoundParams { looped: true, volume: 0.0 });
    }
    let mut frame = Frame::new(640, 360);
    let mut rgba = Vec::new();
    frame.to_rgba(&mut rgba);
    let mut tex = Texture2D::from_rgba8(frame.w as u16, frame.h as u16, &rgba);
    tex.set_filter(FilterMode::Nearest);
    let mut acc = Input::default();
    let mut acc_t = 0.0f32;
    let mut playing: Option<usize> = None;
    let mut pixel_bias = 0i32;
    loop {
        let (sw, sh) = (screen_width(), screen_height());
        let base = (sh / 340.0).floor().max(2.0) as i32;
        let scale = (base + pixel_bias).clamp(1, 8) as f32;
        let (fw, fh) = ((sw / scale).ceil().max(160.0) as i32, (sh / scale).ceil().max(120.0) as i32);
        if (fw, fh) != (frame.w, frame.h) {
            frame.resize(fw, fh);
            frame.to_rgba(&mut rgba);
            tex = Texture2D::from_rgba8(fw as u16, fh as u16, &rgba);
            tex.set_filter(FilterMode::Nearest);
        }
        read_input(&mut acc, scale);
        if acc.pressed(Key::F2) {
            pixel_bias = if base + pixel_bias >= base + 2 { -1 } else { pixel_bias + 1 };
        }
        acc_t += get_frame_time().min(0.1);
        while acc_t >= TICK as f32 {
            app.update(&acc);
            // presses count once
            acc.pressed = 0;
            acc.mpressed = [false; 3];
            acc.mreleased = [false; 3];
            acc.wheel = 0.0;
            acc_t -= TICK as f32;
        }
        for s in app.sfx.drain(..) {
            if let Some(Some((snd, vol))) = sfx.get(audio::sfx_index(s)) {
                play_sound(snd, PlaySoundParams { looped: false, volume: *vol });
            }
        }
        let want = app.music().map(audio::music_index);
        if want != playing {
            if let Some(Some((snd, _))) = playing.and_then(|p| songs.get(p)) {
                stop_sound(snd);
            }
            if let Some(w) = want {
                if songs[w].is_none() {
                    let (samples, vol) = audio::song(w);
                    songs[w] = load(&samples).await.map(|s| (s, vol));
                    get_frame_time();
                    acc_t = 0.0;
                }
                if let Some((snd, vol)) = &songs[w] {
                    play_sound(snd, PlaySoundParams { looped: true, volume: *vol });
                }
            }
            playing = want;
        }
        if let Some(e) = &engine {
            set_sound_volume(e, app.engine_volume() * 0.6);
        }
        if app.quit {
            if cfg!(target_arch = "wasm32") {
                app.quit = false;
            } else {
                break;
            }
        }
        app.draw(&mut frame);
        frame.to_rgba(&mut rgba);
        tex.update_from_bytes(frame.w as u32, frame.h as u32, &rgba);
        clear_background(BLACK);
        draw_texture_ex(&tex, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(frame.w as f32 * scale, frame.h as f32 * scale)), ..Default::default() });
        next_frame().await;
    }
}
