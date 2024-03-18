use std::collections::HashMap;

use image::{GenericImageView, GrayAlphaImage, GrayImage, Luma, LumaA};
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

pub fn draw_centered_text<'a>(
    canvas: &'a mut GrayImage,
    color: Luma<u8>,
    base: (i32, i32),
    scale: Scale,
    font: &'a Font<'a>,
    text: &'a str,
) -> Rect {
    let size = drawing::text_size(scale, font, &text);
    let temp = drawing::draw_text(
        &GrayAlphaImage::new(size.0 as u32, size.1 as u32),
        LumaA([color.0[0], 255]),
        0,
        0,
        scale,
        font,
        &text,
    );
    let temp = {
        let mut min_x = size.0 as u32;
        let mut max_x = 0 as u32;
        let mut min_y = size.1 as u32;
        let mut max_y = 0 as u32;
        for x in 0..(size.0 as u32) {
            for y in 0..(size.1 as u32) {
                if temp.get_pixel(x, y).0[1] as u32 > 0 {
                    if x < min_x {
                        min_x = x
                    };
                    if x > max_x {
                        max_x = x
                    };
                    if y < min_y {
                        min_y = y
                    };
                    if y > max_y {
                        max_y = y
                    };
                }
            }
        }
        image::imageops::crop_imm(&temp, min_x, min_y, max_x - min_x, max_y - min_y)
    };

    let size = (temp.width() as i32, temp.height() as i32);
    let x = base.0 - size.0 / 2;
    let y = base.1 - size.1 / 2;
    for w in 0..size.0 {
        for h in 0..size.1 {
            if (w + x < 0)
                || (h + y < 0)
                || (w + x >= canvas.width() as i32)
                || (h + y >= canvas.height() as i32)
            {
                continue;
            }
            let a = canvas.get_pixel((w + x) as u32, (h + y) as u32);
            let b = temp.get_pixel(w as u32, h as u32);
            let r = b.0[1] as f32 / 255.0;
            let p = Luma([((a.0[0] as f32) * (1.0 - r) + (b.0[0] as f32) * r) as u8]);
            canvas.put_pixel((w + x) as u32, (h + y) as u32, p);
        }
    }
    Rect::at(x, y).of_size(size.0.try_into().unwrap(), size.1.try_into().unwrap())
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
