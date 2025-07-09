use serde::{Deserialize, Serialize};
use sled::IVec;
use std::time::SystemTime;
use tokio::fs::File;

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    #[serde(with = "humantime_serde")]
    pub expiration: SystemTime,
    pub size: u64,
    pub public: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub file: Option<File>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Kind {
    File,
    Text,
}

impl TryFrom<&Metadata> for IVec {
    type Error = serde_json::Error;
    fn try_from(value: &Metadata) -> Result<Self, Self::Error> {
        match serde_json::to_vec(value) {
            Ok(value) => Ok(value.into()),
            Err(err) => Err(err),
        }
    }
}

impl TryFrom<&IVec> for Metadata {
    type Error = serde_json::Error;
    fn try_from(value: &IVec) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value)
    }
}
