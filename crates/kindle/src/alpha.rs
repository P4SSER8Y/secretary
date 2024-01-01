use std::collections::HashMap;
use chrono::Datelike;
use image::{GrayImage, Luma};
use imageproc::{rect::Rect, drawing};
use once_cell::sync::Lazy;
use rusttype::Scale;
use anyhow::{anyhow, Result};
use super::shared::*;

static MAP_WEEKDAY: Lazy<HashMap<u8, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(0, "周日");
    map.insert(1, "周一");
    map.insert(2, "周二");
    map.insert(3, "周三");
    map.insert(4, "周四");
    map.insert(5, "周五");
    map.insert(6, "周六");
    map.insert(7, "周日");
    map
});

pub fn generate(context: &Context) -> Result<GrayImage> {
    let battery = context.battery;
    let now = context.now.ok_or(anyhow!("time not provided"))?;
    let mut img = GrayImage::new(600, 800);
    let font = get_font("main").ok_or(anyhow!("main font not found"))?;

    let rect = Rect::at(0, 0).of_size(img.width(), img.height());
    drawing::draw_filled_rect_mut(&mut img, rect, Luma([255]));

    let text = now.format("%d").to_string();
    draw_aligned_text(
        &mut img,
        Luma([0]),
        (300, 150),
        Scale::uniform(450.0),
        font,
        &text,
        (AlignHorizontal::Center, AlignVertical::Center),
    );
    draw_aligned_text(
        &mut img,
        Luma([0]),
        (300, 400),
        Scale::uniform(150.0),
        font,
        MAP_WEEKDAY
            .get(&(now.weekday().number_from_monday() as u8))
            .expect("?"),
        (AlignHorizontal::Center, AlignVertical::Center),
    );

    if let Some(battery) = battery {
        let battery = format!("电量：{:02}%", battery);
        draw_aligned_text(
            &mut img,
            Luma([0]),
            (600 - 25, 20),
            Scale::uniform(42.0),
            font,
            &battery,
            (AlignHorizontal::Right, AlignVertical::Top),
        );
    }

    draw_aligned_text(
        &mut img,
        Luma([128]),
        (300, 800 - 20),
        Scale::uniform(18.0),
        font,
        &format!("更新：{}", now.format("%Y-%m-%d %H:%M:%S")),
        (AlignHorizontal::Center, AlignVertical::Bottom),
    );

    return Ok(img);
}