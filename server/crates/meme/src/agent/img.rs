use anyhow::{anyhow, Context};
use image::{
    guess_format,
    imageops::{rotate270, ColorMap, FilterType::Gaussian},
    ImageBuffer, ImageFormat, ImageReader, Rgb,
};
#[allow(unused_imports)]
use log::{debug, info};
use palette::{color_difference::Ciede2000, convert::FromColorUnclamped};
use std::{io::Cursor, sync::LazyLock};

pub struct RawImage<'r> {
    pub data: Vec<u8>,
    pub mime_type: &'r str,
    pub extension: &'r str,
}

#[allow(dead_code)]
pub async fn compress(data: &[u8]) -> anyhow::Result<RawImage<'_>> {
    const FORMAT: ImageFormat = ImageFormat::WebP;
    let img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()?
        .decode()?;
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), FORMAT)?;
    Ok(RawImage {
        data: Vec::new(),
        mime_type: FORMAT.to_mime_type(),
        extension: FORMAT.extensions_str()[0],
    })
}

#[allow(dead_code)]
pub async fn guess_image_mime_type(data: &[u8]) -> anyhow::Result<RawImage<'_>> {
    let format = guess_format(data);
    match format {
        Ok(format) => Ok(RawImage {
            data: Vec::new(),
            mime_type: format.to_mime_type(),
            extension: format.extensions_str()[0],
        }),
        Err(_) => Err(anyhow!("guess format failed")),
    }
}

pub async fn generate_thumbnail(data: &[u8]) -> anyhow::Result<RawImage<'_>> {
    const FORMAT: ImageFormat = ImageFormat::WebP;
    let mut img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()?
        .decode()?;
    let nheight = (img.height() as f32 * 512.0 / img.width() as f32) as u32;
    img = img.thumbnail(512, nheight);
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::WebP)?;
    Ok(RawImage {
        data: buf,
        mime_type: FORMAT.to_mime_type(),
        extension: FORMAT.extensions_str()[0],
    })
}

#[cfg(feature = "avif")]
pub fn convert_to_avif(data: &[u8]) -> anyhow::Result<RawImage<'static>> {
    const FORMAT: ImageFormat = ImageFormat::Avif;
    let img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()?
        .decode()?;
    let mut output = Vec::new();
    img.write_to(&mut Cursor::new(&mut output), FORMAT)?;
    Ok(RawImage {
        data: output,
        mime_type: FORMAT.to_mime_type(),
        extension: FORMAT.extensions_str()[0],
    })
}

static CMAP_KWYRBG: LazyLock<Vec<Rgb<u8>>> = LazyLock::new(|| {
    let v: [[u8; 3]; 6] = [
        [0, 0, 0],       // black
        [255, 255, 255], // white
        [255, 255, 0],   // yellow
        [255, 0, 0],     // red
        [0, 0, 255],     //blue
        [0, 255, 0],     // green
    ];
    v.iter().map(|c| Rgb(*c)).collect()
});
fn convert_from_rgb_to_lab(value: &Rgb<u8>) -> palette::Lab {
    use palette::{Lab, Srgb};
    let c = Srgb::new(value.0[0] as f32, value.0[1] as f32, value.0[2] as f32);
    Lab::from_color_unclamped(c)
}

struct KWYRBG;
impl ColorMap for KWYRBG {
    type Color = Rgb<u8>;

    fn index_of(&self, color: &Self::Color) -> usize {
        let c = convert_from_rgb_to_lab(color);
        let mut idx = 0;
        let mut min_diff = f32::MAX;
        for i in 0..CMAP_KWYRBG.len() {
            let lab = convert_from_rgb_to_lab(&CMAP_KWYRBG[i]);
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
        for item in CMAP_KWYRBG.iter() {
            let lab = convert_from_rgb_to_lab(item);
            let diff = c.difference(lab);
            if diff < min_diff {
                min_diff = diff;
                min = item.clone();
            }
        }
        color.0 = min.0;
    }
}

fn try_decode(
    data: &[u8],
    mime: Option<&str>,
) -> anyhow::Result<ImageBuffer<image::Rgb<u8>, Vec<u8>>> {
    debug!("size={}", data.len());
    if let Some(mime) = mime {
        debug!("try decode with mime: {}", mime);
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
            debug!("try decode with guessed format: {:?}", t.format());
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
        debug!("try decode with format: {:?}", f);
        if let Ok(t) = ImageReader::with_format(Cursor::new(data), ImageFormat::Jpeg).decode() {
            return Ok(t.into_rgb8());
        }
    }
    Err(anyhow!("cannot decode image"))
}

pub fn dither(
    data: &[u8],
    mime: &str,
    width: u32,
    height: u32,
    preview: bool,
) -> anyhow::Result<RawImage<'static>> {
    use image::imageops::{crop, dither, index_colors, resize};
    let mut img = try_decode(data, Some(mime))?;

    if img.width() < img.height() {
        img = rotate270(&img);
    }
    let nw: u32;
    let nh: u32;
    if (img.width() as f32 / width as f32) > (img.height() as f32 / height as f32) {
        nh = height as u32;
        nw = (img.width() as f32 / img.height() as f32 * height as f32) as u32;
    } else {
        nw = width as u32;
        nh = (img.height() as f32 / img.width() as f32 * width as f32) as u32;
    }
    img = resize(&img, nw, nh, Gaussian);
    img = crop(&mut img, (nw - width) / 2, (nh - height) / 2, width, height).to_image();

    dither(&mut img, &KWYRBG);
    if preview {
        let mut buf = Vec::new();
        let _ = img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Bmp);
        Ok(RawImage {
            data: buf,
            mime_type: ImageFormat::Bmp.to_mime_type(),
            extension: ImageFormat::Bmp.extensions_str()[0],
        })
    } else {
        let img = index_colors(&img, &KWYRBG).into_raw();
        Ok(RawImage {
            data: img,
            mime_type: "application/octat-stream",
            extension: "bin",
        })
    }
}
