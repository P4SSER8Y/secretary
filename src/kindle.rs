use once_cell::sync::{Lazy, OnceCell};
use rocket::info;
use rusttype::{Font, Scale};
use std::collections::HashMap;
use std::io::Cursor;

use chrono::{self, Datelike};
use image::{GrayImage, Luma};
use imageproc::drawing::{self};
use imageproc::rect::Rect;
use rocket::{http::ContentType, Build, Rocket};

static FONT_MAIN: OnceCell<Font> = OnceCell::new();

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

pub fn build(base: &'static str, build: Rocket<Build>) -> Rocket<Build> {
    FONT_MAIN.get_or_init(|| {
        let path = build
            .figment()
            .find_value("kindle.fonts.main")
            .expect("kindle.fonts.main not found")
            .into_string()
            .expect("cannot parse kindle.fonts.main");
        info!("loading {}", path);
        let data = std::fs::read(&path).expect(&format!("failed to load {}", path));
        Font::try_from_vec(data).unwrap_or_else(|| {
            panic!("cannot load {}", path);
        })
    });

    build.mount(base, routes![main])
}

#[allow(dead_code)]
enum AlignHorizontal {
    Left,
    Center,
    Right,
}

#[allow(dead_code)]
enum AlignVertical {
    Top,
    Center,
    Bottom,
}

fn draw_aligned_text<'a>(
    canvas: &'a mut GrayImage,
    color: Luma<u8>,
    base: (i32, i32),
    scale: Scale,
    font: &'a Font<'a>,
    text: &'a str,
    align: (AlignHorizontal, AlignVertical),
) {
    let size = drawing::text_size(scale, font, &text);
    let x = match align.0 {
        AlignHorizontal::Left => base.0,
        AlignHorizontal::Center => base.0 - size.0 / 2,
        AlignHorizontal::Right => base.0 - size.0,
    };
    let y = match align.1 {
        AlignVertical::Top => base.1,
        AlignVertical::Center => base.1 - size.1 / 2,
        AlignVertical::Bottom => base.1 - size.1,
    };
    drawing::draw_text_mut(canvas, color, x, y, scale, font, &text);
}

#[get("/?<battery>")]
fn main(battery: Option<usize>) -> (ContentType, Vec<u8>) {
    let now = chrono::Local::now();
    info!("now={:?}", now);
    info!("battery={:?}", battery);

    let mut img = GrayImage::new(600, 800);

    let rect = Rect::at(0, 0).of_size(img.width(), img.height());
    drawing::draw_filled_rect_mut(&mut img, rect, Luma([255]));

    let text = now.format("%d").to_string();
    draw_aligned_text(
        &mut img,
        Luma([0]),
        (300, 150),
        Scale::uniform(450.0),
        FONT_MAIN.get().unwrap(),
        &text,
        (AlignHorizontal::Center, AlignVertical::Center),
    );
    draw_aligned_text(
        &mut img,
        Luma([0]),
        (300, 400),
        Scale::uniform(150.0),
        FONT_MAIN.get().unwrap(),
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
            FONT_MAIN.get().unwrap(),
            &battery,
            (AlignHorizontal::Right, AlignVertical::Top),
        );
    }

    draw_aligned_text(
        &mut img,
        Luma([128]),
        (300, 800 - 20),
        Scale::uniform(18.0),
        FONT_MAIN.get().unwrap(),
        &format!("更新：{}", now.format("%Y-%m-%d %H:%M:%S")),
        (AlignHorizontal::Center, AlignVertical::Bottom),
    );

    let mut buffer: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut buffer), image::ImageOutputFormat::Png)
        .expect("failed to encoded image");
    (ContentType::PNG, buffer)
}
