//! Small deterministic random numbers (xorshift). Games never read the clock.

#[derive(Clone, Copy, Debug)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
    pub fn f(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn int(&mut self, n: i32) -> i32 {
        (self.next() % n.max(1) as u32) as i32
    }
}
