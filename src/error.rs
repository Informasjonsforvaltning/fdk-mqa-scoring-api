use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::database;

#[derive(Error, Debug)]
pub enum Error {
    #[error("dataset with FDK ID '{0}' does not exist")]
    NotFound(Uuid),
    #[error("invalid FDK ID: '{0}'")]
    InvalidID(String),
    #[error(transparent)]
    InvalidUri(#[from] http::uri::InvalidUri),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not Implemented: {0}")]
    NotImplemented(String),
    #[error(
        "duplicate dataset_uri: assessment with same URI but different id already stored"
    )]
    DuplicateDatasetUri {
        dataset_uri: String,
        assessment_id: String,
    },
    #[error(transparent)]
    DatabaseError(#[from] database::DatabaseError),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    BlockingError(#[from] actix_web::error::BlockingError),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        use Error::*;
        match self {
            NotFound(_) => HttpResponse::NotFound().json(ErrorReply::message(self)),
            InvalidID(_) => HttpResponse::BadRequest().json(ErrorReply::error(self)),
            InvalidUri(_) => HttpResponse::BadRequest().json(ErrorReply::error(self)),
            Unauthorized(_) => HttpResponse::Unauthorized().json(ErrorReply::error(self)),
            NotImplemented(_) => HttpResponse::NotImplemented().json(ErrorReply::message(self)),
            DuplicateDatasetUri {
                dataset_uri,
                assessment_id,
            } => {
                tracing::error!(
                    dataset_uri = %dataset_uri,
                    assessment_id = %assessment_id,
                    "duplicate dataset_uri: assessment with same URI but different id already stored"
                );
                HttpResponse::Conflict().json(ErrorReply::message(self))
            }
            SerdeJsonError(ref e) => {
                tracing::error!(
                    error = format!("{:?}", e).as_str(),
                    error_message = e.to_string().as_str(),
                    error_type = "SerdeJsonError",
                    "JSON parsing error occurred"
                );
                HttpResponse::BadRequest().json(ErrorReply::error(self))
            }
            _ => {
                tracing::error!(
                    error = format!("{:?}", self).as_str(),
                    error_type = format!("{:?}", std::mem::discriminant(self)).as_str(),
                    "error occurred when processing request"
                );
                HttpResponse::InternalServerError().json(ErrorReply::error(self))
            }
        }
    }
}

#[derive(Default, Serialize)]
pub struct ErrorReply {
    message: Option<String>,
    error: Option<String>,
}

impl ErrorReply {
    pub fn message<S: ToString>(message: S) -> Self {
        ErrorReply {
            message: Some(message.to_string()),
            ..Default::default()
        }
    }
    fn error<S: ToString>(error: S) -> Self {
        ErrorReply {
            error: Some(error.to_string()),
            ..Default::default()
        }
    }
}
