use actix_web::{get, web, Responder};

use crate::{database::PgPool, error::Error};

#[get("/ping")]
pub async fn ping() -> Result<impl Responder, Error> {
    Ok("pong")
}

#[get("/ready")]
pub async fn ready(pool: web::Data<PgPool>) -> Result<impl Responder, Error> {
    let result = web::block(move || {
        let mut conn = pool.get()?;
        conn.test_connection()
    })
    .await
    .map_err(|e| Error::BlockingError(e.into()))?;

    match result {
        Ok(_) => Ok("ok"),
        Err(e) => Err(e.into()),
    }
}
