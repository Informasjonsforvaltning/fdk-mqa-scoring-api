use std::str::from_utf8;

use actix_web::{http::header, web};
use http::uri::InvalidUri;
use http::Uri;
use serde::de::DeserializeOwned;

use crate::error::Error;

fn preview(s: &str, max: usize) -> &str {
    if s.len() > max {
        &s[..max]
    } else {
        s
    }
}

pub fn parse_json_body<T: DeserializeOwned>(body: &web::Bytes, endpoint: &str) -> Result<T, Error> {
    let body_str = from_utf8(body)?;
    tracing::debug!(
        endpoint,
        body_length = body.len(),
        body_preview = preview(body_str, 200),
        "Parsing request body"
    );

    serde_json::from_str(body_str).map_err(|e| {
        tracing::error!(
            endpoint,
            body_length = body.len(),
            body_preview = preview(body_str, 500),
            error = format!("{:?}", e).as_str(),
            "Failed to parse JSON request body"
        );
        Error::from(e)
    })
}

pub fn validate_dataset_uris(uris: &[String]) -> Result<(), Error> {
    uris.iter()
        .map(|uri| uri.parse::<Uri>())
        .collect::<Result<Vec<Uri>, InvalidUri>>()?;
    Ok(())
}

pub fn wants_json_ld(accept: &header::Accept) -> bool {
    accept
        .0
        .iter()
        .any(|qi| qi.item.to_string() == "application/ld+json")
}

pub fn graph_content_type(json_ld: bool) -> &'static str {
    if json_ld {
        "application/ld+json"
    } else {
        "text/turtle"
    }
}
