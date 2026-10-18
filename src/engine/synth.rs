//! Chiptune synth shared by all games. Sound effects and music are
//! generated at start-up as WAV bytes: pulse and triangle waves, noise
//! drums, and an echo on the lead like the SNES sound chip's echo buffer.

pub const SR: f32 = 44100.0;

pub fn wav(samples: &[f32]) -> Vec<u8> {
    let n = samples.len() as u32;
    let mut v = Vec::with_capacity(44 + samples.len() * 2);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + n * 2).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&44100u32.to_le_bytes());
    v.extend_from_slice(&88200u32.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(n * 2).to_le_bytes());
    for s in samples {
        v.extend_from_slice(&((s.clamp(-1.0, 1.0) * 30000.0) as i16).to_le_bytes());
    }
    v
}

#[derive(Clone, Copy)]
pub enum Wave {
    Square(f32),
    Tri,
    Sine,
}

pub fn osc(w: Wave, phase: f32) -> f32 {
    let p = phase.fract();
    match w {
        Wave::Square(d) => {
            if p < d {
                1.0
            } else {
                -1.0
            }
        }
        Wave::Tri => 4.0 * (p - 0.5).abs() - 1.0,
        Wave::Sine => (p * std::f32::consts::TAU).sin(),
    }
}

pub fn midi_hz(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

pub struct Buf {
    pub s: Vec<f32>,
    noise: u32,
}

impl Buf {
    pub fn new(secs: f32) -> Self {
        Buf { s: vec![0.0; (secs * SR) as usize + 1], noise: 0xACE1 }
    }
    pub fn noise(&mut self) -> f32 {
        let mut x = self.noise;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.noise = x;
        (x >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
    /// Tone sweeping from f0 to f1 (Hz). Envelope: attack, then decay with power `p`.
    #[allow(clippy::too_many_arguments)]
    pub fn tone(&mut self, start: f32, dur: f32, f0: f32, f1: f32, w: Wave, vol: f32, attack: f32, p: f32) {
        let i0 = (start * SR) as usize;
        let n = (dur * SR) as usize;
        let mut phase = 0.0f32;
        for k in 0..n {
            let t = k as f32 / SR;
            let u = k as f32 / n as f32;
            let f = f0 + (f1 - f0) * u;
            phase += f / SR;
            let env = (t / attack.max(1e-4)).min(1.0) * (1.0 - u).powf(p);
            if let Some(x) = self.s.get_mut(i0 + k) {
                *x += osc(w, phase) * vol * env;
            }
        }
    }
    /// Filtered noise burst; `lp` in 0..1 (lower = darker).
    pub fn hiss(&mut self, start: f32, dur: f32, vol: f32, lp: f32, attack: f32, p: f32) {
        let i0 = (start * SR) as usize;
        let n = (dur * SR) as usize;
        let mut y = 0.0;
        for k in 0..n {
            let t = k as f32 / SR;
            let u = k as f32 / n as f32;
            let r = self.noise();
            y += (r - y) * lp;
            let env = (t / attack.max(1e-4)).min(1.0) * (1.0 - u).powf(p);
            if let Some(x) = self.s.get_mut(i0 + k) {
                *x += y * vol * env;
            }
        }
    }
    pub fn hat(&mut self, start: f32, dur: f32, vol: f32) {
        let i0 = (start * SR) as usize;
        let n = (dur * SR) as usize;
        let mut lp = 0.0;
        for k in 0..n {
            let u = k as f32 / n as f32;
            let r = self.noise();
            lp += (r - lp) * 0.3;
            if let Some(x) = self.s.get_mut(i0 + k) {
                *x += (r - lp) * vol * (1.0 - u).powi(2);
            }
        }
    }
}

/// A looping song: one chord per bar of 16 steps, a melody line per bar
/// ("E5 - . C#5": note, hold, rest), a bass pattern as offsets from the
/// chord root (R = rest) and a drum pattern (k kick, s snare, h hat).
pub struct Song {
    pub bpm: f32,
    pub chords: &'static [[i32; 3]],
    pub melody: &'static [&'static str],
    pub bass: [i32; 16],
    pub drums: &'static str,
    pub lead_duty: f32,
}

pub fn parse_note(tok: &str) -> Option<i32> {
    let b = tok.as_bytes();
    let base = match b.first()? {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return None,
    };
    let (acc, rest) = if b.get(1) == Some(&b'#') { (1, &tok[2..]) } else { (0, &tok[1..]) };
    let oct: i32 = rest.parse().ok()?;
    Some(12 * (oct + 1) + base + acc)
}

/// Rest marker for `Song::bass`.
pub const R: i32 = 99;

pub fn render_song(song: &Song) -> Vec<f32> {
    let step = 60.0 / song.bpm / 4.0;
    let bars = song.chords.len();
    let len = bars as f32 * 16.0 * step;
    let mut lead = Buf::new(len + 0.1);
    let mut b = Buf::new(len + 0.1);

    // melody: collect note events
    let toks: Vec<&str> = song.melody.iter().flat_map(|bar| bar.split_whitespace()).collect();
    let mut k = 0;
    while k < toks.len() {
        if let Some(n) = parse_note(toks[k]) {
            let mut l = 1;
            while k + l < toks.len() && toks[k + l] == "-" {
                l += 1;
            }
            let start = k as f32 * step;
            let dur = l as f32 * step * 0.95;
            let i0 = (start * SR) as usize;
            let cnt = (dur * SR) as usize;
            let mut ph = 0.0f32;
            let f = midi_hz(n as f32);
            for j in 0..cnt {
                let t = j as f32 / SR;
                let vib = if t > 0.15 { 1.0 + 0.006 * (t * 34.0).sin() } else { 1.0 };
                ph += f * vib / SR;
                let env = (t / 0.004).min(1.0) * (1.0 - 0.35 * (t / 0.2).min(1.0)) * ((cnt - j) as f32 / 300.0).min(1.0);
                if let Some(x) = lead.s.get_mut(i0 + j) {
                    *x += osc(Wave::Square(song.lead_duty), ph) * 0.12 * env;
                }
            }
            k += l;
        } else {
            k += 1;
        }
    }
    // echo: three 16ths later, quieter, twice
    let delay = (step * 3.0 * SR) as usize;
    let dry = lead.s.clone();
    for (i, x) in lead.s.iter_mut().enumerate() {
        if i >= delay {
            *x += dry[i - delay] * 0.35;
        }
        if i >= delay * 2 {
            *x += dry[i - delay * 2] * 0.15;
        }
    }

    let drums: Vec<u8> = song.drums.bytes().collect();
    for bar in 0..bars {
        let ch = song.chords[bar];
        for s in 0..16 {
            let t = (bar * 16 + s) as f32 * step;
            // arpeggio
            let n = ch[s % 3] + 12;
            b.tone(t, step * 0.9, midi_hz(n as f32), midi_hz(n as f32), Wave::Square(0.125), 0.035, 0.002, 1.5);
            // bass
            let off = song.bass[s];
            if off != R {
                let f = midi_hz((ch[0] - 12 + off) as f32);
                b.tone(t, step * 0.85, f, f, Wave::Tri, 0.28, 0.002, 0.4);
            }
            // drums
            match drums[s % drums.len()] {
                b'k' => {
                    b.tone(t, 0.12, 130.0, 42.0, Wave::Sine, 0.6, 0.001, 1.2);
                    b.hiss(t, 0.01, 0.2, 0.9, 0.0005, 1.0);
                }
                b's' => {
                    b.hiss(t, 0.15, 0.32, 0.6, 0.001, 1.6);
                    b.tone(t, 0.06, 200.0, 160.0, Wave::Tri, 0.25, 0.001, 1.0);
                }
                b'h' => b.hat(t, 0.035, 0.18),
                _ => {}
            }
        }
    }
    for (x, l) in b.s.iter_mut().zip(lead.s.iter()) {
        *x = (*x + l) * 0.9;
    }
    b.s.truncate((len * SR) as usize);
    b.s
}

