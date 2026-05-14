use std::io::{self, Read, Write};

pub fn run(owner: &str, password: &str) -> Result<(), rocket::Error> {
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .map_err(|e| rocket::error::ErrorKind::Io(e))?;
    let key = meme::crypto::derive_key(password, owner);
    match meme::crypto::decrypt(&input, &key) {
        Ok(plaintext) => {
            io::stdout().write_all(&plaintext).ok();
            Ok(())
        }
        Err(e) => {
            eprintln!("decrypt failed: {}", e);
            Err(rocket::error::ErrorKind::Io(io::Error::new(
                io::ErrorKind::Other,
                e.to_string(),
            ))
            .into())
        }
    }
}
