use std::io::Cursor;

use crate::agent::{self, MetaData, TokenPayload};
use anyhow::anyhow;
use rocket::{
    form::FromForm,
    http::Status,
    request::{FromRequest, Outcome},
    response::{self, Responder},
    Request, Response,
};
use serde::{Deserialize, Serialize};

#[rocket::async_trait]
impl<'a> FromRequest<'a> for TokenPayload {
    type Error = anyhow::Error;

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        fn parse_from_query<'a>(request: &'a Request<'_>) -> anyhow::Result<&'a str> {
            let bearer = request
                .query_value::<&str>("token")
                .ok_or(anyhow!("No token"))?
                .unwrap();
            Ok(bearer)
        }

        fn parse_from_cookies<'a>(request: &'a Request<'_>) -> anyhow::Result<&'a str> {
            let bearer = request
                .cookies()
                .get("token")
                .ok_or(anyhow!("No token"))?
                .value();
            Ok(bearer)
        }

        fn parse_from_header<'a>(request: &'a Request<'_>) -> anyhow::Result<&'a str> {
            let bearer = request
                .headers()
                .get_one("token")
                .ok_or(anyhow!("No bearer token"))?;
            Ok(bearer)
        }

        fn parse_key<'a>(request: &'a Request<'_>) -> anyhow::Result<&'a str> {
            let key = request
                .query_value::<&str>("key")
                .ok_or(anyhow!("No key"))?
                .unwrap();
            Ok(key)
        }
        // check from header, cookies, query
        let bearer = parse_from_header(request)
            .or_else(|_| parse_from_query(request))
            .or_else(|_| parse_from_cookies(request));
        if let Ok(bearer) = bearer {
            if let Ok(claims) = agent::check(bearer.trim()) {
                return Outcome::Success(claims);
            }
        }
        if let Ok(key) = parse_key(request) {
            if let Ok(claims) = agent::check_key(key) {
                return Outcome::Success(claims.clone());
            }
        }
        Outcome::Forward(Status::Unauthorized)
    }
}

pub struct FileContent {
    pub content_type: String,
    pub name: String,
    pub body: anyhow::Result<Vec<u8>>,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for FileContent {
    fn respond_to(self, _request: &'r Request<'_>) -> response::Result<'o> {
        if let Ok(body) = self.body {
            Ok(Response::build()
                .status(Status::Ok)
                .raw_header("Content-Type", self.content_type.to_string())
                .raw_header("Content-Length", body.len().to_string())
                .raw_header("Content-Disposition", format!("inline; filename={}", self.name))
                .sized_body(body.len(), Cursor::new(body))
                .finalize())
        } else {
            let body = "Not Found".to_string();
            Ok(Response::build()
                .status(Status::NotFound)
                .sized_body(body.len(), Cursor::new(body))
                .finalize())
        }
    }
}

pub struct Password(pub Option<String>);

#[rocket::async_trait]
impl<'a> FromRequest<'a> for Password {
    type Error = ();

    async fn from_request(request: &'a Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(password) = request.headers().get_one("password") {
            return Outcome::Success(Password(Some(password.to_string())));
        }
        if let Some(Ok(password)) = request.query_value::<&str>("pwd") {
            return Outcome::Success(Password(Some(password.to_string())));
        }
        if let Some(cookie) = request.cookies().get("password") {
            return Outcome::Success(Password(Some(cookie.value().to_string())));
        }
        Outcome::Success(Password(None))
    }
}

#[derive(FromForm)]
pub struct UploadedImage<'r> {
    pub file: &'r [u8],
    pub tags: Option<&'r str>,
    pub mime: &'r str,
    pub filename: &'r str,
}

#[derive(Serialize, Debug)]
pub struct BriefMetaData {
    pub uuid: String,
    pub timestamp: String,
    pub mime: String,
    pub tags: Vec<String>,
    pub encrypted: bool,
}

impl From<&MetaData> for BriefMetaData {
    fn from(meta: &MetaData) -> Self {
        let mut tags = meta.tags.clone();
        meta.content_type.split('/').filter(|x| x.len() > 0).map(|x| x.trim()).for_each(|x| tags.push(x.to_string()));
        BriefMetaData {
            uuid: meta.uuid.clone(),
            timestamp: meta.timestamp.clone(),
            mime: meta.content_type.clone(),
            tags: tags,
            encrypted: meta.encrypted,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct UpdateTagsRequest {
    pub tags: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct ListInfo {
    pub meta: Vec<BriefMetaData>,
}

