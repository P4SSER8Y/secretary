use once_cell::sync::OnceCell;

pub mod database;

static DATA_PATH: OnceCell<String> = OnceCell::new();

pub fn init_data_path(data: &str) {
    DATA_PATH.get_or_init(|| data.to_string());
}

pub fn get_data_path() -> &'static str {
    DATA_PATH.get().unwrap()
}
