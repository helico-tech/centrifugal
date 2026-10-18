//! Software framebuffer, drawing primitives and a 5x7 bitmap font.
//! The frame size follows the window (window size / pixel scale), so the
//! game fills modern screens with big chunky pixels. Everything on screen
//! is drawn through this module, so the game renders the same in the
//! window and in headless screenshots.

/// Colors are 0xRRGGBB. Use `rgb` so values snap to the SNES 15-bit grid.
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (((r & 0xF8) as u32) << 16) | (((g & 0xF8) as u32) << 8) | (b & 0xF8) as u32
}

pub fn shade(c: u32, num: u32, den: u32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 255) * num / den).min(255)) << s;
    ch(16) | ch(8) | ch(0)
}

pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |s: u32| {
        let x = ((a >> s) & 255) as f32;
        let y = ((b >> s) & 255) as f32;
        (((x + (y - x) * t) as u32) & 0xF8) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// Bresenham line; calls `f` for every point.
pub fn line_points(x0: i32, y0: i32, x1: i32, y1: i32, mut f: impl FnMut(i32, i32)) {
    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        f(x, y);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[derive(Clone)]
pub struct Frame {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u32>,
}

impl Frame {
    pub fn new(w: i32, h: i32) -> Self {
        Frame { w, h, px: vec![0; (w * h) as usize] }
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        if (w, h) != (self.w, self.h) {
            *self = Frame::new(w, h);
        }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: u32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize]
        } else {
            0
        }
    }

    pub fn copy_from(&mut self, other: &Frame) {
        self.px.copy_from_slice(&other.px);
    }

    pub fn clear(&mut self, c: u32) {
        self.px.fill(c);
    }

    /// Blend `c` over the pixel by `t` (0..1), snapped to the color grid.
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: u32, t: f32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            let i = (y * self.w + x) as usize;
            self.px[i] = mix(self.px[i], c, t);
        }
    }

    /// Set the pixel only where the ordered-dither threshold says so: the
    /// retro way to draw a fraction `t` (0..1) of a color.
    #[inline]
    pub fn dither(&mut self, x: i32, y: i32, c: u32, t: f32) {
        if t > bayer(x, y) {
            self.set(x, y, c);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                self.px[(yy * self.w + xx) as usize] = c;
            }
        }
    }

    pub fn rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        self.hline(x, x + w - 1, y, c);
        self.hline(x, x + w - 1, y + h - 1, c);
        for yy in y..y + h {
            self.set(x, yy, c);
            self.set(x + w - 1, yy, c);
        }
    }

    /// Checkerboard of two colors: the classic way to fake a third color.
    pub fn dither_rect(&mut self, x: i32, y: i32, w: i32, h: i32, a: u32, b: u32) {
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                self.px[(yy * self.w + xx) as usize] = if (xx + yy) & 1 == 0 { a } else { b };
            }
        }
    }

    /// Darken (or lighten when num > den) everything inside a rectangle.
    pub fn shade_rect(&mut self, x: i32, y: i32, w: i32, h: i32, num: u32, den: u32) {
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                let i = (yy * self.w + xx) as usize;
                self.px[i] = shade(self.px[i], num, den);
            }
        }
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, c: u32) {
        if !(0..self.h).contains(&y) {
            return;
        }
        let (a, b) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        for x in a.max(0)..=b.min(self.w - 1) {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        line_points(x0, y0, x1, y1, |x, y| self.set(x, y, c));
    }

    /// Line between float coordinates, clipped to the frame first, so huge
    /// off-screen lines (orbits, trajectories) cost nothing.
    pub fn line_f(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: u32) {
        if let Some((a, b, cx, cy)) = clip(x0, y0, x1, y1, self.w as f64, self.h as f64) {
            self.line(a.floor() as i32, b.floor() as i32, cx.floor() as i32, cy.floor() as i32, c);
        }
    }

    pub fn thick_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, size: i32, c: u32) {
        let lo = -(size / 2);
        let hi = lo + size;
        line_points(x0, y0, x1, y1, |x, y| {
            for dy in lo..hi {
                for dx in lo..hi {
                    self.set(x + dx, y + dy, c);
                }
            }
        });
    }

    pub fn fill_ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
        self.ellipse_span(cx, cy, rx, ry, |f, x0, x1, y| f.hline(x0, x1, y, c));
    }

    pub fn shade_ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, num: u32, den: u32) {
        self.ellipse_span(cx, cy, rx, ry, |f, x0, x1, y| {
            if !(0..f.h).contains(&y) {
                return;
            }
            for x in x0.max(0)..=x1.min(f.w - 1) {
                let i = (y * f.w + x) as usize;
                f.px[i] = shade(f.px[i], num, den);
            }
        });
    }

    fn ellipse_span(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, mut f: impl FnMut(&mut Frame, i32, i32, i32)) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        for dy in -ry..=ry {
            let t = 1.0 - (dy as f32 / (ry as f32 + 0.5)).powi(2);
            if t < 0.0 {
                continue;
            }
            let w = ((rx as f32 + 0.5) * t.sqrt()) as i32;
            f(self, cx - w, cx + w, cy + dy);
        }
    }

    pub fn circle_outline(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        let n = (r * 8).max(16);
        let mut last = (cx + r, cy);
        for i in 1..=n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let p = (cx + (a.cos() * r as f32).round() as i32, cy + (a.sin() * r as f32).round() as i32);
            self.line(last.0, last.1, p.0, p.1, c);
            last = p;
        }
    }

    /// Scanline polygon fill (even-odd).
    pub fn fill_poly(&mut self, pts: &[(i32, i32)], c: u32) {
        if pts.len() < 3 {
            return;
        }
        let miny = pts.iter().map(|p| p.1).min().unwrap().max(0);
        let maxy = pts.iter().map(|p| p.1).max().unwrap().min(self.h - 1);
        let mut xs: Vec<i32> = Vec::with_capacity(8);
        for y in miny..=maxy {
            xs.clear();
            let fy = y as f32 + 0.5;
            for i in 0..pts.len() {
                let (x0, y0) = pts[i];
                let (x1, y1) = pts[(i + 1) % pts.len()];
                let (fy0, fy1) = (y0 as f32, y1 as f32);
                if (fy0 <= fy && fy < fy1) || (fy1 <= fy && fy < fy0) {
                    let t = (fy - fy0) / (fy1 - fy0);
                    xs.push((x0 as f32 + t * (x1 - x0) as f32).round() as i32);
                }
            }
            xs.sort_unstable();
            for pair in xs.chunks(2) {
                if pair.len() == 2 {
                    self.hline(pair[0], pair[1] - 1, y, c);
                }
            }
        }
    }

    /// Vertical gradient made of flat bands with a dithered row between them.
    pub fn band_gradient(&mut self, y0: i32, y1: i32, colors: &[u32]) {
        let n = colors.len() as i32;
        let h = y1 - y0;
        for y in y0..y1 {
            let pos = (y - y0) * n * 4 / h.max(1);
            let band = (pos / 4).min(n - 1);
            let sub = pos % 4;
            let a = colors[band as usize];
            let b = colors[((band + 1).min(n - 1)) as usize];
            for x in 0..self.w {
                let c = if sub == 3 && (x + y) & 1 == 0 { b } else { a };
                self.set(x, y, c);
            }
        }
    }

    pub fn to_rgba(&self, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.px.len() * 4);
        for &c in &self.px {
            out.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
        }
    }

    // ---------------------------------------------------------------- text

    fn text_core(&mut self, x: i32, y: i32, s: &str, bold: bool, mut col: impl FnMut(i32) -> u32) {
        let adv = if bold { 8 } else { 6 };
        let mut cx = x;
        for ch in s.chars() {
            if let Some(g) = glyph(ch.to_ascii_uppercase()) {
                for (gy, row) in g.iter().enumerate() {
                    let c = col(gy as i32);
                    if bold {
                        for (gx, on) in bold_row(row).iter().enumerate() {
                            if *on {
                                self.set(cx + gx as i32, y + gy as i32, c);
                            }
                        }
                    } else {
                        for (gx, b) in row.bytes().enumerate() {
                            if b == b'#' {
                                self.set(cx + gx as i32, y + gy as i32, c);
                            }
                        }
                    }
                }
            }
            cx += adv;
        }
    }

    pub fn text(&mut self, x: i32, y: i32, s: &str, c: u32) {
        self.text_core(x, y, s, false, |_| c);
    }

    pub fn text_bold(&mut self, x: i32, y: i32, s: &str, c: u32) {
        self.text_core(x, y, s, true, |_| c);
    }

    /// Text with a one-pixel drop shadow.
    pub fn text_sh(&mut self, x: i32, y: i32, s: &str, c: u32, sh: u32, bold: bool) {
        self.text_core(x + 1, y + 1, s, bold, |_| sh);
        self.text_core(x, y, s, bold, |_| c);
    }

    /// Text with a full outline and a vertical per-row gradient (very SNES).
    pub fn text_grad(&mut self, x: i32, y: i32, s: &str, grad: &[u32; 7], outline: u32, bold: bool) {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (1, 1), (-1, 1), (1, -1), (-1, -1), (2, 2), (1, 2)] {
            self.text_core(x + dx, y + dy, s, bold, |_| outline);
        }
        self.text_core(x, y, s, bold, |r| grad[r as usize]);
    }

    pub fn text_center(&mut self, cx: i32, y: i32, s: &str, c: u32, sh: u32, bold: bool) {
        let w = text_width(s, bold);
        self.text_sh(cx - w / 2, y, s, c, sh, bold);
    }

    pub fn text_grad_center(&mut self, cx: i32, y: i32, s: &str, grad: &[u32; 7], outline: u32, bold: bool) {
        let w = text_width(s, bold);
        self.text_grad(cx - w / 2, y, s, grad, outline, bold);
    }

    /// Big chunky logo text: italic blocks, thick outline, drop shadow and a
    /// banded gradient fill with a bright top edge.
    pub fn logo(&mut self, cx: i32, y: i32, s: &str, scale: i32, grad: &[u32], outline: u32, shadow: u32) {
        let n = s.chars().count() as i32;
        let adv = 6 * scale;
        let pad = 4;
        let mw = n * adv + 7 * scale / 3 + 1 + pad * 2;
        let mh = 7 * scale + pad * 2;
        let mut m = vec![false; (mw * mh) as usize];
        for (i, ch) in s.chars().enumerate() {
            let Some(g) = glyph(ch.to_ascii_uppercase()) else { continue };
            let on = |x: i32, y: i32| -> bool {
                x >= 0 && y >= 0 && x < 5 && y < 7 && g[y as usize].as_bytes()[x as usize] == b'#'
            };
            let k = (scale + 1) / 2;
            for (gy, row) in g.iter().enumerate() {
                for (gx, b) in row.bytes().enumerate() {
                    if b != b'#' {
                        continue;
                    }
                    let (cx, cy) = (gx as i32, gy as i32);
                    let (u, d, l, r) = (on(cx, cy - 1), on(cx, cy + 1), on(cx - 1, cy), on(cx + 1, cy));
                    let (ul, ur, dl, dr) = (on(cx - 1, cy - 1), on(cx + 1, cy - 1), on(cx - 1, cy + 1), on(cx + 1, cy + 1));
                    // round a corner only where the outline steps diagonally (O, C, S),
                    // so D, B and E keep their square corners
                    // ink two cells away means the gap is a notch (B's waist), not the outside
                    let (uu, dd) = (on(cx, cy - 2), on(cx, cy + 2));
                    let tl = !u && !l && (dl || ur) && !uu;
                    let tr = !u && !r && (dr || ul) && !uu;
                    let bl = !d && !l && (ul || dr) && !dd;
                    let br = !d && !r && (ur || dl) && !dd;
                    for by in 0..scale {
                        // italic: shift every pixel row, so strokes stay connected
                        let shear = (7 * scale - (gy as i32 * scale + by)) / 3;
                        for bx in 0..scale {
                            let (ix, iy) = (scale - 1 - bx, scale - 1 - by);
                            if (tl && bx + by < k) || (tr && ix + by < k) || (bl && bx + iy < k) || (br && ix + iy < k) {
                                continue;
                            }
                            let px = pad + i as i32 * adv + gx as i32 * scale + bx + shear;
                            let py = pad + gy as i32 * scale + by;
                            m[(py * mw + px) as usize] = true;
                        }
                    }
                }
            }
        }
        let at = |x: i32, y: i32| x >= 0 && y >= 0 && x < mw && y < mh && m[(y * mw + x) as usize];
        let ox = cx - mw / 2;
        // outline mask: dilate by 2 (by 1 for small text)
        let reach = if scale <= 2 { 2 } else { 5 };
        let so = if scale <= 2 { 1 } else { 3 };
        let mut o = vec![false; (mw * mh) as usize];
        for py in 0..mh {
            for px in 0..mw {
                'n: for dy in -2..=2 {
                    for dx in -2..=2 {
                        if (dx * dx + dy * dy) <= reach && at(px + dx, py + dy) {
                            o[(py * mw + px) as usize] = true;
                            break 'n;
                        }
                    }
                }
            }
        }
        for py in 0..mh {
            for px in 0..mw {
                if o[(py * mw + px) as usize] {
                    self.set(ox + px + so, y + py + so, shadow);
                }
            }
        }
        for py in 0..mh {
            for px in 0..mw {
                if o[(py * mw + px) as usize] {
                    self.set(ox + px, y + py, outline);
                }
            }
        }
        let gl = grad.len() as i32;
        for py in 0..mh {
            for px in 0..mw {
                if !at(px, py) {
                    continue;
                }
                let gi = ((py - pad) * gl / (7 * scale)).clamp(0, gl - 1);
                let mut c = grad[gi as usize];
                if !at(px, py - 1) {
                    c = mix(c, 0xFFFFFF, 0.6);
                } else if !at(px, py + 1) || !at(px + 1, py) {
                    c = shade(c, 3, 4);
                }
                self.set(ox + px, y + py, c);
            }
        }
    }
}

