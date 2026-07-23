use actix_web::HttpRequest;

use crate::{config::API_KEY, error::Error};

pub fn validate_api_key(request: HttpRequest) -> Result<(), Error> {
    let token = request
        .headers()
        .get("X-API-KEY")
        .ok_or(Error::Unauthorized("X-API-KEY header missing".to_string()))?
        .to_str()
        .map_err(|_| Error::Unauthorized("invalid api key".to_string()))?;

    if token == API_KEY.clone() {
        Ok(())
    } else {
        Err(Error::Unauthorized("Incorrect api key".to_string()))
    }
}
