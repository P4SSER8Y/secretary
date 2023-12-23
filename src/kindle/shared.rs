use std::collections::HashMap;

use image::{GrayImage, Luma};
use imageproc::drawing;
use rusttype::{Scale, Font};

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

pub struct Context<'a> {
    pub battery: Option<usize>,
    pub now: Option<chrono::DateTime<chrono::Local>>,
    pub fonts: &'a HashMap<String, Font<'a>>,
}
