use rocket::info;
use rusttype::{Font, Scale};
use std::collections::HashMap;
use std::io::Cursor;

use chrono::{self, Datelike};
use image::{GrayImage, Luma};
use imageproc::drawing::{self};
use imageproc::rect::Rect;
use rocket::{http::ContentType, Build, Rocket};

lazy_static! {
    static ref FONT: Font<'static> = {
        let path = "SourceHanSerifSC-Heavy.otf";
        let data = std::fs::read(&path).unwrap();
        Font::try_from_vec(data).unwrap_or_else(|| {
            panic!("cannot load {}", path);
        })
    };
    static ref MAP_WEEKDAY: HashMap<u8, &'static str> = {
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
    };
}

pub fn build(base: &'static str, build: Rocket<Build>) -> Rocket<Build> {
    build.mount(base, routes![main])
}

enum AlignHorizontal {
    Left,
    Center,
    Right,
}

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
    drawing::draw_text_mut(canvas, color, x, y, scale, &FONT, &text);
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
        &FONT,
        &text,
        (AlignHorizontal::Center, AlignVertical::Center),
    );
    draw_aligned_text(
        &mut img,
        Luma([0]),
        (300, 400),
        Scale::uniform(150.0),
        &FONT,
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
            &FONT,
            &battery,
            (AlignHorizontal::Right, AlignVertical::Top),
        );
    }

    draw_aligned_text(
        &mut img,
        Luma([128]),
        (300, 800 - 20),
        Scale::uniform(18.0),
        &FONT,
        &format!("更新：{}", now.format("%Y-%m-%d %H:%M:%S")),
        (AlignHorizontal::Center, AlignVertical::Bottom),
    );

    let mut buffer: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut buffer), image::ImageOutputFormat::Png)
        .expect("failed to encoded image");
    (ContentType::PNG, buffer)
}
