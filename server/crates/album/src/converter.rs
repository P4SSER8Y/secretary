use anyhow::{anyhow, Context};
use image::{
    imageops::{self, ColorMap, FilterType::Gaussian},
    ImageBuffer, ImageFormat, ImageReader, Rgb,
};
use palette::{color_difference::Ciede2000, convert::FromColorUnclamped};
use std::{io::Cursor, sync::LazyLock};

use crate::config::AlbumConfig;

// ---- 7-color EPD palette ----

static CMAP_7COLOR: LazyLock<Vec<Rgb<u8>>> = LazyLock::new(|| {
    vec![
        Rgb([0, 0, 0]),         // 0: BLACK
        Rgb([255, 255, 255]),   // 1: WHITE
        Rgb([255, 255, 0]),     // 2: YELLOW
        Rgb([255, 0, 0]),       // 3: RED
        Rgb([255, 165, 0]),     // 4: ORANGE
        Rgb([0, 0, 255]),       // 5: BLUE
        Rgb([0, 255, 0]),       // 6: GREEN
    ]
});

fn convert_from_rgb_to_lab(value: &Rgb<u8>) -> palette::Lab {
    use palette::{Lab, Srgb};
    let c = Srgb::new(
        value.0[0] as f32,
        value.0[1] as f32,
        value.0[2] as f32,
    );
    Lab::from_color_unclamped(c)
}

struct SevenColorEPD;

impl ColorMap for SevenColorEPD {
    type Color = Rgb<u8>;

    fn index_of(&self, color: &Self::Color) -> usize {
        let c = convert_from_rgb_to_lab(color);
        let mut idx = 0;
        let mut min_diff = f32::MAX;
        for i in 0..CMAP_7COLOR.len() {
            let lab = convert_from_rgb_to_lab(&CMAP_7COLOR[i]);
            let diff = c.difference(lab);
            if diff < min_diff {
                min_diff = diff;
                idx = i;
            }
        }
        idx
    }

    fn map_color(&self, color: &mut Self::Color) {
        let c = convert_from_rgb_to_lab(color);
        let mut min = Rgb::<u8>([0, 0, 0]);
        let mut min_diff = f32::MAX;
        for item in CMAP_7COLOR.iter() {
            let lab = convert_from_rgb_to_lab(item);
            let diff = c.difference(lab);
            if diff < min_diff {
                min_diff = diff;
                min = *item;
            }
        }
        color.0 = min.0;
    }
}

// ---- Public types ----

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversionParams {
    pub crop_mode: Option<CropAnchor>,
    pub dither: bool,
    pub rotate_cw: bool,
    pub rotate_ccw: bool,
    pub invert: bool,
}

impl Default for ConversionParams {
    fn default() -> Self {
        Self {
            crop_mode: Some(CropAnchor::Center),
            dither: true,
            rotate_cw: false,
            rotate_ccw: false,
            invert: false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CropAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl CropAnchor {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "topleft" | "top-left" | "tl" | "lt" => Some(CropAnchor::TopLeft),
            "topcenter" | "top" | "t" => Some(CropAnchor::TopCenter),
            "topright" | "top-right" | "tr" | "rt" => Some(CropAnchor::TopRight),
            "centerleft" | "left" | "l" => Some(CropAnchor::CenterLeft),
            "center" | "c" => Some(CropAnchor::Center),
            "centerright" | "right" | "r" => Some(CropAnchor::CenterRight),
            "bottomleft" | "bottom-left" | "bl" | "lb" => Some(CropAnchor::BottomLeft),
            "bottomcenter" | "bottom" | "b" => Some(CropAnchor::BottomCenter),
            "bottomright" | "bottom-right" | "br" | "rb" => Some(CropAnchor::BottomRight),
            _ => None,
        }
    }
}

pub struct ConvertedImage {
    pub clean_png: Vec<u8>,     // clean RGB preview after resize
    pub epd_png: Vec<u8>,       // 7-color preview after dither + map
    pub raw_4bpp: Vec<u8>,
}

// ---- Image decoding ----

fn try_decode(data: &[u8], mime: Option<&str>) -> anyhow::Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    log::debug!("try_decode: size={}", data.len());
    if let Some(mime) = mime {
        log::debug!("try_decode with mime: {}", mime);
        if let Some(mime_format) = ImageFormat::from_mime_type(mime) {
            if let Ok(t) = ImageReader::with_format(Cursor::new(data), mime_format)
                .decode()
                .with_context(|| anyhow!("mime type seems not valid"))
            {
                return Ok(t.into_rgb8());
            }
        }
    }
    if let Ok(t) = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .with_context(|| anyhow!("guess format failed"))
        .and_then(|t| {
            log::debug!("try_decode with guessed format: {:?}", t.format());
            t.decode()
                .with_context(|| anyhow!("decode with guessed format failed"))
        })
    {
        return Ok(t.into_rgb8());
    }
    let list = vec![
        ImageFormat::Jpeg,
        ImageFormat::Png,
        ImageFormat::Gif,
        ImageFormat::WebP,
    ];
    for f in list {
        log::debug!("try_decode with format: {:?}", f);
        if let Ok(t) = ImageReader::with_format(Cursor::new(data), f).decode() {
            return Ok(t.into_rgb8());
        }
    }
    Err(anyhow!("cannot decode image"))
}

