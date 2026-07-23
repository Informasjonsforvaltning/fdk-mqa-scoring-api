use actix_web::{
    get,
    http::header,
    post, web, HttpRequest, HttpResponse, Responder,
};
use uuid::Uuid;

use crate::{
    auth::validate_api_key,
    database::{DatabaseError, PgPool},
    db_models::{DatasetAssessment, Dimension},
    error::{Error, ErrorReply},
    http_utils::{
        graph_content_type, parse_json_body, validate_dataset_uris, wants_json_ld,
    },
    models::{DatasetsRequest, SuccessResponse},
};

fn parse_uuid(uuid: String) -> Result<Uuid, Error> {
    Uuid::parse_str(uuid.as_ref()).map_err(|_| Error::InvalidID(uuid))
}

#[get("/api/assessments/{id}")]
pub async fn assessment_graph(
    accept: web::Header<header::Accept>,
    id: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder, Error> {
    let uuid = parse_uuid(id.into_inner())?;
    let accept_json_ld = wants_json_ld(&accept);

    let result = web::block(move || {
        let mut conn = pool.get()?;
        if accept_json_ld {
            conn.jsonld_assessment(uuid)?.ok_or(Error::NotFound(uuid))
        } else {
            conn.turtle_assessment(uuid)?.ok_or(Error::NotFound(uuid))
        }
    })
    .await
    .map_err(|e| Error::BlockingError(e.into()))?;

    match result {
        Ok(graph) => Ok(HttpResponse::Ok()
            .content_type(graph_content_type(accept_json_ld))
            .message_body(graph)),
        Err(e) => Err(e.into()),
    }
}

#[post("/api/assessments/{id}")]
pub async fn update_assessment(
    request: HttpRequest,
    body: web::Bytes,
    id: web::Path<String>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder, Error> {
    validate_api_key(request)?;
    let uuid = parse_uuid(id.into_inner())?;
    let update: crate::models::ScorePostRequest = parse_json_body(&body, "/api/assessments/{id}")?;
    let dataset_uri = update.scores.as_ref().dataset.id.clone();
    let dataset_uri_for_log = dataset_uri.clone();
    let assessment_id_for_log = uuid.to_string();

    let result: Result<(), DatabaseError> = web::block(move || {
        let mut conn = pool.get()?;

        let assessment = DatasetAssessment {
            id: uuid.to_string(),
            dataset_uri: dataset_uri.clone(),
            turtle_assessment: update.turtle_assessment.clone(),
            jsonld_assessment: update.jsonld_assessment.clone(),
            json_score: serde_json::to_string(&update.scores)?,
        };

        conn.drop_dataset_dimensions(&dataset_uri)?;
        conn.store_dataset_assessment(assessment)?;

        for dimension in &update.scores.dataset.dimensions {
            conn.store_dimension(Dimension {
                dataset_uri: dataset_uri.clone(),
                id: dimension.id.clone(),
                score: dimension.score as i32,
                max_score: dimension.max_score as i32,
            })?;
        }

        Ok(())
    })
    .await
    .map_err(|e| Error::BlockingError(e.into()))?;

    match result {
        Ok(_) => {
            let response = SuccessResponse::new(true);
            Ok(HttpResponse::Accepted()
                .content_type(mime::APPLICATION_JSON)
                .message_body(serde_json::to_string(&response)?))
        }
        Err(e) if e.is_duplicate_dataset_uri() => {
            tracing::error!(
                dataset_uri = %dataset_uri_for_log,
                assessment_id = %assessment_id_for_log,
                "duplicate dataset_uri: assessment with same URI but different id already stored"
            );
            Ok(HttpResponse::Conflict()
                .content_type(mime::APPLICATION_JSON)
                .message_body(serde_json::to_string(&ErrorReply::message(
                    "duplicate dataset_uri: assessment with same URI but different id already stored",
                ))?))
        }
        Err(e) => Err(e.into()),
    }
}

#[post("/api/assessments")]
pub async fn assessments(body: web::Bytes) -> Result<HttpResponse, Error> {
    let data: DatasetsRequest = parse_json_body(&body, "/api/assessments")?;
    validate_dataset_uris(&data.datasets)?;

    Err(Error::NotImplemented(
        "batch assessment retrieval is not implemented".to_string(),
    ))
}
