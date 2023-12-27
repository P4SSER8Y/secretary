use std::collections::HashMap;

use crate::qweather;

use super::shared::*;
use anyhow::{anyhow, Result};
use chrono::Datelike;
use image::{GrayImage, Luma};
use imageproc::{
    drawing::{self, draw_filled_rect_mut},
    rect::Rect,
};
use once_cell::sync::Lazy;
use rusttype::Scale;

fn get_day(day: u32) -> String {
    static MAP: Lazy<HashMap<u32, &'static str>> = Lazy::new(|| {
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
        return map;
    });
    if (day == 10) || (day == 20) || (day == 30) {
        return format!("{}{}", MAP.get(&(day / 10)).unwrap(), MAP.get(&10).unwrap());
    } else {
        return format!(
            "{}{}",
            MAP.get(&(day / 10 * 10)).unwrap(),
            MAP.get(&(day % 10)).unwrap()
        );
    }
}

static MAP_WEEKDAY: Lazy<HashMap<u32, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(0, "日");
    map.insert(1, "一");
    map.insert(2, "二");
    map.insert(3, "三");
    map.insert(4, "四");
    map.insert(5, "五");
    map.insert(6, "六");
    map.insert(7, "日");
    return map;
});

pub fn generate(context: &Context) -> Result<GrayImage> {
    let mut img = GrayImage::new(600, 800);

    let now = context.now.ok_or(anyhow!("time not provided"))?;
    let font = context
        .fonts
        .get("main")
        .ok_or(anyhow!("main font not found"))?;

    let rect = Rect::at(0, 0).of_size(img.width(), img.height());
    drawing::draw_filled_rect_mut(&mut img, rect, Luma([255]));

    let day_name = get_day(now.day());
    draw_aligned_text(
        &mut img,
        Luma([0]),
        (300, 50),
        Scale::uniform(300.0),
        font,
        &day_name,
        (AlignHorizontal::Center, AlignVertical::Top),
    );

    let weekday_name = MAP_WEEKDAY
        .get(&now.weekday().number_from_monday())
        .unwrap();
    let size = 72;
    let center = (600 - 100, size / 2);
    draw_filled_rect_mut(
        &mut img,
        Rect::at(center.0 - size / 2, center.1 - size / 2)
            .of_size(size.try_into().unwrap(), size.try_into().unwrap()),
        Luma([0]),
    );
    draw_aligned_text(
        &mut img,
        Luma([255]),
        (center.0 - 2, center.1 - 6),
        Scale::uniform(size as f32),
        font,
        &weekday_name,
        (AlignHorizontal::Center, AlignVertical::Center),
    );

    if let Ok(forcast) = qweather::get_24h_forcast() {
        draw_aligned_text(
            &mut img,
            Luma([128]),
            (300, 750),
            Scale::uniform(72.0),
            font,
            &format!("{}~{}", forcast.min_temp, forcast.max_temp),
            (AlignHorizontal::Center, AlignVertical::Bottom),
        );
    }

    return Ok(img);
}
