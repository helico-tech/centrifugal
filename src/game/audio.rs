//! Every sound, generated at start-up with the chiptune synth: menu blips,
//! staging thunks, explosions, an engine rumble loop and three songs.

use crate::engine::synth::{Buf, R, Song, Wave, render_song};
use crate::game::app::Music;
use crate::game::play::Sfx;

pub const SFX: [Sfx; 11] =
    [Sfx::Blip, Sfx::Confirm, Sfx::Stage, Sfx::Boom, Sfx::Liftoff, Sfx::Touchdown, Sfx::Chute, Sfx::Beep, Sfx::Node, Sfx::Warn, Sfx::Soi];

pub fn sfx_index(s: Sfx) -> usize {
    SFX.iter().position(|x| *x == s).unwrap_or(0)
}

pub fn music_index(m: Music) -> usize {
    match m {
        Music::Title => 0,
        Music::Flight => 1,
        Music::Win => 2,
    }
}

/// (samples, volume) for one effect.
pub fn sfx(s: Sfx) -> (Vec<f32>, f32) {
    match s {
        Sfx::Blip => {
            let mut b = Buf::new(0.06);
            b.tone(0.0, 0.05, 900.0, 1250.0, Wave::Square(0.25), 0.25, 0.001, 1.0);
            (b.s, 0.5)
        }
        Sfx::Confirm => {
            let mut b = Buf::new(0.2);
            b.tone(0.0, 0.07, 660.0, 660.0, Wave::Square(0.5), 0.22, 0.001, 0.5);
            b.tone(0.07, 0.12, 990.0, 990.0, Wave::Square(0.5), 0.22, 0.001, 1.0);
            (b.s, 0.5)
        }
        Sfx::Stage => {
            let mut b = Buf::new(0.5);
            b.tone(0.0, 0.2, 140.0, 50.0, Wave::Sine, 0.7, 0.001, 1.2);
            b.hiss(0.0, 0.45, 0.5, 0.4, 0.002, 2.0);
            (b.s, 0.7)
        }
        Sfx::Boom => {
            let mut b = Buf::new(1.8);
            b.tone(0.0, 0.8, 110.0, 28.0, Wave::Sine, 0.9, 0.001, 1.5);
            b.hiss(0.0, 1.7, 0.8, 0.25, 0.003, 1.8);
            b.hiss(0.05, 0.6, 0.5, 0.7, 0.001, 2.5);
            (b.s, 0.9)
        }
        Sfx::Liftoff => {
            let mut b = Buf::new(2.2);
            b.hiss(0.0, 2.2, 0.7, 0.15, 0.4, 1.3);
            b.tone(0.0, 2.0, 55.0, 70.0, Wave::Tri, 0.4, 0.3, 1.0);
            (b.s, 0.7)
        }
        Sfx::Touchdown => {
            let mut b = Buf::new(0.7);
            b.tone(0.0, 0.15, 120.0, 60.0, Wave::Sine, 0.6, 0.001, 1.0);
            for (i, n) in [784.0, 988.0, 1175.0, 1568.0].iter().enumerate() {
                b.tone(0.12 + i as f32 * 0.08, 0.2, *n, *n, Wave::Square(0.25), 0.15, 0.002, 1.0);
            }
            (b.s, 0.6)
        }
        Sfx::Chute => {
            let mut b = Buf::new(0.7);
            b.hiss(0.0, 0.08, 0.6, 0.8, 0.001, 1.0);
            b.hiss(0.08, 0.6, 0.25, 0.2, 0.05, 1.0);
            (b.s, 0.6)
        }
        Sfx::Beep => {
            let mut b = Buf::new(0.1);
            b.tone(0.0, 0.08, 1000.0, 1000.0, Wave::Square(0.5), 0.2, 0.001, 0.3);
            (b.s, 0.4)
        }
        Sfx::Node => {
            let mut b = Buf::new(0.35);
            for (i, n) in [523.0, 659.0, 784.0].iter().enumerate() {
                b.tone(i as f32 * 0.06, 0.12, *n, *n, Wave::Square(0.25), 0.18, 0.001, 1.0);
            }
            (b.s, 0.5)
        }
        Sfx::Warn => {
            let mut b = Buf::new(0.6);
            for i in 0..4 {
                let f = if i % 2 == 0 { 880.0 } else { 660.0 };
                b.tone(i as f32 * 0.14, 0.12, f, f, Wave::Square(0.5), 0.18, 0.001, 0.2);
            }
            (b.s, 0.5)
        }
        Sfx::Soi => {
            let mut b = Buf::new(0.8);
            for (i, n) in [440.0, 554.0, 659.0, 880.0, 1109.0].iter().enumerate() {
                b.tone(i as f32 * 0.07, 0.4, *n, *n * 1.01, Wave::Tri, 0.2, 0.01, 1.0);
            }
            (b.s, 0.5)
        }
    }
}

