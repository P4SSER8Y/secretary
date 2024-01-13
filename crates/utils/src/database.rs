use anyhow::{Ok, Result};
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Serialize};

pub fn get_db() -> sled::Db {
    static INSTANCE: Lazy<sled::Db> = Lazy::new(|| {
        let data = super::get_data_path();
        let path = std::path::Path::new(data).join("memory");
        sled::open(path).unwrap()
    });
    return INSTANCE.clone();
}

pub struct Db {
    db: sled::Db,
}

impl Db {
    pub fn new() -> Db {
        Db { db: get_db() }
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let result = self.db.get(key)?;
        if let Some(raw) = result {
            let result: T = bincode::deserialize::<T>(&raw)?;
            return Ok(Some(result));
        } else {
            return Ok(None);
        }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let value = bincode::serialize(value)?;
        let _ = self.db.insert(key, value)?;
        return Ok(());
    }

    pub fn flush(&self) -> Result<()> {
        let _ = self.db.flush()?;
        return Ok(());
    }
}
