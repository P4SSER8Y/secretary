use chrono::{self, Local, NaiveDate};
use kindle::Context;
use rocket::response::status::NotFound;
use rocket::{http::ContentType, Build, Rocket};
use std::collections::HashMap;
use std::io::Cursor;

pub fn build(base: &'static str, build: Rocket<Build>) -> Rocket<Build> {
    kindle::set_default_style(
        build
            .figment()
            .find_value("kindle.style")
            .ok()
            .and_then(|x| x.to_i128().and_then(|x| Some(x as usize))),
    );
    let mut font_map = HashMap::new();
    let list = build.figment().find_value("kindle.fonts");
    if let Ok(list) = list {
        if let Some(list) = list.as_dict() {
            for item in list {
                let name = item.0;
                if let Some(path) = item.1.as_str() {
                    font_map.insert(name.clone(), path.to_string());
                }
            }
        }
    }
    kindle::load_fonts(font_map);
    build.mount(base, routes![main])
}

#[get("/?<battery>&<style>&<now>")]
async fn main(
    battery: Option<usize>,
    style: Option<usize>,
    now: Option<String>,
) -> Result<(ContentType, Vec<u8>), NotFound<()>> {
    info!("{:?}", now);
    let now = match now {
        Some(raw) => {
            let date = NaiveDate::parse_from_str(&raw, "%Y-%m-%d");
            if let Ok(date) = date {
                let naive_datetime = date.and_hms_opt(0, 0, 0).unwrap();
                naive_datetime.and_local_timezone(Local).unwrap()
            } else {
                Local::now()
            }
        }
        None => Local::now(),
    };
    let mut battery = battery;
    if let Some(battery) = battery {
        let db = utils::database::Db::new();
        let _ = db.set("kindle/battery", &battery);
    } else {
        let db = utils::database::Db::new();
        battery = db.get("kindle/battery").unwrap_or(None);
    }
    let context = Context {
        battery: battery,
        now: Some(now),
    };
    info!("style={:?}", style);
    info!("now={:?}", context.now);
    info!("battery={:?}", context.battery);

    match kindle::factory(style, &context) {
        Ok(img) => {
            let mut buffer: Vec<u8> = Vec::new();
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageOutputFormat::Png)
                .expect("failed to encoded image");
            return Ok((ContentType::PNG, buffer));
        }
        Err(e) => {
            error!("{:?}", e);
            return Err(NotFound(()));
        }
    }
}
