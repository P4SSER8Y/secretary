use std::io::Write;

use crate::agent::RawImage;
use anyhow::{anyhow, Context};
use ez_ffmpeg::{FfmpegContext, Output};
#[allow(unused)]
use log::debug;
use tempfile::{self, NamedTempFile};

pub async fn generate_video_thumbnail<'r>(data: &[u8]) -> anyhow::Result<RawImage<'r>> {
    let mut src_file =
        NamedTempFile::new().with_context(|| anyhow!("create temporary src file failed"))?;
    src_file
        .write_all(data)
        .with_context(|| anyhow!("cannot write to {:?}", src_file.path()))?;
    let src_file = src_file.into_temp_path();
    let dst_file = NamedTempFile::with_suffix(".png")
        .with_context(|| anyhow!("create temporary src file failed"))?
        .into_temp_path();
    debug!("src: {:?}", src_file);
    debug!("dst: {:?}", dst_file);
    FfmpegContext::builder()
        .input(src_file.to_str().ok_or(anyhow!("WTF"))?)
        .filter_desc("scale='min(iw,512)':-1")
        .output(Output::from(dst_file.to_str().ok_or(anyhow!("WTF"))?).set_format("png").set_max_video_frames(1))
        .build()?
        .start()?
        .await?;
    // re-read dst file
    let thumbnail = std::fs::read(dst_file)?;
    Ok(RawImage {
        data: thumbnail,
        mime_type: "image/png",
        extension: "png",
    })
}
