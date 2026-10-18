//! The shared look: outline color, the sunset logo gradient and gradient
//! helpers for big logo text.

use super::gfx::{mix, rgb, shade};

/// Dark navy used for every outline and text shadow.
pub const OUTLINE: u32 = rgb(16, 16, 32);
/// Drop shadow under logo text.
pub const LOGO_SHADOW: u32 = rgb(8, 8, 24);

pub const SUNSET: [u32; 6] =
    [rgb(248, 248, 168), rgb(248, 224, 72), rgb(248, 176, 32), rgb(248, 120, 32), rgb(232, 64, 48), rgb(176, 32, 88)];

pub fn grad7(c: u32) -> [u32; 7] {
    [mix(c, 0xFFFFFF, 0.7), mix(c, 0xFFFFFF, 0.35), c, c, shade(c, 7, 8), shade(c, 3, 4), shade(c, 5, 8)]
}

pub fn grad6(c: u32) -> [u32; 6] {
    [mix(c, 0xFFFFFF, 0.6), mix(c, 0xFFFFFF, 0.25), c, shade(c, 7, 8), shade(c, 3, 4), shade(c, 3, 5)]
}
