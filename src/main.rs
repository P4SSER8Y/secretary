#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;

mod kindle;

#[launch]
fn rocket() -> _ {
    let wtf = rocket::build();
    
    use kindle;
    let wtf = kindle::build("/kindle", wtf);
    wtf
}