// ---- Main conversion ----

pub fn convert_to_epd(
    data: &[u8],
    params: &ConversionParams,
    cfg: &AlbumConfig,
) -> anyhow::Result<ConvertedImage> {
    let mut img = try_decode(data, None)?;
    let target_w = cfg.eink_width;
    let target_h = cfg.eink_height;

    // 1. Rotation
    if params.rotate_cw {
        img = imageops::rotate90(&img);
    } else if params.rotate_ccw {
        img = imageops::rotate270(&img);
    }

    let (src_w, src_h) = (img.width(), img.height());
    let target_ratio = target_w as f32 / target_h as f32;

    // 2. Crop or pad-to-fit
    if let Some(ref anchor) = params.crop_mode {
        let src_ratio = src_w as f32 / src_h as f32;
        let (crop_w, crop_h) = if src_ratio > target_ratio {
            (src_h as f32 * target_ratio, src_h as f32)
        } else {
            (src_w as f32, src_w as f32 / target_ratio)
        };

        let crop_w = crop_w as u32;
        let crop_h = crop_h as u32;

        let x = match anchor {
            CropAnchor::TopLeft | CropAnchor::CenterLeft | CropAnchor::BottomLeft => 0,
            CropAnchor::TopRight | CropAnchor::CenterRight | CropAnchor::BottomRight => {
                src_w - crop_w
            }
            _ => (src_w - crop_w) / 2,
        };
        let y = match anchor {
            CropAnchor::TopLeft | CropAnchor::TopCenter | CropAnchor::TopRight => 0,
            CropAnchor::BottomLeft | CropAnchor::BottomCenter | CropAnchor::BottomRight => {
                src_h - crop_h
            }
            _ => (src_h - crop_h) / 2,
        };

        img = imageops::crop(&mut img, x, y, crop_w, crop_h).to_image();
    }

    // 3. Resize to target
    img = imageops::resize(&img, target_w, target_h, Gaussian);

    // 4. Generate clean preview PNG (before any color alteration)
    let mut clean_png = Vec::new();
    img.write_to(
        &mut Cursor::new(&mut clean_png),
        ImageFormat::Png,
    )?;

    // 5. Invert
    if params.invert {
        img = ImageBuffer::from_fn(target_w, target_h, |x, y| {
            let p = img.get_pixel(x, y);
            Rgb([255 - p[0], 255 - p[1], 255 - p[2]])
        });
    }

    // 6. Dither + map to exact EPD colors
    if params.dither {
        imageops::dither(&mut img, &SevenColorEPD);
    } else {
        for py in 0..target_h {
            for px in 0..target_w {
                let p = img.get_pixel(px, py);
                let mut c = *p;
                SevenColorEPD.map_color(&mut c);
                img.put_pixel(px, py, c);
            }
        }
    }

    // 7. Generate 7-color preview PNG
    let mut epd_png = Vec::new();
    img.write_to(
        &mut Cursor::new(&mut epd_png),
        ImageFormat::Png,
    )?;

    // 8. Pack to 4bpp: each byte = (even_px << 4) | odd_px
    let buf_size = (target_w * target_h / 2) as usize;
    let mut raw_4bpp = vec![0u8; buf_size];
    for py in 0..target_h {
        for px in (0..target_w).step_by(2) {
            let c1 = SevenColorEPD.index_of(img.get_pixel(px, py));
            let c2 = if px + 1 < target_w {
                SevenColorEPD.index_of(img.get_pixel(px + 1, py))
            } else {
                1 // WHITE for out-of-bounds
            };
            let idx = ((py * target_w + px) / 2) as usize;
            raw_4bpp[idx] = ((c1 as u8) << 4) | (c2 as u8);
        }
    }

    Ok(ConvertedImage {
        clean_png,
        epd_png,
        raw_4bpp,
    })
}
