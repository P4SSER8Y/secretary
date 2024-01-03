use chrono::Local;
use futures::join;
use rocket::{error, info};

#[macro_use]
extern crate rocket;

mod kindle;
mod qweather;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let wtf = rocket::build();

    bark::build(wtf.figment());
    bark::send(bark::Message {
        body: "Hello World",
        title: Some("Lighter"),
        ..Default::default()
    })
    .await;

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
    let task_wtf = wtf.ignite().await?.launch();
    let _ = join!(task_wtf);
    Ok(())
}
