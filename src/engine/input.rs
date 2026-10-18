//! Keyboard and mouse state for one tick. The window layer fills it in;
//! the game only reads it, so headless runs can script it.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    W,
    A,
    S,
    D,
    Z,
    X,
    C,
    Q,
    E,
    R,
    F,
    T,
    M,
    N,
    P,
    H,
    G,
    Space,
    Enter,
    Esc,
    Tab,
    Shift,
    Ctrl,
    Comma,
    Period,
    Delete,
    Backspace,
    Home,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    F1,
    F2,
}

pub const KEY_COUNT: usize = Key::F2 as usize + 1;

#[derive(Clone, Default, Debug)]
pub struct Input {
    pub down: u64,
    pub pressed: u64,
    /// Mouse in frame pixels.
    pub mx: f32,
    pub my: f32,
    /// Buttons: 0 left, 1 right, 2 middle.
    pub mdown: [bool; 3],
    pub mpressed: [bool; 3],
    pub mreleased: [bool; 3],
    /// Wheel clicks this tick (> 0 = away from you = zoom in).
    pub wheel: f32,
}

impl Input {
    pub fn down(&self, k: Key) -> bool {
        self.down >> k as u8 & 1 == 1
    }
    pub fn pressed(&self, k: Key) -> bool {
        self.pressed >> k as u8 & 1 == 1
    }
    pub fn set(&mut self, k: Key, down: bool, pressed: bool) {
        let bit = 1u64 << k as u8;
        if down {
            self.down |= bit;
        }
        if pressed {
            self.pressed |= bit;
        }
    }
    /// Any key or mouse button went down this tick.
    pub fn any_pressed(&self) -> bool {
        self.pressed != 0 || self.mpressed.iter().any(|b| *b)
    }
    /// Confirm in menus: Enter, Space, Z or a click.
    pub fn confirm(&self) -> bool {
        self.pressed(Key::Enter) || self.pressed(Key::Space) || self.pressed(Key::Z)
    }
}
