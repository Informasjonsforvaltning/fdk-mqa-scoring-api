use actix_web::{get, Responder};

use crate::metrics::get_metrics;

#[get("/metrics")]
pub async fn metrics() -> impl Responder {
    match get_metrics() {
        Ok(metrics) => metrics,
        Err(e) => {
            tracing::error!(error = e.to_string(), "unable to gather metrics");
            "".to_string()
        }
    }
}
