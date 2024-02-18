use rocket::{fs::TempFile, FromForm};
use serde::Serialize;
use std::sync::OnceLock;

pub const PREFIX: &str = "inbox/";
pub static PATH: OnceLock<String> = OnceLock::new();

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Status {
    Ok { ok: bool, message: String },
    NewItem { ok: bool, id: String },
    CommonError { ok: bool, message: String },
}

pub fn init_path(path: &str) {
    PATH.get_or_init(|| path.to_string());
}

#[derive(FromForm)]
pub struct NewData<'r> {
    pub file: TempFile<'r>,
    pub public: Option<bool>,
    pub expire: Option<String>,
}
