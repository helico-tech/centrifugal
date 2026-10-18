//! Write frames to PNG, for headless screenshots.

use super::gfx::Frame;
use std::path::Path;

pub fn save_png(f: &Frame, path: &Path, scale: usize) {
    let (fw, fh) = (f.w as usize, f.h as usize);
    let (w, h) = (fw * scale, fh * scale);
    let mut data = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let c = f.px[(y / scale) * fw + x / scale];
            let i = (y * w + x) * 3;
            data[i] = (c >> 16) as u8;
            data[i + 1] = (c >> 8) as u8;
            data[i + 2] = c as u8;
        }
    }
    write_rgb(path, w, h, &data);
}

pub fn write_rgb(path: &Path, w: usize, h: usize, data: &[u8]) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(data).unwrap();
}
