use std::collections::HashMap;

use chrono::Datelike;
use image::{GrayImage, Luma};
use imageproc::{
    drawing::{self, draw_filled_rect_mut},
    rect::Rect,
};
use once_cell::sync::Lazy;
use rusttype::Scale;

use crate::qweather;

use super::shared::*;
use anyhow::{anyhow, Result};

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
    let font = context
        .fonts
        .get("main")
        .ok_or(anyhow!("main font not found"))?;
    let now = context.now.ok_or(anyhow!("time not provided"))?;
    static MAP: Lazy<HashMap<u8, &'static str>> = Lazy::new(|| {
        let mut map = HashMap::new();
        map.insert(0, "〇");
        map.insert(1, "一");
        map.insert(2, "二");
        map.insert(3, "三");
        map.insert(4, "四");
        map.insert(5, "五");
        map.insert(6, "六");
        map.insert(7, "七");
        map.insert(8, "八");
        map.insert(9, "九");
        map.insert(10, "十");
        map.insert(20, "廿");
        map.insert(30, "卅");
        map.insert(40, "卌");
        map
    });
    let mut img = GrayImage::new(600, 800);

    let rect = Rect::at(0, 0).of_size(img.width(), img.height());
    drawing::draw_filled_rect_mut(&mut img, rect, Luma([255]));

    let font_scale = Scale::uniform(375.0);
    let color = Luma([0]);
    let (base_x, base_y) = (40, 320);
    let (ch0, ch1) = if now.day() % 10 == 0 {
        (
            MAP.get(&((now.day() / 10) as u8)).unwrap(),
            MAP.get(&(10)).unwrap(),
        )
    } else if now.day() < 10 {
        (MAP.get(&((now.day() % 10) as u8)).unwrap(), &"号")
    } else {
        (
            MAP.get(&((now.day() / 10 * 10) as u8)).unwrap(),
            MAP.get(&((now.day() % 10) as u8)).unwrap(),
        )
    };
    draw_aligned_text(
        &mut img,
        color,
        (base_x, base_y - 150),
        font_scale,
        font,
        ch0,
        (AlignHorizontal::Left, AlignVertical::Center),
    );
    draw_aligned_text(
        &mut img,
        color,
        (base_x, base_y + 150),
        font_scale,
        font,
        ch1,
        (AlignHorizontal::Left, AlignVertical::Center),
    );
    draw_aligned_text(
        &mut img,
        color,
        (600 - 40, 60),
        Scale::uniform(150.0),
        font,
        &format!(
            "{}",
            MAP_WEEKDAY
                .get(&(now.weekday().num_days_from_sunday() as u8))
                .unwrap()
        ),
        (AlignHorizontal::Right, AlignVertical::Top),
    );

    draw_filled_rect_mut(&mut img, Rect::at(0, 0).of_size(600, 40), Luma([0]));
    let status_scale = Scale::uniform(36.0);
    if let Some(battery) = context.battery {
        let font = context
            .fonts
            .get("status")
            .ok_or(anyhow!("status font not found"))?;
        draw_aligned_text(
            &mut img,
            Luma([255]),
            (600 - 25, 0),
            status_scale,
            font,
            &format!("Battery: {}%", battery),
            (AlignHorizontal::Right, AlignVertical::Top),
        );
    }
    {
        let font = context
            .fonts
            .get("status")
            .ok_or(anyhow!("status font not found"))?;
        draw_aligned_text(
            &mut img,
            Luma([255]),
            (0 + 25, 0),
            status_scale,
            font,
            &format!("Update: {}", now.format("%H:%M:%S")),
            (AlignHorizontal::Left, AlignVertical::Top),
        );
    }

    if let Ok(forecast) = qweather::get_3d_forecast() {
        if forecast.len() == 3 {
            let font = context
                .fonts
                .get("weather")
                .ok_or(anyhow!("main font not found"))?;
            let y = 725;
            let x = vec![100, 300, 500];
            let color = Luma([96]);
            for i in 0..3 {
                draw_aligned_text(
                    &mut img,
                    color,
                    (x[i], y),
                    Scale::uniform(72.0),
                    font,
                    &forecast[i].text,
                    (AlignHorizontal::Center, AlignVertical::Bottom),
                );
                draw_aligned_text(
                    &mut img,
                    color,
                    (x[i], y),
                    Scale::uniform(48.0),
                    font,
                    &format!("{}-{}", forecast[i].temp_min, forecast[i].temp_max),
                    (AlignHorizontal::Center, AlignVertical::Top),
                );
            }
        }
    }

    return Ok(img);
}
