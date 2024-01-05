use chrono::Local;
use rocket::{error, info};

#[macro_use]
extern crate rocket;

mod kindle;
mod logger;
mod qweather;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    logger::init();

    let wtf = rocket::build();

    bark::build(wtf.figment());
    tokio::spawn(bark::send(bark::Message {
        body: "Hello World",
        title: Some("Lighter"),
        ..Default::default()
    }));

    let db = utils::database::Db::new();
    if let Ok(launch) = db.get::<String>("launch") {
        info!("last launch at {:?}", launch);
    } else {
        info!("never launched before");
    }
    if let Err(_) = db.set("launch", &Local::now().to_rfc3339()) {
        error!("last launch not found");
    }

    let wtf = qweather::build(wtf).await;
    let wtf = kindle::build("/kindle", wtf);

    wtf.ignite().await?.launch().await?;
    let _ = db.flush();
    Ok(())
}
