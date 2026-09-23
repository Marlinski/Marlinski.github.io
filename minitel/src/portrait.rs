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

/// A 3x3 median, which removes single-pixel speckle without softening an
/// edge the way a blur would: the contour stays where it is, the grain in the
/// water behind the subject goes.
fn median3(img: &GrayImage) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let mut out = GrayImage::new(w, h);
    let mut win = [0u8; 9];
    for y in 0..h {
        for x in 0..w {
            let mut n = 0;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                        win[n] = img.get_pixel(nx as u32, ny as u32).0[0];
                        n += 1;
                    }
                }
            }
            win[..n].sort_unstable();
            out.put_pixel(x, y, image::Luma([win[n / 2]]));
        }
    }
    out
}

/// Drops the islands, keeping the subject.
///
/// Thresholding a photograph taken outdoors leaves crumbs: dark patches of
/// water read as ink and scatter around the contour. They are never attached
/// to the subject, so the size of the blob they belong to tells them apart —
/// here anything under an eighth of the largest blob. A fraction rather than
/// a pixel count, so it means the same thing at any size, and so a subject
/// that happens to be split in two survives.
fn keep_subject(ink: &mut [bool], w: usize, h: usize) {
    let mut seen = vec![false; ink.len()];
    let mut blobs: Vec<Vec<usize>> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();

    for start in 0..ink.len() {
        if !ink[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut blob = Vec::new();
        while let Some(p) = stack.pop() {
            blob.push(p);
            let (px, py) = ((p % w) as i32, (p / w) as i32);
            // Eight-connected: a contour that steps diagonally is still one
            // piece, and counting it as two would split the subject.
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (nx, ny) = (px + dx, py + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let n = ny as usize * w + nx as usize;
                    if ink[n] && !seen[n] {
                        seen[n] = true;
                        stack.push(n);
                    }
                }
            }
        }
        blobs.push(blob);
    }

    let Some(biggest) = blobs.iter().map(|b| b.len()).max() else {
        return;
    };
    for blob in blobs {
        if blob.len() * 8 < biggest {
            for p in blob {
                ink[p] = false;
            }
        }
    }
}

/// Spreads the subject's tones across the full range before dithering.
///
/// A face lit from the side is dark almost everywhere — here it occupies
/// roughly the bottom two thirds of the scale — so dithering it directly
/// makes one flat black mass. Stretching first turns that narrow band into
/// actual light and shade. Percentiles rather than the outright darkest and
/// lightest pixel, so a single blown highlight cannot flatten everything else
/// back down, and measured only inside the subject, because the water behind
/// it is not what we are exposing for.
fn stretch(gray: &[u8], mask: &[bool]) -> Vec<u8> {
    let mut vals: Vec<u8> = gray.iter().zip(mask).filter(|(_, m)| **m).map(|(g, _)| *g).collect();
    if vals.is_empty() {
        return gray.to_vec();
    }
    vals.sort_unstable();
    let lo = vals[vals.len() * 2 / 100] as f32;
    let hi = vals[(vals.len() * 98 / 100).min(vals.len() - 1)] as f32;
    let span = (hi - lo).max(1.0);
    gray.iter()
        .map(|g| (((*g as f32 - lo) / span) * 255.0).clamp(0.0, 255.0) as u8)
        .collect()
}

/// Atkinson dithering, inside the subject only.
///
/// Error diffusion turns shades into densities of dots, which is the only way
/// a two-colour picture shows a cheekbone. Atkinson's, from the early
/// Macintosh, passes on just six eighths of each pixel's error rather than
/// all of it: highlights stay clean and shadows stay solid instead of
/// everything drifting to a speckled middle grey. Floyd-Steinberg keeps all
/// the error and, on this photograph, reads as noise rather than as a face.
///
/// The error stays inside the mask, so the clean outline survives the
/// texture: nothing leaks into the water.
fn atkinson(tone: &[u8], mask: &[bool], w: usize, h: usize) -> Vec<bool> {
    let mut buf: Vec<f32> = tone.iter().map(|v| *v as f32).collect();
    let mut ink = vec![false; w * h];
    const SPREAD: [(i32, i32); 6] = [(1, 0), (2, 0), (-1, 1), (0, 1), (1, 1), (0, 2)];

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !mask[i] {
                continue;
            }
            let old = buf[i];
            let dark = old < 128.0;
            ink[i] = dark;
            let err = (old - if dark { 0.0 } else { 255.0 }) / 8.0;
            for (dx, dy) in SPREAD {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let n = ny as usize * w + nx as usize;
                if mask[n] {
                    buf[n] += err;
                }
            }
        }
    }
    ink
}

/// Renders the image as braille, `cells` characters wide.
///
/// A braille cell is 2 dots across and 4 down, while a terminal character is
/// about twice as tall as it is wide — so the dots come out square, and a
/// square picture needs half as many rows as columns.
pub fn dots(img: &GrayImage, cells: usize) -> Vec<String> {
    // Detail needs pixels: a dithered face at 40 cells is a smudge, and each
    // cell buys two dots across and four down.
    let cells = cells.clamp(12, 48);
    let px_w = (cells * 2) as u32;
    // Keep the photo's own proportions rather than assuming it is square.
    let px_h = ((px_w as f64) * img.height() as f64 / img.width() as f64 / 4.0).round() as u32 * 4;
    let px_h = px_h.max(4);

    let small = image::imageops::resize(img, px_w, px_h, image::imageops::FilterType::Lanczos3);
    let (w, h) = (px_w as usize, px_h as usize);

    // Two passes over the same photograph. The median'd copy decides where
    // the subject is — a clean silhouette, free of the speckle the water
    // leaves behind. The original decides what is inside it, because the
    // median would have smoothed away the very detail we are after.
    let smoothed = median3(&small);
    let t = otsu(&smoothed);
    let mut mask = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            // Dark pixels are the subject: it is darker than the water.
            mask[y * w + x] = smoothed.get_pixel(x as u32, y as u32).0[0] < t;
        }
    }
    keep_subject(&mut mask, w, h);

    let gray: Vec<u8> = (0..w * h)
        .map(|i| small.get_pixel((i % w) as u32, (i / w) as u32).0[0])
        .collect();
    let ink = atkinson(&stretch(&gray, &mask), &mask, w, h);

    let mut out = Vec::new();
    for cy in (0..h).step_by(4) {
        let mut line = String::new();
        for cx in (0..w).step_by(2) {
            let mut v = 0u8;
            for dy in 0..4 {
                for dx in 0..2 {
                    if ink[(cy + dy) * w + cx + dx] {
                        v |= BITS[dy][dx];
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
