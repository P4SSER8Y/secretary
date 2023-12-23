mod alpha;
mod bravo;
mod charlie;
mod shared;

use anyhow::{anyhow, Result};
use chrono::{self, DateTime, Local};
use image::GrayImage;
use once_cell::sync::{Lazy, OnceCell};
use rand::Rng;
use rocket::{error, info};
use rocket::{http::ContentType, Build, Rocket};
use rusttype::Font;
use shared::Context;
use std::collections::HashMap;
use std::io::Cursor;

static FONTS: OnceCell<HashMap<String, Font>> = OnceCell::new();
static DEFAULT_STYLE: OnceCell<Option<usize>> = OnceCell::new();

pub fn build(base: &'static str, build: Rocket<Build>) -> Rocket<Build> {
    load_fonts(&build);
    info!(
        "default style={:?}",
        DEFAULT_STYLE.get_or_init(|| {
            build
                .figment()
                .find_value("kindle.style")
                .ok()
                .and_then(|x| x.to_i128().and_then(|x| Some(x as usize)))
        })
    );
    build.mount(base, routes![main])
}

fn load_fonts(build: &Rocket<Build>) {
    FONTS.get_or_init(|| {
        let mut map = HashMap::new();
        let fonts = build
            .figment()
            .find_value("kindle.fonts")
            .expect("kindle.fonts not found")
            .into_dict()
            .expect("kindle.fonts not found");
        for x in fonts.iter() {
            let name = x.0.as_str();
            let path =
                x.1.as_str()
                    .expect(&format!("cannot parse kindle.fonts.{}", &name));
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

static STYLE_LIST: Lazy<Vec<fn(&Context) -> Result<GrayImage>>> =
    Lazy::new(|| vec![alpha::generate, bravo::generate, charlie::generate]);

fn factory(n: usize, context: &Context) -> Result<GrayImage> {
    if n >= STYLE_LIST.len() {
        return Err(anyhow!("invalid style number={}", n));
    }
    return STYLE_LIST[n](context);
}

#[get("/?<battery>&<style>&<now>")]
fn main(
    battery: Option<usize>,
    style: Option<usize>,
    now: Option<String>,
) -> (ContentType, Vec<u8>) {
    info!("{:?}", now);
    let now = match now {
        Some(raw) => DateTime::parse_from_str(&raw, "%Y-%m-%d").unwrap().into(),
        None => Local::now(),
    };
    let context = Context {
        battery: battery,
        now: Some(now),
        fonts: FONTS.get().unwrap(),
    };
    let style = style.or_else(|| *DEFAULT_STYLE.get().unwrap()).unwrap_or_else(|| rand::thread_rng().gen_range(0..STYLE_LIST.len()));
    info!("style={}", style);
    info!("now={:?}", context.now);
    info!("battery={:?}", context.battery);

    match factory(style, &context) {
        Ok(img) => {
            let mut buffer: Vec<u8> = Vec::new();
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageOutputFormat::Png)
                .expect("failed to encoded image");
            return (ContentType::PNG, buffer);
        }
        Err(e) => {
            error!("{:?}", e);
            return (ContentType::Text, format!("{:?}", e).into_bytes());
        }
    }
}
