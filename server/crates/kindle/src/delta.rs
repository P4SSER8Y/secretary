use super::shared::*;
use anyhow::{anyhow, Result};
use image::{GrayImage, Luma};
use imageproc::drawing::draw_filled_rect_mut;
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

    Ok(img)
}