/// Liang-Barsky clip of a segment to [-1, w+1] x [-1, h+1].
pub fn clip(x0: f64, y0: f64, x1: f64, y1: f64, w: f64, h: f64) -> Option<(f64, f64, f64, f64)> {
    if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
        return None;
    }
    let (dx, dy) = (x1 - x0, y1 - y0);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, x0 + 1.0), (dx, w + 1.0 - x0), (-dy, y0 + 1.0), (dy, h + 1.0 - y0)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t1 {
                    return None;
                }
                t0 = t0.max(r);
            } else {
                if r < t0 {
                    return None;
                }
                t1 = t1.min(r);
            }
        }
    }
    Some((x0 + dx * t0, y0 + dy * t0, x0 + dx * t1, y0 + dy * t1))
}

/// 4x4 Bayer threshold in 0..1 for ordered dithering.
#[inline]
pub fn bayer(x: i32, y: i32) -> f32 {
    const M: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    (M[((y & 3) * 4 + (x & 3)) as usize] as f32 + 0.5) / 16.0
}

pub fn text_width(s: &str, bold: bool) -> i32 {
    let n = s.chars().count() as i32;
    if n == 0 {
        0
    } else {
        n * if bold { 8 } else { 6 } - 1
    }
}

/// Widen a 5-pixel glyph row to 7 for the bold font: the outer columns get
/// 2px stems, lone middle strokes get thickened, and 1px gaps stay open.
fn bold_row(row: &str) -> [bool; 7] {
    let b: Vec<bool> = row.bytes().map(|c| c == b'#').collect();
    let mut m = [b[0], b[0], b[1], b[2], b[3], b[4], b[4]];
    // thicken lone strokes outward (left of centre to the left), so V and Y stay symmetric
    let src = m;
    for i in 2..=4 {
        if !src[i] || src[i - 1] || src[i + 1] {
            continue;
        }
        if i == 2 {
            if !src[0] {
                m[1] = true;
            }
        } else if i == 3 {
            // centre stroke: grow both ways so it stays centred
            if !src[1] && !src[5] {
                m[2] = true;
                m[4] = true;
            }
        } else if i + 2 >= 7 || !src[i + 2] {
            m[i + 1] = true;
        }
    }
    m
}