/// Low rumble that loops without a click.
pub fn engine_loop() -> Vec<f32> {
    let len = 1.5;
    let mut b = Buf::new(len + 0.3);
    b.hiss(0.0, len + 0.3, 0.9, 0.06, 0.0001, 0.0);
    b.tone(0.0, len + 0.3, 48.0, 48.0, Wave::Tri, 0.25, 0.0001, 0.0);
    let n = (len * 44100.0) as usize;
    let fade = (0.3 * 44100.0) as usize;
    let mut out: Vec<f32> = b.s[..n].to_vec();
    for i in 0..fade {
        let t = i as f32 / fade as f32;
        out[i] = out[i] * t + b.s[n + i] * (1.0 - t);
    }
    out
}

const TITLE: Song = Song {
    bpm: 96.0,
    chords: &[[57, 60, 64], [53, 57, 60], [48, 52, 55], [55, 59, 62], [57, 60, 64], [53, 57, 60], [50, 53, 57], [52, 56, 59]],
    melody: &[
        "A4 - - - C5 - E5 - - - D5 - C5 - B4 -",
        "A4 - - - - - F4 - A4 - C5 - - - - -",
        "G4 - - - C5 - E5 - G5 - - - E5 - D5 -",
        "D5 - - - B4 - - - G4 - - - - - - -",
        "A4 - - - C5 - E5 - A5 - - - G5 - E5 -",
        "F5 - - - E5 - C5 - A4 - - - - - - -",
        "D5 - - - F5 - A5 - G5 - F5 - E5 - D5 -",
        "E5 - - - - - - - G#4 - - - B4 - - -",
    ],
    bass: [0, R, R, 0, R, R, 7, R, 0, R, R, 0, R, 12, 7, R],
    drums: "k...h...s...h.h.",
    lead_duty: 0.25,
};

const FLIGHT: Song = Song {
    bpm: 132.0,
    chords: &[[50, 53, 57], [46, 50, 53], [48, 52, 55], [45, 49, 52]],
    melody: &[
        "D5 - F5 - A5 - - F5 - - D5 - E5 - F5 -",
        "D5 - - - A4 - - - A#4 - D5 - F5 - - -",
        "E5 - G5 - C6 - - G5 - - E5 - G5 - E5 -",
        "C#5 - - - E5 - - - A5 - - - G5 - E5 -",
    ],
    bass: [0, R, 0, 12, R, 0, R, 12, 0, R, 0, 12, R, 7, 10, 12],
    drums: "k.h.s.h.k.k.s.hh",
    lead_duty: 0.125,
};

const WIN: Song = Song {
    bpm: 120.0,
    chords: &[[48, 52, 55], [53, 57, 60], [55, 59, 62], [48, 52, 55]],
    melody: &[
        "C5 - E5 - G5 - C6 - - - G5 - E5 - G5 -",
        "F5 - A5 - C6 - - - A5 - F5 - A5 - - -",
        "G5 - B5 - D6 - - - B5 - G5 - B5 - D6 -",
        "C6 - - - - - G5 - E5 - - - C5 - - -",
    ],
    bass: [0, R, 7, R, 12, R, 7, R, 0, R, 7, R, 12, R, 7, R],
    drums: "k.h.s.h.k.h.s.hs",
    lead_duty: 0.25,
};

pub fn song(i: usize) -> (Vec<f32>, f32) {
    match i {
        0 => (render_song(&TITLE), 0.45),
        1 => (render_song(&FLIGHT), 0.32),
        _ => (render_song(&WIN), 0.45),
    }
}
