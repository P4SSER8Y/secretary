#[macro_use] extern crate rocket;

mod kindle;
mod qweather;

#[launch]
fn rocket() -> _ {
    let wtf = rocket::build();
    
    let wtf = qweather::build(wtf);
    let wtf = kindle::build("/kindle", wtf);
    wtf
}
