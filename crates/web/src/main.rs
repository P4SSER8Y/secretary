use chrono::Local;
use rocket::{info, error};

#[macro_use]
extern crate rocket;

mod kindle;
mod qweather;

#[launch]
fn rocket() -> _ {
    let wtf = rocket::build();

    let db = utils::database::Db::new();
    if let Ok(launch) = db.get::<String>("launch") {
        info!("last launch at {:?}", launch);
    } else {
        info!("never launched before");
    }
    if let Err(_) = db.set("launch", &Local::now().to_rfc3339()) {
        error!("last launch not found");
    }
    let _ = db.flush();

    let wtf = qweather::build(wtf);
    let wtf = kindle::build("/kindle", wtf);
    wtf
}
