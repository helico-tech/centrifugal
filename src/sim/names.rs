//! Procedural names for stars, planets, moons and ships.

use crate::engine::rng::Rng;

const START: &[&str] = &[
    "KA", "VE", "LU", "ZOR", "THA", "MI", "OR", "BEL", "XA", "QUI", "DRA", "NO", "SEL", "UR", "PHO", "GRI", "TY", "AE",
    "ZE", "MON", "CAL", "RHE", "IO", "VAN", "SKY", "BRO", "OL", "PEL", "NYX", "FAR", "TOR", "HEL", "ME", "JUN", "KOR",
];
const MID: &[&str] = &[
    "RA", "LI", "NA", "VO", "TE", "RI", "LO", "MA", "DE", "SA", "KE", "NU", "GA", "ZI", "BO", "TH", "RU", "PE", "XI",
];
const END: &[&str] = &[
    "N", "S", "RA", "IS", "ON", "IA", "AR", "UX", "ETH", "OS", "EA", "UM", "AX", "ION", "OR", "A", "US", "IR", "O", "EL",
];
const GREEK: &[&str] = &["ALPHA", "BETA", "GAMMA", "DELTA", "TAU", "SIGMA", "OMEGA", "PRIME"];
const SILLY: &[&str] = &[
    "BIG", "LITTLE", "OLD", "NEW", "GREAT", "MINI", "MEGA", "SAINT", "FORT", "LAKE", "MOUNT", "PORT", "UPPER",
];

/// A pronounceable made-up word of 2-3 syllables.
pub fn word(rng: &mut Rng) -> String {
    let mut s = String::new();
    s.push_str(START[rng.int(START.len() as i32) as usize]);
    if rng.f() < 0.55 {
        s.push_str(MID[rng.int(MID.len() as i32) as usize]);
    }
    s.push_str(END[rng.int(END.len() as i32) as usize]);
    s
}

pub fn star(rng: &mut Rng) -> String {
    format!("{} {}", word(rng), GREEK[rng.int(GREEK.len() as i32) as usize])
}

pub fn planet(rng: &mut Rng) -> String {
    let w = word(rng);
    match rng.int(10) {
        0 => format!("{} {}", SILLY[rng.int(SILLY.len() as i32) as usize], w),
        1 => format!("{w} IV"),
        2 => format!("{w} MAJOR"),
        _ => w,
    }
}

pub fn moon(rng: &mut Rng) -> String {
    let w = word(rng);
    if rng.f() < 0.15 { format!("LITTLE {w}") } else { w }
}

const SHIP_A: &[&str] = &[
    "JUST READ THE", "OF COURSE I", "PLEASE DO NOT", "NOBODY ASKED", "IT WAS LIKE", "HOLD MY", "MOSTLY",
];
const SHIP_B: &[&str] = &["MANUAL", "STILL FLOAT", "TOUCH THIS", "THE COWS", "THAT WHEN I GOT HERE", "COFFEE", "HARMLESS"];

/// Names for drone ships and satellites, in the spirit of real ones.
pub fn ship(rng: &mut Rng) -> String {
    format!("{} {}", SHIP_A[rng.int(SHIP_A.len() as i32) as usize], SHIP_B[rng.int(SHIP_B.len() as i32) as usize])
}
