use std::io::Write;

use crate::agent::RawImage;
use anyhow::{anyhow, Context};
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
    let mut child = tokio::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(src_file.as_os_str())
        .arg("-vf")
        .arg("scale='min(iw,512)':-1")
        .arg("-vframes")
        .arg("1")
        .arg("-update")
        .arg("true")
        .arg(dst_file.as_os_str())
        .spawn()
        .expect("cannot spawn ffmpeg");
    let status = child.wait().await?;
    if !status.success() {
        return Err(anyhow!("ffmpeg failed with {}", status));
    }
    let thumbnail = std::fs::read(&dst_file)?;
    Ok(RawImage {
        data: thumbnail,
        mime_type: "image/png",
        extension: "png",
    })
}
