#[macro_use] extern crate rocket;

mod kindle;

#[launch]
fn rocket() -> _ {
    let wtf = rocket::build();
    
    use kindle;
    let wtf = kindle::build("/kindle", wtf);
    wtf
}
