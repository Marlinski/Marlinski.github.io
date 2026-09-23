//! The photo from the website, as dots.
//!
//! Braille is the only block in Unicode that addresses individual dots, so a
//! cell carries a 2x4 grid rather than one character's worth of ink: the
//! picture gets eight times the resolution the terminal nominally has, which
//! is the difference between a portrait and a smudge.
//!
//! The dots are drawn in the theme's foreground, so the subject is light on
//! dark under DARK and dark on light under DOS, and reads the same either way.

use image::GrayImage;

/// Dot bit values inside a braille cell, by (row, column).
///
/// The layout is historical, not sequential: the first six dots were numbered
/// down the left column then down the right, and dots 7 and 8 were added
/// underneath afterwards, which is why the bottom row jumps to 0x40/0x80.
const BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

/// Otsu's method: the threshold that best splits the histogram into two
/// groups. Rather than a number tuned to one photograph, this reads whatever
/// the website is showing today and finds its own dividing line.
fn otsu(img: &GrayImage) -> u8 {
    let mut hist = [0u32; 256];
    for p in img.pixels() {
        hist[p.0[0] as usize] += 1;
    }
    let total: u32 = img.width() * img.height();
    let sum_all: f64 = (0..256).map(|i| i as f64 * hist[i] as f64).sum();
    let (mut sum_b, mut w_b, mut best, mut best_var) = (0.0f64, 0u32, 128u8, -1.0f64);
    for t in 0..256 {
        w_b += hist[t];
        if w_b == 0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0 {
            break;
        }
        sum_b += t as f64 * hist[t] as f64;
        let m_b = sum_b / w_b as f64;
        let m_f = (sum_all - sum_b) / w_f as f64;
        let var = w_b as f64 * w_f as f64 * (m_b - m_f) * (m_b - m_f);
        if var > best_var {
            best_var = var;
            best = t as u8;
        }
    }
    best
}

/// Renders the image as braille, `cells` characters wide.
///
/// A braille cell is 2 dots across and 4 down, while a terminal character is
/// about twice as tall as it is wide — so the dots come out square, and a
/// square picture needs half as many rows as columns.
pub fn dots(img: &GrayImage, cells: usize) -> Vec<String> {
    let cells = cells.clamp(12, 40);
    let px_w = (cells * 2) as u32;
    // Keep the photo's own proportions rather than assuming it is square.
    let px_h = ((px_w as f64) * img.height() as f64 / img.width() as f64 / 4.0).round() as u32 * 4;
    let px_h = px_h.max(4);

    let small = image::imageops::resize(img, px_w, px_h, image::imageops::FilterType::Lanczos3);
    let t = otsu(&small);

    let mut out = Vec::new();
    for cy in (0..px_h).step_by(4) {
        let mut line = String::new();
        for cx in (0..px_w).step_by(2) {
            let mut v = 0u8;
            for dy in 0..4u32 {
                for dx in 0..2u32 {
                    // Dark pixels are the ink: the subject is darker than the
                    // sky behind it, and the sky should stay empty.
                    if small.get_pixel(cx + dx, cy + dy).0[0] < t {
                        v |= BITS[dy as usize][dx as usize];
                    }
                }
            }
            line.push(char::from_u32(0x2800 + v as u32).unwrap_or(' '));
        }
        out.push(line);
    }
    out
}

/// Decodes whatever the website serves. Any failure is a missing picture, not
/// an error screen — the ABOUT pane reads perfectly well without it.
pub fn decode(bytes: &[u8]) -> Option<GrayImage> {
    image::load_from_memory(bytes).ok().map(|i| i.to_luma8())
}
