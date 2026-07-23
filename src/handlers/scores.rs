use actix_web::{post, web, HttpResponse, Responder};

use crate::{
    database::{DatabaseError, PgPool},
    error::Error,
    http_utils::{parse_json_body, validate_dataset_uris},
    models::{DatasetsRequest, DatasetsScores},
};

#[post("/api/scores")]
pub async fn scores(pool: web::Data<PgPool>, body: web::Bytes) -> Result<impl Responder, Error> {
    let data: DatasetsRequest = parse_json_body(&body, "/api/scores")?;
    validate_dataset_uris(&data.datasets)?;

    let result: Result<DatasetsScores, DatabaseError> = web::block(move || {
        let mut conn = pool.get()?;

        Ok(crate::models::DatasetsScores {
            scores: conn.json_scores(&data.datasets)?,
            aggregations: conn.dimension_aggregates(&data.datasets)?,
        })
    })
    .await
    .map_err(|e| Error::BlockingError(e.into()))?;

    match result {
        Ok(scores) => Ok(HttpResponse::Ok()
            .content_type(mime::APPLICATION_JSON)
            .message_body(serde_json::to_string(&scores)?)),
        Err(e) => Err(e.into()),
    }
}
