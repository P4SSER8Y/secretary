mod alpha;
mod bravo;
mod charlie;
mod shared;

use anyhow::{anyhow, Result};
use image::GrayImage;
use log::info;
use once_cell::sync::OnceCell;
use rand::Rng;
pub use shared::{load_fonts, Context};

static DEFAULT_STYLE: OnceCell<Option<usize>> = OnceCell::new();

pub fn set_default_style(style: Option<usize>) {
    if let Some(style) = DEFAULT_STYLE.get_or_init(|| style) {
        info!("kindle's default style={}", style);
    }
}

pub async fn factory(style: Option<usize>, context: &Context) -> Result<GrayImage> {
    let n = style
        .or_else(|| *DEFAULT_STYLE.get().unwrap())
        .unwrap_or_else(|| rand::thread_rng().gen_range(0..3));
    return match n {
        0 => alpha::generate(context).await,
        1 => bravo::generate(context).await,
        2 => charlie::generate(context).await,
        _ => Err(anyhow!("unknown style = {}", n)),
    };
}
