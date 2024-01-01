mod alpha;
mod bravo;
mod charlie;
mod shared;

use anyhow::{anyhow, Result};
use image::GrayImage;
use log::info;
use once_cell::sync::{Lazy, OnceCell};
use rand::Rng;
pub use shared::{Context, load_fonts};

static DEFAULT_STYLE: OnceCell<Option<usize>> = OnceCell::new();

pub fn set_default_style(style: Option<usize>) {
    DEFAULT_STYLE.get_or_init(|| style);
    info!("{:#?}", DEFAULT_STYLE.get());
}

static STYLE_LIST: Lazy<Vec<fn(&Context) -> Result<GrayImage>>> =
    Lazy::new(|| vec![alpha::generate, bravo::generate, charlie::generate]);

pub fn factory(style: Option<usize>, context: &Context) -> Result<GrayImage> {
    let n = style
        .or_else(|| *DEFAULT_STYLE.get().unwrap())
        .unwrap_or_else(|| rand::thread_rng().gen_range(0..STYLE_LIST.len()));
    if n >= STYLE_LIST.len() {
        return Err(anyhow!("invalid style number={}", n));
    }
    return STYLE_LIST[n](context);
}
