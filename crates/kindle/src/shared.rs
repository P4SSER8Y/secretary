use std::collections::HashMap;

use image::{GrayImage, Luma};
use imageproc::{drawing, rect::Rect};
use log::info;
use once_cell::sync::OnceCell;
use rusttype::{Font, Scale};

#[allow(dead_code)]
pub enum AlignHorizontal {
    Left,
    Center,
    Right,
}

#[allow(dead_code)]
pub enum AlignVertical {
    Top,
    Center,
    Bottom,
}

pub fn draw_aligned_text<'a>(
    canvas: &'a mut GrayImage,
    color: Luma<u8>,
    base: (i32, i32),
    scale: Scale,
    font: &'a Font<'a>,
    text: &'a str,
    align: (AlignHorizontal, AlignVertical),
) -> Rect {
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
    return Rect::at(x, y).of_size(size.0.try_into().unwrap(), size.1.try_into().unwrap());
}

pub struct Context {
    pub battery: Option<usize>,
    pub now: Option<chrono::DateTime<chrono::Local>>,
}

static FONTS: OnceCell<HashMap<String, Font>> = OnceCell::new();
pub fn load_fonts(fonts: HashMap<String, String>) {
    FONTS.get_or_init(|| {
        let mut map = HashMap::new();
        for x in fonts.iter() {
            let name = x.0;
            let path = x.1;
            info!("loading {}: {}", name, path);
            let data = std::fs::read(&path).expect(&format!("failed to load {}", path));
            let font = Font::try_from_vec(data).unwrap_or_else(|| {
                panic!("cannot load {}", path);
            });
            map.insert(name.to_string(), font);
        }
        return map;
    });
}

pub fn get_font(name: &str) -> Option<&Font> {
    return FONTS.get().unwrap().get(name);
}