pub fn glyph(c: char) -> Option<[&'static str; 7]> {
    Some(match c {
        'A' => [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'B' => ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
        'C' => [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
        'D' => ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
        'E' => ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
        'F' => ["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
        'G' => [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####"],
        'H' => ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'I' => [".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###."],
        'J' => ["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."],
        'K' => ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
        'L' => ["#....", "#....", "#....", "#....", "#....", "#....", "#####"],
        'M' => ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
        'N' => ["#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#"],
        'O' => [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'P' => ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
        'Q' => [".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"],
        'R' => ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
        'S' => [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
        'T' => ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
        'U' => ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'V' => ["#...#", "#...#", "#...#", ".#.#.", ".#.#.", ".#.#.", "..#.."],
        'W' => ["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#."],
        'X' => ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"],
        'Y' => ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
        'Z' => ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
        '0' => [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        '1' => ["..#..", ".##..", "#.#..", "..#..", "..#..", "..#..", "#####"],
        '2' => [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"],
        '3' => ["#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###."],
        '4' => ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."],
        '5' => ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
        '6' => ["..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."],
        '7' => ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."],
        '8' => [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."],
        '9' => [".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."],
        '!' => ["..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#.."],
        '?' => [".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."],
        '.' => [".....", ".....", ".....", ".....", ".....", ".##..", ".##.."],
        ',' => [".....", ".....", ".....", ".....", ".##..", "..#..", ".#..."],
        '\'' => ["..#..", "..#..", ".#...", ".....", ".....", ".....", "....."],
        '-' => [".....", ".....", ".....", ".###.", ".....", ".....", "....."],
        ':' => [".....", ".##..", ".##..", ".....", ".##..", ".##..", "....."],
        '/' => ["....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#...."],
        '(' => ["...#.", "..#..", ".#...", ".#...", ".#...", "..#..", "...#."],
        ')' => [".#...", "..#..", "...#.", "...#.", "...#.", "..#..", ".#..."],
        '+' => [".....", "..#..", "..#..", "#####", "..#..", "..#..", "....."],
        '=' => [".....", ".....", "#####", ".....", "#####", ".....", "....."],
        '>' => [".#...", "..#..", "...#.", "....#", "...#.", "..#..", ".#..."],
        '<' => ["...#.", "..#..", ".#...", "#....", ".#...", "..#..", "...#."],
        '*' => [".....", "#.#.#", ".###.", "#####", ".###.", "#.#.#", "....."],
        '%' => ["##..#", "##..#", "...#.", "..#..", ".#...", "#..##", "#..##"],
        '#' => [".#.#.", ".#.#.", "#####", ".#.#.", "#####", ".#.#.", ".#.#."],
        '[' => [".###.", ".#...", ".#...", ".#...", ".#...", ".#...", ".###."],
        ']' => [".###.", "...#.", "...#.", "...#.", "...#.", "...#.", ".###."],
        '_' => [".....", ".....", ".....", ".....", ".....", ".....", "#####"],
        '|' => ["..#..", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
        '"' => [".#.#.", ".#.#.", ".....", ".....", ".....", ".....", "....."],
        '^' => ["..#..", ".#.#.", "#...#", ".....", ".....", ".....", "....."],
        '~' => [".....", ".....", ".#...", "#.#.#", "...#.", ".....", "....."],
        '&' => [".##..", "#..#.", "#.#..", ".#...", "#.#.#", "#..#.", ".##.#"],
        ' ' => [".....", ".....", ".....", ".....", ".....", ".....", "....."],
        _ => return None,
    })
}
