use super::shared::*;
use anyhow::{anyhow, Result};
use image::{GrayImage, Luma};
use imageproc::drawing::{self, draw_filled_rect_mut};
use imageproc::rect::Rect;
use log::info;

pub async fn generate(context: &Context) -> Result<GrayImage> {
    let path = context
        .dashboard_path
        .as_deref()
        .ok_or(anyhow!("dashboard_path not provided"))?;

    info!("loading dashboard from: {}", path);
    let img = image::open(path)
        .map_err(|e| anyhow!("failed to load dashboard image: {}", e))?;
    let gray = img.to_luma8();

    // Ensure target size 600x800 (center crop or pad)
    let img = if gray.width() != 600 || gray.height() != 800 {
        let mut canvas = GrayImage::new(600, 800);
        draw_filled_rect_mut(
            &mut canvas,
            Rect::at(0, 0).of_size(600, 800),
            Luma([255]),
        );
        let ox = (600i32 - gray.width() as i32) / 2;
        let oy = (800i32 - gray.height() as i32) / 2;
        for y in 0..gray.height().min(800) {
            for x in 0..gray.width().min(600) {
                let px = gray.get_pixel(x, y);
                canvas.put_pixel((ox + x as i32) as u32, (oy + y as i32) as u32, *px);
            }
        }
        canvas
    } else {
        gray
    };

/// Apply 16-level grayscale with Floyd-Steinberg dithering + contrast enhancement.
/// Returns the processed image ready for e-ink display, no overlays.
    let mut img = auto_contrast(&img, 3);
    // Step 2: Boost contrast
    img = contrast_boost(&img, 1.8);
    // Step 3: Custom S-curve - darks darker, lights lighter
    img = s_curve(&img);
    // Step 4: Sharpen
    img = sharpen(&img);
    // Step 5: Floyd-Steinberg dithering to 16 levels (4-bit)
    img = dither_16level(&img);

    Ok(img)
}

/// Auto contrast stretch: map pixel range to full 0-255, clipping given % from each end.
fn auto_contrast(img: &GrayImage, clip_pct: u8) -> GrayImage {
    let mut hist = [0u32; 256];
    for p in img.pixels() {
        hist[p.0[0] as usize] += 1;
    }
    let total = (img.width() * img.height()) as u32;
    let clip = total * clip_pct as u32 / 100;
    let mut min: u8 = 0;
    let mut acc = 0u32;
    for i in 0..256 {
        acc += hist[i];
        if acc > clip {
            min = i as u8;
            break;
        }
    }
    let mut max: u8 = 255;
    acc = 0;
    for i in (0..256).rev() {
        acc += hist[i];
        if acc > clip {
            max = i as u8;
            break;
        }
    }
    if max <= min {
        return img.clone();
    }
    let range = (max - min) as f32;
    let mut out = GrayImage::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let v = (p.0[0].saturating_sub(min)) as f32 / range;
        out.put_pixel(x, y, Luma([(v * 255.0) as u8]));
    }
    out
}

/// Boost contrast by scaling around midpoint.
fn contrast_boost(img: &GrayImage, factor: f32) -> GrayImage {
    let mut out = GrayImage::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let v = p.0[0] as f32;
        let v = ((v - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8;
        out.put_pixel(x, y, Luma([v]));
    }
    out
}

/// S-curve: push darks darker, lights lighter.
fn s_curve(img: &GrayImage) -> GrayImage {
    let mut out = GrayImage::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let v = p.0[0] as f32;
        let v = if v < 128.0 {
            (v * 0.80).clamp(0.0, 255.0)
        } else {
            (128.0 + (v - 128.0) * 1.25).clamp(0.0, 255.0)
        };
        out.put_pixel(x, y, Luma([v as u8]));
    }
    out
}

/// Simple sharpen: laplacian-like unsharp mask.
fn sharpen(img: &GrayImage) -> GrayImage {
    let mut out = GrayImage::new(img.width(), img.height());
    for y in 1..img.height() - 1 {
        for x in 1..img.width() - 1 {
            let center = img.get_pixel(x, y).0[0] as i32;
            let sum: i32 = [
                img.get_pixel(x - 1, y - 1).0[0] as i32,
                img.get_pixel(x, y - 1).0[0] as i32,
                img.get_pixel(x + 1, y - 1).0[0] as i32,
                img.get_pixel(x - 1, y).0[0] as i32,
                img.get_pixel(x + 1, y).0[0] as i32,
                img.get_pixel(x - 1, y + 1).0[0] as i32,
                img.get_pixel(x, y + 1).0[0] as i32,
                img.get_pixel(x + 1, y + 1).0[0] as i32,
            ]
            .iter()
            .sum();
            let avg = sum / 8;
            let sharp = (center + (center - avg)).clamp(0, 255) as u8;
            out.put_pixel(x, y, Luma([sharp]));
        }
    }
    // Copy edges
    for x in 0..img.width() {
        out.put_pixel(x, 0, *img.get_pixel(x, 0));
        out.put_pixel(x, img.height() - 1, *img.get_pixel(x, img.height() - 1));
    }
    for y in 0..img.height() {
        out.put_pixel(0, y, *img.get_pixel(0, y));
        out.put_pixel(img.width() - 1, y, *img.get_pixel(img.width() - 1, y));
    }
    out
}

/// Floyd-Steinberg dither to 16 levels (4-bit).
fn dither_16level(img: &GrayImage) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let mut pixels: Vec<Vec<f32>> = (0..h)
        .map(|y| (0..w).map(|x| img.get_pixel(x, y).0[0] as f32).collect())
        .collect();

    for y in 0..h {
        for x in 0..w {
            let old = pixels[y as usize][x as usize];
            // 16 levels: step=16 → 0, 16, 32, ..., 240, 255
            let new = (old / 16.0).round() * 16.0;
            let new = new.clamp(0.0, 255.0);
            pixels[y as usize][x as usize] = new;
            let err = old - new;

            if x + 1 < w {
                pixels[y as usize][x as usize + 1] += err * 7.0 / 16.0;
            }
            if y + 1 < h {
                if x > 0 {
                    pixels[y as usize + 1][x as usize - 1] += err * 3.0 / 16.0;
                }
                pixels[y as usize + 1][x as usize] += err * 5.0 / 16.0;
                if x + 1 < w {
                    pixels[y as usize + 1][x as usize + 1] += err * 1.0 / 16.0;
                }
            }
        }
    }

    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let v = pixels[y as usize][x as usize].clamp(0.0, 255.0) as u8;
            out.put_pixel(x, y, Luma([v]));
        }
    }
    out
}
