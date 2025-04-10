use anyhow::anyhow;
use image::{guess_format, ImageFormat, ImageReader};
#[allow(unused_imports)]
use log::{debug, info};
use std::io::Cursor;

pub struct RawImage<'r> {
    pub data: Vec<u8>,
    pub mime_type: &'r str,
    pub extension: &'r str,
}

#[allow(dead_code)]
pub async fn compress(data: &[u8]) -> anyhow::Result<RawImage> {
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

pub async fn guess_image_mime_type(data: &[u8]) -> anyhow::Result<RawImage> {
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

pub async fn generate_thumbnail(data: &[u8]) -> anyhow::Result<RawImage> {
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

#[cfg(feature="avif")]
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
